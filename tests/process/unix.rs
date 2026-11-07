//! The Unix cases: they use the system's own programs (`sh`, `yes`, `seq`, `cat`, ...).

use std::time::{Duration, Instant};

use super::harness::{Run, run};

#[test]
fn yes_head_terminates_by_sigpipe() {
    let r = run("yes | head -1; echo status=$?");
    assert_eq!(r.stdout, "y\nstatus=0\n", "{r:?}");
}

#[test]
fn external_pipeline_moves_data_in_order() {
    let r = run("seq 1 200000 | cat | cat | tail -1; seq 1 200000 | wc -l | tr -d ' '");
    assert_eq!(r.stdout, "200000\n200000\n", "{r:?}");
}

#[test]
fn here_doc_past_the_pipe_buffer_into_an_external() {
    let line = "x".repeat(99);
    let body: String = (0..2000).map(|_| format!("{line}\n")).collect();
    let script =
        format!("cat <<'EOF' | wc -c | tr -d ' '\n{body}EOF\nwc -l <<EOF | tr -d ' '\n{body}EOF\n");
    let r = run(&script);
    assert_eq!(r.stdout, "200000\n2000\n", "{r:?}");
}

#[test]
fn here_string_and_builtin_into_an_external() {
    let r = run("cat <<< hello; printf '%s\\n' a b c | sort -r");
    assert_eq!(r.stdout, "hello\nc\nb\na\n", "{r:?}");
}

#[test]
fn signal_exit_codes_are_128_plus_n() {
    let r = run(
        "sh -c 'kill -TERM $$'; echo $?; sh -c 'kill -KILL $$'; echo $?; sh -c 'kill -INT $$'; echo $?; sh -c 'exit 7'; echo $?",
    );
    assert_eq!(r.stdout, "143\n137\n130\n7\n", "{r:?}");
}

#[test]
fn cancel_mid_pipeline_leaves_no_process() {
    let start = Instant::now();
    let r = Run::new("echo start; sleep 30 | sh -c 'trap \"\" TERM; sleep 30' | cat; echo never")
        .cancel_after(Duration::from_millis(300))
        .go();
    assert_eq!(r.status, 143, "{r:?}");
    assert_eq!(r.stdout, "start\n", "{r:?}");
    // The leak check (in `go`) waited for TERM, the grace period and KILL.
    assert!(start.elapsed() < Duration::from_secs(10));
}

#[test]
fn cancel_in_a_substitution_leaves_no_process() {
    let r = Run::new("x=$(sleep 30; echo no); echo $x")
        .cancel_after(Duration::from_millis(200))
        .go();
    assert_eq!(r.status, 143, "{r:?}");
}

#[test]
fn output_after_the_shells_exit_is_collected() {
    // The shell is done at once; a background process still writes to its stdout,
    // and the reader gets that at the end of the pipe, as with bash -c.
    let r = Run::new("echo first; sh -c 'sleep 0.3; echo late' &")
        .keep_leftovers()
        .go();
    assert_eq!(r.status, 0);
    assert_eq!(r.stdout, "first\nlate\n", "{r:?}");
    assert!(r.done_after < Duration::from_millis(250), "{r:?}");
}

#[test]
fn grandchild_holding_the_pipe_is_waited_for() {
    // The command is complete at the end of its output, not at its exit.
    let r = run("x=$(sh -c '(sleep 0.2; echo inner) & echo outer'); echo \"$x\"");
    assert_eq!(r.stdout, "outer\ninner\n", "{r:?}");
}

#[test]
fn extra_descriptors_reach_externals() {
    let r = run("exec 3>&1; sh -c 'echo to3 >&3'; sh -c 'echo err >&2' 2>&1 | tr a-z A-Z");
    assert_eq!(r.stdout, "to3\nERR\n", "{r:?}");
}

#[test]
fn fifo_between_a_background_vproc_and_an_external() {
    let r = run("cd \"$TMPDIR_CASE\" && mkfifo f && { echo hi > f & } && cat < f && rm f");
    assert_eq!(r.stdout, "hi\n", "{r:?}");
}

