//! Engines run one case invocation: a shell binary as a subprocess (bash for the
//! goldens, psh for invocation tests), or psh in-process (a stub until the engine runs
//! commands).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Exit(i32),
    Signal(i32),
    Timeout,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Exit(n) => write!(f, "{n}"),
            Status::Signal(n) => write!(f, "signal {n}"),
            Status::Timeout => f.write_str("timeout"),
        }
    }
}

impl Status {
    pub fn parse(s: &str) -> Option<Status> {
        if s == "timeout" {
            Some(Status::Timeout)
        } else if let Some(n) = s.strip_prefix("signal ") {
            n.parse().ok().map(Status::Signal)
        } else {
            s.parse().ok().map(Status::Exit)
        }
    }
}

pub struct RunRequest<'a> {
    pub args: &'a [String],
    pub stdin: Option<&'a [u8]>,
    /// The complete environment (nothing is inherited).
    pub env: &'a [(String, String)],
    pub cwd: &'a Path,
    pub timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct RunOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: Status,
}

pub trait Engine: Sync {
    /// The shell's name as it appears in its messages (`bash`, `psh`).
    fn name(&self) -> &str;
    /// A path to a binary of this shell, for `$SH` and cases that start a nested shell.
    fn binary(&self) -> &Path;
    fn run(&self, req: &RunRequest) -> std::io::Result<RunOutput>;
}

/// At most this much of stdout and stderr is kept; the rest is read and dropped.
pub const OUTPUT_LIMIT: usize = 1 << 20;

/// A shell binary run as a subprocess, in its own process group (Unix), with the
/// shell's base name as `argv[0]`. On timeout, and after it exits, the whole group is
/// killed so no background process of a case outlives it.
pub struct External {
    path: PathBuf,
    name: String,
}

impl External {
    pub fn new(path: PathBuf) -> External {
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "sh".into());
        External { path, name }
    }
}

fn read_limited(mut r: impl Read) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        match r.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let keep = n.min(OUTPUT_LIMIT.saturating_sub(out.len()));
                out.extend_from_slice(&buf[..keep]);
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
    out
}

#[cfg(unix)]
fn kill_group(pid: u32) {
    // std has no killpg; kill(1) can signal a process group (`-PGID`).
    let _guard = crate::sandbox::SPAWN_LOCK
        .read()
        .unwrap_or_else(|e| e.into_inner());
    let _ = Command::new("kill")
        .args(["-KILL", "--", &format!("-{pid}")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

impl Engine for External {
    fn name(&self) -> &str {
        &self.name
    }

    fn binary(&self) -> &Path {
        &self.path
    }

    fn run(&self, req: &RunRequest) -> std::io::Result<RunOutput> {
        let mut cmd = Command::new(&self.path);
        cmd.args(req.args)
            .env_clear()
            .envs(req.env.iter().map(|(k, v)| (k, v)))
            .current_dir(req.cwd)
            .stdin(if req.stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.arg0(&self.name).process_group(0);
        }
        let mut child = {
            let _guard = crate::sandbox::SPAWN_LOCK
                .read()
                .unwrap_or_else(|e| e.into_inner());
            cmd.spawn()?
        };
        let deadline = Instant::now() + req.timeout;
        let stdin_thread = match (child.stdin.take(), req.stdin) {
            (Some(mut pipe), Some(data)) => {
                let data = data.to_vec();
                // A shell may exit without reading all of its input: ignore EPIPE.
                Some(std::thread::spawn(move || {
                    let _ = pipe.write_all(&data);
                }))
            }
            _ => None,
        };
        let out = child.stdout.take().unwrap();
        let err = child.stderr.take().unwrap();
        let out_thread = std::thread::spawn(move || read_limited(out));
        let err_thread = std::thread::spawn(move || read_limited(err));

        let mut timed_out = false;
        let exit = loop {
            if let Some(status) = child.try_wait()? {
                break Some(status);
            }
            if Instant::now() >= deadline {
                timed_out = true;
                break None;
            }
            std::thread::sleep(Duration::from_millis(2));
        };
        // Output is complete when both pipes close, which background processes of the
        // case can delay: wait for that until the deadline, then kill the group.
        while !(out_thread.is_finished() && err_thread.is_finished()) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        if !(out_thread.is_finished() && err_thread.is_finished()) {
            timed_out = true;
        }
        #[cfg(unix)]
        kill_group(child.id());
        let _ = child.kill();
        let exit = match exit {
            Some(status) => status,
            None => child.wait()?,
        };
        let stdout = out_thread.join().unwrap_or_default();
        let stderr = err_thread.join().unwrap_or_default();
        if let Some(t) = stdin_thread {
            let _ = t.join();
        }
        let status = if timed_out {
            Status::Timeout
        } else if let Some(code) = exit.code() {
            Status::Exit(code)
        } else {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                Status::Signal(exit.signal().unwrap_or(0))
            }
            #[cfg(not(unix))]
            {
                Status::Exit(-1)
            }
        };
        Ok(RunOutput {
            stdout,
            stderr,
            status,
        })
    }
}

/// psh in-process. A stub until the engine runs commands: every case fails the way the
/// `psh` binary does today.
pub struct InProcess {
    binary: PathBuf,
}

impl InProcess {
    pub fn new(binary: PathBuf) -> InProcess {
        InProcess { binary }
    }
}

impl Engine for InProcess {
    fn name(&self) -> &str {
        "psh"
    }

    fn binary(&self) -> &Path {
        &self.binary
    }

    fn run(&self, _req: &RunRequest) -> std::io::Result<RunOutput> {
        Ok(RunOutput {
            stdout: Vec::new(),
            stderr: b"psh: not implemented\n".to_vec(),
            status: Status::Exit(2),
        })
    }
}
