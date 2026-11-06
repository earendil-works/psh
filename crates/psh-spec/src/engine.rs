//! Engines run one case invocation: a shell binary as a subprocess (bash for the
//! goldens, psh for invocation tests), or psh in-process.

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

/// psh in-process: a fresh shell per case on its own engine thread (with the stack the
/// engine asks for), over the std host, with the case's cwd and environment. stdout and
/// stderr are OS pipes read by helper threads, so externals the case starts write to
/// them directly, as they would under a real shell.
pub struct InProcess {
    binary: PathBuf,
}

impl InProcess {
    pub fn new(binary: PathBuf) -> InProcess {
        InProcess { binary }
    }
}

fn file_from_writer(w: std::io::PipeWriter) -> std::fs::File {
    #[cfg(unix)]
    {
        std::fs::File::from(std::os::fd::OwnedFd::from(w))
    }
    #[cfg(windows)]
    {
        std::fs::File::from(std::os::windows::io::OwnedHandle::from(w))
    }
}

fn file_from_reader(r: std::io::PipeReader) -> std::fs::File {
    #[cfg(unix)]
    {
        std::fs::File::from(std::os::fd::OwnedFd::from(r))
    }
    #[cfg(windows)]
    {
        std::fs::File::from(std::os::windows::io::OwnedHandle::from(r))
    }
}

fn os_bytes(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

impl Engine for InProcess {
    fn name(&self) -> &str {
        "psh"
    }

    fn binary(&self) -> &Path {
        &self.binary
    }

    fn run(&self, req: &RunRequest) -> std::io::Result<RunOutput> {
        use psh_engine::invocation::{Invocation, parse_args};
        use psh_engine::std_host::{StdFile, StdHost, drive_until, engine_thread};
        use psh_engine::{HostFile, Script, Shell, ShellConfig};
        use std::rc::Rc;

        let fail = |msg: String| {
            Ok(RunOutput {
                stdout: Vec::new(),
                stderr: format!("psh: {msg}\n").into_bytes(),
                status: Status::Exit(2),
            })
        };
        // A child of an in-process shell is a child of the runner: a case that signals
        // `$PPID` would hit the runner. Those run in the psh binary instead.
        let mentions = |b: &[u8]| b.windows(4).any(|w| w == b"PPID");
        if req.args.iter().any(|a| mentions(a.as_bytes())) || req.stdin.is_some_and(mentions) {
            return External::new(self.binary.clone()).run(req);
        }
        let args: Vec<Vec<u8>> = req.args.iter().map(|a| os_bytes(a)).collect();
        let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
        // The script, and the shell's stdin (consumed when the script came from it).
        let (script, stdin_data) = match parse_args(&args) {
            Err(msg) => return fail(msg),
            Ok(Invocation::Help) => return fail("--help".into()),
            Ok(Invocation::Command { source, arg0, args }) => (
                Script {
                    source: text(&source),
                    arg0: arg0.unwrap_or_else(|| b"psh".to_vec()),
                    args,
                    dash_c: true,
                },
                req.stdin.map(<[u8]>::to_vec),
            ),
            Ok(Invocation::File { path, args }) => {
                let full = req.cwd.join(text(&path));
                match std::fs::read(&full) {
                    Ok(src) => (
                        Script {
                            source: text(&src),
                            arg0: path,
                            args,
                            dash_c: false,
                        },
                        req.stdin.map(<[u8]>::to_vec),
                    ),
                    Err(_) => {
                        return Ok(RunOutput {
                            stdout: Vec::new(),
                            stderr: format!("psh: {}: No such file or directory\n", text(&path))
                                .into_bytes(),
                            status: Status::Exit(127),
                        });
                    }
                }
            }
            Ok(Invocation::Stdin { args }) => (
                Script {
                    source: text(req.stdin.unwrap_or_default()),
                    arg0: b"psh".to_vec(),
                    args,
                    dash_c: false,
                },
                None,
            ),
        };

        let (out_r, out_w) = std::io::pipe()?;
        let (err_r, err_w) = std::io::pipe()?;
        let stdin_file = match &stdin_data {
            Some(_) => None,
            None => Some(std::fs::File::open(if cfg!(windows) {
                "NUL"
            } else {
                "/dev/null"
            })?),
        };
        let (in_r, in_w) = std::io::pipe()?;
        let stdin_thread = match stdin_data {
            Some(data) => {
                let mut w = in_w;
                Some(std::thread::spawn(move || {
                    let _ = w.write_all(&data);
                }))
            }
            None => {
                drop(in_w);
                None
            }
        };
        let stdin_file = stdin_file.unwrap_or_else(|| file_from_reader(in_r));
        let out_thread = std::thread::spawn(move || read_limited(out_r));
        let err_thread = std::thread::spawn(move || read_limited(err_r));

        let env: Vec<(Vec<u8>, Vec<u8>)> = req
            .env
            .iter()
            .map(|(k, v)| (os_bytes(k), os_bytes(v)))
            .collect();
        let cwd = os_bytes(&req.cwd.to_string_lossy());
        let deadline = Instant::now() + req.timeout;
        let out_file = file_from_writer(out_w);
        let err_file = file_from_writer(err_w);
        let engine = engine_thread(move || {
            // `$$` must not be the runner: cases `kill $$`. No pid is this large.
            let mut cfg = ShellConfig::new(Rc::new(StdHost::with_pid(99_999_999)));
            cfg.env = env;
            cfg.cwd = cwd;
            cfg.stdin = Some(Rc::new(StdFile::new(stdin_file)) as Rc<dyn HostFile>);
            cfg.stdout = Some(Rc::new(StdFile::new(out_file)) as Rc<dyn HostFile>);
            cfg.stderr = Some(Rc::new(StdFile::new(err_file)) as Rc<dyn HostFile>);
            let mut shell = Shell::new(cfg);
            shell.start(script);
            drive_until(&mut shell, Some(deadline))
        })?;
        let result = engine
            .join()
            .map_err(|_| std::io::Error::other("the engine thread panicked"))?;
        // Output is complete when both pipes close; a process the case left behind can
        // hold them: wait until the deadline.
        while !(out_thread.is_finished() && err_thread.is_finished()) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        let complete = out_thread.is_finished() && err_thread.is_finished();
        let (stdout, stderr) = if complete {
            (
                out_thread.join().unwrap_or_default(),
                err_thread.join().unwrap_or_default(),
            )
        } else {
            (Vec::new(), Vec::new())
        };
        if let Some(t) = stdin_thread
            && t.is_finished()
        {
            let _ = t.join();
        }
        let status = match result {
            Some(s) if complete => Status::Exit(s),
            _ => Status::Timeout,
        };
        Ok(RunOutput {
            stdout,
            stderr,
            status,
        })
    }
}
