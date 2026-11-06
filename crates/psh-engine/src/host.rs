//! The `Host` trait: every side effect of the engine goes through it (ARCH.md "The Host
//! trait", D1052).
//!
//! Everything that may wait is pollable: file system operations return a [`HostFuture`],
//! open files have `poll_read`/`poll_write`, external processes have `poll_wait`. The
//! engine polls them with a `Context` built from its own waker; a host that finishes an
//! operation on another thread wakes that waker. Nothing here may block the engine
//! thread on something another part of the same engine has to do first.
//!
//! Paths are bytes, absolute and `/`-separated as the shell sees them; the host turns
//! them into native paths at its boundary.
//!
//! Resources are released by `Drop`: dropping an [`ExtProc`] that is still running
//! abandons it (the host terminates it and reaps it in the background), dropping a
//! [`HostFile`] closes it. The engine cancels by dropping its tasks, so this is what
//! keeps a cancel from leaking processes or descriptors.

use std::any::Any;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use std::time::Duration;

/// An operation the host may finish later.
pub type HostFuture<T> = Pin<Box<dyn Future<Output = T>>>;

/// A signal number. POSIX numbering on every platform; hosts map them (Windows: TERM and
/// INT become a console break, KILL terminates the job).
pub type Signal = i32;

pub const SIGINT: Signal = 2;
pub const SIGKILL: Signal = 9;
pub const SIGPIPE: Signal = 13;
pub const SIGTERM: Signal = 15;

