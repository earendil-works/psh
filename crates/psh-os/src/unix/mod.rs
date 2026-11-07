//! The Unix host: macOS, Linux and Android (ARCH.md "psh-os").
//!
//! - Processes: [`spawn`] (posix_spawn or fork + exec, a session per external, pooled
//!   `waitid(WNOWAIT)` waiters, TERM → grace → KILL on abandon).
//! - Files: [`UnixFile`], whose I/O never waits in the kernel for another part of the
//!   engine ([`ready`]: one readiness thread, `select` on macOS). Pipes to externals are
//!   our private, non-blocking ends; the engine pumps them, so no thread per pump.
//! - FIFO opens, which wait for the other end, run on a helper thread.
//! - Timers on one process-wide thread.
//!
//! Embedded, the host installs no signal handlers and changes no process-wide state.

mod file;
pub mod ready;
pub mod signals;
mod spawn;

use std::ffi::{CString, OsStr, OsString};
use std::io;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};
use std::time::Duration;

use psh_engine::HostFile;
use psh_engine::host::{
    Access, DirEntry, FileKind, Host, HostFuture, Metadata, OpenOptions, Signal, SpawnSpec, Spawned,
};

pub use file::UnixFile;
pub use spawn::{Placement, settle};

use spawn::Shared;

/// Settings of a [`UnixHost`].
#[derive(Clone, Copy, Debug)]
pub struct UnixConfig {
    pub placement: Placement,
    /// Time between TERM and KILL when a running external is abandoned.
    pub grace: Duration,
    /// `$$`, when it must not be the process's own id (an embedder running shells
    /// in-process, where `kill $$` must not reach the embedder).
    pub pid: Option<u32>,
}

impl UnixConfig {
    pub const DEFAULT_GRACE: Duration = Duration::from_secs(2);
}

impl Default for UnixConfig {
    fn default() -> Self {
        UnixConfig {
            placement: Placement::Session,
            grace: UnixConfig::DEFAULT_GRACE,
            pid: None,
        }
    }
}

/// The host.
pub struct UnixHost {
    shared: Arc<Shared>,
    pid: Option<u32>,
}

impl Default for UnixHost {
    fn default() -> Self {
        UnixHost::new(UnixConfig::default())
    }
}

impl UnixHost {
    pub fn new(cfg: UnixConfig) -> UnixHost {
        UnixHost {
            shared: Arc::new(Shared {
                grace: cfg.grace,
                placement: cfg.placement,
                scopes: Mutex::new(Vec::new()),
                ignored_at_start: spawn::ignored_now(),
                fork_only: Default::default(),
            }),
            pid: cfg.pid,
        }
    }

    /// The process groups (or, under [`Placement::Inherit`], processes) this host
    /// spawned that still have a live member, zombies included; released externals are
    /// not counted. Empty after a run that leaked nothing, once abandoned externals have
    /// been killed and reaped (up to the grace period).
    pub fn leftovers(&self) -> Vec<i32> {
        let mut scopes = self.shared.scopes.lock().unwrap_or_else(|e| e.into_inner());
        scopes.retain(|s| s.alive());
        scopes.iter().map(|s| s.id()).collect()
    }
}

fn path(b: &[u8]) -> PathBuf {
    PathBuf::from(OsStr::from_bytes(b))
}

fn ready<T: 'static>(v: T) -> HostFuture<T> {
    Box::pin(std::future::ready(v))
}

fn metadata(m: &std::fs::Metadata) -> Metadata {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    let ft = m.file_type();
    let kind = if ft.is_symlink() {
        FileKind::Symlink
    } else if ft.is_dir() {
        FileKind::Dir
    } else if ft.is_file() {
        FileKind::File
    } else if ft.is_fifo() {
        FileKind::Fifo
    } else if ft.is_socket() {
        FileKind::Socket
    } else if ft.is_block_device() {
        FileKind::BlockDevice
    } else if ft.is_char_device() {
        FileKind::CharDevice
    } else {
        FileKind::Other
    };
    Metadata {
        kind,
        size: m.len(),
        mode: m.mode() & 0o7777,
        mtime_ns: m.mtime().saturating_mul(1_000_000_000) + m.mtime_nsec(),
        dev: m.dev(),
        ino: m.ino(),
        uid: m.uid(),
        gid: m.gid(),
    }
}

