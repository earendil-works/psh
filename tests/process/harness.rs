//! Runs a script in-process over the OS host (Unix or Windows) with OS pipes for stdin,
//! stdout and stderr, under a deadline, then checks that no process the host created
//! survived. `$H` is `psh-test-helper`, the external program the portable cases use;
//! `$TMPDIR_CASE` is a fresh directory, the script's cwd.

use std::io::{Read, Write};
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use psh_engine::std_host::{drive_until, engine_thread};
use psh_engine::{HostFile, Script, Shell, ShellConfig, Step};
use psh_os::Leftovers;

/// TERM → KILL in tests: short, so leak checks after a cancel finish quickly.
const GRACE: Duration = Duration::from_millis(300);

/// A factor for every deadline: `PSH_TEST_TIME_SCALE` (≥ 1), set where the gate runs
/// the tests slowed down, such as x64 under emulation.
fn time_scale() -> f64 {
    std::env::var("PSH_TEST_TIME_SCALE")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|s| *s >= 1.0)
        .unwrap_or(1.0)
}

/// Everything a test may wait for, in total.
fn deadline_total() -> Duration {
    Duration::from_secs_f64(30.0 * time_scale())
}

#[derive(Debug)]
pub struct Out {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
    /// When the engine reported `Done`.
    pub done_after: Duration,
}

pub struct Run {
    script: String,
    stdin: Option<Vec<u8>>,
    cancel_after: Option<Duration>,
    kill_run_after: Option<Duration>,
    leftovers: Leftovers,
    expect_survivors: bool,
}

pub fn run(script: &str) -> Out {
    Run::new(script).go()
}

/// The helper program, as the engine writes paths.
pub fn helper() -> String {
    env!("CARGO_BIN_EXE_psh-test-helper").replace('\\', "/")
}

#[cfg(unix)]
mod os {
    use super::*;
    use psh_os::unix::{RunKiller, UnixConfig, UnixFile, UnixHost};

    pub type OsHost = UnixHost;

    pub fn host(leftovers: Leftovers) -> UnixHost {
        UnixHost::new(UnixConfig {
            grace: GRACE,
            pid: Some(99_999_999),
            leftovers,
            ..UnixConfig::default()
        })
    }

    pub fn file(f: std::fs::File) -> Option<Rc<dyn HostFile>> {
        Some(Rc::new(UnixFile::new(f)) as Rc<dyn HostFile>)
    }

    pub fn env(cwd: &str) -> Vec<(Vec<u8>, Vec<u8>)> {
        let path = std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".into());
        vec![
            (b"PATH".to_vec(), path.into_bytes()),
            (b"TMPDIR_CASE".to_vec(), cwd.as_bytes().to_vec()),
            (b"LC_ALL".to_vec(), b"C".to_vec()),
            (b"H".to_vec(), helper().into_bytes()),
        ]
    }

    /// What is left of a run once its host is gone: its process groups.
    pub struct Watch(Vec<i32>, RunKiller);

    impl Watch {
        pub fn new(h: &UnixHost) -> Watch {
            Watch(Vec::new(), h.killer())
        }

        /// Before the host is dropped: every group it made.
        pub fn record(&mut self, h: &UnixHost) {
            self.0 = h.groups();
        }

        pub fn live(&self) -> Vec<i32> {
            let alive = |g: &i32| match psh_os::unix::kill(-g, 0) {
                Ok(()) => true,
                Err(e) => e.kind() == std::io::ErrorKind::PermissionDenied,
            };
            self.0.iter().copied().filter(alive).collect()
        }

        pub fn kill(&self) {
            for g in self.live() {
                let _ = psh_os::unix::kill(-g, psh_engine::host::SIGKILL);
            }
        }

        pub fn killer(&self) -> RunKiller {
            self.1.clone()
        }
    }

    pub fn settle(d: Duration) {
        psh_os::unix::settle(d);
    }
}

