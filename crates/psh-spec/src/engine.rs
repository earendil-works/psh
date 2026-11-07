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
    /// Engine remarks about the run: the wasm engine lists the commands the case looked
    /// for and did not find (it needed external programs).
    pub notes: Vec<String>,
    /// Processes the case left running that the runner had to kill (0030): the case
    /// fails with them.
    pub leaked: Vec<String>,
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
        // The case's group must empty once killed (0030); a member that outlives KILL
        // (a process stuck in the kernel) is a leak.
        #[cfg(unix)]
        let leaked = {
            let pgid = child.id() as i32;
            let until = Instant::now() + Duration::from_secs(2);
            let alive = || match psh_os::unix::kill(-pgid, 0) {
                Ok(()) => true,
                Err(e) => e.kind() == std::io::ErrorKind::PermissionDenied,
            };
            // The leader is reaped below; the others are reaped by init.
            let _ = child.try_wait();
            while alive() && Instant::now() < until {
                std::thread::sleep(Duration::from_millis(5));
            }
            if alive() {
                vec![format!("process group {pgid}")]
            } else {
                Vec::new()
            }
        };
        #[cfg(not(unix))]
        let leaked = Vec::new();
        let exit = match exit {
            Some(status) => status,
            None => child.wait()?,
        };
        // A process outside the group (psh puts each external in a group of its own,
        // and releases background jobs at exit) can still hold the pipes: then give up
        // on the output rather than wait forever.
        let grace = Instant::now() + Duration::from_secs(2);
        while !(out_thread.is_finished() && err_thread.is_finished()) && Instant::now() < grace {
            std::thread::sleep(Duration::from_millis(2));
        }
        let (stdout, stderr) = if out_thread.is_finished() && err_thread.is_finished() {
            (
                out_thread.join().unwrap_or_default(),
                err_thread.join().unwrap_or_default(),
            )
        } else {
            timed_out = true;
            (Vec::new(), Vec::new())
        };
        if let Some(t) = stdin_thread
            && t.is_finished()
        {
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
            notes: Vec::new(),
            leaked,
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

/// Runs after a case: the processes it left that had to be killed.
type LeakCheck = Box<dyn FnOnce() -> Vec<String> + Send>;

/// Turns an OS file into a file of the host.
type WrapFile = fn(std::fs::File) -> std::rc::Rc<dyn psh_engine::HostFile>;

/// The OS host for an in-process shell whose `$$` is `pid`, how it wraps files, and the
/// leak check to run once the shell and the host are gone.
#[cfg(unix)]
fn os_host(pid: u32) -> (std::rc::Rc<dyn psh_engine::Host>, WrapFile, LeakCheck) {
    use psh_os::unix::{UnixConfig, UnixFile, UnixHost};
    let host = UnixHost::new(UnixConfig {
        pid: Some(pid),
        // Nothing a case starts may outlive it, `nohup` jobs included.
        detached_survive: false,
        ..UnixConfig::default()
    });
    let killer = host.killer();
    // Dropping the host killed what the run left (TERM, KILL after the grace period):
    // whatever is still alive a little after that is a leak (0030).
    let check: LeakCheck = Box::new(move || {
        let deadline =
            Instant::now() + psh_os::unix::UnixConfig::DEFAULT_GRACE + Duration::from_secs(1);
        loop {
            let left = killer.leftovers();
            if left.is_empty() {
                return Vec::new();
            }
            if Instant::now() >= deadline {
                for g in &left {
                    let _ = psh_os::unix::kill(-g, 9);
                }
                return left.iter().map(|g| format!("process group {g}")).collect();
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    });
    (
        std::rc::Rc::new(host),
        |f| std::rc::Rc::new(UnixFile::new(f)),
        check,
    )
}

/// The OS host for an in-process shell whose `$$` is `pid`, and how it wraps files.
#[cfg(windows)]
fn os_host(pid: u32) -> (std::rc::Rc<dyn psh_engine::Host>, WrapFile, LeakCheck) {
    use psh_os::windows::{WinFile, WindowsConfig, WindowsHost};
    let host = WindowsHost::new(WindowsConfig {
        pid: Some(pid),
        // Nothing a case starts may outlive it, `nohup` jobs included.
        detached_survive: false,
        ..WindowsConfig::default()
    });
    let killer = host.killer();
    // Dropping the host killed what the run left (a ctrl-break, then the job after the
    // grace period): a job that still has a process a little after that is a leak.
    let check: LeakCheck = Box::new(move || {
        let deadline =
            Instant::now() + psh_os::windows::WindowsConfig::DEFAULT_GRACE + Duration::from_secs(1);
        loop {
            let left = killer.leftovers();
            if left.is_empty() {
                return Vec::new();
            }
            if Instant::now() >= deadline {
                killer.kill_all();
                psh_os::windows::settle(Duration::from_secs(5));
                return left.iter().map(|p| format!("the job of pid {p}")).collect();
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    });
    (
        std::rc::Rc::new(host),
        |f| std::rc::Rc::new(WinFile::new(std::os::windows::io::OwnedHandle::from(f))),
        check,
    )
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

/// The case's directory as the engine writes paths: on Windows `/`-separated and
/// without the `\\?\` prefix that canonical paths carry (`C:/Users/...`).
fn engine_cwd(p: &Path) -> Vec<u8> {
    let s = p.to_string_lossy();
    if cfg!(windows) {
        let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
        s.replace('\\', "/").into_bytes()
    } else {
        s.into_owned().into_bytes()
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
        use psh_engine::std_host::{drive_until, engine_thread};
        use psh_engine::{Script, Shell, ShellConfig};

        let fail = |msg: String| {
            Ok(RunOutput {
                stdout: Vec::new(),
                stderr: format!("psh: {msg}\n").into_bytes(),
                status: Status::Exit(2),
                notes: Vec::new(),
                leaked: Vec::new(),
            })
        };
        // A child of an in-process shell is a child of the runner, in its process
        // group: a case that signals `$PPID` or its group (`kill 0`) would hit the
        // runner. Cases that mention either run in the psh binary instead, in a
        // process group of their own.
        let mentions = |b: &[u8]| b.windows(4).any(|w| w == b"PPID" || w == b"kill");
        if req.args.iter().any(|a| mentions(a.as_bytes())) || req.stdin.is_some_and(mentions) {
            return External::new(self.binary.clone()).run(req);
        }
        let args: Vec<Vec<u8>> = req.args.iter().map(|a| os_bytes(a)).collect();
        let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
        // The script, and the shell's stdin (consumed when the script came from it).
        let (script, stdin_data) = match parse_args(&args) {
            Err(msg) => return fail(msg),
            Ok(Invocation::Help) => return fail("--help".into()),
            Ok(Invocation::Command {
                source,
                arg0,
                args,
                options,
            }) => (
                Script {
                    source,
                    arg0: arg0.unwrap_or_else(|| b"psh".to_vec()),
                    args,
                    dash_c: true,
                    options,
                },
                req.stdin.map(<[u8]>::to_vec),
            ),
            Ok(Invocation::File {
                path,
                args,
                options,
            }) => {
                let full = req.cwd.join(text(&path));
                match std::fs::read(&full) {
                    Ok(src) => (
                        Script {
                            source: src,
                            arg0: path,
                            args,
                            dash_c: false,
                            options,
                        },
                        req.stdin.map(<[u8]>::to_vec),
                    ),
                    Err(_) => {
                        return Ok(RunOutput {
                            stdout: Vec::new(),
                            stderr: format!("psh: {}: No such file or directory\n", text(&path))
                                .into_bytes(),
                            status: Status::Exit(127),
                            notes: Vec::new(),
                            leaked: Vec::new(),
                        });
                    }
                }
            }
            Ok(Invocation::Stdin { args, options }) => (
                Script {
                    source: req.stdin.unwrap_or_default().to_vec(),
                    arg0: b"psh".to_vec(),
                    args,
                    dash_c: false,
                    options,
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

        let mut env: Vec<(Vec<u8>, Vec<u8>)> = req
            .env
            .iter()
            .map(|(k, v)| (os_bytes(k), os_bytes(v)))
            .collect();
        // As the psh binary does (D1192).
        let force = env
            .iter()
            .position(|(k, _)| k == psh_utils::FORCE_VAR.as_bytes())
            .map(|i| psh_utils::force_names(&env.remove(i).1))
            .unwrap_or_default();
        let cwd = engine_cwd(req.cwd);
        let deadline = Instant::now() + req.timeout;
        let out_file = file_from_writer(out_w);
        let err_file = file_from_writer(err_w);
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let _engine = engine_thread(move || {
            // `$$` must not be the runner: cases `kill $$`. No pid is this large.
            let (host, file, leak_check) = os_host(99_999_999);
            let noting = std::rc::Rc::new(Noting::new(host));
            let mut cfg = ShellConfig::new(noting.clone());
            cfg.utilities = psh_utils::policy(false);
            cfg.utilities.force_names = force;
            cfg.env = env;
            cfg.cwd = cwd;
            cfg.stdin = Some(file(stdin_file));
            cfg.stdout = Some(file(out_file));
            cfg.stderr = Some(file(err_file));
            // Background jobs of the case may finish for a moment after the script,
            // then they are killed: nothing may outlive the case.
            cfg.exit_grace = Some(Duration::from_millis(500));
            let mut shell = Shell::new(cfg);
            shell.start(script);
            let r = drive_until(&mut shell, Some(deadline));
            drop(shell);
            let _ = done_tx.send((r, leak_check(), noting.missing()));
        })?;
        // A shell stuck in the kernel (a v1 host limit) cannot be cancelled: give up
        // on it a little after the deadline and leave its thread behind.
        let wait = deadline.saturating_duration_since(Instant::now()) + Duration::from_secs(2);
        let (result, leaked, notes) = match done_rx.recv_timeout(wait) {
            Ok(r) => r,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => (None, Vec::new(), Vec::new()),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err(std::io::Error::other("the engine thread panicked"));
            }
        };
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
            notes,
            leaked,
        })
    }
}

/// The OS host with a record of the commands the shell looked for and never found: the
/// run's notes, which tell a platform ratchet that a case needed a program this system
/// lacks (D1914). A name counts once no lookup of it succeeded.
struct Noting {
    inner: std::rc::Rc<dyn psh_engine::Host>,
    looked: std::rc::Rc<std::cell::RefCell<Looked>>,
}

/// Names looked up, and those of them found.
type Looked = (Vec<Vec<u8>>, Vec<Vec<u8>>);

impl Noting {
    fn new(inner: std::rc::Rc<dyn psh_engine::Host>) -> Noting {
        Noting {
            inner,
            looked: Default::default(),
        }
    }

    /// The names looked up and never found, in the order first asked.
    fn missing(&self) -> Vec<String> {
        let l = self.looked.borrow();
        l.0.iter()
            .filter(|n| !l.1.contains(n))
            .map(|n| String::from_utf8_lossy(n).into_owned())
            .collect()
    }
}

impl psh_engine::Host for Noting {
    fn open(
        &self,
        path: &[u8],
        opts: psh_engine::host::OpenOptions,
    ) -> psh_engine::host::HostFuture<std::io::Result<std::rc::Rc<dyn psh_engine::HostFile>>> {
        self.inner.open(path, opts)
    }
    fn stat(
        &self,
        path: &[u8],
        follow: bool,
    ) -> psh_engine::host::HostFuture<std::io::Result<psh_engine::host::Metadata>> {
        self.inner.stat(path, follow)
    }
    fn access(
        &self,
        path: &[u8],
        mode: psh_engine::host::Access,
    ) -> psh_engine::host::HostFuture<std::io::Result<bool>> {
        self.inner.access(path, mode)
    }
    fn read_dir(
        &self,
        path: &[u8],
    ) -> psh_engine::host::HostFuture<std::io::Result<Vec<psh_engine::host::DirEntry>>> {
        self.inner.read_dir(path)
    }
    fn canonicalize(&self, path: &[u8]) -> psh_engine::host::HostFuture<std::io::Result<Vec<u8>>> {
        self.inner.canonicalize(path)
    }
    fn resolve(
        &self,
        name: &[u8],
        path: &[u8],
        cwd: &[u8],
    ) -> psh_engine::host::HostFuture<Option<Vec<u8>>> {
        let fut = self.inner.resolve(name, path, cwd);
        let looked = self.looked.clone();
        let name = name.to_vec();
        Box::pin(async move {
            let r = fut.await;
            let mut l = looked.borrow_mut();
            if !l.0.contains(&name) {
                l.0.push(name.clone());
            }
            if r.is_some() && !l.1.contains(&name) {
                l.1.push(name);
            }
            r
        })
    }
    fn path_entries(&self, path: &[u8]) -> Vec<Vec<u8>> {
        self.inner.path_entries(path)
    }
    fn spawn(
        &self,
        spec: psh_engine::host::SpawnSpec,
    ) -> std::io::Result<psh_engine::host::Spawned> {
        self.inner.spawn(spec)
    }
    fn pipe(
        &self,
    ) -> std::io::Result<(
        std::rc::Rc<dyn psh_engine::HostFile>,
        std::rc::Rc<dyn psh_engine::HostFile>,
    )> {
        self.inner.pipe()
    }
    fn kill(&self, pid: i32, sig: psh_engine::host::Signal) -> std::io::Result<()> {
        self.inner.kill(pid, sig)
    }
    fn native_paths(&self) -> Option<psh_engine::host::NativePathFn> {
        self.inner.native_paths()
    }
    fn info(&self) -> psh_engine::host::HostInfo {
        self.inner.info()
    }
    fn umask(&self) -> u32 {
        self.inner.umask()
    }
    fn standard_path(&self) -> Vec<u8> {
        self.inner.standard_path()
    }
    fn stat_extra(
        &self,
        path: &[u8],
        follow: bool,
    ) -> psh_engine::host::HostFuture<
        std::io::Result<(psh_engine::host::Metadata, psh_engine::host::ExtraMetadata)>,
    > {
        self.inner.stat_extra(path, follow)
    }
    fn mkdir(&self, path: &[u8], mode: u32) -> psh_engine::host::HostFuture<std::io::Result<()>> {
        self.inner.mkdir(path, mode)
    }
    fn remove_file(&self, path: &[u8]) -> psh_engine::host::HostFuture<std::io::Result<()>> {
        self.inner.remove_file(path)
    }
    fn remove_dir(&self, path: &[u8]) -> psh_engine::host::HostFuture<std::io::Result<()>> {
        self.inner.remove_dir(path)
    }
    fn rename(&self, from: &[u8], to: &[u8]) -> psh_engine::host::HostFuture<std::io::Result<()>> {
        self.inner.rename(from, to)
    }
    fn symlink(
        &self,
        target: &[u8],
        link: &[u8],
    ) -> psh_engine::host::HostFuture<std::io::Result<()>> {
        self.inner.symlink(target, link)
    }
    fn hard_link(
        &self,
        existing: &[u8],
        link: &[u8],
    ) -> psh_engine::host::HostFuture<std::io::Result<()>> {
        self.inner.hard_link(existing, link)
    }
    fn read_link(&self, path: &[u8]) -> psh_engine::host::HostFuture<std::io::Result<Vec<u8>>> {
        self.inner.read_link(path)
    }
    fn set_mode(
        &self,
        path: &[u8],
        mode: u32,
    ) -> psh_engine::host::HostFuture<std::io::Result<()>> {
        self.inner.set_mode(path, mode)
    }
    fn set_times(
        &self,
        path: &[u8],
        atime_ns: Option<i64>,
        mtime_ns: Option<i64>,
        follow: bool,
    ) -> psh_engine::host::HostFuture<std::io::Result<()>> {
        self.inner.set_times(path, atime_ns, mtime_ns, follow)
    }
    fn user_name(&self, uid: u32) -> Option<Vec<u8>> {
        self.inner.user_name(uid)
    }
    fn group_name(&self, gid: u32) -> Option<Vec<u8>> {
        self.inner.group_name(gid)
    }
    fn utc_offset(&self, epoch_secs: i64) -> i32 {
        self.inner.utc_offset(epoch_secs)
    }
    fn home_dir(&self, user: &[u8]) -> Option<Vec<u8>> {
        self.inner.home_dir(user)
    }
    fn now(&self) -> Duration {
        self.inner.now()
    }
    fn sleep(&self, duration: Duration) -> psh_engine::host::HostFuture<()> {
        self.inner.sleep(duration)
    }
    fn pid(&self) -> u32 {
        self.inner.pid()
    }
}

#[cfg(test)]
mod noting_tests {
    /// `Noting` must forward every `Host` method: one left to its default would quietly
    /// change what the cases see (a file system change "not supported").
    #[test]
    fn noting_forwards_every_host_method() {
        let host = include_str!("../../psh-engine/src/host.rs");
        let trait_start = host.find("pub trait Host {").expect("trait Host");
        let body = &host[trait_start..];
        let body = &body[..body.find("\n}\n").expect("end of trait")];
        let ours = include_str!("engine.rs");
        let imp = &ours[ours.find("impl psh_engine::Host for Noting").expect("impl")..];
        for line in body.lines() {
            let t = line.trim_start();
            if let Some(rest) = t.strip_prefix("fn ") {
                let name = &rest[..rest.find(['(', '<']).unwrap_or(rest.len())];
                assert!(
                    imp.contains(&format!("fn {name}(")),
                    "Noting does not forward Host::{name}"
                );
            }
        }
    }
}
