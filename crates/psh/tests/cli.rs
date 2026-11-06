//! Invocation tests for the `psh` binary. Every spawned process runs under a deadline
//! (AGENTS.md: timeouts on everything that runs a shell).

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

struct Output {
    status: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Runs psh with `args`, killing it if it has not exited after 10 seconds.
fn psh(args: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_psh"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn psh");
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait for psh") {
            break status;
        }
        if Instant::now() > deadline {
            child.kill().ok();
            child.wait().ok();
            panic!("psh {args:?} did not exit within 10 seconds");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    // The stub writes a line at most, well below the pipe buffer, so reading after the
    // exit cannot deadlock.
    let mut stdout = String::new();
    let mut stderr = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    Output {
        status: status.code(),
        stdout,
        stderr,
    }
}

#[test]
fn dash_c_is_not_implemented() {
    let out = psh(&["-c", "echo hi"]);
    assert_eq!(out.status, Some(2));
    assert_eq!(out.stdout, "");
    assert_eq!(out.stderr, "psh: not implemented\n");
}

#[test]
fn dash_c_without_argument() {
    let out = psh(&["-c"]);
    assert_eq!(out.status, Some(2));
    assert_eq!(out.stderr, "psh: -c: option requires an argument\n");
}

#[test]
fn no_arguments_prints_usage() {
    let out = psh(&[]);
    assert_eq!(out.status, Some(2));
    assert!(out.stderr.starts_with("usage: psh"), "{}", out.stderr);
}