#[cfg(windows)]
mod os {
    use super::*;
    use psh_os::windows::{RunKiller, WinFile, WindowsConfig, WindowsHost, shell_env};

    pub type OsHost = WindowsHost;

    pub fn host(leftovers: Leftovers) -> WindowsHost {
        WindowsHost::new(WindowsConfig {
            grace: GRACE,
            pid: Some(99_999_999),
            leftovers,
            ..WindowsConfig::default()
        })
    }

    pub fn file(f: std::fs::File) -> Option<Rc<dyn HostFile>> {
        let h = std::os::windows::io::OwnedHandle::from(f);
        Some(Rc::new(WinFile::new(h)) as Rc<dyn HostFile>)
    }

    pub fn env(cwd: &str) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut env: Vec<(Vec<u8>, Vec<u8>)> = std::env::vars()
            .map(|(k, v)| (k.into_bytes(), v.into_bytes()))
            .collect();
        env.push((b"H".to_vec(), helper().into_bytes()));
        env.push((b"TMPDIR_CASE".to_vec(), cwd.as_bytes().to_vec()));
        shell_env(env)
    }

    /// What is left of a run once its host is gone: its jobs, which outlive the host
    /// through the run killer.
    pub struct Watch(RunKiller);

    impl Watch {
        pub fn new(h: &WindowsHost) -> Watch {
            Watch(h.killer())
        }

        pub fn record(&mut self, _h: &WindowsHost) {}

        pub fn live(&self) -> Vec<i32> {
            self.0.leftovers()
        }

        pub fn kill(&self) {
            self.0.kill_all();
            psh_os::windows::settle(GRACE + Duration::from_secs(2));
        }

        pub fn killer(&self) -> RunKiller {
            self.0.clone()
        }
    }

    pub fn settle(d: Duration) {
        psh_os::windows::settle(d);
    }
}

impl Run {
    pub fn new(script: &str) -> Run {
        Run {
            script: script.to_string(),
            stdin: None,
            cancel_after: None,
            kill_run_after: None,
            leftovers: Leftovers::Kill,
            expect_survivors: false,
        }
    }

    /// The host keeps the run's leftovers (bash's behavior) instead of killing them.
    pub fn keep_leftovers(mut self) -> Run {
        self.leftovers = Leftovers::Keep;
        self
    }

    pub fn kill_run_after(mut self, d: Duration) -> Run {
        self.kill_run_after = Some(d);
        self
    }

    /// Processes of the run survive it on purpose: the check must see them (and then
    /// kills them).
    pub fn expect_survivors(mut self) -> Run {
        self.expect_survivors = true;
        self
    }

    pub fn stdin(mut self, data: &str) -> Run {
        self.stdin = Some(data.as_bytes().to_vec());
        self
    }

    pub fn cancel_after(mut self, d: Duration) -> Run {
        self.cancel_after = Some(d);
        self
    }

