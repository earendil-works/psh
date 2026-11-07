//! Runs a script in-process over the Unix host with OS pipes for stdout and stderr,
//! under a deadline, then checks that no process the host created survived.

use std::io::{Read, Write};
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use psh_engine::std_host::{drive_until, engine_thread};
use psh_engine::{HostFile, Script, Shell, ShellConfig, Step};
use psh_os::unix::{UnixConfig, UnixFile, UnixHost};

/// Everything a test may wait for, in total.
const DEADLINE: Duration = Duration::from_secs(30);
/// TERM → KILL in tests: short, so leak checks after a cancel finish quickly.
const GRACE: Duration = Duration::from_millis(300);

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
    expect_leak: bool,
}

pub fn run(script: &str) -> Out {
    Run::new(script).go()
}

impl Run {
    pub fn new(script: &str) -> Run {
        Run {
            script: script.to_string(),
            stdin: None,
            cancel_after: None,
            expect_leak: false,
        }
    }

    /// The script leaves a process behind on purpose: the leak check must see it (and
    /// then kills it).
    pub fn expect_leak(mut self) -> Run {
        self.expect_leak = true;
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
        let deadline = start + DEADLINE;
        let script = self.script;
        let cancel_after = self.cancel_after;
        let expect_leak = self.expect_leak;
        let cwd = dir.clone();
        let engine = engine_thread(move || {
            let host = Rc::new(UnixHost::new(UnixConfig {
                grace: GRACE,
                pid: Some(99_999_999),
                ..UnixConfig::default()
            }));
            let mut cfg = ShellConfig::new(host.clone());
            let path = std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".into());
            cfg.env = vec![
                (b"PATH".to_vec(), path.into_bytes()),
                (b"TMPDIR_CASE".to_vec(), cwd.clone().into_bytes()),
                (b"LC_ALL".to_vec(), b"C".to_vec()),
            ];
            cfg.cwd = cwd.into_bytes();
            let file = |f: std::fs::File| Some(Rc::new(UnixFile::new(f)) as Rc<dyn HostFile>);
            cfg.stdin = file(std::fs::File::from(std::os::fd::OwnedFd::from(in_r)));
            cfg.stdout = file(std::fs::File::from(std::os::fd::OwnedFd::from(out_w)));
            cfg.stderr = file(std::fs::File::from(std::os::fd::OwnedFd::from(err_w)));
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
            // The leak check: every group this host made empties (abandoned ones after
            // TERM, the grace period and KILL).
            if expect_leak {
                let left = host.leftovers();
                for g in &left {
                    let _ = psh_os::unix::kill(-g, sigkill());
                }
                return (
                    status,
                    done_after,
                    if left.is_empty() { vec![-1] } else { vec![] },
                );
            }
            let leak_deadline = Instant::now() + GRACE + Duration::from_secs(5);
            let leftovers = loop {
                let left = host.leftovers();
                if left.is_empty() || Instant::now() > leak_deadline {
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
            String::from_utf8_lossy(&rx.recv_timeout(left).expect("output never ended"))
                .into_owned()
        };
        let stdout = wait(out_rx);
        let stderr = wait(err_rx);
        if let Some(f) = feeder {
            let _ = f.join();
        }
        let _ = std::fs::remove_dir_all(&dir);
        if expect_leak {
            assert!(
                leftovers.is_empty(),
                "the leak check missed a leftover process"
            );
        } else {
            assert!(leftovers.is_empty(), "leaked process groups: {leftovers:?}");
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

fn sigkill() -> i32 {
    psh_engine::host::SIGKILL
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
    dir.to_string_lossy().into_owned()
}
