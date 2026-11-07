//! The Windows host (ARCH.md "psh-os", Windows part and "Windows rules").
//!
//! - Processes: [`spawn`] (`CreateProcessW` suspended, a Job Object per external,
//!   only the stdio handles inherited, waiter threads, ctrl-break then
//!   `TerminateJobObject` on abandon).
//! - Files: [`WinFile`], whose I/O never waits for another part of the engine: pump
//!   pipes run on helper threads, shared pipes are read only as far as data is there
//!   ([`poller`]).
//! - Paths: the engine's `C:/x` (also `/c/x`, `/dev/null`, `/tmp`) at the boundary
//!   ([`path`]); command lookup with `PATHEXT`.
//! - Timers on the shared timer thread.
//!
//! Embedded, the host installs no handlers and changes no process-wide state (it never
//! allocates or changes a console).

mod cmdline;
mod file;
mod path;
mod poller;
mod spawn;

use std::io;
use std::os::windows::fs::MetadataExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use psh_engine::HostFile;
use psh_engine::host::{
    Access, DirEntry, FileKind, Host, HostFuture, Metadata, OpenOptions, Signal, SpawnSpec, Spawned,
};
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_REPARSE_POINT,
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, GetFileInformationByHandle,
};

pub use crate::Leftovers;
pub use file::{Decode, WinFile};
pub use spawn::{Placement, settle};

use spawn::Shared;

/// Owns a handle a Win32 call returned (null or `INVALID_HANDLE_VALUE` = the error).
pub(crate) fn owned(h: HANDLE) -> io::Result<OwnedHandle> {
    if h.is_null() || h as isize == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a fresh handle the call returned to us, owned by nobody else.
    Ok(unsafe { OwnedHandle::from_raw_handle(h as RawHandle) })
}

/// Settings of a [`WindowsHost`].
#[derive(Clone, Copy, Debug)]
pub struct WindowsConfig {
    pub placement: Placement,
    /// Time between TERM (a ctrl-break) and termination when a running external is
    /// abandoned.
    pub grace: Duration,
    /// `$$`, when it must not be the process's own id (an embedder running shells
    /// in-process).
    pub pid: Option<u32>,
    /// What happens to the run's leftovers when the host is dropped.
    pub leftovers: Leftovers,
    /// How output of native programs read by the engine is decoded.
    pub decode: Decode,
}

impl WindowsConfig {
    pub const DEFAULT_GRACE: Duration = Duration::from_secs(2);
}

impl Default for WindowsConfig {
    fn default() -> Self {
        WindowsConfig {
            placement: Placement::Group,
            grace: WindowsConfig::DEFAULT_GRACE,
            pid: None,
            leftovers: Leftovers::Kill,
            decode: Decode::Off,
        }
    }
}

/// The host. One per shell: everything it spawned is that shell's run, and dropping
/// it ends the run ([`Leftovers`]).
pub struct WindowsHost {
    shared: Arc<Shared>,
    pid: Option<u32>,
    leftovers: Leftovers,
}

/// Kills a run's leftovers from any thread ([`WindowsHost::killer`]).
#[derive(Clone)]
pub struct RunKiller(Arc<Shared>);

impl RunKiller {
    pub fn kill_all(&self) {
        self.0.kill_all();
    }

    /// Like [`WindowsHost::leftovers`], also after the host is gone (leak checks).
    pub fn leftovers(&self) -> Vec<i32> {
        let jobs = self.0.jobs.lock().unwrap_or_else(|e| e.into_inner());
        jobs.iter()
            .filter(|j| j.active() > 0)
            .map(|j| j.pid() as i32)
            .collect()
    }
}

impl Drop for WindowsHost {
    fn drop(&mut self) {
        match self.leftovers {
            Leftovers::Kill => self.shared.kill_all(),
            Leftovers::Keep => self.shared.keep_all(),
        }
    }
}