/// An open file, pipe end or terminal of the host. Shared between descriptors (`2>&1`
/// shares one object), hence `&self`.
pub trait HostFile: Any {
    /// Reads into `buf`; `Ready(Ok(0))` is end of file.
    fn poll_read(&self, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>>;
    /// Writes from `buf`, at least one byte unless `buf` is empty. A pipe without readers
    /// is `ErrorKind::BrokenPipe`.
    fn poll_write(&self, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>>;
    /// A terminal (`test -t`).
    fn is_terminal(&self) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OpenOptions {
    pub read: bool,
    pub write: bool,
    pub append: bool,
    pub create: bool,
    pub truncate: bool,
    /// Fail if the file exists (`set -o noclobber`).
    pub create_new: bool,
}

impl OpenOptions {
    pub fn read() -> OpenOptions {
        OpenOptions {
            read: true,
            ..OpenOptions::default()
        }
    }

    /// `>`: create or truncate.
    pub fn write() -> OpenOptions {
        OpenOptions {
            write: true,
            create: true,
            truncate: true,
            ..OpenOptions::default()
        }
    }

    /// `>>`
    pub fn append() -> OpenOptions {
        OpenOptions {
            write: true,
            append: true,
            create: true,
            ..OpenOptions::default()
        }
    }

    /// `<>`
    pub fn read_write() -> OpenOptions {
        OpenOptions {
            read: true,
            write: true,
            create: true,
            ..OpenOptions::default()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    File,
    Dir,
    Symlink,
    Fifo,
    Socket,
    BlockDevice,
    CharDevice,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub kind: FileKind,
    pub size: u64,
    /// Permission bits including setuid, setgid and sticky (`0o7777`).
    pub mode: u32,
    /// Modification time in nanoseconds since the epoch.
    pub mtime_ns: i64,
    pub dev: u64,
    pub ino: u64,
    pub uid: u32,
    pub gid: u32,
}

/// What `access` checks (`test -r/-w/-x`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
    Execute,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirEntry {
    pub name: Vec<u8>,
}

/// How an external process ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitStatus {
    Exited(i32),
    Signaled(Signal),
}

impl ExitStatus {
    /// The shell's `$?` for it: the code, or `128 + signal`.
    pub fn code(self) -> i32 {
        match self {
            ExitStatus::Exited(c) => c,
            ExitStatus::Signaled(s) => 128 + s,
        }
    }
}

/// A descriptor of a process to spawn.
pub enum ChildFd {
    /// A file of this host, passed to the child as it is.
    File(Rc<dyn HostFile>),
    /// A new pipe: the child gets one end, the engine the other (in
    /// [`Spawned::pipes`]), which it pumps to or from an engine object. `input` is a
    /// pipe the child reads from.
    Pipe { input: bool },
    /// The same as an earlier descriptor of the list (`|&`, `2>&1` into a pipe: one
    /// pipe, so the order of the writes survives).
    Dup(u32),
    /// The null device.
    Null,
}

pub struct SpawnSpec {
    /// The program to execute, as resolved (`resolve`, or a name with a slash).
    pub program: Vec<u8>,
    /// The arguments, `argv[0]` first (the name as the user wrote it).
    pub argv: Vec<Vec<u8>>,
    /// The complete environment.
    pub env: Vec<(Vec<u8>, Vec<u8>)>,
    pub cwd: Vec<u8>,
    /// The child's descriptors: at least 0, 1 and 2, in any order.
    pub fds: Vec<(u32, ChildFd)>,
}

/// A running external process. Dropping it while it runs abandons it: the host sends
/// TERM, KILL after a grace period, and reaps it in the background.
pub trait ExtProc {
    fn poll_wait(&mut self, cx: &mut Context<'_>) -> Poll<ExitStatus>;
    fn signal(&mut self, sig: Signal);
    /// Lets the process run on: no signal, reaped in the background (`nohup`, and
    /// background jobs still running when the shell exits, as bash leaves them).
    fn release(self: Box<Self>);
}

pub struct Spawned {
    pub proc: Box<dyn ExtProc>,
    /// The engine's ends of the `ChildFd::Pipe` descriptors, by child fd.
    pub pipes: Vec<(u32, Rc<dyn HostFile>)>,
}

/// The engine's view of the outside world. Minimal on purpose; it grows with the
/// features that need it (ARCH.md lists what is planned).
pub trait Host {
    /// Opens a file. The path is absolute.
    fn open(&self, path: &[u8], opts: OpenOptions) -> HostFuture<io::Result<Rc<dyn HostFile>>>;
    /// `stat` (`follow`) or `lstat`.
    fn stat(&self, path: &[u8], follow: bool) -> HostFuture<io::Result<Metadata>>;
    /// Whether the current user may read, write or execute the file.
    fn access(&self, path: &[u8], mode: Access) -> HostFuture<io::Result<bool>>;
    /// The entries of a directory, without `.` and `..`, in no particular order.
    fn read_dir(&self, path: &[u8]) -> HostFuture<io::Result<Vec<DirEntry>>>;
    /// The physical path: symlinks, `.` and `..` resolved (`pwd -P`, `cd -P`).
    fn canonicalize(&self, path: &[u8]) -> HostFuture<io::Result<Vec<u8>>>;
    /// Finds a command name (no slash) in `path` (the shell's `PATH`, relative entries
    /// against `cwd`): the program to spawn, or `None`.
    fn resolve(&self, name: &[u8], path: &[u8], cwd: &[u8]) -> HostFuture<Option<Vec<u8>>>;
    /// Starts an external process. Errors are the reasons it could not start
    /// (`NotFound`, `PermissionDenied`, ...).
    fn spawn(&self, spec: SpawnSpec) -> io::Result<Spawned>;
    /// Completes after `duration`.
    fn sleep(&self, duration: Duration) -> HostFuture<()>;
    /// The shell's process id (`$$`).
    fn pid(&self) -> u32;
}

/// The text bash prints for an error (`strerror`), without Rust's `(os error N)`.
pub fn error_text(err: &io::Error) -> String {
    use io::ErrorKind::*;
    let s = match err.kind() {
        NotFound => "No such file or directory",
        PermissionDenied => "Permission denied",
        AlreadyExists => "File exists",
        BrokenPipe => "Broken pipe",
        IsADirectory => "Is a directory",
        NotADirectory => "Not a directory",
        DirectoryNotEmpty => "Directory not empty",
        ReadOnlyFilesystem => "Read-only file system",
        InvalidFilename => "File name too long",
        Interrupted => "Interrupted system call",
        Unsupported => "Operation not supported",
        _ => {
            let text = err.to_string();
            return match text.find(" (os error") {
                Some(i) => text[..i].to_string(),
                None => text,
            };
        }
    };
    s.to_string()
}
