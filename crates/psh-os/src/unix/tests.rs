//! The host without the engine: spawn, wait, pipes, abandon, release.

use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::time::{Duration, Instant};

use psh_engine::host::{ChildFd, ExitStatus, Host, SpawnSpec};

use super::*;

struct Unpark(std::thread::Thread);
impl Wake for Unpark {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

fn waker() -> Waker {
    Waker::from(Arc::new(Unpark(std::thread::current())))
}

/// Polls `f` until ready, parking in between; panics after 20 s.
fn block<T>(mut f: impl FnMut(&mut Context<'_>) -> Poll<T>) -> T {
    let w = waker();
    let mut cx = Context::from_waker(&w);
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Poll::Ready(v) = f(&mut cx) {
            return v;
        }
        assert!(Instant::now() < deadline, "timed out");
        std::thread::park_timeout(Duration::from_millis(100));
    }
}

fn host() -> UnixHost {
    UnixHost::new(UnixConfig {
        grace: Duration::from_millis(300),
        ..UnixConfig::default()
    })
}

#[test]
fn fork_path() {
    let h = host();
    h.shared
        .fork_only
        .store(true, std::sync::atomic::Ordering::Relaxed);
    let mut p = h.spawn(sh("exit 4", nulls())).unwrap().proc;
    assert_eq!(block(|cx| p.poll_wait(cx)), ExitStatus::Exited(4));
    drop(p);
    let s = h
        .spawn(sh(
            "echo \"$(pwd)\" >&3",
            vec![
                (0, ChildFd::Null),
                (1, ChildFd::Null),
                (2, ChildFd::Null),
                (3, ChildFd::Pipe { input: false }),
            ],
        ))
        .unwrap();
    let text = String::from_utf8(read_all(&s.pipes[0].1)).unwrap();
    assert_eq!(text, "/\n");
    drop(s);
    let mut spec = sh("", nulls());
    spec.program = b"/nonexistent/program".to_vec();
    assert_eq!(h.spawn(spec).err().unwrap().kind(), io::ErrorKind::NotFound);
    let mut spec = sh("", nulls());
    spec.cwd = b"/nonexistent".to_vec();
    assert_eq!(h.spawn(spec).err().unwrap().kind(), io::ErrorKind::NotFound);
    let mut p = h
        .spawn(sh(
            "exec yes",
            vec![
                (0, ChildFd::Null),
                (1, ChildFd::Pipe { input: false }),
                (2, ChildFd::Null),
            ],
        ))
        .unwrap();
    drop(p.pipes);
    assert_eq!(block(|cx| p.proc.poll_wait(cx)), ExitStatus::Signaled(13));
    drop(p.proc);
    let p = h.spawn(sh("sleep 30", nulls())).unwrap().proc;
    drop(p);
    no_leftovers(&h);
}

fn sh(script: &str, fds: Vec<(u32, ChildFd)>) -> SpawnSpec {
    SpawnSpec {
        program: b"/bin/sh".to_vec(),
        argv: vec![b"sh".to_vec(), b"-c".to_vec(), script.as_bytes().to_vec()],
        env: vec![(b"PATH".to_vec(), b"/usr/bin:/bin".to_vec())],
        cwd: b"/".to_vec(),
        fds,
        ..SpawnSpec::default()
    }
}

fn nulls() -> Vec<(u32, ChildFd)> {
    vec![(0, ChildFd::Null), (1, ChildFd::Null), (2, ChildFd::Null)]
}

fn read_all(f: &Rc<dyn HostFile>) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = block(|cx| f.poll_read(cx, &mut buf)).unwrap();
        if n == 0 {
            return out;
        }
        out.extend_from_slice(&buf[..n]);
    }
}

/// Waits until the host has no live groups left (bounded).
fn no_leftovers(h: &UnixHost) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let left = h.leftovers();
        if left.is_empty() {
            return;
        }
        assert!(Instant::now() < deadline, "leaked groups: {left:?}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn exit_status_and_signal_death() {
    let h = host();
    let mut p = h.spawn(sh("exit 3", nulls())).unwrap().proc;
    assert_eq!(block(|cx| p.poll_wait(cx)), ExitStatus::Exited(3));
    drop(p);
    let mut p = h.spawn(sh("kill -TERM $$", nulls())).unwrap().proc;
    assert_eq!(block(|cx| p.poll_wait(cx)), ExitStatus::Signaled(15));
    drop(p);
    no_leftovers(&h);
}

