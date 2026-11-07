//! Signal names, numbers and descriptions (`kill -l`, `kill -SIG`, job states).
//!
//! Numbers are the OS's own: the engine sends and reports real signals (BSD numbering on
//! macOS and the BSDs, Linux numbering elsewhere, wasm included). Descriptions are
//! glibc's `strsignal` texts on every platform, as bash prints them on Linux (the
//! recorded goldens).

use crate::host::Signal;

/// Signals by number, without the `SIG` prefix; index = number.
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
const NAMES: &[&str] = &[
    "", "HUP", "INT", "QUIT", "ILL", "TRAP", "ABRT", "EMT", "FPE", "KILL", "BUS", "SEGV", "SYS",
    "PIPE", "ALRM", "TERM", "URG", "STOP", "TSTP", "CONT", "CHLD", "TTIN", "TTOU", "IO", "XCPU",
    "XFSZ", "VTALRM", "PROF", "WINCH", "INFO", "USR1", "USR2",
];

#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
const NAMES: &[&str] = &[
    "", "HUP", "INT", "QUIT", "ILL", "TRAP", "ABRT", "BUS", "FPE", "KILL", "USR1", "SEGV", "USR2",
    "PIPE", "ALRM", "TERM", "STKFLT", "CHLD", "CONT", "STOP", "TSTP", "TTIN", "TTOU", "URG",
    "XCPU", "XFSZ", "VTALRM", "PROF", "WINCH", "IO", "PWR", "SYS",
];

/// Linux's real-time signals (`RTMIN` 34 … `RTMAX` 64), as bash names them.
#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
const RT: Option<(Signal, Signal)> = Some((34, 64));
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
const RT: Option<(Signal, Signal)> = None;

/// The highest signal number.
pub(crate) fn max() -> Signal {
    match RT {
        Some((_, hi)) => hi,
        None => NAMES.len() as Signal - 1,
    }
}

/// `SIGTERM` for 15 (Linux), `SIGRTMIN+2` for 36; `None` when there is no such signal.
pub(crate) fn name(n: Signal) -> Option<String> {
    if n > 0 && (n as usize) < NAMES.len() {
        return Some(format!("SIG{}", NAMES[n as usize]));
    }
    let (lo, hi) = RT?;
    if n < lo || n > hi {
        return None;
    }
    let mid = lo + (hi - lo) / 2;
    Some(if n == lo {
        "SIGRTMIN".to_string()
    } else if n == hi {
        "SIGRTMAX".to_string()
    } else if n <= mid {
        format!("SIGRTMIN+{}", n - lo)
    } else {
        format!("SIGRTMAX-{}", hi - n)
    })
}

/// The number of a signal name: `TERM`, `SIGTERM`, `term` (any case), `RTMIN+2`.
/// `EXIT` and `0` are 0.
pub(crate) fn number(s: &str) -> Option<Signal> {
    let up = s.to_ascii_uppercase();
    let bare = up.strip_prefix("SIG").unwrap_or(&up);
    if bare == "EXIT" {
        return Some(0);
    }
    if let Some(i) = NAMES.iter().position(|n| !n.is_empty() && *n == bare) {
        return Some(i as Signal);
    }
    if bare == "IOT" {
        return number("ABRT");
    }
    if bare == "POLL" {
        return number("IO");
    }
    let (lo, hi) = RT?;
    let rel = |rest: &str| -> Option<Signal> {
        if rest.is_empty() {
            Some(0)
        } else {
            rest.parse().ok()
        }
    };
    if let Some(rest) = bare.strip_prefix("RTMIN") {
        let off = match rest.strip_prefix('+') {
            Some(r) => rel(r)?,
            None if rest.is_empty() => 0,
            None => return None,
        };
        return (lo + off <= hi).then_some(lo + off);
    }
    if let Some(rest) = bare.strip_prefix("RTMAX") {
        let off = match rest.strip_prefix('-') {
            Some(r) => rel(r)?,
            None if rest.is_empty() => 0,
            None => return None,
        };
        return (hi - off >= lo).then_some(hi - off);
    }
    None
}

/// A signal given by name or number (`kill -s`, `kill -SIG`): `None` when invalid.
pub(crate) fn parse(s: &str) -> Option<Signal> {
    if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
        let n: Signal = s.parse().ok()?;
        return (n == 0 || name(n).is_some()).then_some(n);
    }
    number(s)
}

/// What `jobs` and job reports say for a process a signal ended (glibc's texts).
pub(crate) fn describe(n: Signal) -> String {
    let base = name(n);
    let text = match base.as_deref().map(|s| &s[3..]) {
        Some("HUP") => "Hangup",
        Some("INT") => "Interrupt",
        Some("QUIT") => "Quit",
        Some("ILL") => "Illegal instruction",
        Some("TRAP") => "Trace/breakpoint trap",
        Some("ABRT") => "Aborted",
        Some("BUS") => "Bus error",
        Some("FPE") => "Floating point exception",
        Some("KILL") => "Killed",
        Some("USR1") => "User defined signal 1",
        Some("SEGV") => "Segmentation fault",
        Some("USR2") => "User defined signal 2",
        Some("PIPE") => "Broken pipe",
        Some("ALRM") => "Alarm clock",
        Some("TERM") => "Terminated",
        Some("STKFLT") => "Stack fault",
        Some("CHLD") => "Child exited",
        Some("CONT") => "Continued",
        Some("STOP") => "Stopped (signal)",
        Some("TSTP") => "Stopped",
        Some("TTIN") => "Stopped (tty input)",
        Some("TTOU") => "Stopped (tty output)",
        Some("URG") => "Urgent I/O condition",
        Some("XCPU") => "CPU time limit exceeded",
        Some("XFSZ") => "File size limit exceeded",
        Some("VTALRM") => "Virtual timer expired",
        Some("PROF") => "Profiling timer expired",
        Some("WINCH") => "Window changed",
        Some("IO") => "I/O possible",
        Some("PWR") => "Power failure",
        Some("SYS") => "Bad system call",
        Some("EMT") => "EMT trap",
        Some("INFO") => "Information request",
        _ => return format!("Signal {n}"),
    };
    text.to_string()
}

/// Signals whose default action does not end a process: a job sent one runs on.
pub(crate) fn harmless(n: Signal) -> bool {
    n == 0
        || matches!(
            name(n).as_deref(),
            Some("SIGCHLD" | "SIGCONT" | "SIGURG" | "SIGWINCH" | "SIGINFO")
        )
}

/// Signals that stop rather than end a process.
pub(crate) fn stops(n: Signal) -> bool {
    matches!(
        name(n).as_deref(),
        Some("SIGSTOP" | "SIGTSTP" | "SIGTTIN" | "SIGTTOU")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_numbers_round_trip() {
        // Linux leaves 32 and 33 to the C library: no names.
        for n in 1..=max() {
            if let Some(s) = name(n) {
                assert_eq!(number(&s), Some(n), "{s}");
            }
        }
        assert!(name(15).is_some() && name(max() + 1).is_none());
        assert_eq!(number("term"), Some(15));
        assert_eq!(number("SigTerm"), Some(15));
        assert_eq!(number("EXIT"), Some(0));
        assert_eq!(parse("9"), Some(9));
        assert_eq!(parse("9999"), None);
        assert_eq!(parse("NOPE"), None);
        assert_eq!(describe(15), "Terminated");
        assert_eq!(describe(9), "Killed");
    }
}
