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
        Ok(Invocation::Command {
            source,
            arg0,
            args,
            options,
        }) => Script {
            source,
            arg0: arg0.unwrap_or_else(|| b"psh".to_vec()),
            args,
            dash_c: true,
            options,
        },
        Ok(Invocation::File {
            path,
            args,
            options,
        }) => match std::fs::read(os_path(&path)) {
            Ok(src) => Script {
                source: src,
                arg0: path,
                args,
                dash_c: false,
                options,
            },
            Err(e) => {
                eprintln!("psh: {}: {}", text(&path), psh_engine::host::error_text(&e));
                return ExitCode::from(127);
            }
        },
        Ok(Invocation::Stdin { args, options }) => {
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
                options,
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
            cfg.env = psh_os::windows::shell_env(std::mem::take(&mut cfg.env));
        }
        cfg.stdin = stdin;
        cfg.stdout = stdout;
        cfg.stderr = stderr;
        let mut shell = Shell::new(cfg);
        Signals::attach(&control, shell.control());
        shell.start(script);
        let status = drive(&mut shell);
        (status, shell.killed_by())
    });
    match thread.map(|t| t.join()) {
        Ok(Ok((status, killed_by))) => {
            signals.finish(killed_by);
            ExitCode::from((status & 0xff) as u8)
        }
        _ => ExitCode::from(2),
    }
}

/// The standalone shell's signals (ARCH.md "psh-os", D1903): the ones a script may trap
/// are caught and sent to the shell (`ControlHandle::signal`), which runs a trap,
/// ignores them, or ends with `128+n`; a foreground command is abandoned then (TERM,
/// KILL after the grace period). Ended by a signal's default action, psh dies by it,
/// so its parent sees what bash shows. Embedded shells install nothing.
struct Signals {
    /// The running shell's control, once it exists, and the first signal caught.
    control: Control,
}

/// The signals the standalone shell catches (Linux numbers): those a script may trap
/// whose default action ends a process, but not QUIT (bash ignores it), PIPE (writes
/// see EPIPE) or the synchronous faults.
#[cfg(unix)]
const CAUGHT: [psh_engine::host::Signal; 12] = [1, 2, 10, 12, 14, 15, 24, 25, 26, 27, 29, 30];

type Control = std::sync::Arc<std::sync::Mutex<Option<psh_engine::ControlHandle>>>;

impl Signals {
    #[cfg(unix)]
    fn catch() -> Signals {
        let control: Control = Default::default();
        let c = control.clone();
        let _ = psh_os::unix::signals::catch(&CAUGHT, move |sig| {
            // The handler set the signal's bit already; this wakes the shell.
            if let Some(ctl) = &*c.lock().unwrap_or_else(|e| e.into_inner()) {
                ctl.signal(sig);
            }
        });
        // Only the engine thread takes these signals (it unblocks them): a `kill $$`
        // from the script then acts before `kill` returns, as bash dies at once.
        psh_os::unix::signals::block_in_thread(&CAUGHT);
        Signals { control }
    }

    #[cfg(windows)]
    fn catch() -> Signals {
        let control: Control = Default::default();
        let c = control.clone();
        let _ = psh_os::windows::signals::catch(move |sig| {
            if let Some(ctl) = &*c.lock().unwrap_or_else(|e| e.into_inner()) {
                ctl.signal(sig);
            }
        });
        Signals { control }
    }

    /// The shell exists, on the engine thread: it takes the signals from now on (the
    /// handler cancels it directly), and a signal caught before cancels it now.
    fn attach(
        control: &std::sync::Mutex<Option<psh_engine::ControlHandle>>,
        ctl: psh_engine::ControlHandle,
    ) {
        #[cfg(unix)]
        {
            // One shell per process: the handle lives as long as the process.
            let word = Box::leak(Box::new(ctl.clone())).signal_word();
            psh_os::unix::signals::forward_to(word);
            psh_os::unix::signals::unblock_in_thread(&CAUGHT);
        }
        // A console event before the shell existed.
        #[cfg(windows)]
        if let Some(sig) = psh_os::windows::signals::first_caught() {
            ctl.signal(sig);
        }
        *control.lock().unwrap_or_else(|e| e.into_inner()) = Some(ctl);
    }

    /// The run is over: if a signal's default action ended it, wait until the
    /// abandoned externals were killed and reaped, and die by the signal.
    fn finish(self, killed_by: Option<i32>) {
        #[cfg(unix)]
        if let Some(sig) = killed_by {
            let grace = psh_os::unix::UnixConfig::DEFAULT_GRACE;
            psh_os::unix::settle(grace + std::time::Duration::from_secs(1));
            psh_os::unix::signals::die_by(sig);
        }
        #[cfg(windows)]
        if let Some(sig) = killed_by {
            let grace = psh_os::windows::WindowsConfig::DEFAULT_GRACE;
            psh_os::windows::settle(grace + std::time::Duration::from_secs(1));
            psh_os::windows::signals::die_by(sig);
        }
        let _ = (self, killed_by);
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
