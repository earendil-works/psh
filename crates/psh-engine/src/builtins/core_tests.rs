//! Tests of the core builtins (`read`, `eval`, `source`, `command`, `type`, ...) on the
//! test host.

use crate::tests::{check, check_status, run, run_setup};

#[test]
fn read_splits_and_escapes() {
    check("read x y <<< '  a  b c  '; echo \"[$x][$y]\"", "[a][b c]\n");
    check("read <<< '  a b  '; echo \"[$REPLY]\"", "[  a b  ]\n");
    check("read x <<< 'a\\ b\\\\'; echo \"[$x]\"", "[a b\\]\n");
    check("read -r x <<< 'a\\ b'; echo \"[$x]\"", "[a\\ b]\n");
    check("IFS=: read a b <<< '1:2:'; echo \"[$a][$b]\"", "[1][2]\n");
    check("IFS= read a <<< '  x  '; echo \"[$a]\"", "[  x  ]\n");
    check("read -n 2 x <<< abc; echo $x", "ab\n");
    check("read -N 4 x y <<< 'a b c'; echo \"[$x][$y]\"", "[a b ][]\n");
    check("read -d , x <<< 'a,b'; echo $x", "a\n");
    check(
        "printf 'a\\nb' | { read x; read y; echo \"$? $x $y\"; }",
        "1 a b\n",
    );
}

#[test]
fn read_errors() {
    check_status("read -z", 2);
    check_status("read -u 9 x", 1);
    check_status("read -t abc x", 1);
    let o = run("read 'a b' <<< x; echo $?");
    assert_eq!(o.stdout, "1\n");
    assert!(o.stderr.contains("not a valid identifier"), "{}", o.stderr);
    check("readonly r; read r <<< x; echo $?", "1\n");
}

#[test]
fn eval_runs_text() {
    check("eval 'echo a; echo b'", "a\nb\n");
    check("x='echo hi'; eval \"$x\"; eval; echo $?", "hi\n0\n");
    check("eval 'false'; echo $?", "1\n");
    check("f() { eval 'return 3'; echo no; }; f; echo $?", "3\n");
    check("for i in 1 2; do eval break; done; echo $i", "1\n");
    let o = run("eval 'echo )'; echo st=$?");
    assert_eq!(o.stdout, "st=2\n");
    assert!(
        o.stderr.contains("eval: line 1: syntax error"),
        "{}",
        o.stderr
    );
    check("eval -- 'echo x'", "x\n");
    check_status("eval -x", 2);
}

#[test]
fn prefix_assignments_on_builtins_are_temporary() {
    check("x=1 eval 'echo $x'; echo \"[$x]\"", "1\n[]\n");
    check("y=0; y=1 eval 'y=2'; echo $y", "0\n");
    check("x=1 :; echo \"[$x]\"", "[]\n");
    let o = run("x=1 eval env; env");
    assert_eq!(o.stdout.matches("x=1\n").count(), 1, "{}", o.stdout);
}

#[test]
fn source_files() {
    let o = run_setup(
        "set -- o; . /tmp/s.sh a b; echo \"$? [$*]\"; source /tmp/r.sh; echo $?; \
         . nosuch; echo $?; . /tmp; echo $?",
        |h| {
            h.fs.write_file("/tmp/s.sh", b"echo \"in $# $*\"\nv=set\n");
            h.fs.write_file("/tmp/r.sh", b"return 4\necho no\n");
        },
    );
    assert_eq!(o.stdout, "in 2 a b\n0 [o]\n4\n1\n1\n");
    assert!(o.stderr.contains("nosuch: No such file"), "{}", o.stderr);
    assert!(o.stderr.contains(".: /tmp: is a directory"), "{}", o.stderr);
    // Errors inside name the sourced file.
    let o = run_setup(". /tmp/e.sh", |h| {
        h.fs.write_file("/tmp/e.sh", b"\nnosuchcmd\n")
    });
    assert!(
        o.stderr.starts_with("/tmp/e.sh: line 2: nosuchcmd"),
        "{}",
        o.stderr
    );
    check(". /dev/stdin <<< 'echo from-stdin'", "from-stdin\n");
    check_status("source", 2);
    check_status("return 1", 2);
}

#[test]
fn command_and_type() {
    check(
        "f() { echo f; }; command -v f echo if cat",
        "f\necho\nif\n/bin/cat\n",
    );
    check("command -v nosuch; echo $?", "1\n");
    check("echo() { :; }; command echo hi", "hi\n");
    check(
        "command -V echo; type -t if f cat echo",
        "echo is a shell builtin\nkeyword\nfile\nbuiltin\n",
    );
    check(
        "f() { echo hi; }; type f",
        "f is a function\nf () \n{ \n    echo hi\n}\n",
    );
    check(
        "type cat; cat </dev/null; type cat",
        "cat is /bin/cat\ncat is hashed (/bin/cat)\n",
    );
    check("type nosuch 2>/dev/null; echo $?", "1\n");
    check("builtin echo x; builtin cat 2>/dev/null; echo $?", "x\n1\n");
    check("enable -s | head -n 2", "enable .\nenable :\n");
    check(
        "enable -n true; type -t true; enable true; type -t true",
        "file\nbuiltin\n",
    );
}

#[test]
fn hash_table() {
    check("hash", "hash: hash table empty\n");
    check(
        "hash -p /x/y foo; hash -t foo; hash -l; hash -d foo; hash",
        "/x/y\nbuiltin hash -p /x/y foo\nhash: hash table empty\n",
    );
    check(
        "cat </dev/null; cat </dev/null; hash",
        "hits\tcommand\n   2\t/bin/cat\n",
    );
    check(
        "cat </dev/null; PATH=/other; hash",
        "hash: hash table empty\n",
    );
    check_status("hash -t", 1);
}

#[test]
fn exec_runs_and_ends() {
    check("(exec argv a; echo no); echo $?", "[a]\n0\n");
    check("(exec false); echo $?", "1\n");
    check("(exec nosuch; echo no) 2>/dev/null; echo $?", "127\n");
    check_status("exec -x", 2);
    check("exec; echo still", "still\n");
    check("export q=1; (exec -c env); (exec -a zz argv x)", "[x]\n");
}
