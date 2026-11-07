//! Windows cases: batch files, Windows paths, CRLF scripts, System32 programs found
//! through the real `PATH`.

use super::harness::run;

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
fn system_programs_run_through_path_lookup() {
    // The real PATH (`C:\WINDOWS\system32;...`): drive letters must survive every
    // lookup, the engine's (`type`, `command -v`, `hash`) and the host's (D1910).
    let r = run("ping -n 1 127.0.0.1 >/dev/null; echo $?; \
         cmd /d /c echo via-cmd; echo hi | findstr hi; \
         type -P cmd; command -v findstr; hash -r; where.exe /q cmd && echo where-ok");
    let lines: Vec<String> = r.stdout.lines().map(str::to_ascii_lowercase).collect();
    assert_eq!(lines[..3], ["0", "via-cmd", "hi"], "{r:?}");
    assert!(
        lines[3].ends_with("/cmd.exe") && lines[3].contains(":/"),
        "{r:?}"
    );
    assert!(lines[4].ends_with("/findstr.exe"), "{r:?}");
    assert_eq!(lines[5], "where-ok", "{r:?}");
}

#[test]
fn sh_scripts_and_plain_files_run_in_psh() {
    // No /bin/sh on Windows: psh runs these itself, with their arguments.
    let r = run("cd \"$TMPDIR_CASE\" && \
         printf '#!/bin/sh\\necho sh $1 $#\\n' > s.sh && ./s.sh a b; \
         printf '#!/usr/bin/env bash\\necho env-bash $0\\n' > e && ./e; \
         printf 'echo plain \"$@\"\\n' > plain && ./plain x 'y z'");
    assert_eq!(r.stdout, "sh a 2\nenv-bash ./e\nplain x y z\n", "{r:?}");
}

#[test]
fn cd_keeps_drive_paths() {
    let r = run("cd C:/Windows && pwd; cd 'C:\\Windows\\System32' && pwd; \
         cd C:/ && cd .. && pwd; cd /c/Windows && pwd");
    let lines: Vec<String> = r.stdout.lines().map(str::to_ascii_lowercase).collect();
    assert_eq!(
        lines,
        ["c:/windows", "c:/windows/system32", "c:/", "/c/windows"],
        "{r:?}"
    );
}

#[test]
fn command_p_uses_the_standard_path() {
    let r = run("PATH=; command -p cmd /d /c echo hi; command -pv cmd");
    let lines: Vec<String> = r.stdout.lines().map(str::to_ascii_lowercase).collect();
    assert_eq!(lines.first().map(String::as_str), Some("hi"), "{r:?}");
    assert!(
        lines
            .get(1)
            .is_some_and(|l| l.ends_with("/system32/cmd.exe") && l.contains(":/")),
        "{r:?}"
    );
}
