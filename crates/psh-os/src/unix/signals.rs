//! Signals for the standalone `psh` binary only; an embedded shell installs nothing
//! (research/execution-core.md §5, pitfall 4).
//!
//! The handlers are async-signal-safe: they set a bit in an atomic word and write a
//! byte into a self-pipe. A watcher thread reads the pipe and hands each signal to the
//! driver's callback (which cancels the run). Children never see these handlers: an
//! exec resets caught signals, and the spawner resets every disposition anyway.
//! A signal ignored when psh started stays ignored and gets no handler, as in bash.

use std::io::{self, Read};
use std::os::fd::{AsRawFd, IntoRawFd};
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};

use psh_engine::host::Signal;

/// Signals caught and not yet handed to the callback, one bit each.
static PENDING: AtomicU64 = AtomicU64::new(0);
/// The first signal caught (0: none).
static FIRST: AtomicI32 = AtomicI32::new(0);
/// The write end of the self-pipe; -1 until `catch` ran.
static WAKE_FD: AtomicI32 = AtomicI32::new(-1);

extern "C" fn on_signal(sig: libc::c_int) {
    let saved = io::Error::last_os_error().raw_os_error().unwrap_or(0);
    if (0..64).contains(&sig) {
        let _ = FIRST.compare_exchange(0, sig, Ordering::SeqCst, Ordering::SeqCst);
        PENDING.fetch_or(1 << sig, Ordering::SeqCst);
    }
    let fd = WAKE_FD.load(Ordering::SeqCst);
    if fd >= 0 {
        let b = 1u8;
        // SAFETY: write(2) is async-signal-safe; the pipe end lives for the process.
        // A full pipe (EAGAIN) already guarantees a wakeup.
        unsafe { libc::write(fd, (&raw const b).cast(), 1) };
    }
    // errno is per thread and the interrupted code may look at it.
    set_errno(saved);
}

fn set_errno(e: i32) {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    // SAFETY: __error returns this thread's errno location.
    unsafe {
        *libc::__error() = e;
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    // SAFETY: __errno_location returns this thread's errno location.
    unsafe {
        *libc::__errno_location() = e;
    }
    #[cfg(not(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "linux",
        target_os = "android"
    )))]
    let _ = e;
}

/// The first signal caught, set by the handler itself: the driver checks it before it
/// exits, since the watcher thread may not have passed the signal on yet.
pub fn first_caught() -> Option<Signal> {
    match FIRST.load(Ordering::SeqCst) {
        0 => None,
        s => Some(s),
    }
}

/// Whether the process ignores `sig` now.
pub fn is_ignored(sig: Signal) -> bool {
    // SAFETY: sigaction is plain data, valid zeroed.
    let mut old: libc::sigaction = unsafe { std::mem::zeroed() };
    // SAFETY: a null new action only queries; `old` is writable.
    let rc = unsafe { libc::sigaction(sig, std::ptr::null(), &raw mut old) };
    rc == 0 && old.sa_sigaction == libc::SIG_IGN
}

/// Catches `sigs` (except those ignored now) and calls `callback` for each delivery,
/// on a watcher thread. Once per process.
pub fn catch(sigs: &[Signal], callback: impl Fn(Signal) + Send + 'static) -> io::Result<()> {
    let (mut r, w) = io::pipe()?;
    super::ready::set_nonblocking(w.as_raw_fd())?;
    let wfd = w.into_raw_fd();
    if WAKE_FD
        .compare_exchange(-1, wfd, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        // SAFETY: `wfd` is ours and unused.
        unsafe { libc::close(wfd) };
        return Err(io::Error::other("signals are caught already"));
    }
    std::thread::Builder::new()
        .name("psh-signals".into())
        .stack_size(64 << 10)
        .spawn(move || {
            let mut buf = [0u8; 64];
            loop {
                match r.read(&mut buf) {
                    Ok(0) => return,
                    Ok(_) => {}
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                    Err(_) => return,
                }
                let bits = PENDING.swap(0, Ordering::SeqCst);
                for sig in 1..64 {
                    if bits & (1 << sig) != 0 {
                        callback(sig);
                    }
                }
            }
        })?;
    for &sig in sigs {
        if is_ignored(sig) {
            continue;
        }
        // SAFETY: sigaction is plain data; the handler only does async-signal-safe
        // work (atomics, write).
        unsafe {
            let mut act: libc::sigaction = std::mem::zeroed();
            act.sa_sigaction = on_signal as *const () as libc::sighandler_t;
            act.sa_flags = libc::SA_RESTART;
            libc::sigemptyset(&raw mut act.sa_mask);
            if libc::sigaction(sig, &raw const act, std::ptr::null_mut()) != 0 {
                return Err(io::Error::last_os_error());
            }
        }
    }
    Ok(())
}

/// Ends the process by `sig` with its default action, so the parent sees the signal
/// (as bash does when a signal ends it); exits with `128 + sig` if that fails. When the
/// process leads its process group (a harness started it in a group of its own), the
/// signal goes to the whole group: what the shell's externals left behind there dies
/// with it, even when only the shell's pid was signalled (D1067).
pub fn die_by(sig: Signal) -> ! {
    // SAFETY: plain calls with valid arguments; the process ends here.
    unsafe {
        libc::signal(sig, libc::SIG_DFL);
        let mut set: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&raw mut set);
        libc::sigaddset(&raw mut set, sig);
        libc::pthread_sigmask(libc::SIG_UNBLOCK, &raw const set, std::ptr::null_mut());
        if libc::getpgrp() == libc::getpid() {
            libc::kill(0, sig);
        }
        libc::raise(sig);
    }
    std::process::exit(128 + sig)
}
