//! A deterministic host for tests: an in-memory file system, fake external commands
//! written as futures, and captured stdio. No OS, no threads.
//!
//! In latency mode every operation is `Pending` once before it completes (the waker is
//! woken at once), which proves that no engine path expects a synchronous answer
//! (ARCH.md "Hosts", TESTING.md L6). Counters of live processes, open files and pipes
//! let tests check that a cancel leaks nothing.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::future::{Future, poll_fn};
use std::io;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use crate::host::{
    Access, ChildFd, DirEntry, ExitStatus, ExtProc, FileKind, Host, HostFile, HostFuture, Metadata,
    OpenOptions, Signal, SpawnSpec, Spawned,
};

/// What a fake command gets.
pub struct FakeProc {
    pub argv: Vec<Vec<u8>>,
    pub env: Vec<(Vec<u8>, Vec<u8>)>,
    pub cwd: Vec<u8>,
    pub stdin: Rc<dyn HostFile>,
    pub stdout: Rc<dyn HostFile>,
    pub stderr: Rc<dyn HostFile>,
    pub fs: Rc<MemFs>,
}

impl FakeProc {
    pub async fn read_all(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            match poll_fn(|cx| self.stdin.poll_read(cx, &mut buf)).await {
                Ok(0) | Err(_) => return out,
                Ok(n) => out.extend_from_slice(&buf[..n]),
            }
        }
    }

    /// Reads one line (with its newline); empty at end of input.
    pub async fn read_line(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut b = [0u8; 1];
        loop {
            match poll_fn(|cx| self.stdin.poll_read(cx, &mut b)).await {
                Ok(0) | Err(_) => return out,
                Ok(_) => {
                    out.push(b[0]);
                    if b[0] == b'\n' {
                        return out;
                    }
                }
            }
        }
    }

    pub async fn write(&self, data: &[u8]) -> io::Result<()> {
        write_all(&*self.stdout, data).await
    }

    pub async fn write_err(&self, data: &[u8]) -> io::Result<()> {
        write_all(&*self.stderr, data).await
    }
}

async fn write_all(f: &dyn HostFile, mut data: &[u8]) -> io::Result<()> {
    while !data.is_empty() {
        let n = poll_fn(|cx| f.poll_write(cx, data)).await?;
        data = &data[n..];
    }
    Ok(())
}

pub type FakeCommand = Rc<dyn Fn(FakeProc) -> Pin<Box<dyn Future<Output = i32>>>>;

/// Counts of live resources.
#[derive(Default)]
struct Counters {
    procs: Cell<usize>,
    /// Pids handed to fakes (`ExtProc::pid`).
    spawned: Cell<u32>,
    files: Cell<usize>,
    pipes: Cell<usize>,
}

/// Pending once, then ready: the latency mode's answer to every operation.
struct Delay {
    on: bool,
}

impl Delay {
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if self.on {
            self.on = false;
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

fn delayed<T: 'static>(latency: bool, value: T) -> HostFuture<T> {
    let mut d = Delay { on: latency };
    let mut v = Some(value);
    Box::pin(poll_fn(move |cx| match d.poll(cx) {
        Poll::Ready(()) => Poll::Ready(v.take().expect("polled after completion")),
        Poll::Pending => Poll::Pending,
    }))
}

/// Toggles between `Pending` and ready for every call when latency is on.
struct Toggle {
    latency: bool,
    next_pending: Cell<bool>,
}

impl Toggle {
    fn new(latency: bool) -> Toggle {
        Toggle {
            latency,
            next_pending: Cell::new(true),
        }
    }

