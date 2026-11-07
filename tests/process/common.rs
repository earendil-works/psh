//! Cases for every platform, with `psh-test-helper` as the external program (`$H`).

use std::time::Duration;

use super::harness::{Run, run};

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
        .expect_survivors()
        .go();
    assert_eq!(r.stdout, "started\n", "{r:?}");
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

#[test]
fn the_embedder_kills_the_run_from_another_thread() {
    // The foreground external dies with the run, so the script goes on.
    let r = Run::new("\"$H\" stubborn 30 & \"$H\" sleep 30; echo after $?")
        .kill_run_after(Duration::from_millis(500))
        .go();
    assert_eq!(r.stdout, "after 143\n", "{r:?}");
}