#[test]
fn stdin_of_externals_is_the_shells() {
    let r = Run::new("head -1; cat").stdin("one\ntwo\n").go();
    // head may read more than its line from a pipe; what matters is that both see the
    // shell's stdin and nothing hangs.
    assert!(r.stdout.starts_with("one\n"), "{r:?}");
}

#[test]
fn many_short_externals() {
    let script: String = (1..=100)
        .map(|i| format!("true | /bin/echo {i} | cat\n"))
        .collect();
    let r = run(&script);
    let want: String = (1..=100).map(|i| format!("{i}\n")).collect();
    assert_eq!(r.stdout, want, "{r:?}");
}

#[test]
fn command_not_found() {
    let r = run("no_such_command_psh_l5; echo $?");
    assert_eq!(r.stdout, "127\n", "{r:?}");
    assert!(r.stderr.contains("command not found"), "{r:?}");
}

#[test]
fn a_vproc_that_never_reads_ends_the_writer_by_sigpipe() {
    let r = run("yes | true; echo done");
    assert_eq!(r.stdout, "done\n", "{r:?}");
}

#[test]
fn a_big_here_doc_into_an_external_that_does_not_read() {
    let body = "y\n".repeat(100_000);
    let r = run(&format!("true <<EOF\n{body}EOF\necho done"));
    assert_eq!(r.stdout, "done\n", "{r:?}");
}

#[test]
fn pumps_on_both_sides_of_a_brace_group() {
    let r = run("seq 1 100000 | { cat; } | { cat; cat; } | wc -l | tr -d ' '");
    assert_eq!(r.stdout, "100000\n", "{r:?}");
}

#[test]
fn exec_an_external() {
    let r = run("exec sh -c 'echo replaced; exit 5'; echo never");
    assert_eq!((r.status, r.stdout.as_str()), (5, "replaced\n"), "{r:?}");
}

#[test]
fn cancel_while_a_fifo_open_waits() {
    let start = Instant::now();
    let r = Run::new("cd \"$TMPDIR_CASE\" && mkfifo f && cat < f")
        .cancel_after(Duration::from_millis(200))
        .go();
    assert_eq!(r.status, 143, "{r:?}");
    assert!(start.elapsed() < Duration::from_secs(5));
}