impl Default for WindowsHost {
    fn default() -> Self {
        WindowsHost::new(WindowsConfig::default())
    }
}

impl WindowsHost {
    pub fn new(cfg: WindowsConfig) -> WindowsHost {
        WindowsHost {
            shared: Arc::new(Shared {
                grace: cfg.grace,
                placement: cfg.placement,
                console: spawn::has_console(),
                decode: cfg.decode,
                jobs: Mutex::new(Vec::new()),
            }),
            pid: cfg.pid,
            leftovers: cfg.leftovers,
        }
    }

    /// Kills the run's leftovers now (see [`Leftovers::Kill`]). Returns at once;
    /// [`settle`] waits.
    pub fn kill_all(&self) {
        self.shared.kill_all();
    }

    /// A handle that kills the run's leftovers from any thread.
    pub fn killer(&self) -> RunKiller {
        RunKiller(self.shared.clone())
    }

    /// The pids of every external of the run recorded so far, live or not.
    pub fn groups(&self) -> Vec<i32> {
        let jobs = self.shared.jobs.lock().unwrap_or_else(|e| e.into_inner());
        jobs.iter().map(|j| j.pid() as i32).collect()
    }

    /// The externals of the run whose job still has a live process (released externals
    /// included, detached and disowned ones not). Empty after a run that leaked nothing,
    /// once abandoned externals have been killed (up to the grace period).
    pub fn leftovers(&self) -> Vec<i32> {
        let jobs = self.shared.jobs.lock().unwrap_or_else(|e| e.into_inner());
        jobs.iter()
            .filter(|j| j.active() > 0)
            .map(|j| j.pid() as i32)
            .collect()
    }
}

fn ready<T: 'static>(v: T) -> HostFuture<T> {
    Box::pin(std::future::ready(v))
}

/// `PATHEXT`, lower case with the dot (`.com`, `.exe`, ...), from the process's
/// environment.
pub(crate) fn pathext() -> Vec<String> {
    let v = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
    v.split(';')
        .filter(|e| e.len() > 1 && e.starts_with('.'))
        .map(|e| e.to_ascii_lowercase())
        .collect()
}

/// Whether the file starts with `#!`.
fn has_shebang(p: &std::path::Path) -> bool {
    use std::io::Read;
    let mut head = [0u8; 2];
    std::fs::File::open(p).is_ok_and(|mut f| f.read_exact(&mut head).is_ok() && head == *b"#!")
}

/// A file a command name can resolve to: a regular file, or an App Execution Alias
/// (the 0-byte reparse points in WindowsApps, like `winget.exe`), which std's metadata
/// does not see as a file.
fn is_command_file(p: &std::path::Path) -> bool {
    match std::fs::metadata(p) {
        Ok(m) => m.is_file(),
        Err(_) => std::fs::symlink_metadata(p)
            .is_ok_and(|m| m.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0),
    }
}

/// Looks `name` up in the `PATH` value `path_var` (D1076): for each entry, `name` as it
/// is when it already has a `PATHEXT` extension, else `name` plus each extension of
/// `PATHEXT` in order, then the bare name if it is a `#!` script. Empty and relative
/// entries are relative to `cwd`; the cwd is never searched implicitly.
pub(crate) fn resolve_in(name: &[u8], path_var: &[u8], cwd: &[u8]) -> Option<Vec<u8>> {
    let name_s = std::str::from_utf8(name).ok()?;
    let exts = pathext();
    let lower = name_s.to_ascii_lowercase();
    let has_ext = exts.iter().any(|e| lower.ends_with(e.as_str()));
    for dir in path::split_path_var(path_var) {
        let mut base = if path::is_absolute(dir) {
            dir.to_vec()
        } else {
            let mut abs = cwd.to_vec();
            if !dir.is_empty() {
                abs.push(b'/');
                abs.extend_from_slice(dir);
            }
            abs
        };
        while base.len() > 1 && matches!(base.last(), Some(b'/' | b'\\')) {
            base.pop();
        }
        base.push(b'/');
        base.extend_from_slice(name);
        let Ok(native) = path::native(&base) else {
            continue;
        };
        if has_ext && is_command_file(&native) {
            return Some(path::engine_path(&native));
        }
        for e in &exts {
            let mut cand = native.clone().into_os_string();
            cand.push(e);
            let cand = std::path::PathBuf::from(cand);
            if is_command_file(&cand) {
                return Some(path::engine_path(&cand));
            }
        }
        if !has_ext && std::fs::metadata(&native).is_ok_and(|m| m.is_file()) && has_shebang(&native)
        {
            return Some(path::engine_path(&native));
        }
    }
    None
}

