//! The utilities on the engine's test host (in-memory fs, fake programs), each script
//! also run with every host operation pending once. Behavior against real GNU and BSD
//! is L4 (tests/utils, psh-spec).

use std::rc::Rc;

use psh_engine::testhost::TestHost;
use psh_engine::{Script, Shell, Step};

struct Out {
    status: i32,
    stdout: String,
    stderr: String,
}

fn run_on(src: &str, stdin: &[u8], latency: bool) -> Out {
    let host = Rc::new(TestHost::new());
    host.set_latency(latency);
    host.set_stdin(stdin);
    host.fs.write_file("/tmp/f", b"one\ntwo\nthree\nfour\n");
    host.fs.write_file("/tmp/g", b"a b c\n");
    host.fs.write_file("/tmp/nonl", b"x\ny");
    host.fs.mkdir_all("/tmp/dir");
    let mut cfg = host.config();
    cfg.utilities = crate::policy(false);
    let mut shell = Shell::new(cfg);
    shell.start(Script::command(src));
    let status = loop {
        match shell.step(13) {
            Step::Done(s) => break s,
            Step::Yield => {}
            other => panic!("{other:?} running {src:?}"),
        }
    };
    assert_eq!(shell.task_count(), 0, "tasks left after {src:?}");
    Out {
        status,
        stdout: String::from_utf8_lossy(&host.stdout()).into_owned(),
        stderr: String::from_utf8_lossy(&host.stderr()).into_owned(),
    }
}

fn run_in(src: &str, stdin: &str) -> Out {
    let a = run_on(src, stdin.as_bytes(), false);
    let b = run_on(src, stdin.as_bytes(), true);
    assert_eq!(
        (a.status, &a.stdout, &a.stderr),
        (b.status, &b.stdout, &b.stderr),
        "latency changed {src:?}"
    );
    a
}

#[track_caller]
fn check(src: &str, stdout: &str) {
    let o = run_in(src, "");
    assert_eq!(o.stdout, stdout, "stdout of {src:?} (stderr: {})", o.stderr);
}

#[track_caller]
fn check_all(src: &str, stdin: &str, status: i32, stdout: &str, stderr: &str) {
    let o = run_in(src, stdin);
    assert_eq!(
        (o.status, o.stdout.as_str(), o.stderr.as_str()),
        (status, stdout, stderr),
        "{src:?}"
    );
}

/// Utilities the classifier handles with rules in code (wrappers and the like run other
/// commands), not with a table entry (D1090).
const CLASSIFIED_IN_CODE: &[&str] = &["timeout", "env", "xargs", "nice", "nohup", "find"];