    fn gate(&self, cx: &mut Context<'_>) -> bool {
        if !self.latency {
            return true;
        }
        if self.next_pending.get() {
            self.next_pending.set(false);
            cx.waker().wake_by_ref();
            false
        } else {
            self.next_pending.set(true);
            true
        }
    }
}

enum Node {
    File {
        data: Rc<RefCell<Vec<u8>>>,
        mode: u32,
        mtime: i64,
    },
    Dir,
}

/// An in-memory file system: absolute `/`-separated paths.
pub struct MemFs {
    nodes: RefCell<BTreeMap<Vec<u8>, Node>>,
    clock: Cell<i64>,
}

fn parent(path: &[u8]) -> &[u8] {
    match path.iter().rposition(|&b| b == b'/') {
        Some(0) => b"/",
        Some(i) => &path[..i],
        None => b"/",
    }
}

fn clean(path: &[u8]) -> Vec<u8> {
    crate::builtins::normalize(path)
}

impl MemFs {
    fn new() -> MemFs {
        let mut nodes = BTreeMap::new();
        nodes.insert(b"/".to_vec(), Node::Dir);
        MemFs {
            nodes: RefCell::new(nodes),
            clock: Cell::new(1),
        }
    }

    pub fn mkdir_all(&self, path: &str) {
        let p = clean(path.as_bytes());
        let mut cur = Vec::new();
        for c in p.split(|&b| b == b'/').filter(|c| !c.is_empty()) {
            cur.push(b'/');
            cur.extend_from_slice(c);
            self.nodes
                .borrow_mut()
                .entry(cur.clone())
                .or_insert(Node::Dir);
        }
    }

    pub fn write_file(&self, path: &str, data: &[u8]) {
        let p = clean(path.as_bytes());
        self.mkdir_all(&String::from_utf8_lossy(parent(&p)));
        let t = self.tick();
        self.nodes.borrow_mut().insert(
            p,
            Node::File {
                data: Rc::new(RefCell::new(data.to_vec())),
                mode: 0o644,
                mtime: t,
            },
        );
    }

    pub fn read_file(&self, path: &str) -> Option<Vec<u8>> {
        match self.nodes.borrow().get(&clean(path.as_bytes())) {
            Some(Node::File { data, .. }) => Some(data.borrow().clone()),
            _ => None,
        }
    }

    fn tick(&self) -> i64 {
        self.clock.set(self.clock.get() + 1);
        self.clock.get()
    }

    fn stat(&self, path: &[u8]) -> io::Result<Metadata> {
        let nodes = self.nodes.borrow();
        let node = nodes.get(&clean(path)).ok_or(io::ErrorKind::NotFound)?;
        let (kind, size, mode, mtime) = match node {
            Node::Dir => (FileKind::Dir, 0, 0o755, 0),
            Node::File { data, mode, mtime } => {
                (FileKind::File, data.borrow().len() as u64, *mode, *mtime)
            }
        };
        Ok(Metadata {
            kind,
            size,
            mode,
            mtime_ns: mtime,
            dev: 1,
            ino: 0,
            uid: 1000,
            gid: 1000,
        })
    }

    fn open(&self, path: &[u8], opts: OpenOptions) -> io::Result<Rc<RefCell<Vec<u8>>>> {
        let p = clean(path);
        let mut nodes = self.nodes.borrow_mut();
        match nodes.get(&p) {
            Some(Node::Dir) => {
                return Err(io::ErrorKind::IsADirectory.into());
            }
            Some(Node::File { data, .. }) => {
                if opts.create_new {
                    return Err(io::ErrorKind::AlreadyExists.into());
                }
                if opts.truncate {
                    data.borrow_mut().clear();
                }
                return Ok(data.clone());
            }
            None => {}
        }
        if !opts.create {
            return Err(io::ErrorKind::NotFound.into());
        }
        match nodes.get(parent(&p)) {
            Some(Node::Dir) => {}
            Some(_) => return Err(io::ErrorKind::NotADirectory.into()),
            None => return Err(io::ErrorKind::NotFound.into()),
        }
        let data = Rc::new(RefCell::new(Vec::new()));
        let t = self.clock.get() + 1;
        self.clock.set(t);
        nodes.insert(
            p,
            Node::File {
                data: data.clone(),
                mode: opts.mode.unwrap_or(0o644) & 0o7777,
                mtime: t,
            },
        );
        Ok(data)
    }

