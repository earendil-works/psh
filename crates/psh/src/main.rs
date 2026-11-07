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
use psh_engine::std_host::{drive, engine_thread};
use psh_engine::{Host, HostFile, Script, Shell, ShellConfig};

/// The shell's descriptors 0, 1 and 2.
type Stdio = [Option<Rc<dyn HostFile>>; 3];

/// The OS host. The standalone shell keeps its externals in its own process group, as a
/// non-interactive bash does, so whoever kills psh's group (a terminal's Ctrl-C, a
/// harness, `timeout`) reaches everything it started; background jobs outlive it as
/// with bash (D1067).
#[cfg(unix)]
fn os_host() -> (Rc<dyn Host>, Stdio) {
    use psh_os::unix::{Leftovers, Placement, UnixConfig, UnixFile, UnixHost};
    let host = UnixHost::new(UnixConfig {
        placement: Placement::Inherit,
        leftovers: Leftovers::Keep,
        ..UnixConfig::default()
    });
    let stdio = |n| {
        UnixFile::stdio(n)
            .ok()
            .map(|f| Rc::new(f) as Rc<dyn HostFile>)
    };
    (Rc::new(host), [stdio(0), stdio(1), stdio(2)])
}

/// The OS host. As on Unix, externals stay in psh's console group (a Ctrl-C typed in
/// the console reaches them) and background jobs outlive it.
#[cfg(windows)]
fn os_host() -> (Rc<dyn Host>, Stdio) {
    use psh_os::windows::{Leftovers, Placement, WinFile, WindowsConfig, WindowsHost};
    let host = WindowsHost::new(WindowsConfig {
        placement: Placement::Inherit,
        leftovers: Leftovers::Keep,
        ..WindowsConfig::default()
    });
    let stdio = |n| {
        WinFile::stdio(n)
            .ok()
            .map(|f| Rc::new(f) as Rc<dyn HostFile>)
    };
    (Rc::new(host), [stdio(0), stdio(1), stdio(2)])
}

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

/// A path from the command line's bytes (not necessarily UTF-8 on Unix).
fn os_path(b: &[u8]) -> std::path::PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        std::path::PathBuf::from(std::ffi::OsStr::from_bytes(b))
    }
    #[cfg(not(unix))]
    {
        std::path::PathBuf::from(String::from_utf8_lossy(b).into_owned())
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
            source,
            arg0: arg0.unwrap_or_else(|| b"psh".to_vec()),
            args,
            dash_c: true,
        },
        Ok(Invocation::File { path, args }) => match std::fs::read(os_path(&path)) {
            Ok(src) => Script {
                source: src,
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
                source: src,
                arg0: b"psh".to_vec(),
                args,
                dash_c: false,
            }
        }
    };
    let signals = Signals::catch();
    let control = signals.control.clone();
    let thread = engine_thread(move || {
        let (host, [stdin, stdout, stderr]) = os_host();
        let mut cfg = ShellConfig::new(host);
        cfg.env = std::env::vars_os()
            .map(|(k, v)| (os_bytes(k), os_bytes(v)))
            .collect();
        cfg.cwd = std::env::current_dir()
            .map(|p| os_bytes(p.into_os_string()))
            .unwrap_or_else(|_| b"/".to_vec());
        #[cfg(windows)]
        {
            cfg.cwd = cfg
                .cwd
                .iter()
                .map(|&b| if b == b'\\' { b'/' } else { b })
                .collect();
            let defaults = psh_os::windows::env_defaults(&cfg.env);
            cfg.env.extend(defaults);
        }
        cfg.stdin = stdin;
        cfg.stdout = stdout;
        cfg.stderr = stderr;
        let mut shell = Shell::new(cfg);
        Signals::attach(&control, shell.control());
        shell.start(script);
        drive(&mut shell)
    });
    let result = thread.map(|t| t.join());
    signals.finish();
    match result {
        Ok(Ok(status)) => ExitCode::from((status & 0xff) as u8),
        _ => ExitCode::from(2),
    }
}

/// The standalone shell's signals (ARCH.md "psh-os"): INT, TERM and HUP cancel the run
/// (its externals get TERM, KILL after the grace period), then psh ends by that signal,
/// so its parent sees what bash would show. Embedded shells install nothing.
struct Signals {
    /// The running shell's control, once it exists, and the first signal caught.
    control: Control,
}

type Control = std::sync::Arc<std::sync::Mutex<(Option<psh_engine::ControlHandle>, i32)>>;

impl Signals {
    #[cfg(unix)]
    fn catch() -> Signals {
        use psh_engine::host::{SIGHUP, SIGINT, SIGTERM};
        let control: Control = Default::default();
        let c = control.clone();
        let _ = psh_os::unix::signals::catch(&[SIGINT, SIGTERM, SIGHUP], move |sig| {
            let mut g = c.lock().unwrap_or_else(|e| e.into_inner());
            if g.1 == 0 {
                g.1 = sig;
            }
            if let Some(ctl) = &g.0 {
                ctl.cancel();
            }
        });
        Signals { control }
    }

    #[cfg(not(unix))]
    fn catch() -> Signals {
        Signals {
            control: Default::default(),
        }
    }

    /// The shell exists: a signal caught before cancels it now.
    fn attach(
        control: &std::sync::Mutex<(Option<psh_engine::ControlHandle>, i32)>,
        ctl: psh_engine::ControlHandle,
    ) {
        let mut g = control.lock().unwrap_or_else(|e| e.into_inner());
        if g.1 != 0 {
            ctl.cancel();
        }
        g.0 = Some(ctl);
    }

    /// The run is over: if a signal ended it, wait until the abandoned externals were
    /// killed and reaped, and die by the signal.
    fn finish(self) {
        let sig = self.control.lock().unwrap_or_else(|e| e.into_inner()).1;
        // A signal the watcher thread has not passed on yet (it came as the run ended).
        #[cfg(unix)]
        let sig = match sig {
            0 => psh_os::unix::signals::first_caught().unwrap_or(0),
            s => s,
        };
        #[cfg(unix)]
        if sig != 0 {
            let grace = psh_os::unix::UnixConfig::DEFAULT_GRACE;
            psh_os::unix::settle(grace + std::time::Duration::from_secs(1));
            psh_os::unix::signals::die_by(sig);
        }
        let _ = sig;
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
