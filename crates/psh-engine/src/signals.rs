//! Signal helpers for `kill`, `jobs` and `wait` (D1900), over the engine's one signal
//! table in `builtins::trap` (D1283): Linux numbering on every platform, as the goldens
//! and `kill -l` show it; hosts map the numbers to the platform's own. Descriptions are
//! glibc's `strsignal` texts, as bash prints them on Linux.

use crate::builtins::trap::{signal_name, signal_number};
use crate::host::Signal;

/// The highest signal number (`SIGRTMAX`).
pub(crate) fn max() -> Signal {
    64
}

/// `SIGTERM` for 15, `SIGRTMIN+2` for 36; `None` when there is no such signal.
pub(crate) fn name(n: Signal) -> Option<String> {
    signal_name(n)
}

/// The number of a signal name: `TERM`, `SIGTERM`, `term` (any case), `RTMIN+2`.
/// `EXIT` is 0.
pub(crate) fn number(s: &str) -> Option<Signal> {
    if s.eq_ignore_ascii_case("EXIT") || s.eq_ignore_ascii_case("SIGEXIT") {
        return Some(0);
    }
    if s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let up = s.to_ascii_uppercase();
    match up.strip_prefix("SIG").unwrap_or(&up) {
        "IOT" => Some(6),
        "POLL" => Some(29),
        _ => signal_number(s),
    }
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
        _ => return format!("Signal {n}"),
    };
    text.to_string()
}

/// Signals whose default action does not end a process: a job sent one runs on.
pub(crate) fn harmless(n: Signal) -> bool {
    n == 0
        || matches!(
            name(n).as_deref(),
            Some("SIGCHLD" | "SIGCONT" | "SIGURG" | "SIGWINCH")
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
        assert_eq!(number("USR1"), Some(10));
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