#[test]
fn output_pipe_and_shared_dup() {
    let h = host();
    let s = h
        .spawn(sh(
            "echo out; echo err >&2",
            vec![
                (0, ChildFd::Null),
                (1, ChildFd::Pipe { input: false }),
                (2, ChildFd::Dup(1)),
            ],
        ))
        .unwrap();
    let mut p = s.proc;
    let (n, out) = &s.pipes[0];
    assert_eq!(*n, 1);
    assert_eq!(read_all(out), b"out\nerr\n");
    assert_eq!(block(|cx| p.poll_wait(cx)), ExitStatus::Exited(0));
}

#[test]
fn input_pipe_and_extra_fd() {
    let h = host();
    let s = h
        .spawn(sh(
            "cat; echo three >&3",
            vec![
                (0, ChildFd::Pipe { input: true }),
                (1, ChildFd::Pipe { input: false }),
                (2, ChildFd::Null),
                (3, ChildFd::Dup(1)),
            ],
        ))
        .unwrap();
    let mut p = s.proc;
    let input = s.pipes.iter().find(|(n, _)| *n == 0).unwrap().1.clone();
    let output = s.pipes.iter().find(|(n, _)| *n == 1).unwrap().1.clone();
    let mut data: &[u8] = b"hello\n";
    while !data.is_empty() {
        let n = block(|cx| input.poll_write(cx, data)).unwrap();
        data = &data[n..];
    }
    drop(s.pipes);
    drop(input);
    assert_eq!(read_all(&output), b"hello\nthree\n");
    assert_eq!(block(|cx| p.poll_wait(cx)), ExitStatus::Exited(0));
}

#[test]
fn cwd_env_argv0() {
    let h = host();
    let mut spec = sh(
        "echo \"$0 $(pwd) $X\"",
        vec![
            (0, ChildFd::Null),
            (1, ChildFd::Pipe { input: false }),
            (2, ChildFd::Null),
        ],
    );
    spec.argv[0] = b"named".to_vec();
    spec.argv[2] = b"echo \"$X $(pwd)\"".to_vec();
    spec.cwd = b"/tmp".to_vec();
    spec.env.push((b"X".to_vec(), b"y".to_vec()));
    let s = h.spawn(spec).unwrap();
    let out = read_all(&s.pipes[0].1);
    let text = String::from_utf8(out).unwrap();
    assert!(text == "y /tmp\n" || text == "y /private/tmp\n", "{text}");
}

#[test]
fn spawn_errors() {
    let h = host();
    let mut spec = sh("", nulls());
    spec.program = b"/nonexistent/program".to_vec();
    let e = h.spawn(spec).err().unwrap();
    assert_eq!(e.kind(), io::ErrorKind::NotFound);
    let mut spec = sh("", nulls());
    spec.program = b"/".to_vec();
    let e = h.spawn(spec).err().unwrap();
    assert_eq!(e.kind(), io::ErrorKind::PermissionDenied);
}

#[test]
fn sigpipe_is_default_in_children() {
    // Rust ignores SIGPIPE; the child must get it back, or `yes | head` never ends.
    let h = host();
    let s = h
        .spawn(sh(
            "exec yes",
            vec![
                (0, ChildFd::Null),
                (1, ChildFd::Pipe { input: false }),
                (2, ChildFd::Null),
            ],
        ))
        .unwrap();
    let mut p = s.proc;
    let out = s.pipes[0].1.clone();
    drop(s.pipes);
    let mut buf = [0u8; 16];
    block(|cx| out.poll_read(cx, &mut buf)).unwrap();
    drop(out);
    assert_eq!(block(|cx| p.poll_wait(cx)), ExitStatus::Signaled(13));
}

#[test]
fn abandon_kills_the_whole_group() {
    let h = host();
    // The child ignores TERM and has a child of its own: KILL after the grace period
    // reaches both.
    let p = h
        .spawn(sh("trap '' TERM; sleep 30 & sleep 30; wait", nulls()))
        .unwrap()
        .proc;
    std::thread::sleep(Duration::from_millis(100));
    let start = Instant::now();
    drop(p);
    assert!(!h.leftovers().is_empty());
    no_leftovers(&h);
    assert!(start.elapsed() >= Duration::from_millis(250));
}

#[test]
fn signal_reaches_the_group() {
    let h = host();
    let mut p = h
        .spawn(sh("sleep 30 & sleep 30; wait", nulls()))
        .unwrap()
        .proc;
    std::thread::sleep(Duration::from_millis(100));
    p.signal(libc::SIGKILL);
    assert_eq!(block(|cx| p.poll_wait(cx)), ExitStatus::Signaled(9));
    drop(p);
    no_leftovers(&h);
}

