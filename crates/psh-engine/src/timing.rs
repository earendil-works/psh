//! The `time` keyword's report (`TIMEFORMAT`) and the `times` builtin's numbers.
//!
//! Real time comes from `Host::now`, user and system time from `Host::cpu_times`: the
//! process's own and its waited-for children's, summed, as bash reports them.

use std::fmt::Write as _;
use std::time::Duration;

use crate::host::CpuTimes;
use crate::state::Vp;

/// bash's format when `TIMEFORMAT` is unset.
const DEFAULT: &[u8] = b"\nreal\t%3lR\nuser\t%3lU\nsys\t%3lS";
/// `time -p`, whatever `TIMEFORMAT` says.
const POSIX: &[u8] = b"real %2R\nuser %2U\nsys %2S";

/// The clocks when a timed pipeline starts.
pub(crate) struct Start {
    real: Duration,
    cpu: CpuTimes,
}

impl Start {
    pub(crate) fn now(vp: &Vp) -> Start {
        Start {
            real: vp.host().now(),
            cpu: vp.host().cpu_times(),
        }
    }
}

/// Writes the report of a pipeline timed since `start` to the shell's stderr. An empty
/// `TIMEFORMAT` prints nothing; an invalid one prints an error instead of the report.
pub(crate) async fn report(vp: &Vp, posix: bool, start: &Start) {
    let fmt = match (posix, vp.var("TIMEFORMAT")) {
        (true, _) => POSIX.to_vec(),
        (false, Some(f)) => f.to_vec(),
        (false, None) => DEFAULT.to_vec(),
    };
    if fmt.is_empty() {
        return;
    }
    let end = Start::now(vp);
    let real = end.real.saturating_sub(start.real);
    let user =
        (end.cpu.user + end.cpu.child_user).saturating_sub(start.cpu.user + start.cpu.child_user);
    let sys = (end.cpu.sys + end.cpu.child_sys).saturating_sub(start.cpu.sys + start.cpu.child_sys);
    match format(&fmt, real, user, sys) {
        Ok(mut text) => {
            text.push(b'\n');
            vp.error_raw(&text).await;
        }
        Err(c) => {
            let c = if c.is_ascii() {
                char::from(c)
            } else {
                '\u{fffd}'
            };
            vp.error(&format!("TIMEFORMAT: `{c}': invalid format character"))
                .await;
        }
    }
}

/// `times`: the shell's user and system time, then its children's.
pub(crate) fn times(cpu: CpuTimes) -> Vec<u8> {
    let mut out = Vec::new();
    for (i, d) in [cpu.user, cpu.sys, cpu.child_user, cpu.child_sys]
        .into_iter()
        .enumerate()
    {
        seconds(&mut out, d, 3, true);
        out.push(if i % 2 == 0 { b' ' } else { b'\n' });
    }
    out
}

/// Expands a `TIMEFORMAT`: `%[p][l]R`, `U` and `S` are the real, user and system time
/// with `p` (0-6, default 3) decimals, `l` as `MmS.FFFs`; `%P` is the CPU percentage,
/// `%%` a `%`. Anything else after a `%` is an error, naming the offending byte (NUL at
/// the end of the format); a `%` that ends the format stands for itself.
pub(crate) fn format(
    fmt: &[u8],
    real: Duration,
    user: Duration,
    sys: Duration,
) -> Result<Vec<u8>, u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < fmt.len() {
        let c = fmt[i];
        i += 1;
        if c != b'%' {
            out.push(c);
            continue;
        }
        let Some(&next) = fmt.get(i) else {
            out.push(b'%');
            break;
        };
        match next {
            b'%' => {
                out.push(b'%');
                i += 1;
                continue;
            }
            b'P' => {
                // bash truncates to a whole percent and always shows `.00`.
                let pct = ((user + sys).as_micros() * 100)
                    .checked_div(real.as_micros())
                    .unwrap_or(0);
                let _ = write!(Text(&mut out), "{pct}.00");
                i += 1;
                continue;
            }
            _ => {}
        }
        let mut precision = 3;
        if next.is_ascii_digit() {
            precision = u32::from(next - b'0').min(6);
            i += 1;
        }
        let long = fmt.get(i) == Some(&b'l');
        if long {
            i += 1;
        }
        let d = match fmt.get(i) {
            Some(b'R') => real,
            Some(b'U') => user,
            Some(b'S') => sys,
            other => return Err(other.copied().unwrap_or(0)),
        };
        i += 1;
        seconds(&mut out, d, precision, long);
    }
    Ok(out)
}