    fn read_dir(&self, path: &[u8]) -> io::Result<Vec<DirEntry>> {
        let p = clean(path);
        let nodes = self.nodes.borrow();
        match nodes.get(&p) {
            Some(Node::Dir) => {}
            Some(_) => return Err(io::ErrorKind::NotADirectory.into()),
            None => return Err(io::ErrorKind::NotFound.into()),
        }
        let mut prefix = p.clone();
        if prefix != b"/" {
            prefix.push(b'/');
        }
        Ok(nodes
            .keys()
            .filter(|k| {
                k.starts_with(&prefix)
                    && k.len() > prefix.len()
                    && !k[prefix.len()..].contains(&b'/')
            })
            .map(|k| DirEntry {
                name: k[prefix.len()..].to_vec(),
            })
            .collect())
    }
}

/// An open file of the memory fs, or a capture buffer.
struct MemFile {
    data: Rc<RefCell<Vec<u8>>>,
    pos: Cell<usize>,
    read: bool,
    write: bool,
    append: bool,
    toggle: Toggle,
    counters: Rc<Counters>,
}

impl Drop for MemFile {
    fn drop(&mut self) {
        self.counters.files.set(self.counters.files.get() - 1);
    }
}

impl HostFile for MemFile {
    fn poll_read(&self, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        if !self.toggle.gate(cx) {
            return Poll::Pending;
        }
        if !self.read {
            return Poll::Ready(Err(crate::fd::bad_fd()));
        }
        let data = self.data.borrow();
        let pos = self.pos.get().min(data.len());
        let n = buf.len().min(data.len() - pos);
        buf[..n].copy_from_slice(&data[pos..pos + n]);
        self.pos.set(pos + n);
        Poll::Ready(Ok(n))
    }

    fn poll_write(&self, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        if !self.toggle.gate(cx) {
            return Poll::Pending;
        }
        if !self.write {
            return Poll::Ready(Err(crate::fd::bad_fd()));
        }
        let mut data = self.data.borrow_mut();
        let pos = if self.append {
            data.len()
        } else {
            self.pos.get()
        };
        if data.len() < pos + buf.len() {
            data.resize(pos + buf.len(), 0);
        }
        data[pos..pos + buf.len()].copy_from_slice(buf);
        self.pos.set(pos + buf.len());
        Poll::Ready(Ok(buf.len()))
    }
}

struct TPipeInner {
    buf: VecDeque<u8>,
    reader: bool,
    writer: bool,
    waker: Option<Waker>,
    wwaker: Option<Waker>,
}

/// One end of a pipe between the engine and a fake process.
struct TPipe {
    inner: Rc<RefCell<TPipeInner>>,
    read_end: bool,
    toggle: Toggle,
    counters: Rc<Counters>,
}

impl Drop for TPipe {
    fn drop(&mut self) {
        let mut p = self.inner.borrow_mut();
        if self.read_end {
            p.reader = false;
            if let Some(w) = p.wwaker.take() {
                w.wake();
            }
        } else {
            p.writer = false;
            if let Some(w) = p.waker.take() {
                w.wake();
            }
        }
        if !p.reader && !p.writer {
            self.counters.pipes.set(self.counters.pipes.get() - 1);
        }
    }
}

impl HostFile for TPipe {
    fn poll_read(&self, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        if !self.read_end {
            return Poll::Ready(Err(crate::fd::bad_fd()));
        }
        if !self.toggle.gate(cx) {
            return Poll::Pending;
        }
        let mut p = self.inner.borrow_mut();
        if !p.buf.is_empty() {
            let n = buf.len().min(p.buf.len());
            for (i, b) in p.buf.drain(..n).enumerate() {
                buf[i] = b;
            }
            if let Some(w) = p.wwaker.take() {
                w.wake();
            }
            return Poll::Ready(Ok(n));
        }
        if !p.writer {
            return Poll::Ready(Ok(0));
        }
        p.waker = Some(cx.waker().clone());
        Poll::Pending
    }

