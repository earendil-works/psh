//! `printf [-v var] format [arguments]`: the format is reused until the arguments are
//! consumed; conversions `d i o u x X c s b q f F e E g G %`.

use crate::escape::{Dialect, decode};
use crate::state::{R, Vp, is_name};

use super::{out, shell_quote};

fn s(b: &[u8]) -> std::borrow::Cow<'_, str> {
    String::from_utf8_lossy(b)
}

pub(super) async fn printf(vp: &mut Vp, args: &[Vec<u8>]) -> R {
    let mut i = 1;
    let mut var: Option<String> = None;
    while i < args.len() {
        match &args[i][..] {
            b"--" => {
                i += 1;
                break;
            }
            b"-v" => {
                let Some(n) = args.get(i + 1) else {
                    vp.error("printf: -v: option requires an argument").await;
                    return Ok(2);
                };
                if !is_name(n) {
                    vp.error(&format!("printf: `{}': not a valid identifier", s(n)))
                        .await;
                    return Ok(2);
                }
                var = Some(s(n).into_owned());
                i += 2;
            }
            a if a.len() > 1 && a[0] == b'-' => {
                vp.error(&format!("printf: {}: invalid option", s(&a[..2])))
                    .await;
                vp.error_raw(b"printf: usage: printf [-v var] format [arguments]\n")
                    .await;
                return Ok(2);
            }
            _ => break,
        }
    }
    let Some(format) = args.get(i) else {
        vp.error_raw(b"printf: usage: printf [-v var] format [arguments]\n")
            .await;
        return Ok(2);
    };
    let mut f = Formatter {
        args: &args[i + 1..],
        next: 0,
        out: Vec::new(),
        errors: Vec::new(),
        stop: false,
    };
    loop {
        let before = f.next;
        f.run(format);
        if f.stop || f.next >= f.args.len() || f.next == before {
            break;
        }
    }
    let status = i32::from(!f.errors.is_empty());
    for e in &f.errors {
        vp.error(&format!("printf: {e}")).await;
    }
    match var {
        Some(n) => {
            vp.set_var(&n, f.out);
            Ok(status)
        }
        None => {
            let st = out(vp, "printf", &f.out).await?;
            Ok(if st != 0 { st } else { status })
        }
    }
}

struct Formatter<'a> {
    args: &'a [Vec<u8>],
    next: usize,
    out: Vec<u8>,
    errors: Vec<String>,
    stop: bool,
}

#[derive(Default)]
struct Spec {
    left: bool,
    zero: bool,
    plus: bool,
    space: bool,
    alt: bool,
    width: Option<usize>,
    prec: Option<usize>,
}