/// `d` in seconds with `precision` decimals (rounded; whole seconds are truncated), or
/// as `MmS.FFFs` when `long`.
fn seconds(out: &mut Vec<u8>, d: Duration, precision: u32, long: bool) {
    let mut secs = d.as_secs();
    if long {
        let _ = write!(Text(out), "{}m", secs / 60);
        secs %= 60;
    }
    let _ = write!(Text(out), "{secs}");
    if precision > 0 {
        let us = u64::from(d.subsec_micros());
        let fraction = if precision < 6 {
            let unit = 10u64.pow(6 - precision);
            (us + unit / 2) / unit
        } else {
            us
        };
        let rest = 10u64.pow(precision - 1);
        out.push(b'.');
        // Rounding up from .95 does not carry into the seconds in bash: the first
        // decimal becomes ten, shown as the byte after `9` (`0.:`).
        out.push(b'0' + (fraction / rest) as u8);
        if precision > 1 {
            let _ = write!(
                Text(out),
                "{:0w$}",
                fraction % rest,
                w = (precision - 1) as usize
            );
        }
    }
    if long {
        out.push(b's');
    }
}

/// `fmt::Write` into a byte buffer.
struct Text<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for Text<'_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn fmt(f: &str, real: Duration, user: Duration, sys: Duration) -> Result<String, u8> {
        format(f.as_bytes(), real, user, sys).map(|v| String::from_utf8(v).unwrap())
    }

    #[test]
    fn default_and_posix_formats() {
        let r = fmt(
            std::str::from_utf8(DEFAULT).unwrap(),
            ms(61_234),
            ms(5),
            ms(1_500),
        );
        assert_eq!(
            r.unwrap(),
            "\nreal\t1m1.234s\nuser\t0m0.005s\nsys\t0m1.500s"
        );
        let r = fmt(std::str::from_utf8(POSIX).unwrap(), ms(1_234), ms(5), ms(0));
        assert_eq!(r.unwrap(), "real 1.23\nuser 0.01\nsys 0.00");
    }

    #[test]
    fn precision_and_long_form() {
        let real = Duration::from_micros(59_709_412);
        let z = Duration::ZERO;
        assert_eq!(
            fmt("%R|%0R|%1R|%2R|%4R|%6R|%9R", real, z, z).unwrap(),
            "59.709|59|59.7|59.71|59.7094|59.709412|59.709412"
        );
        assert_eq!(
            fmt("%lR %0lR %1lR", real, z, z).unwrap(),
            "0m59.709s 0m59s 0m59.7s"
        );
        // Whole seconds truncate, decimals round.
        assert_eq!(fmt("%0R %1R", ms(777), z, z).unwrap(), "0 0.8");
    }

    #[test]
    fn rounding_past_nine_does_not_carry() {
        let z = Duration::ZERO;
        assert_eq!(fmt("%1R %2R", ms(979), z, z).unwrap(), "0.: 0.98");
        assert_eq!(fmt("%2R", ms(999), z, z).unwrap(), "0.:0");
    }

    #[test]
    fn percent_and_literals() {
        assert_eq!(fmt("%P", ms(397), ms(308), ms(37)).unwrap(), "86.00");
        assert_eq!(fmt("%P", Duration::ZERO, ms(1), ms(1)).unwrap(), "0.00");
        assert_eq!(fmt("a%%b%", ms(1), ms(1), ms(1)).unwrap(), "a%b%");
        assert_eq!(fmt("ab%Ucd", ms(1), ms(2), ms(3)).unwrap(), "ab0.002cd");
    }

    #[test]
    fn invalid_format_characters() {
        let z = Duration::ZERO;
        for (f, bad) in [
            ("%x", b'x'),
            ("%l1R", b'1'),
            ("%10R", b'0'),
            ("%0P", b'P'),
            ("%lP", b'P'),
            ("%ll", b'l'),
            ("%5", 0),
            ("%l", 0),
            ("%q%R", b'q'),
        ] {
            assert_eq!(fmt(f, z, z, z), Err(bad), "{f}");
        }
    }

    #[test]
    fn times_lines() {
        let cpu = CpuTimes {
            user: ms(3),
            sys: ms(4),
            child_user: Duration::from_secs(75),
            child_sys: ms(0),
        };
        assert_eq!(times(cpu), b"0m0.003s 0m0.004s\n1m15.000s 0m0.000s\n");
    }
}
