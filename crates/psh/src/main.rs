//! `psh -c CMD [NAME [ARGS…]]`, `psh FILE [ARGS…]`, or a script on stdin: the standalone
//! shell. It drives the engine on a thread of its own with an explicit stack (D1050),
//! parking while the engine is blocked.
//!
//! `psh --parse [--compact] [-O extglob] [-c CMD | FILE]` prints the syntax tree as
//! JSON (stdin when neither `-c` nor a file is given); status 0 if it parses, 2 if not.
//!
//! `psh --classify [--compact] [-c CMD | FILE]` prints what the script does as JSON
//! (`classify_json.rs`); status 0 if it parses, 2 if not.

mod classify_json;
mod json;
mod parse_json;

use std::io::{Read, Write};
use std::process::ExitCode;
use std::rc::Rc;

use psh_engine::invocation::{Invocation, parse_args};
use psh_engine::std_host::{StdFile, StdHost, drive, engine_thread};
use psh_engine::{HostFile, Script, Shell, ShellConfig};

fn os_bytes(s: std::ffi::OsString) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        s.into_vec()
    }
    #[cfg(not(unix))]
    {
        s.to_string_lossy().into_owned().into_bytes()
    }
}

fn usage() -> ExitCode {
    eprintln!("usage: psh [-c COMMAND [NAME [ARGS...]] | FILE [ARGS...]]");
    eprintln!("       psh --parse [--compact] [-O extglob] [-c COMMAND | FILE]");
    eprintln!("       psh --classify [--compact] [-c COMMAND | FILE]");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<Vec<u8>> = std::env::args_os().skip(1).map(os_bytes).collect();
    if args.first().is_some_and(|a| a == b"--parse") {
        let rest: Vec<String> = args[1..]
            .iter()
            .map(|a| String::from_utf8_lossy(a).into_owned())
            .collect();
        return parse_command(&rest);
    }
    if args.first().is_some_and(|a| a == b"--classify") {
        let rest: Vec<String> = args[1..]
            .iter()
            .map(|a| String::from_utf8_lossy(a).into_owned())
            .collect();
        return classify_command(&rest);
    }
    let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    let script = match parse_args(&args) {
        Err(msg) => {
            eprintln!("psh: {msg}");
            return usage();
        }
        Ok(Invocation::Help) => return usage(),
        Ok(Invocation::Command { source, arg0, args }) => Script {
            source: text(&source),
            arg0: arg0.unwrap_or_else(|| b"psh".to_vec()),
            args,
            dash_c: true,
        },
        Ok(Invocation::File { path, args }) => match std::fs::read(text(&path)) {
            Ok(src) => Script {
                source: text(&src),
                arg0: path,
                args,
                dash_c: false,
            },
            Err(e) => {
                eprintln!("psh: {}: {}", text(&path), psh_engine::host::error_text(&e));
                return ExitCode::from(127);
            }
        },
        Ok(Invocation::Stdin { args }) => {
            // v1 reads the whole script first; bash reads it as it goes, so a command
            // in the script could read the rest of it (a follow-up).
            let mut src = Vec::new();
            if std::io::stdin().read_to_end(&mut src).is_err() {
                return ExitCode::from(2);
            }
            Script {
                source: text(&src),
                arg0: b"psh".to_vec(),
                args,
                dash_c: false,
            }
        }
    };
    let thread = engine_thread(move || {
        let mut cfg = ShellConfig::new(Rc::new(StdHost::new()));
        cfg.env = std::env::vars_os()
            .map(|(k, v)| (os_bytes(k), os_bytes(v)))
            .collect();
        cfg.cwd = std::env::current_dir()
            .map(|p| os_bytes(p.into_os_string()))
            .unwrap_or_else(|_| b"/".to_vec());
        if cfg!(windows) {
            cfg.cwd = cfg
                .cwd
                .iter()
                .map(|&b| if b == b'\\' { b'/' } else { b })
                .collect();
        }
        let stdio = |n| {
            StdFile::stdio(n)
                .ok()
                .map(|f| Rc::new(f) as Rc<dyn HostFile>)
        };
        cfg.stdin = stdio(0);
        cfg.stdout = stdio(1);
        cfg.stderr = stdio(2);
        let mut shell = Shell::new(cfg);
        shell.start(script);
        drive(&mut shell)
    });
    match thread.map(|t| t.join()) {
        Ok(Ok(status)) => ExitCode::from((status & 0xff) as u8),
        _ => ExitCode::from(2),
    }
}