impl Formatter<'_> {
    fn arg(&mut self) -> Option<&[u8]> {
        let a = self.args.get(self.next);
        if a.is_some() {
            self.next += 1;
        }
        a.map(Vec::as_slice)
    }

    fn num_arg(&mut self) -> i64 {
        match self.arg() {
            None => 0,
            Some(a) => {
                let a = a.to_vec();
                let (v, ok) = parse_num(&a);
                if !ok {
                    self.errors.push(format!("{}: invalid number", s(&a)));
                }
                v
            }
        }
    }

    fn float_arg(&mut self) -> f64 {
        match self.arg() {
            None => 0.0,
            Some(a) => {
                let a = a.to_vec();
                let t = s(&a);
                let t = t.trim();
                if let Some(c) = t.strip_prefix(['\'', '"']) {
                    return c.chars().next().map_or(0.0, |c| f64::from(c as u32));
                }
                match t.parse::<f64>() {
                    Ok(v) => v,
                    Err(_) => {
                        let (v, ok) = parse_num(&a);
                        if !ok {
                            self.errors.push(format!("{}: invalid number", s(&a)));
                        }
                        v as f64
                    }
                }
            }
        }
    }

    fn run(&mut self, fmt: &[u8]) {
        let mut i = 0;
        while i < fmt.len() && !self.stop {
            let c = fmt[i];
            if c == b'\\' {
                // One escape at a time, so `\c` is literal here and octal is `\nnn`.
                let mut j = i + 1;
                if j < fmt.len() {
                    j += 1;
                    let lead = fmt[i + 1];
                    let max = match lead {
                        b'0'..=b'7' => 2,
                        b'x' => 2,
                        b'u' => 4,
                        b'U' => 8,
                        _ => 0,
                    };
                    let mut n = 0;
                    while n < max && j < fmt.len() && fmt[j].is_ascii_hexdigit() {
                        if matches!(lead, b'0'..=b'7') && !(b'0'..=b'7').contains(&fmt[j]) {
                            break;
                        }
                        j += 1;
                        n += 1;
                    }
                }
                self.out
                    .extend_from_slice(&decode(&fmt[i..j], Dialect::Printf).bytes);
                i = j;
                continue;
            }
            if c != b'%' {
                self.out.push(c);
                i += 1;
                continue;
            }
            i += 1;
            if i >= fmt.len() {
                self.out.push(b'%');
                break;
            }
            if fmt[i] == b'%' {
                self.out.push(b'%');
                i += 1;
                continue;
            }
            let mut spec = Spec::default();
            while i < fmt.len() {
                match fmt[i] {
                    b'-' => spec.left = true,
                    b'0' => spec.zero = true,
                    b'+' => spec.plus = true,
                    b' ' => spec.space = true,
                    b'#' => spec.alt = true,
                    _ => break,
                }
                i += 1;
            }
            if i < fmt.len() && fmt[i] == b'*' {
                let w = self.num_arg();
                if w < 0 {
                    spec.left = true;
                }
                spec.width = Some(w.unsigned_abs() as usize);
                i += 1;
            } else {
                let st = i;
                while i < fmt.len() && fmt[i].is_ascii_digit() {
                    i += 1;
                }
                if i > st {
                    spec.width = s(&fmt[st..i]).parse().ok();
                }
            }
            if i < fmt.len() && fmt[i] == b'.' {
                i += 1;
                if i < fmt.len() && fmt[i] == b'*' {
                    spec.prec = Some(self.num_arg().max(0) as usize);
                    i += 1;
                } else {
                    let st = i;
                    while i < fmt.len() && fmt[i].is_ascii_digit() {
                        i += 1;
                    }
                    spec.prec = Some(s(&fmt[st..i]).parse().unwrap_or(0));
                }
            }
            // Length modifiers are accepted and ignored.
            while i < fmt.len() && b"hlLqjzt".contains(&fmt[i]) {
                i += 1;
            }
            let Some(&conv) = fmt.get(i) else {
                self.errors.push("`%': missing format character".into());
                self.stop = true;
                break;
            };
            i += 1;
            self.convert(conv, &spec);
        }
    }

    fn pad(&mut self, body: &[u8], spec: &Spec, numeric: bool) {
        let w = spec.width.unwrap_or(0);
        if body.len() >= w {
            self.out.extend_from_slice(body);
            return;
        }
        let fill = w - body.len();
        if spec.left {
            self.out.extend_from_slice(body);
            self.out.extend(std::iter::repeat_n(b' ', fill));
        } else if spec.zero && numeric {
            // Zeros go after the sign or 0x prefix.
            let split = if body.starts_with(b"0x") || body.starts_with(b"0X") {
                2
            } else {
                usize::from(matches!(body.first(), Some(b'-' | b'+' | b' ')))
            };
            self.out.extend_from_slice(&body[..split]);
            self.out.extend(std::iter::repeat_n(b'0', fill));
            self.out.extend_from_slice(&body[split..]);
        } else {
            self.out.extend(std::iter::repeat_n(b' ', fill));
            self.out.extend_from_slice(body);
        }
    }

    fn convert(&mut self, conv: u8, spec: &Spec) {
        match conv {
            b's' | b'b' | b'q' => {
                let a = self.arg().map(<[u8]>::to_vec).unwrap_or_default();
                let mut body = match conv {
                    b's' => a,
                    b'q' => shell_quote(&a),
                    _ => {
                        let d = decode(&a, Dialect::Echo);
                        if d.stop {
                            self.stop = true;
                        }
                        d.bytes
                    }
                };
                if let Some(p) = spec.prec {
                    body.truncate(p);
                }
                self.pad(&body, spec, false);
            }
            b'c' => {
                let a = self.arg().map(<[u8]>::to_vec).unwrap_or_default();
                let body: Vec<u8> = a.first().map(|b| vec![*b]).unwrap_or_default();
                self.pad(&body, spec, false);
            }
            b'd' | b'i' => {
                let v = self.num_arg();
                let mut digits = v.unsigned_abs().to_string();
                if let Some(p) = spec.prec {
                    while digits.len() < p {
                        digits.insert(0, '0');
                    }
                    if p == 0 && v == 0 {
                        digits.clear();
                    }
                }
                let sign = if v < 0 {
                    "-"
                } else if spec.plus {
                    "+"
                } else if spec.space {
                    " "
                } else {
                    ""
                };
                let body = format!("{sign}{digits}");
                let numeric = spec.prec.is_none();
                self.pad(body.as_bytes(), spec, numeric);
            }
            b'u' | b'o' | b'x' | b'X' => {
                let v = self.num_arg() as u64;
                let mut digits = match conv {
                    b'u' => v.to_string(),
                    b'o' => format!("{v:o}"),
                    b'x' => format!("{v:x}"),
                    _ => format!("{v:X}"),
                };
                if let Some(p) = spec.prec {
                    while digits.len() < p {
                        digits.insert(0, '0');
                    }
                }
                if spec.alt && v != 0 {
                    match conv {
                        b'o' if !digits.starts_with('0') => digits.insert(0, '0'),
                        b'x' => digits.insert_str(0, "0x"),
                        b'X' => digits.insert_str(0, "0X"),
                        _ => {}
                    }
                }
                let numeric = spec.prec.is_none();
                self.pad(digits.as_bytes(), spec, numeric);
            }
            b'f' | b'F' | b'e' | b'E' | b'g' | b'G' => {
                let v = self.float_arg();
                let prec = spec.prec.unwrap_or(6);
                let mut body = match conv {
                    b'f' | b'F' => format!("{v:.prec$}"),
                    b'e' | b'E' => fmt_e(v, prec),
                    _ => fmt_g(v, prec, spec.alt),
                };
                if conv.is_ascii_uppercase() {
                    body = body.to_uppercase();
                }
                if v >= 0.0 && !body.starts_with('-') {
                    if spec.plus {
                        body.insert(0, '+');
                    } else if spec.space {
                        body.insert(0, ' ');
                    }
                }
                self.pad(body.as_bytes(), spec, true);
            }
            other => {
                self.errors
                    .push(format!("%{}: invalid format character", other as char));
                self.stop = true;
            }
        }
    }
}