    pub fn go(self) -> Out {
        let dir = tempdir();
        let (out_r, out_w) = std::io::pipe().unwrap();
        let (err_r, err_w) = std::io::pipe().unwrap();
        let (in_r, in_w) = std::io::pipe().unwrap();
        let feeder = self.stdin.map(|data| {
            let mut w = in_w;
            thread::spawn(move || {
                let _ = w.write_all(&data);
            })
        });
        let reader = |mut r: std::io::PipeReader| {
            let (tx, rx) = mpsc::channel();
            thread::spawn(move || {
                let mut buf = Vec::new();
                let _ = r.read_to_end(&mut buf);
                let _ = tx.send(buf);
            });
            rx
        };
        let out_rx = reader(out_r);
        let err_rx = reader(err_r);
        let start = Instant::now();
        let deadline = start + deadline_total();
        let Run {
            script,
            cancel_after,
            kill_run_after,
            leftovers,
            expect_survivors,
            ..
        } = self;
        let cwd = dir.clone();
        let engine = engine_thread(move || {
            let host: Rc<os::OsHost> = Rc::new(os::host(leftovers));
            let mut watch = os::Watch::new(&host);
            let killer = kill_run_after.map(|after| {
                let k = watch.killer();
                thread::spawn(move || {
                    thread::sleep(after);
                    k.kill_all();
                })
            });
            let mut cfg = ShellConfig::new(host.clone());
            cfg.env = os::env(&cwd);
            cfg.cwd = cwd.into_bytes();
            cfg.stdin = os::file(std::fs::File::from(pipe_handle(in_r)));
            cfg.stdout = os::file(std::fs::File::from(pipe_handle(out_w)));
            cfg.stderr = os::file(std::fs::File::from(pipe_handle(err_w)));
            let mut shell = Shell::new(cfg);
            shell.start(Script::command(&script));
            let status = match cancel_after {
                Some(after) => {
                    let control = shell.control();
                    let t = thread::spawn(move || {
                        thread::sleep(after);
                        control.cancel();
                    });
                    let s = drive_until(&mut shell, Some(deadline));
                    t.join().unwrap();
                    s
                }
                None => drive_until(&mut shell, Some(deadline)),
            };
            let done_after = start.elapsed();
            assert_eq!(shell.step(1), Step::Idle);
            drop(shell);
            if let Some(k) = killer {
                k.join().unwrap();
            }
            // The leak check: dropping the host ends the run (killing its leftovers in
            // `Kill` mode); then everything it made must be gone: abandoned and killed
            // externals after TERM, the grace period and KILL, plus a moment for the
            // system to reap killed grandchildren.
            watch.record(&host);
            drop(host);
            os::settle(GRACE + Duration::from_secs(2));
            if expect_survivors {
                let left = watch.live();
                watch.kill();
                let missing = if left.is_empty() { vec![-1] } else { vec![] };
                return (status, done_after, missing);
            }
            let leak_deadline = Instant::now() + Duration::from_secs(5);
            let leftovers = loop {
                let left = watch.live();
                if left.is_empty() || Instant::now() > leak_deadline {
                    watch.kill();
                    break left;
                }
                thread::sleep(Duration::from_millis(20));
            };
            (status, done_after, leftovers)
        })
        .unwrap();
        let (status, done_after, leftovers) = engine.join().expect("engine thread");
        let wait = |rx: mpsc::Receiver<Vec<u8>>| {
            let left = deadline.saturating_duration_since(Instant::now());
            let text = String::from_utf8_lossy(&rx.recv_timeout(left).expect("output never ended"))
                .into_owned();
            // Native Windows programs end lines with CRLF.
            if cfg!(windows) {
                text.replace("\r\n", "\n")
            } else {
                text
            }
        };
        let stdout = wait(out_rx);
        let stderr = wait(err_rx);
        if let Some(f) = feeder {
            let _ = f.join();
        }
        let _ = std::fs::remove_dir_all(&dir);
        if expect_survivors {
            assert!(leftovers.is_empty(), "nothing survived the run");
        } else {
            assert!(leftovers.is_empty(), "leaked processes: {leftovers:?}");
        }
        let status = status.expect("the script hit the test deadline");
        Out {
            status,
            stdout,
            stderr,
            done_after,
        }
    }
}

#[cfg(unix)]
fn pipe_handle<T: Into<std::os::fd::OwnedFd>>(p: T) -> std::os::fd::OwnedFd {
    p.into()
}

#[cfg(windows)]
fn pipe_handle<T: Into<std::os::windows::io::OwnedHandle>>(
    p: T,
) -> std::os::windows::io::OwnedHandle {
    p.into()
}

fn tempdir() -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let base = std::env::temp_dir().canonicalize().unwrap();
    let dir = base.join(format!(
        "psh-l5-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    // The engine's paths: `/`-separated, without Windows' `\\?\` prefix.
    let s = dir.to_string_lossy().into_owned();
    let s = s.strip_prefix(r"\\?\").map(str::to_string).unwrap_or(s);
    s.replace('\\', "/")
}
