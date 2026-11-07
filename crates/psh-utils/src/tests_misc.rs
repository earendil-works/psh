//! base64, the checksums, cmp, which, printenv and env's flags on the test host.
//! Behavior against GNU and BSD is L4.

use super::{check, check_all};

#[test]
fn base64() {
    check("base64 /tmp/g", "YSBiIGMK\n");
    check("base64 /tmp/g | base64 -d", "a b c\n");
    check("printf foobar | base64 -w 4", "Zm9v\nYmFy\n");
    check("base64 -i /tmp/g -o /tmp/o; base64 -D -i /tmp/o", "a b c\n");
    check_all(
        "echo 'Zm9v!' | base64 -d",
        "",
        1,
        "foo",
        "base64: invalid input\n",
    );
    check("echo 'Zm9v!' | base64 -d -i", "foo");
}

#[test]
fn sums() {
    check(
        "md5sum /tmp/g",
        "3da904d79fb03e6e3936ff2127039b1a  /tmp/g\n",
    );
    check("md5sum /tmp/g > /tmp/s; md5sum -c /tmp/s", "/tmp/g: OK\n");
    check("md5 -q /tmp/g", "3da904d79fb03e6e3936ff2127039b1a\n");
    check("shasum -a 256 /tmp/g | shasum -c", "/tmp/g: OK\n");
}

#[test]
fn cmp() {
    check_all("cmp /tmp/f /tmp/f; echo $?", "", 0, "0\n", "");
    check_all(
        "cmp /tmp/f /tmp/g; echo $?",
        "",
        0,
        "/tmp/f /tmp/g differ: byte 1, line 1\n1\n",
        "",
    );
    check_all(
        "head -n 1 /tmp/f > /tmp/h; cmp /tmp/f /tmp/h; echo $?",
        "",
        0,
        "1\n",
        "cmp: EOF on /tmp/h after byte 4, line 1\n",
    );
}

#[test]
fn which() {
    // A program on PATH wins (`$(which cat)` runs); a name only a utility provides
    // reports it; -a lists both.
    check("which cat", "/bin/cat\n");
    check("which base64", "base64: shell builtin\n");
    check("which -a cat", "cat: shell builtin\n/bin/cat\n");
    check("which argv", "/bin/argv\n");
    check_all("which nothere; echo $?", "", 0, "1\n", "");
}

#[test]
fn printenv() {
    check("export A=1; printenv A", "1\n");
    check_all("printenv NOPE; echo $?", "", 0, "1\n", "");
}

#[test]
fn env_flags() {
    check("env -S 'echo a  b'", "a b\n");
    check("cd /; env -C /tmp pwd", "/tmp\n");
    check_all(
        "env -C /nodir pwd; echo $?",
        "",
        0,
        "125\n",
        "env: cannot change directory to '/nodir': No such file or directory\n",
    );
}
