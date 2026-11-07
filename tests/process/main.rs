//! L5 process tests (TESTING.md): the engine over the real OS host, in-process. Every
//! test runs under a deadline and ends with a leak check: no process group the host
//! created may still have a live member.

#![cfg(unix)]

mod harness;

use std::time::{Duration, Instant};

use harness::{Run, run};

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
    let r = Run::new("echo first; sh -c 'sleep 0.3; echo late' &").go();
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
fn the_leak_check_sees_a_leftover() {
    // The grandchild stays in the external's session after the command is done; the
    // check must notice (and kills it).
    let r = Run::new("sh -c 'sleep 30 >/dev/null 2>&1 &'")
        .expect_leak()
        .go();
    assert_eq!(r.status, 0, "{r:?}");
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