#[test]
fn specs_match_the_classifier() {
    let mut problems = Vec::new();
    for spec in crate::SPECS {
        if CLASSIFIED_IN_CODE.contains(&spec.name) {
            continue;
        }
        problems.extend(crate::opts::check_against_classifier(spec));
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_entry_has_a_spec() {
    for e in crate::REGISTRY {
        assert!(
            crate::SPECS.iter().any(|s| s.name == e.names[0]),
            "{}",
            e.names[0]
        );
    }
}

#[test]
fn cat() {
    check("cat f", "one\ntwo\nthree\nfour\n");
    check_all("cat", "in\n", 0, "in\n", "");
    check("cat g - nonl", "a b c\nx\ny");
    check("echo mid | cat g - g", "a b c\nmid\na b c\n");
    check("cat -n g nonl", "     1\ta b c\n     2\tx\n     3\ty");
    check("printf 'a\\n\\n\\n\\nb\\n' | cat -s", "a\n\nb\n");
    check("printf 'a\\n\\nb\\n' | cat -b", "     1\ta\n\n     2\tb\n");
    check("printf 'a\\tb\\001\\n' | cat -A", "a^Ib^A$\n");
    check("printf 'a\\tb\\n' | cat -T", "a^Ib\n");
    check("printf 'a\\n' | cat -E", "a$\n");
    check_all(
        "cat missing g; echo $?",
        "",
        0,
        "a b c\n1\n",
        "cat: missing: No such file or directory\n",
    );
    check_all("cat dir", "", 1, "", "cat: dir: Is a directory\n");
}

#[test]
fn head() {
    check("head -n 2 f", "one\ntwo\n");
    check("head -2 f", "one\ntwo\n");
    check("head -c 5 f", "one\nt");
    check("head -n -1 f", "one\ntwo\nthree\n");
    check("head -c -15 f", "one\n");
    check(
        "head f g",
        "==> f <==\none\ntwo\nthree\nfour\n\n==> g <==\na b c\n",
    );
    check("head -q -n1 f g", "one\na b c\n");
    check("head -v -n1 g", "==> g <==\na b c\n");
    check_all("printf 'a\\nb\\nc\\n' | head -n1", "", 0, "a\n", "");
    // head stops reading: the writer gets EPIPE, nothing hangs.
    check(
        "i=0; while :; do echo $i; i=$((i+1)); done | head -n 3",
        "0\n1\n2\n",
    );
    check_all(
        "head -n x f",
        "",
        1,
        "",
        "head: invalid number of lines: 'x'\n",
    );
    // An unknown option runs the real program (the test host has a fake `head`), or
    // fails clearly when there is none.
    check_all("head --frobnicate f", "", 0, "", "");
    check_all(
        "tail --frobnicate f",
        "",
        2,
        "",
        "tail: unsupported option '--frobnicate' (psh built-in tail)\n",
    );
    check_all(
        "head -n",
        "",
        1,
        "",
        "head: option requires an argument -- 'n'\nTry 'head --help' for more information.\n",
    );
}

#[test]
fn tail() {
    check("tail -n 2 f", "three\nfour\n");
    check("tail -1 f", "four\n");
    check("tail -n +3 f", "three\nfour\n");
    check("tail +3 f", "three\nfour\n");
    check("tail -c 3 f", "ur\n");
    check("tail -c +17 f", "ur\n");
    check("tail -r f", "four\nthree\ntwo\none\n");
    check("tail -n 2 -r f", "four\nthree\n");
    check("tail nonl", "x\ny");
    check("echo q | tail", "q\n");
    check("tail -n1 f g", "==> f <==\nfour\n\n==> g <==\na b c\n");
}

#[test]
fn wc() {
    check("wc f", " 4  4 19 f\n");
    check("wc -l f", "4 f\n");
    check("wc -l < f", "4\n");
    check("printf 'a b\\nc\\n' | wc", "      2       3       6\n");
    check("printf 'x\\n' | wc -l", "1\n");
    check("wc f g", " 4  4 19 f\n 1  3  6 g\n 5  7 25 total\n");
    check("cat f | wc -c f -", "     19 f\n     19 -\n     38 total\n");
    check("wc -L f", "5 f\n");
    check("wc --total=only -l f g", "5\n");
}

#[test]
fn sleep_and_timeout() {
    check_all("sleep 0.01; echo done", "", 0, "done\n", "");
    check_all(
        "sleep 1m2",
        "",
        1,
        "",
        "sleep: invalid time interval '1m2'\nTry 'sleep --help' for more information.\n",
    );
    // The test host's time is virtual (a sleep ends at once): `hang` never ends.
    check_all("timeout 0.05 hang; echo $?", "", 0, "124\n", "");
    check_all("timeout -s KILL 0.05 hang; echo $?", "", 0, "137\n", "");
    check_all(
        "timeout --preserve-status 0.05 hang; echo $?",
        "",
        0,
        "143\n",
        "",
    );
    check_all("timeout 5 echo fast; echo $?", "", 0, "fast\n0\n", "");
    check_all("timeout 1 false; echo $?", "", 0, "1\n", "");
    check_all("gtimeout 0 true; echo $?", "", 0, "0\n", "");
    check_all(
        "timeout -v 0.01 hang",
        "",
        124,
        "",
        "timeout: sending signal TERM to command 'hang'\n",
    );
    check_all(
        "timeout x sleep 1",
        "",
        125,
        "",
        "timeout: invalid time interval 'x'\nTry 'timeout --help' for more information.\n",
    );
    // An infinite in-process loop is stopped too.
    check_all(
        "timeout 0.05 tail -f f > /dev/null; echo $?",
        "",
        0,
        "124\n",
        "",
    );
}
