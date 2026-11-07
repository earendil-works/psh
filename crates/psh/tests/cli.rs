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

/// How long one psh run may take: 10 s, times `PSH_TEST_TIME_SCALE` where the gate runs
/// the tests slowed down (x64 under emulation in the Windows VM, D1912).
fn run_limit() -> Duration {
    let scale = std::env::var("PSH_TEST_TIME_SCALE")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|s| *s >= 1.0)
        .unwrap_or(1.0);
    Duration::from_secs_f64(10.0 * scale)
}

/// Runs psh with `args`, killing it if it has not exited within [`run_limit`].
fn psh(args: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_psh"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn psh");
    let limit = run_limit();
    let deadline = Instant::now() + limit;
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait for psh") {
            break status;
        }
        if Instant::now() > deadline {
            child.kill().ok();
            child.wait().ok();
            panic!("psh {args:?} did not exit within {limit:?}");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    // The tests write a few lines at most, well below the pipe buffer, so reading after
    // the exit cannot deadlock.
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
fn dash_c_runs_commands() {
    let out = psh(&["-c", "echo hi; echo err >&2; exit 3"]);
    assert_eq!(out.status, Some(3));
    assert_eq!(out.stdout, "hi\n");
    assert_eq!(out.stderr, "err\n");
}

#[test]
fn dash_c_name_and_args() {
    let out = psh(&["-c", "echo $0 $1 $#; nosuchcommand_psh", "name", "a"]);
    assert_eq!(out.stdout, "name a 1\n");
    assert_eq!(
        out.stderr,
        "name: line 1: nosuchcommand_psh: command not found\n"
    );
    assert_eq!(out.status, Some(127));
}

#[test]
fn script_file() {
    let dir = std::env::temp_dir().join(format!("psh-cli-script-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("s.sh");
    std::fs::write(&file, "echo \"$1\"\necho )\n").unwrap();
    let out = psh(&[file.to_str().unwrap(), "arg"]);
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(out.stdout, "arg\n");
    assert_eq!(out.status, Some(2));
    assert!(
        out.stderr
            .contains("line 2: syntax error near unexpected token `)'"),
        "{}",
        out.stderr
    );
}

#[cfg(unix)]
#[test]
fn pipeline_through_externals() {
    let out = psh(&[
        "-c",
        "printf 'b\\na\\n' | sort | head -n 1; x=$(echo hi | cat); echo $x",
    ]);
    assert_eq!(out.stdout, "a\nhi\n");
    assert_eq!(out.status, Some(0));
}

#[test]
fn dash_c_without_argument() {
    let out = psh(&["-c"]);
    assert_eq!(out.status, Some(2));
    assert!(
        out.stderr
            .starts_with("psh: -c: option requires an argument\n"),
        "{}",
        out.stderr
    );
}

#[test]
fn no_arguments_reads_stdin() {
    let out = psh(&[]);
    assert_eq!(out.status, Some(0));
    assert_eq!(out.stdout, "");
}

#[test]
fn help_prints_usage() {
    let out = psh(&["--help"]);
    assert_eq!(out.status, Some(2));
    assert!(out.stderr.starts_with("usage: psh"), "{}", out.stderr);
}

#[test]
fn parse_prints_json() {
    let out = psh(&["--parse", "--compact", "-c", "echo hi"]);
    assert_eq!(out.status, Some(0), "{}", out.stderr);
    assert!(
        out.stdout
            .starts_with(r#"{"ok":true,"script":{"type":"list","span":[0,7],"#),
        "{}",
        out.stdout
    );
    assert!(out.stdout.ends_with("}\n"));
    // Pretty by default.
    let out = psh(&["--parse", "-c", "echo hi"]);
    assert!(
        out.stdout.starts_with("{\n  \"ok\": true,\n"),
        "{}",
        out.stdout
    );
}

#[test]
fn parse_reports_syntax_errors() {
    let out = psh(&["--parse", "--compact", "-c", "echo a\nfi"]);
    assert_eq!(out.status, Some(2));
    assert_eq!(
        out.stdout,
        "{\"ok\":false,\"error\":{\"message\":\"syntax error near unexpected token `fi'\",\"span\":[7,9],\"line\":2,\"column\":1,\"incomplete\":false}}\n"
    );
}

#[test]
fn parse_options_and_files() {
    let out = psh(&["--parse", "--compact", "-c", "echo @(a)"]);
    assert_eq!(out.status, Some(2));
    let out = psh(&["--parse", "--compact", "-O", "extglob", "-c", "echo @(a)"]);
    assert_eq!(out.status, Some(0));
    assert!(out.stdout.contains(r#""type":"extglob""#), "{}", out.stdout);

    let dir = std::env::temp_dir().join(format!("psh-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("script.sh");
    std::fs::write(&file, "ls | head\n").unwrap();
    let out = psh(&["--parse", "--compact", file.to_str().unwrap()]);
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(out.status, Some(0));
    assert!(out.stdout.contains(r#""pipes":["|"]"#), "{}", out.stdout);

    // No script: stdin (here empty).
    let out = psh(&["--parse", "--compact"]);
    assert_eq!(out.status, Some(0));
    assert!(out.stdout.contains(r#""items":[]"#), "{}", out.stdout);
}

#[test]
fn classify_prints_json() {
    let out = psh(&[
        "--classify",
        "--compact",
        "-c",
        "git status && rm -rf build",
    ]);
    assert_eq!(out.status, Some(0), "{}", out.stderr);
    assert!(out.stdout.starts_with(r#"{"ok":true,"error":null,"primary":"mutate","categories":["read","mutate"],"read_only":false"#), "{}", out.stdout);
    assert!(
        out.stdout
            .contains(r#"{"path":"build","span":[21,26],"source":null,"command":1}"#),
        "{}",
        out.stdout
    );

    let out = psh(&["--classify", "-c", "ls"]);
    assert_eq!(out.status, Some(0));
    assert!(out.stdout.contains("\"read_only\": true"), "{}", out.stdout);

    let out = psh(&["--classify", "--compact", "-c", "echo a\nfi"]);
    assert_eq!(out.status, Some(2));
    assert!(
        out.stdout.contains(r#""primary":"unknown""#),
        "{}",
        out.stdout
    );

    // No script: stdin (here empty), which does nothing.
    let out = psh(&["--classify", "--compact"]);
    assert_eq!(out.status, Some(0));
    assert!(
        out.stdout
            .contains(r#""read_only":true,"truncated":false,"commands":[]"#),
        "{}",
        out.stdout
    );
}

#[test]
fn parse_keeps_bytes_that_are_not_utf8() {
    let dir = std::env::temp_dir().join(format!("psh-cli-bytes-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("latin1.sh");
    std::fs::write(&file, b"echo caf\xe9 ok\n").unwrap();
    let out = psh(&["--parse", "--compact", file.to_str().unwrap()]);
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(out.status, Some(0));
    assert!(out.stdout.contains(r#""utf8":false"#), "{}", out.stdout);
    // `ok` starts at byte 10: the byte offsets are those of the file.
    assert!(
        out.stdout.contains(r#""span":[10,12],"text":"ok""#),
        "{}",
        out.stdout
    );
    assert!(out.stdout.contains("caf\u{f7e9}"), "{}", out.stdout);
}

/// A ctrl-break to psh's console group cancels the run: its externals are killed and
/// psh ends with 128 + TERM.
#[cfg(windows)]
#[test]
fn ctrl_break_cancels_the_run() {
    use psh_engine::host::{ChildFd, ExitStatus, Host, SIGTERM, SpawnSpec};
    use psh_os::windows::{WindowsConfig, WindowsHost};
    use std::task::{Context, Poll, Waker};

    let host = WindowsHost::new(WindowsConfig::default());
    let exe = env!("CARGO_BIN_EXE_psh").replace('\\', "/");
    let spec = SpawnSpec {
        program: exe.clone().into_bytes(),
        argv: vec![
            exe.into_bytes(),
            b"-c".to_vec(),
            b"echo start; \"$SystemRoot/System32/ping.exe\" -n 30 127.0.0.1 >/dev/null; echo after $?".to_vec(),
        ],
        env: std::env::vars()
            .map(|(k, v)| (k.into_bytes(), v.into_bytes()))
            .collect(),
        cwd: std::env::temp_dir()
            .to_string_lossy()
            .replace('\\', "/")
            .into_bytes(),
        fds: vec![
            (0, ChildFd::Null),
            (1, ChildFd::Pipe { input: false }),
            (2, ChildFd::Dup(1)),
        ],
        ..SpawnSpec::default()
    };
    let spawned = host.spawn(spec).expect("spawn psh");
    let mut p = spawned.proc;
    let out = spawned.pipes[0].1.clone();
    std::thread::sleep(Duration::from_millis(1500));
    p.signal(SIGTERM);
    let mut cx = Context::from_waker(Waker::noop());
    let deadline = Instant::now() + Duration::from_secs(15);
    let status = loop {
        if let Poll::Ready(s) = p.poll_wait(&mut cx) {
            break s;
        }
        assert!(
            Instant::now() < deadline,
            "psh did not end after a ctrl-break"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    let mut text = Vec::new();
    let mut buf = [0u8; 1024];
    while let Poll::Ready(Ok(n)) = out.poll_read(&mut cx, &mut buf) {
        if n == 0 {
            break;
        }
        text.extend_from_slice(&buf[..n]);
    }
    let text = String::from_utf8_lossy(&text);
    assert_eq!(status, ExitStatus::Exited(143), "{text}");
    drop(p);
    assert!(host.leftovers().is_empty(), "{:?}", host.leftovers());
}