fn parse_command(args: &[String]) -> ExitCode {
    let mut compact = false;
    let mut opts = psh_syntax::Options::default();
    let mut source = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--compact" => compact = true,
            "-O" => match it.next().map(String::as_str) {
                Some("extglob") => opts.extglob = true,
                Some(other) => {
                    eprintln!("psh: --parse: -O {other}: only extglob affects parsing");
                    return ExitCode::from(2);
                }
                None => {
                    eprintln!("psh: -O: option requires an argument");
                    return ExitCode::from(2);
                }
            },
            "-c" => match it.next() {
                Some(cmd) => source = Some(cmd.clone().into_bytes()),
                None => {
                    eprintln!("psh: -c: option requires an argument");
                    return ExitCode::from(2);
                }
            },
            path if source.is_none() && !path.starts_with('-') => match std::fs::read(path) {
                Ok(bytes) => source = Some(bytes),
                Err(e) => {
                    eprintln!("psh: {path}: {e}");
                    return ExitCode::from(2);
                }
            },
            _ => return usage(),
        }
    }
    let Some(bytes) = read_bytes(source) else {
        return ExitCode::from(2);
    };
    let (json, ok) = parse_json::parse_to_json(&bytes, opts);
    print_json(&json, compact, ok)
}

fn classify_command(args: &[String]) -> ExitCode {
    let mut compact = false;
    let mut source = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--compact" => compact = true,
            "-c" => match it.next() {
                Some(cmd) => source = Some(cmd.clone().into_bytes()),
                None => {
                    eprintln!("psh: -c: option requires an argument");
                    return ExitCode::from(2);
                }
            },
            path if source.is_none() && !path.starts_with('-') => match std::fs::read(path) {
                Ok(bytes) => source = Some(bytes),
                Err(e) => {
                    eprintln!("psh: {path}: {e}");
                    return ExitCode::from(2);
                }
            },
            _ => return usage(),
        }
    }
    let Some(text) = read_input(source) else {
        return ExitCode::from(2);
    };
    let c = psh_classify::classify(&text);
    let ok = c.error.is_none();
    print_json(&classify_json::to_json(&c, &text), compact, ok)
}

/// The script from `-c` or a file, else stdin, as text: invalid UTF-8 becomes U+FFFD,
/// which moves the byte offsets after it.
fn read_input(source: Option<Vec<u8>>) -> Option<String> {
    read_bytes(source).map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// The script from `-c` or a file, else stdin, byte for byte.
fn read_bytes(source: Option<Vec<u8>>) -> Option<Vec<u8>> {
    let bytes = match source {
        Some(b) => b,
        None => {
            let mut b = Vec::new();
            if let Err(e) = std::io::stdin().read_to_end(&mut b) {
                eprintln!("psh: stdin: {e}");
                return None;
            }
            b
        }
    };
    Some(bytes)
}

/// Prints the JSON; status 0 if the input parsed, 2 if not or if stdout fails.
fn print_json(json: &json::Json, compact: bool, ok: bool) -> ExitCode {
    let out = if compact {
        json.to_compact()
    } else {
        json.to_pretty()
    };
    let mut stdout = std::io::stdout().lock();
    if writeln!(stdout, "{out}")
        .and_then(|_| stdout.flush())
        .is_err()
    {
        return ExitCode::from(2);
    }
    ExitCode::from(if ok { 0 } else { 2 })
}