    fn poll_write(&self, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        if self.read_end {
            return Poll::Ready(Err(crate::fd::bad_fd()));
        }
        if !self.toggle.gate(cx) {
            return Poll::Pending;
        }
        let mut p = self.inner.borrow_mut();
        if !p.reader {
            return Poll::Ready(Err(io::ErrorKind::BrokenPipe.into()));
        }
        // A small buffer, so that back pressure is exercised.
        let space = 4096usize.saturating_sub(p.buf.len());
        if space == 0 {
            p.wwaker = Some(cx.waker().clone());
            return Poll::Pending;
        }
        let n = space.min(buf.len());
        p.buf.extend(&buf[..n]);
        if let Some(w) = p.waker.take() {
            w.wake();
        }
        Poll::Ready(Ok(n))
    }
}

struct NullFile;

impl HostFile for NullFile {
    fn poll_read(&self, _cx: &mut Context<'_>, _buf: &mut [u8]) -> Poll<io::Result<usize>> {
        Poll::Ready(Ok(0))
    }
    fn poll_write(&self, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        Poll::Ready(Ok(buf.len()))
    }
}

struct FakeExt {
    pid: u32,
    /// The waiter to wake when a signal ends the fake.
    waker: Option<std::task::Waker>,
    fut: Option<Pin<Box<dyn Future<Output = i32>>>>,
    status: Option<ExitStatus>,
    delay: Delay,
    counters: Rc<Counters>,
}

impl ExtProc for FakeExt {
    fn pid(&self) -> Option<u32> {
        Some(self.pid)
    }

