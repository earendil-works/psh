//! `psh -c CMD [ARGS]`: the standalone shell. A stub until the engine runs commands.
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

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--parse") => parse_command(&args[1..]),
        Some("--classify") => classify_command(&args[1..]),
        Some("-c") if args.len() >= 2 => match psh_engine::run(&args[1]) {
            Ok(status) => ExitCode::from(status.0 as u8),
            Err(message) => {
                eprintln!("psh: {message}");
                ExitCode::from(2)
            }
        },
        Some("-c") => {
            eprintln!("psh: -c: option requires an argument");
            ExitCode::from(2)
        }
        _ => usage(),
    }
}

fn usage() -> ExitCode {
    eprintln!("usage: psh -c COMMAND [ARGS...]");
    eprintln!("       psh --parse [--compact] [-O extglob] [-c COMMAND | FILE]");
    eprintln!("       psh --classify [--compact] [-c COMMAND | FILE]");
    ExitCode::from(2)
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
    let Some(text) = read_input(source) else {
        return ExitCode::from(2);
    };
    let (json, ok) = parse_json::parse_to_json(&text, opts);
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

/// The script from `-c` or a file, else stdin. The parser works on UTF-8; invalid bytes
/// become U+FFFD, which moves the byte offsets after them.
fn read_input(source: Option<Vec<u8>>) -> Option<String> {
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
    Some(String::from_utf8_lossy(&bytes).into_owned())
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
