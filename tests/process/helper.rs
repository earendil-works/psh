//! `psh-test-helper`: the external programs the L5 tests need on every platform,
//! where Windows has no `sleep`, `cat` or `yes` (TESTING.md "L5").
//!
//!   sleep SECS          sleep (fractions allowed)
//!   cat                 copy stdin to stdout
//!   yes                 print `y` lines until stdout fails
//!   args ARGS...        print each argument as `[ARG]` on its own line
//!   exit N              exit with status N
//!   crash               abort (a crash: SIGABRT, or fail-fast on Windows)
//!   tree SECS           start a grandchild (`sleep SECS`), then sleep SECS
//!   stubborn SECS       ignore TERM (Unix) or ctrl-break (Windows), then sleep SECS
//!   bytes N             write N bytes of `x`

use std::io::{Read, Write};
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize| args.get(i).map(String::as_str).unwrap_or("");
    let secs = |i: usize| Duration::from_secs_f64(arg(i).parse().unwrap_or(1.0));
    match arg(0) {
        "sleep" => std::thread::sleep(secs(1)),
        "cat" => {
            let mut buf = [0u8; 16384];
            let (mut i, mut o) = (std::io::stdin().lock(), std::io::stdout().lock());
            loop {
                match i.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if o.write_all(&buf[..n]).is_err() {
                            std::process::exit(141);
                        }
                    }
                }
            }
        }
        "yes" => {
            let mut o = std::io::stdout().lock();
            while o.write_all(b"y\n").is_ok() {}
            std::process::exit(141);
        }
        "args" => {
            for a in &args[1..] {
                println!("[{a}]");
            }
        }
        "exit" => std::process::exit(arg(1).parse().unwrap_or(1)),
        "crash" => std::process::abort(),
        "tree" => {
            let me = std::env::current_exe().unwrap();
            let mut child = std::process::Command::new(me)
                .args(["sleep", arg(1)])
                .spawn()
                .unwrap();
            std::thread::sleep(secs(1));
            let _ = child.wait();
        }
        "stubborn" => {
            ignore_term();
            std::thread::sleep(secs(1));
        }
        "bytes" => {
            let n: usize = arg(1).parse().unwrap_or(0);
            let _ = std::io::stdout().lock().write_all(&vec![b'x'; n]);
        }
        other => {
            eprintln!("psh-test-helper: unknown mode {other:?}");
            std::process::exit(2);
        }
    }
}

#[cfg(unix)]
fn ignore_term() {
    // SAFETY: setting a disposition to SIG_IGN has no handler to be unsafe in.
    unsafe { libc::signal(libc::SIGTERM, libc::SIG_IGN) };
}

#[cfg(windows)]
fn ignore_term() {
    use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
    unsafe extern "system" fn swallow(_event: u32) -> i32 {
        1
    }
    // SAFETY: the handler is a plain function that lives for the process.
    unsafe { SetConsoleCtrlHandler(Some(swallow), 1) };
}