/// `faccessat` with the effective ids, as `test -r/-w/-x` checks.
fn eaccess(p: &[u8], mode: libc::c_int) -> io::Result<bool> {
    let c = CString::new(p).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    // SAFETY: a valid NUL-terminated path; no other pointers.
    let rc = unsafe { libc::faccessat(libc::AT_FDCWD, c.as_ptr(), mode, libc::AT_EACCESS) };
    if rc == 0 {
        return Ok(true);
    }
    let e = io::Error::last_os_error();
    match e.raw_os_error() {
        Some(libc::EACCES) | Some(libc::EROFS) | Some(libc::ETXTBSY) => Ok(false),
        _ => Err(e),
    }
}

fn std_open_options(opts: OpenOptions) -> std::fs::OpenOptions {
    let mut o = std::fs::OpenOptions::new();
    o.read(opts.read)
        .write(opts.write && !opts.append)
        .append(opts.append)
        .truncate(opts.truncate)
        .create(opts.create && !opts.create_new)
        .create_new(opts.create_new);
    o
}

/// Opens a FIFO on a helper thread: the open waits for the other end, which may be a
/// vproc of the same engine. Dropping the future while the open waits opens the other
/// end briefly (non-blocking), so the thread finishes and closes what it got.
fn open_fifo(p: Vec<u8>, opts: OpenOptions) -> HostFuture<io::Result<Rc<dyn HostFile>>> {
    #[derive(Default)]
    struct Slot {
        result: Option<io::Result<std::fs::File>>,
        waker: Option<Waker>,
        cancelled: bool,
        /// The engine took the result.
        taken: bool,
    }
    let slot = Arc::new(Mutex::new(Slot::default()));
    let s = slot.clone();
    let tp = p.clone();
    let started = std::thread::Builder::new()
        .name("psh-fifo".into())
        .stack_size(64 << 10)
        .spawn(move || {
            let r = std_open_options(opts).open(path(&tp));
            let mut st = s.lock().unwrap_or_else(|e| e.into_inner());
            if st.cancelled {
                return;
            }
            st.result = Some(r);
            if let Some(w) = st.waker.take() {
                w.wake();
            }
        });
    if let Err(e) = started {
        return ready(Err(e));
    }
    struct Open {
        slot: Arc<Mutex<Slot>>,
        path: Vec<u8>,
        reading: bool,
    }
    impl Drop for Open {
        fn drop(&mut self) {
            let mut st = self.slot.lock().unwrap_or_else(|e| e.into_inner());
            if st.result.is_some() || st.taken {
                return;
            }
            st.cancelled = true;
            drop(st);
            let flags = if self.reading {
                libc::O_WRONLY
            } else {
                libc::O_RDONLY
            };
            let _ = std::fs::OpenOptions::new()
                .read(!self.reading)
                .write(self.reading)
                .custom_flags(flags | libc::O_NONBLOCK)
                .open(path(&self.path));
        }
    }
    use std::os::unix::fs::OpenOptionsExt;
    let open = Open {
        slot,
        path: p,
        reading: opts.read,
    };
    Box::pin(std::future::poll_fn(move |cx| {
        let mut st = open.slot.lock().unwrap_or_else(|e| e.into_inner());
        match st.result.take() {
            Some(r) => {
                st.taken = true;
                Poll::Ready(r.map(|f| Rc::new(UnixFile::new(f)) as Rc<dyn HostFile>))
            }
            None => {
                st.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }))
}

impl Host for UnixHost {
    fn open(&self, p: &[u8], opts: OpenOptions) -> HostFuture<io::Result<Rc<dyn HostFile>>> {
        // Read-write opens of a FIFO never wait.
        if opts.read != opts.write
            && std::fs::metadata(path(p)).is_ok_and(|m| {
                use std::os::unix::fs::FileTypeExt;
                m.file_type().is_fifo()
            })
        {
            return open_fifo(p.to_vec(), opts);
        }
        let r = std_open_options(opts)
            .open(path(p))
            .map(|f| Rc::new(UnixFile::new(f)) as Rc<dyn HostFile>);
        ready(r)
    }

    fn stat(&self, p: &[u8], follow: bool) -> HostFuture<io::Result<Metadata>> {
        let m = if follow {
            std::fs::metadata(path(p))
        } else {
            std::fs::symlink_metadata(path(p))
        };
        ready(m.map(|m| metadata(&m)))
    }

    fn access(&self, p: &[u8], mode: Access) -> HostFuture<io::Result<bool>> {
        let m = match mode {
            Access::Read => libc::R_OK,
            Access::Write => libc::W_OK,
            Access::Execute => libc::X_OK,
        };
        ready(eaccess(p, m))
    }

    fn read_dir(&self, p: &[u8]) -> HostFuture<io::Result<Vec<DirEntry>>> {
        let r = std::fs::read_dir(path(p)).map(|it| {
            it.filter_map(Result::ok)
                .map(|e| DirEntry {
                    name: e.file_name().into_vec(),
                })
                .collect()
        });
        ready(r)
    }

    fn canonicalize(&self, p: &[u8]) -> HostFuture<io::Result<Vec<u8>>> {
        ready(std::fs::canonicalize(path(p)).map(|p| OsString::from(p).into_vec()))
    }

    fn resolve(&self, name: &[u8], path_var: &[u8], cwd: &[u8]) -> HostFuture<Option<Vec<u8>>> {
        for dir in path_var.split(|&b| b == b':') {
            let mut cand = if dir.first() == Some(&b'/') {
                dir.to_vec()
            } else {
                // An empty or relative entry is relative to the cwd.
                let mut abs = cwd.to_vec();
                if !dir.is_empty() {
                    abs.push(b'/');
                    abs.extend_from_slice(dir);
                }
                abs
            };
            cand.push(b'/');
            cand.extend_from_slice(name);
            let is_file = std::fs::metadata(path(&cand)).is_ok_and(|m| m.is_file());
            if is_file && eaccess(&cand, libc::X_OK).unwrap_or(false) {
                return ready(Some(cand));
            }
        }
        ready(None)
    }

    fn spawn(&self, spec: SpawnSpec) -> io::Result<Spawned> {
        spawn::spawn(&self.shared, spec)
    }

    fn pipe(&self) -> io::Result<(Rc<dyn HostFile>, Rc<dyn HostFile>)> {
        let (r, w) = io::pipe()?;
        Ok((Rc::new(UnixFile::new(r)), Rc::new(UnixFile::new(w))))
    }

    fn kill(&self, pid: i32, sig: Signal) -> io::Result<()> {
        kill(pid, sig)
    }

    fn sleep(&self, duration: Duration) -> HostFuture<()> {
        crate::timer::sleep(duration)
    }

    fn pid(&self) -> u32 {
        self.pid.unwrap_or_else(std::process::id)
    }
}

/// Whether descriptor `fd` is a terminal whose foreground process group is ours: the
/// shell runs in the foreground of a terminal (a person ran it), so externals should
/// share the terminal ([`Placement::Inherit`]).
pub fn terminal_foreground(fd: i32) -> bool {
    // SAFETY: plain syscalls without pointers.
    unsafe { libc::isatty(fd) == 1 && libc::tcgetpgrp(fd) == libc::getpgrp() }
}

/// Sends `sig` to process `pid` (negative: a process group), as `kill(2)`.
pub fn kill(pid: i32, sig: Signal) -> io::Result<()> {
    // SAFETY: plain syscall without pointers.
    if unsafe { libc::kill(pid, sig) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(test)]
mod tests;
