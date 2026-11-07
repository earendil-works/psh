//! Tests of the core builtins (`read`, `eval`, `source`, `command`, `type`, ...) on the
//! test host.

use crate::tests::{check, check_status, run};

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
