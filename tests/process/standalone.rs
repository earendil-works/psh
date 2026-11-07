//! L5 tests of the standalone `psh` binary's signals (TESTING.md): INT, TERM and HUP
//! cancel the run, the externals' groups are killed (TERM, KILL after the grace
//! period), and psh ends by the signal. Every psh runs in a process group of its own
//! under a deadline; the externals report their pids into files, and every test checks
//! that their groups are gone.

#![cfg(unix)]

use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use psh_os::unix::kill;

const SIGHUP: i32 = 1;
const SIGINT: i32 = 2;
const SIGKILL: i32 = 9;
const SIGTERM: i32 = 15;

/// psh keeps the signals ignored when it starts ignored, as bash does: the tests start it
/// with default dispositions, however the test binary was started (D1304).
fn reset_signals() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| psh_os::unix::signals::reset_ignored(&psh_os::unix::signals::HARNESS_RESET));
}

struct Case {
    dir: PathBuf,
    child: Child,
}

impl Case {
    /// Starts `psh -c script` in a fresh directory (the script's cwd).
    fn start(script: &str) -> Case {
        Case::start_with(Command::new(env!("CARGO_BIN_EXE_psh")).args(["-c", script]))
    }

    fn start_with(cmd: &mut Command) -> Case {
        reset_signals();
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "psh-l5s-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let child = cmd
            .current_dir(&dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .expect("spawn psh");
        Case { dir, child }
    }

    /// Waits until the file `name` holds a pid (written by an external).
    fn pid(&self, name: &str) -> i32 {
        let path = self.dir.join(name);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Ok(s) = std::fs::read_to_string(&path)
                && let Ok(p) = s.trim().parse()
            {
                return p;
            }
            assert!(Instant::now() < deadline, "{name} never appeared");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn signal(&self, sig: i32) {
        kill(self.child.id() as i32, sig).unwrap();
    }

    fn wait(&mut self, within: Duration) -> ExitStatus {
        let deadline = Instant::now() + within;
        loop {
            if let Some(s) = self.child.try_wait().unwrap() {
                return s;
            }
            assert!(Instant::now() < deadline, "psh did not exit");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = kill(-(self.child.id() as i32), SIGKILL);
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Whether process `pid` (or group `-pid`) still exists (zombies included).
fn alive(pid: i32) -> bool {
    match kill(pid, 0) {
        Ok(()) => true,
        Err(e) => e.kind() == std::io::ErrorKind::PermissionDenied,
    }
}

/// Waits until `pid` is gone (killed orphans are reaped by init a moment later).
fn gone(pid: i32, within: Duration) -> bool {
    let deadline = Instant::now() + within;
    while alive(pid) {
        if Instant::now() > deadline {
            let _ = kill(pid, SIGKILL);
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    true
}

/// An external that writes its pid to `file` and sleeps, ignoring TERM if `stubborn`.
fn sleeper(file: &str, stubborn: bool) -> String {
    let trap = if stubborn { "trap \"\" TERM; " } else { "" };
    format!("sh -c '{trap}echo $$ > {file}; exec sleep 30'")
}

fn assert_signaled(s: ExitStatus, sig: i32) {
    assert_eq!(s.signal(), Some(sig), "{s:?}");
}

#[test]
fn term_cancels_a_pipeline_and_psh_dies_by_term() {
    let mut c = Case::start(&format!(
        "{} | {}",
        sleeper("a", false),
        sleeper("b", false)
    ));
    let (a, b) = (c.pid("a"), c.pid("b"));
    let start = Instant::now();
    c.signal(SIGTERM);
    assert_signaled(c.wait(Duration::from_secs(10)), SIGTERM);
    // Externals that obey TERM end at once; psh does not wait out the grace period.
    assert!(start.elapsed() < Duration::from_millis(1500));
    assert!(gone(a, Duration::from_secs(1)) && gone(b, Duration::from_secs(1)));
}

#[test]
fn int_and_hup_too() {
    for sig in [SIGINT, SIGHUP] {
        let mut c = Case::start(&sleeper("a", false));
        let a = c.pid("a");
        c.signal(sig);
        assert_signaled(c.wait(Duration::from_secs(10)), sig);
        assert!(gone(a, Duration::from_secs(1)));
    }
}

#[test]
fn an_external_ignoring_term_is_killed_before_psh_exits() {
    let mut c = Case::start(&sleeper("a", true));
    let a = c.pid("a");
    let start = Instant::now();
    c.signal(SIGTERM);
    assert_signaled(c.wait(Duration::from_secs(15)), SIGTERM);
    // KILL came after the grace period (2 s), before psh ended.
    assert!(start.elapsed() >= Duration::from_millis(1500));
    assert!(gone(a, Duration::from_secs(2)), "the external survived psh");
}

#[test]
fn a_signal_ignored_at_start_stays_ignored() {
    // bash: signals ignored on entry cannot be trapped or reset; children inherit that.
    let script = "sh -c 'kill -INT $$; echo $$ > a; sleep 0.3; echo alive > b'";
    let mut c = Case::start_with(Command::new("/bin/sh").args([
        "-c",
        "trap '' INT; exec \"$0\" -c \"$1\"",
        env!("CARGO_BIN_EXE_psh"),
        script,
    ]));
    c.pid("a");
    c.signal(SIGINT);
    let s = c.wait(Duration::from_secs(10));
    assert_eq!(s.code(), Some(0), "{s:?}");
    assert_eq!(std::fs::read_to_string(c.dir.join("b")).unwrap(), "alive\n");
}

#[test]
fn normal_exit_codes_are_unchanged() {
    let mut c = Case::start("sh -c 'exit 3'");
    assert_eq!(c.wait(Duration::from_secs(10)).code(), Some(3));
    let mut c = Case::start("sh -c 'kill -TERM $$'");
    assert_eq!(c.wait(Duration::from_secs(10)).code(), Some(143));
}

#[test]
fn background_jobs_stay_in_psh_group_and_die_with_it() {
    // As with bash: the job outlives psh, in psh's process group, where the harness's
    // group kill reaches it.
    let mut c = Case::start(&format!("{} & echo started", sleeper("a", false)));
    let a = c.pid("a");
    assert_eq!(c.wait(Duration::from_secs(10)).code(), Some(0));
    let psh = c.child.id() as i32;
    assert!(alive(a) && alive(-psh), "the job left psh's group");
    kill(-psh, SIGTERM).unwrap();
    assert!(gone(a, Duration::from_secs(2)));
}

#[test]
fn what_an_external_left_dies_with_psh_even_if_only_psh_is_signalled() {
    // `sh` leaves a background sleep in psh's group; TERM to psh's pid alone: psh
    // abandons `sh` and, leading its group, ends the group too.
    let mut c = Case::start("sh -c 'sleep 30 & echo $! > a; wait'");
    let a = c.pid("a");
    c.signal(SIGTERM);
    assert_signaled(c.wait(Duration::from_secs(10)), SIGTERM);
    assert!(gone(a, Duration::from_secs(2)), "the grandchild survived");
}

/// Runs `psh -c script` to its end with stdout captured.
fn output(script: &str) -> (ExitStatus, String) {
    reset_signals();
    let mut child = Command::new(env!("CARGO_BIN_EXE_psh"))
        .args(["-c", script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .expect("spawn psh");
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        if Instant::now() > deadline {
            let _ = kill(-(child.id() as i32), SIGKILL);
            panic!("psh did not exit: {script}");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let mut out = String::new();
    use std::io::Read;
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut out)
        .unwrap();
    (status, out)
}

#[test]
fn traps_run_for_signals_from_outside() {
    // Signals reach the shell's traps (D1903), with the OS's numbers mapped.
    let (s, out) = output("trap 'echo t' USR1; /bin/kill -USR1 $$; sleep 0.1; echo after");
    assert_eq!((s.code(), out.as_str()), (Some(0), "t\nafter\n"));
    // A trapped signal ends `wait` with 128+n, then the trap runs.
    let (s, out) = output(
        "trap 'echo t' USR1; sh -c 'sleep 0.2; kill -USR1 '$$ & wait; echo w=$?; wait; echo end",
    );
    assert_eq!((s.code(), out.as_str()), (Some(0), "t\nw=138\nend\n"));
    // An ignored one does nothing.
    let (s, out) = output("trap '' USR1; /bin/kill -USR1 $$; echo after");
    assert_eq!((s.code(), out.as_str()), (Some(0), "after\n"));
}

#[test]
fn kill_dollar_dollar_ends_the_shell_by_the_signal() {
    let (s, out) = output("kill -HUP $$; echo no");
    assert_signaled(s, SIGHUP);
    assert_eq!(out, "");
    let (s, out) = output("trap 'echo bye' EXIT; kill $$; echo no");
    assert_signaled(s, SIGTERM);
    assert_eq!(out, "bye\n");
}

#[test]
fn a_fatal_signal_ends_a_foreground_wait() {
    // No trap: the foreground program is abandoned, psh ends at once by the signal.
    let start = Instant::now();
    let (s, _) = output("sh -c 'sleep 0.2; kill -TERM '$$ & sleep 30; echo no");
    assert_signaled(s, SIGTERM);
    assert!(start.elapsed() < Duration::from_secs(5));
}
