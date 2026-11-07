//! L5 process tests on Windows (TESTING.md): the engine over the Windows host,
//! in-process, with `psh-test-helper` as the external program (`$H`) since Windows has
//! no `sleep`, `cat` or `yes`. Every test runs under a deadline and ends with a leak
//! check: no job the host created may still have a live process.

#![cfg(windows)]

use std::io::{Read, Write};
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use psh_engine::std_host::{drive_until, engine_thread};
use psh_engine::{HostFile, Script, Shell, ShellConfig, Step};
use psh_os::windows::{Leftovers, WinFile, WindowsConfig, WindowsHost, settle, shell_env};

/// Everything a test may wait for, in total.
const DEADLINE: Duration = Duration::from_secs(60);
/// TERM → termination in tests: short, so leak checks after a cancel finish quickly.
const GRACE: Duration = Duration::from_millis(300);

#[derive(Debug)]
struct Out {
    status: i32,
    stdout: String,
    stderr: String,
    done_after: Duration,
}

struct Run {
    script: String,
    stdin: Option<Vec<u8>>,
    cancel_after: Option<Duration>,
    leftovers: Leftovers,
    expect_survivors: bool,
}

fn run(script: &str) -> Out {
    Run::new(script).go()
}

/// The helper as the engine writes paths.
fn helper() -> String {
    env!("CARGO_BIN_EXE_psh-test-helper").replace('\\', "/")
}

fn tempdir() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "psh-l5w-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

impl Run {
    fn new(script: &str) -> Run {
        Run {
            script: script.to_string(),
            stdin: None,
            cancel_after: None,
            leftovers: Leftovers::Kill,
            expect_survivors: false,
        }
    }

    fn stdin(mut self, data: &str) -> Run {
        self.stdin = Some(data.as_bytes().to_vec());
        self
    }

    fn cancel_after(mut self, d: Duration) -> Run {
        self.cancel_after = Some(d);
        self
    }

    fn keep_leftovers(mut self) -> Run {
        self.leftovers = Leftovers::Keep;
        self.expect_survivors = true;
        self
    }

    fn go(self) -> Out {
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
        let Run {
            script,
            cancel_after,
            leftovers,
            expect_survivors,
            ..
        } = self;
        let cwd = dir.to_string_lossy().replace('\\', "/");
        let engine = engine_thread(move || {
            let host = Rc::new(WindowsHost::new(WindowsConfig {
                grace: GRACE,
                pid: Some(99_999_999),
                leftovers,
                ..WindowsConfig::default()
            }));
            let watch = host.killer();
            let mut cfg = ShellConfig::new(host.clone());
            let mut env: Vec<(Vec<u8>, Vec<u8>)> = std::env::vars()
                .map(|(k, v)| (k.into_bytes(), v.into_bytes()))
                .collect();
            env.push((b"H".to_vec(), helper().into_bytes()));
            env.push((b"TMPDIR_CASE".to_vec(), cwd.clone().into_bytes()));
            cfg.env = shell_env(env);
            cfg.cwd = cwd.into_bytes();
            let file = |h: std::os::windows::io::OwnedHandle| {
                Some(Rc::new(WinFile::new(h)) as Rc<dyn HostFile>)
            };
            cfg.stdin = file(in_r.into());
            cfg.stdout = file(out_w.into());
            cfg.stderr = file(err_w.into());
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
            // The leak check: dropping the host ends the run; then every job it made
            // must empty.
            drop(host);
            settle(GRACE + Duration::from_secs(2));
            let leak_deadline = Instant::now() + Duration::from_secs(5);
            let left = loop {
                let left = watch.leftovers();
                if left.is_empty() || expect_survivors || Instant::now() > leak_deadline {
                    break left;
                }
                thread::sleep(Duration::from_millis(20));
            };
            if !left.is_empty() {
                watch.kill_all();
                settle(GRACE + Duration::from_secs(2));
            }
            (status, done_after, left)
        })
        .unwrap();
        let (status, done_after, left) = engine.join().expect("engine thread");
        let wait = |rx: mpsc::Receiver<Vec<u8>>| {
            let left = deadline.saturating_duration_since(Instant::now());
            String::from_utf8_lossy(&rx.recv_timeout(left).expect("output never ended"))
                .replace("\r\n", "\n")
        };
        let stdout = wait(out_rx);
        let stderr = wait(err_rx);
        if let Some(f) = feeder {
            let _ = f.join();
        }
        let _ = std::fs::remove_dir_all(&dir);
        if expect_survivors {
            assert!(!left.is_empty(), "nothing survived the run");
        } else {
            assert!(left.is_empty(), "leaked jobs: {left:?}");
        }
        Out {
            status: status.expect("the script hit the test deadline"),
            stdout,
            stderr,
            done_after,
        }
    }
}