    fn poll_wait(&mut self, cx: &mut Context<'_>) -> Poll<ExitStatus> {
        if self.delay.poll(cx).is_pending() {
            return Poll::Pending;
        }
        if let Some(s) = self.status {
            return Poll::Ready(s);
        }
        let Some(fut) = self.fut.as_mut() else {
            return Poll::Ready(ExitStatus::Signaled(crate::host::SIGKILL));
        };
        match fut.as_mut().poll(cx) {
            Poll::Ready(code) => {
                self.fut = None;
                self.status = Some(ExitStatus::Exited(code));
                Poll::Ready(ExitStatus::Exited(code))
            }
            Poll::Pending => {
                self.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }

    fn release(self: Box<Self>) {
        // A fake cannot run without being polled: released, it simply stops.
    }

    fn signal(&mut self, sig: Signal) {
        if self.status.is_none() {
            // The fake dies at once: dropping its future closes its pipes.
            self.fut = None;
            self.status = Some(ExitStatus::Signaled(sig));
            if let Some(w) = self.waker.take() {
                w.wake();
            }
        }
    }
}

impl Drop for FakeExt {
    fn drop(&mut self) {
        self.counters.procs.set(self.counters.procs.get() - 1);
    }
}

/// The test host. Create with [`TestHost::new`], add commands and files, then make a
/// shell with [`TestHost::config`].
pub struct TestHost {
    pub fs: Rc<MemFs>,
    commands: RefCell<HashMap<Vec<u8>, FakeCommand>>,
    latency: Cell<bool>,
    /// What `path_entries` splits on (`;` plays a Windows host).
    path_sep: Cell<u8>,
    counters: Rc<Counters>,
    stdin: Rc<RefCell<Vec<u8>>>,
    stdout: Rc<RefCell<Vec<u8>>>,
    stderr: Rc<RefCell<Vec<u8>>>,
}

impl Default for TestHost {
    fn default() -> Self {
        TestHost::new()
    }
}

impl TestHost {
    /// A host with `/tmp`, the standard fake commands in `/bin` and empty stdio.
    pub fn new() -> TestHost {
        let fs = Rc::new(MemFs::new());
        fs.mkdir_all("/tmp");
        fs.mkdir_all("/bin");
        let host = TestHost {
            fs,
            commands: RefCell::new(HashMap::new()),
            latency: Cell::new(false),
            path_sep: Cell::new(b':'),
            counters: Rc::new(Counters::default()),
            stdin: Rc::new(RefCell::new(Vec::new())),
            stdout: Rc::new(RefCell::new(Vec::new())),
            stderr: Rc::new(RefCell::new(Vec::new())),
        };
        add_standard_commands(&host);
        host
    }

    /// Every operation is `Pending` once before it completes.
    pub fn set_latency(&self, on: bool) {
        self.latency.set(on);
    }

    /// The separator of `PATH` entries (`path_entries`).
    pub fn set_path_sep(&self, sep: u8) {
        self.path_sep.set(sep);
    }

    pub fn set_stdin(&self, data: &[u8]) {
        *self.stdin.borrow_mut() = data.to_vec();
    }

    /// Adds a fake command, found on any `PATH` as `/bin/NAME`.
    pub fn add_command(
        &self,
        name: &str,
        f: impl Fn(FakeProc) -> Pin<Box<dyn Future<Output = i32>>> + 'static,
    ) {
        self.commands
            .borrow_mut()
            .insert(name.as_bytes().to_vec(), Rc::new(f));
    }

    pub fn stdout(&self) -> Vec<u8> {
        self.stdout.borrow().clone()
    }

    pub fn stderr(&self) -> Vec<u8> {
        self.stderr.borrow().clone()
    }

    pub fn live_procs(&self) -> usize {
        self.counters.procs.get()
    }

    pub fn open_files(&self) -> usize {
        self.counters.files.get()
    }

    pub fn live_pipes(&self) -> usize {
        self.counters.pipes.get()
    }

    fn mem_file(
        &self,
        data: Rc<RefCell<Vec<u8>>>,
        read: bool,
        write: bool,
        append: bool,
    ) -> Rc<dyn HostFile> {
        self.counters.files.set(self.counters.files.get() + 1);
        Rc::new(MemFile {
            data,
            pos: Cell::new(0),
            read,
            write,
            append,
            toggle: Toggle::new(self.latency.get()),
            counters: self.counters.clone(),
        })
    }

    /// A shell config for this host: its stdio, `/tmp` as the cwd, a small env.
    pub fn config(self: &Rc<Self>) -> crate::ShellConfig {
        let mut cfg = crate::ShellConfig::new(self.clone());
        cfg.stdin = Some(self.mem_file(self.stdin.clone(), true, false, false));
        cfg.stdout = Some(self.mem_file(self.stdout.clone(), false, true, true));
        cfg.stderr = Some(self.mem_file(self.stderr.clone(), false, true, true));
        cfg.cwd = b"/tmp".to_vec();
        cfg.env = vec![
            (b"PATH".to_vec(), b"/bin".to_vec()),
            (b"HOME".to_vec(), b"/tmp".to_vec()),
        ];
        cfg
    }

    fn pipe_pair(&self) -> (TPipe, TPipe) {
        self.counters.pipes.set(self.counters.pipes.get() + 1);
        let inner = Rc::new(RefCell::new(TPipeInner {
            buf: VecDeque::new(),
            reader: true,
            writer: true,
            waker: None,
            wwaker: None,
        }));
        let mk = |read_end| TPipe {
            inner: inner.clone(),
            read_end,
            toggle: Toggle::new(self.latency.get()),
            counters: self.counters.clone(),
        };
        (mk(true), mk(false))
    }
}

impl Host for TestHost {
    fn open(&self, path: &[u8], opts: OpenOptions) -> HostFuture<io::Result<Rc<dyn HostFile>>> {
        let r = if path == b"/dev/null" {
            Ok(Rc::new(NullFile) as Rc<dyn HostFile>)
        } else {
            self.fs
                .open(path, opts)
                .map(|data| self.mem_file(data, opts.read, opts.write, opts.append))
        };
        delayed(self.latency.get(), r)
    }

    fn stat(&self, path: &[u8], _follow: bool) -> HostFuture<io::Result<Metadata>> {
        delayed(self.latency.get(), self.fs.stat(path))
    }

    fn access(&self, path: &[u8], mode: Access) -> HostFuture<io::Result<bool>> {
        let r = self.fs.stat(path).map(|m| match mode {
            Access::Read => m.mode & 0o444 != 0,
            Access::Write => m.mode & 0o222 != 0,
            Access::Execute => m.mode & 0o111 != 0,
        });
        delayed(self.latency.get(), r)
    }

    fn read_dir(&self, path: &[u8]) -> HostFuture<io::Result<Vec<DirEntry>>> {
        delayed(self.latency.get(), self.fs.read_dir(path))
    }

    fn canonicalize(&self, path: &[u8]) -> HostFuture<io::Result<Vec<u8>>> {
        let r = self.fs.stat(path).map(|_| clean(path));
        delayed(self.latency.get(), r)
    }

    fn path_entries(&self, path: &[u8]) -> Vec<Vec<u8>> {
        let sep = self.path_sep.get();
        path.split(|&b| b == sep).map(<[u8]>::to_vec).collect()
    }

    fn resolve(&self, name: &[u8], _path: &[u8], _cwd: &[u8]) -> HostFuture<Option<Vec<u8>>> {
        let found = self.commands.borrow().contains_key(name).then(|| {
            let mut p = b"/bin/".to_vec();
            p.extend_from_slice(name);
            p
        });
        delayed(self.latency.get(), found)
    }

    fn spawn(&self, spec: SpawnSpec) -> io::Result<Spawned> {
        let name = spec
            .program
            .strip_prefix(b"/bin/")
            .ok_or(io::ErrorKind::NotFound)?;
        let cmd = self
            .commands
            .borrow()
            .get(name)
            .cloned()
            .ok_or(io::ErrorKind::NotFound)?;
        let mut stdio: [Option<Rc<dyn HostFile>>; 3] = [None, None, None];
        let mut pipes = Vec::new();
        let mut given: Vec<(u32, Rc<dyn HostFile>)> = Vec::new();
        for (n, fd) in spec.fds {
            let theirs: Rc<dyn HostFile> = match fd {
                ChildFd::File(f) => f,
                ChildFd::Dup(m) => given
                    .iter()
                    .find(|(k, _)| *k == m)
                    .map(|(_, f)| f.clone())
                    .ok_or(io::ErrorKind::InvalidInput)?,
                ChildFd::Null => Rc::new(NullFile),
                ChildFd::Pipe { input } => {
                    let (r, w) = self.pipe_pair();
                    if input {
                        pipes.push((n, Rc::new(w) as Rc<dyn HostFile>));
                        Rc::new(r)
                    } else {
                        pipes.push((n, Rc::new(r) as Rc<dyn HostFile>));
                        Rc::new(w)
                    }
                }
            };
            given.push((n, theirs.clone()));
            if (n as usize) < 3 {
                stdio[n as usize] = Some(theirs);
            }
        }
        let [i, o, e] = stdio;
        let null = || Rc::new(NullFile) as Rc<dyn HostFile>;
        let proc = FakeProc {
            argv: spec.argv,
            env: spec.env,
            cwd: spec.cwd,
            stdin: i.unwrap_or_else(null),
            stdout: o.unwrap_or_else(null),
            stderr: e.unwrap_or_else(null),
            fs: self.fs.clone(),
        };
        self.counters.procs.set(self.counters.procs.get() + 1);
        self.counters.spawned.set(self.counters.spawned.get() + 1);
        Ok(Spawned {
            proc: Box::new(FakeExt {
                pid: 5000 + self.counters.spawned.get(),
                waker: None,
                fut: Some(cmd(proc)),
                status: None,
                delay: Delay {
                    on: self.latency.get(),
                },
                counters: self.counters.clone(),
            }),
            pipes,
        })
    }

    /// A fixed instant (2023-11-14T22:13:20Z): tests stay deterministic.
    fn now(&self) -> Duration {
        Duration::from_secs(1_700_000_000)
    }

    fn sleep(&self, _duration: Duration) -> HostFuture<()> {
        // Virtual time: a sleep is over at once (after one `Pending` in latency mode).
        delayed(self.latency.get(), ())
    }

    fn pid(&self) -> u32 {
        4242
    }
}

fn arg_num(p: &FakeProc, flag: &[u8], default: usize) -> usize {
    p.argv
        .windows(2)
        .find(|w| w[0] == flag)
        .and_then(|w| std::str::from_utf8(&w[1]).ok()?.parse().ok())
        .unwrap_or(default)
}

/// `cat [FILE…]`, `yes`, `head -n N`, `wc -l`, `argv` (one `[arg]` per line), `env`,
/// `true`, `false`, `hang` (never exits), `exit N`.
fn add_standard_commands(host: &TestHost) {
    host.add_command("cat", |p| {
        Box::pin(async move {
            if p.argv.len() > 1 {
                for a in &p.argv[1..] {
                    let mut path = p.cwd.clone();
                    path.push(b'/');
                    path.extend_from_slice(a);
                    let path = if a.starts_with(b"/") { a.clone() } else { path };
                    match p.fs.read_file(&String::from_utf8_lossy(&path)) {
                        Some(d) => {
                            if p.write(&d).await.is_err() {
                                return 1;
                            }
                        }
                        None => {
                            let _ = p.write_err(b"cat: no such file\n").await;
                            return 1;
                        }
                    }
                }
                return 0;
            }
            let mut buf = [0u8; 1000];
            loop {
                let n = match poll_fn(|cx| p.stdin.poll_read(cx, &mut buf)).await {
                    Ok(0) | Err(_) => return 0,
                    Ok(n) => n,
                };
                if p.write(&buf[..n]).await.is_err() {
                    return 141;
                }
            }
        })
    });
    host.add_command("yes", |p| {
        Box::pin(async move {
            loop {
                if p.write(b"y\n").await.is_err() {
                    return 141;
                }
            }
        })
    });
    host.add_command("head", |p| {
        Box::pin(async move {
            let n = arg_num(&p, b"-n", 10);
            for _ in 0..n {
                let line = p.read_line().await;
                if line.is_empty() {
                    break;
                }
                if p.write(&line).await.is_err() {
                    return 1;
                }
            }
            0
        })
    });
    host.add_command("wc", |p| {
        Box::pin(async move {
            let data = p.read_all().await;
            let n = data.iter().filter(|&&b| b == b'\n').count();
            let _ = p.write(format!("{n}\n").as_bytes()).await;
            0
        })
    });
    host.add_command("argv", |p| {
        Box::pin(async move {
            let mut out = Vec::new();
            for a in &p.argv[1..] {
                out.push(b'[');
                out.extend_from_slice(a);
                out.extend_from_slice(b"]\n");
            }
            let _ = p.write(&out).await;
            0
        })
    });
    host.add_command("env", |p| {
        Box::pin(async move {
            let mut out = Vec::new();
            for (k, v) in &p.env {
                out.extend_from_slice(k);
                out.push(b'=');
                out.extend_from_slice(v);
                out.push(b'\n');
            }
            let _ = p.write(&out).await;
            0
        })
    });
    host.add_command("true", |_| Box::pin(async { 0 }));
    host.add_command("false", |_| Box::pin(async { 1 }));
    // Keeps its descriptors open, as a real process that hangs does.
    host.add_command("hang", |p| {
        Box::pin(async move {
            let _fds = p;
            std::future::pending::<i32>().await
        })
    });
    host.add_command("exit", |p| {
        Box::pin(async move {
            p.argv
                .get(1)
                .and_then(|a| std::str::from_utf8(a).ok()?.parse().ok())
                .unwrap_or(0)
        })
    });
}