#[test]
fn release_lets_it_run_and_reaps() {
    let h = host();
    let p = h.spawn(sh("sleep 0.2", nulls())).unwrap().proc;
    p.release();
    assert!(h.leftovers().is_empty(), "released is not a leak");
}

#[test]
fn many_spawns_reuse_waiters() {
    let h = host();
    for i in 0..50 {
        let mut p = h
            .spawn(sh(&format!("exit {}", i % 7), nulls()))
            .unwrap()
            .proc;
        assert_eq!(block(|cx| p.poll_wait(cx)), ExitStatus::Exited(i % 7));
    }
    no_leftovers(&h);
}

#[test]
fn fifo_open_waits_off_thread_and_cancels() {
    let dir = std::env::temp_dir().join(format!("psh-os-fifo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let fifo = dir.join("f");
    let c = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    // SAFETY: a valid NUL-terminated path.
    assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
    let h = host();
    let p = fifo.as_os_str().as_bytes().to_vec();
    // An open nobody answers is pending, and dropping it does not leave a thread stuck.
    let mut f = h.open(&p, OpenOptions::read());
    let w = waker();
    assert!(f.as_mut().poll(&mut Context::from_waker(&w)).is_pending());
    drop(f);
    // A reader and a writer meet.
    let mut r = h.open(&p, OpenOptions::read());
    assert!(r.as_mut().poll(&mut Context::from_waker(&w)).is_pending());
    let mut wf = h.open(&p, OpenOptions::write());
    let wr = block(|cx| wf.as_mut().poll(cx)).unwrap();
    let rd = block(|cx| r.as_mut().poll(cx)).unwrap();
    assert!(block(|cx| wr.poll_write(cx, b"x")).is_ok());
    drop(wr);
    assert_eq!(read_all(&rd), b"x");
    std::fs::remove_dir_all(&dir).unwrap();
}

fn piped_out() -> Vec<(u32, ChildFd)> {
    vec![
        (0, ChildFd::Null),
        (1, ChildFd::Pipe { input: false }),
        (2, ChildFd::Null),
    ]
}

#[test]
fn ignored_signals_and_umask_take_effect() {
    let h = host();
    let mut spec = sh("kill -INT $$; echo alive; umask", piped_out());
    spec.sig_ignore = vec![libc::SIGINT];
    spec.umask = Some(0o077);
    let s = h.spawn(spec).unwrap();
    let mut p = s.proc;
    assert_eq!(read_all(&s.pipes[0].1), b"alive\n0077\n");
    assert_eq!(block(|cx| p.poll_wait(cx)), ExitStatus::Exited(0));
    // Without them: INT kills, the umask is the process's.
    let s = h
        .spawn(sh("kill -INT $$; echo alive", piped_out()))
        .unwrap();
    let mut p = s.proc;
    assert_eq!(read_all(&s.pipes[0].1), b"");
    assert_eq!(block(|cx| p.poll_wait(cx)), ExitStatus::Signaled(2));
    let mut spec = sh("umask", piped_out());
    spec.umask = Some(super::spawn::process_umask());
    let s = h.spawn(spec).unwrap();
    let want = format!("{:04o}\n", super::spawn::process_umask());
    assert_eq!(read_all(&s.pipes[0].1), want.as_bytes());
}

#[test]
fn os_pipe_between_two_externals() {
    let h = host();
    let (r, w) = h.pipe().unwrap();
    let a = h
        .spawn(sh(
            "seq 1 50000",
            vec![
                (0, ChildFd::Null),
                (1, ChildFd::File(w)),
                (2, ChildFd::Null),
            ],
        ))
        .unwrap();
    let b = h
        .spawn(sh(
            "wc -l",
            vec![
                (0, ChildFd::File(r)),
                (1, ChildFd::Pipe { input: false }),
                (2, ChildFd::Null),
            ],
        ))
        .unwrap();
    let out = String::from_utf8(read_all(&b.pipes[0].1)).unwrap();
    assert_eq!(out.trim(), "50000");
    let (mut pa, mut pb) = (a.proc, b.proc);
    assert!(pa.pid().is_some());
    assert_eq!(block(|cx| pa.poll_wait(cx)), ExitStatus::Exited(0));
    assert_eq!(block(|cx| pb.poll_wait(cx)), ExitStatus::Exited(0));
}

#[test]
fn kill_reaches_a_pid() {
    let h = host();
    let mut p = h.spawn(sh("exec sleep 30", nulls())).unwrap().proc;
    let pid = p.pid().unwrap() as i32;
    h.kill(pid, libc::SIGTERM).unwrap();
    assert_eq!(block(|cx| p.poll_wait(cx)), ExitStatus::Signaled(15));
    assert!(h.kill(-1_000_000, 0).is_err());
}