/// File identity (volume serial and file index), for `-ef` and loop checks.
fn file_id(p: &std::path::Path, follow: bool) -> Option<(u64, u64)> {
    use std::os::windows::fs::OpenOptionsExt;
    let mut flags = FILE_FLAG_BACKUP_SEMANTICS;
    if !follow {
        flags |= FILE_FLAG_OPEN_REPARSE_POINT;
    }
    let f = std::fs::OpenOptions::new()
        .access_mode(0)
        .custom_flags(flags)
        .open(p)
        .ok()?;
    // SAFETY: plain data, filled by the call.
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: an open handle and a writable struct.
    if unsafe { GetFileInformationByHandle(f.as_raw_handle() as HANDLE, &raw mut info) } == 0 {
        return None;
    }
    let ino = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
    Some((u64::from(info.dwVolumeSerialNumber), ino))
}

fn is_executable(p: &std::path::Path, m: &std::fs::Metadata) -> bool {
    if m.is_dir() {
        return true;
    }
    let lower = p.to_string_lossy().to_ascii_lowercase();
    pathext().iter().any(|e| lower.ends_with(e.as_str())) || has_shebang(p)
}

fn metadata(p: &std::path::Path, m: &std::fs::Metadata, follow: bool) -> Metadata {
    let ft = m.file_type();
    let kind = if ft.is_symlink() {
        FileKind::Symlink
    } else if ft.is_dir() {
        FileKind::Dir
    } else if ft.is_file() {
        FileKind::File
    } else {
        FileKind::Other
    };
    let mtime_ns = m
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos() as i64);
    let mut mode = if m.is_dir() { 0o755 } else { 0o644 };
    if !m.is_dir() && is_executable(p, m) {
        mode |= 0o111;
    }
    if m.file_attributes() & FILE_ATTRIBUTE_READONLY != 0 {
        mode &= !0o222;
    }
    let (dev, ino) = file_id(p, follow).unwrap_or((0, 0));
    Metadata {
        kind,
        size: m.len(),
        mode,
        mtime_ns,
        dev,
        ino,
        uid: 0,
        gid: 0,
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

impl Host for WindowsHost {
    fn open(&self, p: &[u8], opts: OpenOptions) -> HostFuture<io::Result<Rc<dyn HostFile>>> {
        let r = path::native(p).and_then(|np| {
            std_open_options(opts)
                .open(np)
                .map(|f| Rc::new(WinFile::new(OwnedHandle::from(f))) as Rc<dyn HostFile>)
        });
        ready(r)
    }

    fn stat(&self, p: &[u8], follow: bool) -> HostFuture<io::Result<Metadata>> {
        let r = path::native(p).and_then(|np| {
            let m = if follow {
                std::fs::metadata(&np)
            } else {
                std::fs::symlink_metadata(&np)
            }?;
            Ok(metadata(&np, &m, follow))
        });
        ready(r)
    }

    fn access(&self, p: &[u8], mode: Access) -> HostFuture<io::Result<bool>> {
        let r = path::native(p).and_then(|np| {
            let m = match std::fs::metadata(&np) {
                Ok(m) => m,
                Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
                Err(e) => return Err(e),
            };
            Ok(match mode {
                Access::Read => m.is_dir() || std::fs::File::open(&np).is_ok(),
                Access::Write => m.file_attributes() & FILE_ATTRIBUTE_READONLY == 0,
                Access::Execute => is_executable(&np, &m),
            })
        });
        ready(r)
    }

    fn read_dir(&self, p: &[u8]) -> HostFuture<io::Result<Vec<DirEntry>>> {
        let r = path::native(p).and_then(|np| {
            Ok(std::fs::read_dir(np)?
                .filter_map(Result::ok)
                .map(|e| DirEntry {
                    name: e.file_name().to_string_lossy().into_owned().into_bytes(),
                })
                .collect())
        });
        ready(r)
    }

    fn canonicalize(&self, p: &[u8]) -> HostFuture<io::Result<Vec<u8>>> {
        let r = path::native(p)
            .and_then(std::fs::canonicalize)
            .map(|c| path::engine_path(&c));
        ready(r)
    }

    fn resolve(&self, name: &[u8], path_var: &[u8], cwd: &[u8]) -> HostFuture<Option<Vec<u8>>> {
        ready(resolve_in(name, path_var, cwd))
    }

    fn spawn(&self, spec: SpawnSpec) -> io::Result<Spawned> {
        spawn::spawn(&self.shared, spec)
    }

    fn pipe(&self) -> io::Result<(Rc<dyn HostFile>, Rc<dyn HostFile>)> {
        let (r, w) = file::os_pipe()?;
        Ok((Rc::new(r), Rc::new(w)))
    }

    fn kill(&self, pid: i32, sig: Signal) -> io::Result<()> {
        let me = std::process::id() as i32;
        // Embedded (a fake `$$`), the process is the embedder's.
        if self.pid.is_some() && (pid == me || pid == -me) {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        // 0 and -1: every external of the run (psh's own group, which has no other
        // meaning here).
        if pid == 0 || pid == -1 {
            let jobs: Vec<_> = self
                .shared
                .jobs
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            for j in jobs.iter().filter(|j| j.active() > 0) {
                let _ = j.signal(sig);
            }
            return Ok(());
        }
        // An external of the run (a negative pid: its group, which is its job).
        if let Some(job) = self.shared.job_of(pid.unsigned_abs()) {
            return job.signal(sig);
        }
        if pid < 0 {
            return Err(io::Error::from_raw_os_error(87));
        }
        spawn::kill_pid(pid as u32, sig)
    }

    fn sleep(&self, duration: Duration) -> HostFuture<()> {
        crate::timer::sleep(duration)
    }

    fn pid(&self) -> u32 {
        self.pid.unwrap_or_else(std::process::id)
    }
}

/// The shell's environment from the process's (D1075): Windows names are
/// case-insensitive, shell variables are not, so `Path` (however it is spelled) becomes
/// `PATH`, the name scripts use; `HOME` defaults to `USERPROFILE` and `TMPDIR` to the
/// temp directory, both `/`-separated. For the standalone shell and embedders.
pub fn shell_env(env: Vec<(Vec<u8>, Vec<u8>)>) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut env: Vec<(Vec<u8>, Vec<u8>)> = env
        .into_iter()
        .map(|(k, v)| {
            if k.eq_ignore_ascii_case(b"PATH") {
                (b"PATH".to_vec(), v)
            } else {
                (k, v)
            }
        })
        .collect();
    let has = |env: &[(Vec<u8>, Vec<u8>)], k: &[u8]| env.iter().any(|(n, _)| n == k);
    if !has(&env, b"HOME")
        && let Some(home) = std::env::var_os("USERPROFILE")
    {
        let home = path::engine_path(std::path::Path::new(&home));
        env.push((b"HOME".to_vec(), home));
    }
    if !has(&env, b"TMPDIR") {
        env.push((b"TMPDIR".to_vec(), path::temp_dir()));
    }
    env
}

#[cfg(test)]
mod tests;