/// C's `%e`: `d.dddddde+XX`.
fn fmt_e(v: f64, prec: usize) -> String {
    if !v.is_finite() {
        return if v.is_nan() {
            "nan".into()
        } else if v > 0.0 {
            "inf".into()
        } else {
            "-inf".into()
        };
    }
    let s = format!("{v:.prec$e}");
    let (mant, exp) = s.split_once('e').unwrap_or((&s, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let sign = if exp < 0 { '-' } else { '+' };
    format!("{mant}e{sign}{:02}", exp.abs())
}

/// C's `%g`: the shorter of `%e` and `%f` with `prec` significant digits, trailing
/// zeros removed unless `#`.
fn fmt_g(v: f64, prec: usize, alt: bool) -> String {
    if !v.is_finite() {
        return fmt_e(v, 0);
    }
    let p = if prec == 0 { 1 } else { prec };
    let exp = if v == 0.0 {
        0
    } else {
        let e = format!("{v:.*e}", p - 1);
        e.split_once('e')
            .and_then(|(_, x)| x.parse::<i32>().ok())
            .unwrap_or(0)
    };
    let mut out = if exp < -4 || exp >= p as i32 {
        fmt_e(v, p - 1)
    } else {
        let decimals = (p as i32 - 1 - exp).max(0) as usize;
        format!("{v:.decimals$}")
    };
    if !alt {
        let (num, suffix) = match out.find('e') {
            Some(i) => (out[..i].to_string(), out[i..].to_string()),
            None => (out.clone(), String::new()),
        };
        let num = if num.contains('.') {
            num.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            num
        };
        out = format!("{num}{suffix}");
    }
    out
}

/// Parses an integer argument like `strtol` with base 0, plus `'c` for a char code.
/// Returns the value and whether the whole argument was a number.
fn parse_num(a: &[u8]) -> (i64, bool) {
    let t = s(a);
    let t = t.trim_start();
    if let Some(c) = t.strip_prefix(['\'', '"']) {
        return (c.chars().next().map_or(0, |c| i64::from(c as u32)), true);
    }
    let (neg, body) = match t.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let (radix, digits) =
        if let Some(h) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
            (16, h)
        } else if body.len() > 1 && body.starts_with('0') {
            (8, &body[1..])
        } else {
            (10, body)
        };
    let end = digits
        .find(|c: char| !c.is_digit(radix))
        .unwrap_or(digits.len());
    let v = i64::from_str_radix(&digits[..end], radix).unwrap_or(0);
    let ok = end == digits.len() && (end > 0 || radix == 8);
    (if neg { -v } else { v }, ok && !t.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats() {
        assert_eq!(fmt_e(1234.5, 6), "1.234500e+03");
        assert_eq!(fmt_g(0.0001, 6, false), "0.0001");
        assert_eq!(fmt_g(1e10, 6, false), "1e+10");
        assert_eq!(fmt_g(3.0, 6, false), "3");
    }

    #[test]
    fn numbers() {
        assert_eq!(parse_num(b"0x1f"), (31, true));
        assert_eq!(parse_num(b"010"), (8, true));
        assert_eq!(parse_num(b"'A"), (65, true));
        assert_eq!(parse_num(b"12abc"), (12, false));
    }
}