#[test]
fn concurrent_shells_do_not_interfere() {
    let handles: Vec<_> = (0..8)
        .map(|i| {
            std::thread::spawn(move || {
                let r = run(&format!(
                    "seq 1 {} | cat | wc -l | tr -d ' '",
                    1000 * (i + 1)
                ));
                assert_eq!(r.stdout, format!("{}\n", 1000 * (i + 1)), "{r:?}");
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
}

// What survives a run (D1067): only detached and disowned processes. A loop that
// ignores TERM, started in the background, is what the gates once left behind by the
// dozen.
const STUBBORN_LOOP: &str = "sh -c 'trap \"\" TERM; while :; do sleep 0.05; done'";

#[test]
fn a_normal_end_kills_background_jobs() {
    let r = run(&format!("{STUBBORN_LOOP} & {STUBBORN_LOOP} & echo started"));
    assert_eq!(r.stdout, "started\n", "{r:?}");
}

#[test]
fn a_normal_end_kills_what_a_finished_external_left() {
    let r = run("sh -c 'sleep 30 >/dev/null 2>&1 &'; echo done");
    assert_eq!(r.stdout, "done\n", "{r:?}");
}

#[test]
fn a_cancel_kills_background_jobs() {
    let r = Run::new(&format!("{STUBBORN_LOOP} & sleep 30"))
        .cancel_after(Duration::from_millis(200))
        .go();
    assert_eq!(r.status, 143, "{r:?}");
}

#[test]
fn the_embedder_kills_the_run_from_another_thread() {
    // The foreground external dies with the run, so the script goes on.
    let r = Run::new(&format!("{STUBBORN_LOOP} & sleep 30; echo after $?"))
        .kill_run_after(Duration::from_millis(300))
        .go();
    assert_eq!(r.stdout, "after 143\n", "{r:?}");
}

#[test]
fn keep_leaves_background_jobs_running() {
    let r = Run::new("sh -c 'sleep 30' & echo started")
        .keep_leftovers()
        .expect_survivors()
        .go();
    assert_eq!(r.stdout, "started\n", "{r:?}");
}

#[test]
fn builtins_and_externals_share_a_pipe_in_order() {
    // The pipe may become an OS pipe in the middle (D1068): order must hold either way.
    for _ in 0..20 {
        let r = run(
            "{ echo a; seq 2 3; echo b; sh -c 'echo c'; echo d; } | cat; seq 1 3 | { cat; echo e; } | tr -d '\\n'",
        );
        assert_eq!(r.stdout, "a\n2\n3\nb\nc\nd\n123e", "{r:?}");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn two_externals_share_one_os_pipe() {
    // (dash moves its own fds around for `>&2`: ask from outside.)
    let r = run("cd \"$TMPDIR_CASE\"; \
        sh -c 'echo $$ > a; sleep 0.5' | sh -c 'echo $$ > b; cat' & \
        sleep 0.3; readlink /proc/$(cat a)/fd/1 /proc/$(cat b)/fd/0");
    let lines: Vec<&str> = r.stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{r:?}");
    assert!(
        lines[0].starts_with("pipe:") && lines[0] == lines[1],
        "{r:?}"
    );
}

// Jobs (D1900): `&`, `wait`, `$!`, `kill`.

#[test]
fn background_and_wait() {
    let r = run(
        "sleep 0.2 & echo started; wait $!; echo $?; (exit 3) & wait; echo $?; wait %1; echo $?",
    );
    assert_eq!(r.stdout, "started\n0\n0\n127\n", "{r:?}");
}

#[test]
fn bang_is_the_programs_real_pid() {
    let r = run("cd \"$TMPDIR_CASE\"; sh -c 'echo $$ > p; exec sleep 30' & \
         while [ ! -s p ]; do sleep 0.02; done; \
         test \"$(cat p)\" = \"$!\" && echo same; kill $!; wait $!; echo $?");
    assert_eq!(r.stdout, "same\n143\n", "{r:?}");
}

#[test]
fn kill_reaches_the_jobs_whole_group() {
    let r = run(
        "cd \"$TMPDIR_CASE\"; sh -c 'sleep 30 & echo $! > gc; wait' & \
         while [ ! -s gc ]; do sleep 0.02; done; \
         kill $!; wait $!; echo $?; sleep 0.2; kill -0 $(cat gc) 2>/dev/null || echo gone",
    );
    assert_eq!(r.stdout, "143\ngone\n", "{r:?}");
}

/// A pid file outside the case directory (which the harness removes).
fn pidfile(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("psh-l5-{tag}-{}", std::process::id()))
}

fn survived(file: &std::path::Path) {
    let pid: i32 = std::fs::read_to_string(file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let _ = std::fs::remove_file(file);
    let alive = psh_os::unix::kill(pid, 0).is_ok();
    let _ = psh_os::unix::kill(pid, 9);
    assert!(alive, "the detached process did not survive the run");
}

#[test]
fn nohup_and_disown_survive_the_runs_end() {
    let (a, b) = (pidfile("nohup"), pidfile("disown"));
    let r = run(&format!(
        "nohup sh -c 'echo $$ > {}; exec sleep 30' >/dev/null 2>&1 & \
         sh -c 'echo $$ > {}; exec sleep 30' >/dev/null 2>&1 & disown; \
         while [ ! -s {} ] || [ ! -s {} ]; do sleep 0.02; done; echo ok",
        a.display(),
        b.display(),
        a.display(),
        b.display()
    ));
    assert_eq!(r.stdout, "ok\n", "{r:?}");
    survived(&a);
    survived(&b);
}

#[test]
fn a_builtin_loop_into_head_ends_by_sigpipe() {
    // The head stage holds no pipe end of its own (D1902): once head exits, the loop's
    // next write fails and ends the stage with 141.
    let r = run("while :; do echo y; done | head -n 1; echo ${PIPESTATUS[0]} ${PIPESTATUS[1]}");
    assert_eq!(r.stdout, "y\n141 0\n", "{r:?}");
}

#[test]
fn ignored_traps_pass_on_to_children() {
    let r = run("trap '' USR1; sh -c 'kill -USR1 $$; echo survived'");
    assert_eq!(r.stdout, "survived\n", "{r:?}");
}

#[test]
fn a_file_without_an_interpreter_line_runs_in_psh() {
    // bash runs a file execve rejects itself; so does psh (D1300), afresh: only
    // exported variables, no functions.
    let r = run(
        "cd \"$TMPDIR_CASE\"; printf 'echo \"$0 $1 x=$X y=${Y-unset}\"; type f 2>/dev/null || echo nof; exit 4\\n' > s; chmod +x s; \
         Y=1; export X=2; f() { :; }; ./s a; echo st=$?",
    );
    assert_eq!(r.stdout, "./s a x=2 y=unset\nnof\nst=4\n", "{r:?}");
}

#[test]
fn a_foreground_programs_signal_death_is_reported() {
    // As bash prints it non-interactively (D1303): TERM bare, the others with the line
    // and pid; INT and PIPE not at all; nothing for `$(…)`.
    let r = run(
        "sh -c 'kill -TERM $$'; sh -c 'kill -9 $$'; sh -c 'kill -INT $$'; x=$(sh -c 'kill -HUP $$'); echo after",
    );
    assert_eq!(r.stdout, "after\n", "{r:?}");
    let lines: Vec<&str> = r.stderr.lines().collect();
    assert_eq!(lines.len(), 2, "{r:?}");
    assert_eq!(lines[0], "Terminated                 sh -c 'kill -TERM $$'");
    assert!(lines[1].starts_with("psh: line 1: "), "{r:?}");
    assert!(
        lines[1].ends_with(" Killed                     sh -c 'kill -9 $$'"),
        "{r:?}"
    );
}

#[test]
fn signal_numbers_are_linuxs_everywhere() {
    // One numbering on every system (D1904): 10 is USR1 on macOS too, and `$?` of a
    // death by it is 138.
    let r =
        run("kill -l 10; sh -c 'kill -USR1 $$'; echo $?; sleep 30 & kill -10 $!; wait $!; echo $?");
    assert_eq!(r.stdout, "USR1\n138\n138\n", "{r:?}");
    // A signal the system lacks fails clearly.
    let r = run("kill -STKFLT 99999; echo $?");
    assert_eq!(r.stdout, "1\n", "{r:?}");
    if cfg!(target_os = "macos") {
        assert!(
            r.stderr
                .contains("kill: SIGSTKFLT: no such signal on this system"),
            "{r:?}"
        );
    }
}

#[test]
fn stray_descriptors_do_not_reach_forked_externals() {
    // A descriptor another thread left without close-on-exec (std on macOS creates
    // pipes with pipe + fcntl) must not live on in an external started by the fork path
    // (`trap ''` makes psh fork), or a reader of that pipe never sees its end (D1905).
    let (r, w) = std::io::pipe().expect("a pipe");
    let fd = std::os::fd::AsRawFd::as_raw_fd(&w);
    // SAFETY: fcntl on a descriptor this test owns.
    unsafe { libc::fcntl(fd, libc::F_SETFD, 0) };
    let out = run(&format!(
        "trap '' USR1; sh -c 'if [ -e /dev/fd/{fd} ]; then echo leaked; else echo closed; fi'"
    ));
    drop((r, w));
    assert_eq!(out.stdout, "closed\n", "{out:?}");
}
