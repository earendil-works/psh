//! The wasm engine (`psh-spec --wasm`): psh.wasm under node through js/psh.mjs's MemHost
//! (0011, D1113). Each worker thread takes a driver process (`js/test/spec-driver.mjs`)
//! from a pool, sends it the case, and reads the answer; the driver mounts the case's
//! directories from disk and writes their changes back, so the snapshot works as for the
//! other engines.
//!
//! Request frame (u32 little-endian length, then): source, `$0`, the arguments (count,
//! each), dash_c (u32), has_stdin (u32), stdin, the environment (count, name, value),
//! cwd, HOME, REPO_ROOT, the timeout in milliseconds (u32). Strings are a u32 length and
//! the bytes. Answer frame: timed out (u32), status (u32), stdout, stderr, and the
//! command names the shell looked for and did not find, one per line.

use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

use psh_engine::invocation::{Invocation, parse_args};

use crate::engine::{Engine, RunOutput, RunRequest, Status};

struct Driver {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

pub struct WasmEngine {
    node: PathBuf,
    driver: PathBuf,
    wasm: PathBuf,
    /// For `$SH`: the native binary (nested shells are not available in wasm anyway).
    binary: PathBuf,
    pool: Mutex<Vec<Driver>>,
}

impl WasmEngine {
    pub fn new(root: &Path, binary: PathBuf) -> Result<WasmEngine, String> {
        let wasm = root.join("target/wasm/psh.wasm");
        if !wasm.is_file() {
            return Err(format!(
                "{} is missing; run scripts/build-wasm",
                wasm.display()
            ));
        }
        Ok(WasmEngine {
            node: PathBuf::from("node"),
            driver: root.join("js/test/spec-driver.mjs"),
            wasm,
            binary,
            pool: Mutex::new(Vec::new()),
        })
    }

    fn take_driver(&self) -> std::io::Result<Driver> {
        if let Some(d) = self.pool.lock().unwrap_or_else(|e| e.into_inner()).pop() {
            return Ok(d);
        }
        let mut child = {
            let _guard = crate::sandbox::SPAWN_LOCK
                .read()
                .unwrap_or_else(|e| e.into_inner());
            Command::new(&self.node)
                .arg(&self.driver)
                .arg(&self.wasm)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()?
        };
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Ok(Driver {
            child,
            stdin,
            stdout,
        })
    }
}

impl Drop for WasmEngine {
    fn drop(&mut self) {
        for mut d in self
            .pool
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .drain(..)
        {
            drop(d.stdin);
            let _ = d.child.wait();
        }
    }
}

struct Frame(Vec<u8>);

impl Frame {
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn bytes(&mut self, b: &[u8]) {
        self.u32(b.len() as u32);
        self.0.extend_from_slice(b);
    }
}

fn read_u32(r: &mut impl Read) -> std::io::Result<u32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

fn read_bytes(r: &mut &[u8]) -> std::io::Result<Vec<u8>> {
    let n = read_u32(r)? as usize;
    if r.len() < n {
        return Err(std::io::Error::other("short answer from the wasm driver"));
    }
    let (head, rest) = r.split_at(n);
    *r = rest;
    Ok(head.to_vec())
}

impl Engine for WasmEngine {
    fn name(&self) -> &str {
        "psh"
    }

    fn binary(&self) -> &Path {
        &self.binary
    }

    fn run(&self, req: &RunRequest) -> std::io::Result<RunOutput> {
        let args: Vec<Vec<u8>> = req.args.iter().map(|a| a.as_bytes().to_vec()).collect();
        let fail = |msg: String| {
            Ok(RunOutput {
                stdout: Vec::new(),
                stderr: format!("psh: {msg}\n").into_bytes(),
                status: Status::Exit(2),
                notes: Vec::new(),
            })
        };
        let (source, arg0, script_args, dash_c, stdin) = match parse_args(&args) {
            Err(msg) => return fail(msg),
            Ok(Invocation::Help) => return fail("--help".into()),
            Ok(Invocation::Command {
                source, arg0, args, ..
            }) => (
                source,
                arg0.unwrap_or_else(|| b"psh".to_vec()),
                args,
                true,
                req.stdin.map(<[u8]>::to_vec),
            ),
            Ok(Invocation::File { path, args, .. }) => {
                let full = req.cwd.join(String::from_utf8_lossy(&path).as_ref());
                match std::fs::read(&full) {
                    Ok(src) => (src, path, args, false, req.stdin.map(<[u8]>::to_vec)),
                    Err(_) => {
                        return Ok(RunOutput {
                            stdout: Vec::new(),
                            stderr: format!(
                                "psh: {}: No such file or directory\n",
                                String::from_utf8_lossy(&path)
                            )
                            .into_bytes(),
                            status: Status::Exit(127),
                            notes: Vec::new(),
                        });
                    }
                }
            }
            Ok(Invocation::Stdin { args, .. }) => (
                req.stdin.unwrap_or_default().to_vec(),
                b"psh".to_vec(),
                args,
                false,
                None,
            ),
        };
        let env_value = |name: &str| {
            req.env
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };
        let mut f = Frame(Vec::new());
        f.bytes(&source);
        f.bytes(&arg0);
        f.u32(script_args.len() as u32);
        for a in &script_args {
            f.bytes(a);
        }
        f.u32(u32::from(dash_c));
        f.u32(u32::from(stdin.is_some()));
        f.bytes(stdin.as_deref().unwrap_or_default());
        f.u32(req.env.len() as u32);
        for (k, v) in req.env {
            f.bytes(k.as_bytes());
            f.bytes(v.as_bytes());
        }
        f.bytes(req.cwd.to_string_lossy().as_bytes());
        f.bytes(env_value("HOME").as_bytes());
        f.bytes(env_value("REPO_ROOT").as_bytes());
        f.u32(req.timeout.as_millis().min(u32::MAX as u128) as u32);

        let mut d = self.take_driver()?;
        let answer = (|| -> std::io::Result<Vec<u8>> {
            d.stdin.write_all(&(f.0.len() as u32).to_le_bytes())?;
            d.stdin.write_all(&f.0)?;
            d.stdin.flush()?;
            let n = read_u32(&mut d.stdout)? as usize;
            let mut buf = vec![0; n];
            d.stdout.read_exact(&mut buf)?;
            Ok(buf)
        })();
        let buf = match answer {
            Ok(b) => {
                self.pool.lock().unwrap_or_else(|e| e.into_inner()).push(d);
                b
            }
            Err(e) => {
                let _ = d.child.kill();
                let _ = d.child.wait();
                return Err(e);
            }
        };
        let mut r: &[u8] = &buf;
        let timed_out = read_u32(&mut r)? != 0;
        let status = read_u32(&mut r)? as i32;
        let stdout = read_bytes(&mut r)?;
        let stderr = read_bytes(&mut r)?;
        let missing = read_bytes(&mut r)?;
        let notes = String::from_utf8_lossy(&missing)
            .lines()
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect();
        Ok(RunOutput {
            stdout,
            stderr,
            status: if timed_out {
                Status::Timeout
            } else {
                Status::Exit(status)
            },
            notes,
        })
    }
}