#[test]
fn exit_codes_and_crashes() {
    let r = run("\"$H\" exit 3; echo $?; \"$H\" crash; echo $?");
    assert_eq!(r.stdout, "3\n134\n", "{r:?}");
}

#[test]
fn external_pipeline_moves_data_in_order() {
    let r = run("x=$(\"$H\" bytes 300000 | \"$H\" cat | \"$H\" cat); echo ${#x}");
    assert_eq!(r.stdout, "300000\n", "{r:?}");
}

#[test]
fn yes_into_a_reader_that_stops_ends() {
    let r = run("\"$H\" yes | { read -r a; echo \"$a\"; }; echo done");
    assert_eq!(r.stdout, "y\ndone\n", "{r:?}");
}

#[test]
fn here_doc_past_the_pipe_buffer_into_an_external() {
    let body = "line\n".repeat(30_000);
    let r = run(&format!("x=$(\"$H\" cat <<EOF\n{body}EOF\n); echo ${{#x}}"));
    assert_eq!(r.stdout, "149999\n", "{r:?}");
}

#[test]
fn cancel_kills_a_tree_with_grandchildren() {
    let r = Run::new("\"$H\" tree 30")
        .cancel_after(Duration::from_millis(500))
        .go();
    assert_eq!(r.status, 143, "{r:?}");
    assert!(r.done_after < Duration::from_secs(10), "{r:?}");
}

#[test]
fn cancel_mid_pipeline_leaves_no_process() {
    let r = Run::new("\"$H\" yes | \"$H\" cat | \"$H\" stubborn 30")
        .cancel_after(Duration::from_millis(500))
        .go();
    assert_eq!(r.status, 143, "{r:?}");
}

#[test]
fn a_normal_end_kills_background_jobs() {
    let r = run("\"$H\" stubborn 30 & \"$H\" tree 30 & echo started");
    assert_eq!(r.stdout, "started\n", "{r:?}");
}

#[test]
fn keep_leaves_background_jobs_running() {
    let r = Run::new("\"$H\" sleep 30 & echo started")
        .keep_leftovers()
        .go();
    assert_eq!(r.stdout, "started\n", "{r:?}");
}

#[test]
fn cmd_shims_get_their_arguments() {
    // npm-style: a .cmd on PATH that runs a program with `%*`.
    let r = run(
        "cd \"$TMPDIR_CASE\" && printf '@\"%s\" args %%*\\r\\n' \"$H\" > shim.cmd && \
         PATH=\"$TMPDIR_CASE;$PATH\" && shim 'a b' 'x&y' '100%' 'q\"uote'",
    );
    assert_eq!(r.stdout, "[a b]\n[x&y]\n[100%]\n[q\"uote]\n", "{r:?}");
}

#[test]
fn arguments_reach_programs_intact() {
    let r = run("\"$H\" args '' 'a b' 'back\\\\' 'q\"t' 'tab\tx'");
    assert_eq!(
        r.stdout, "[]\n[a b]\n[back\\\\]\n[q\"t]\n[tab\tx]\n",
        "{r:?}"
    );
}

#[test]
fn stdin_of_externals_is_the_shells() {
    let r = Run::new("read -r a; echo \"got $a\"; \"$H\" cat")
        .stdin("one\ntwo\n")
        .go();
    assert_eq!(r.stdout, "got one\ntwo\n", "{r:?}");
}

#[test]
fn windows_paths() {
    let r = run("cd \"$TMPDIR_CASE\" && echo x > a.txt && \
         d=$(pwd); cd /c/ && cd \"$d\" && \"$H\" cat < a.txt && \
         echo y > /dev/null && \"$H\" cat < \"$TMPDIR_CASE/a.txt\"");
    assert_eq!(r.stdout, "x\nx\n", "{r:?}");
}

#[test]
fn crlf_scripts_run() {
    let r = run("echo a\r\necho b\r\n");
    assert_eq!(r.stdout, "a\nb\n", "{r:?}");
}

#[test]
fn command_not_found() {
    let r = run("no_such_command_psh_l5; echo $?");
    assert_eq!(r.stdout, "127\n", "{r:?}");
    assert!(r.stderr.contains("command not found"), "{r:?}");
}

#[test]
fn many_short_externals() {
    let script: String = (1..=50).map(|i| format!("\"$H\" args {i}\n")).collect();
    let r = run(&script);
    let want: String = (1..=50).map(|i| format!("[{i}]\n")).collect();
    assert_eq!(r.stdout, want, "{r:?}");
}
