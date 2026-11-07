//! Dates for the file utilities: the civil calendar, local time through the host's
//! zone, GNU date strings for `touch -d` (a subset: ISO dates and times, `@SECONDS`,
//! month names, zones, relative items like `2 hours ago`) and `touch -t` stamps.
//! Written for psh after GNU's "Date input formats" manual and POSIX `touch`.

use psh_engine::util::UtilCtx;

pub const NS: i64 = 1_000_000_000;

/// Days since 1970-01-01 of a proleptic Gregorian date.
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = i64::from(m);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// The date of a day number.
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// A broken-down time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tm {
    pub year: i64,
    pub mon: u32,
    pub day: u32,
    pub hour: u32,
    pub min: u32,
    pub sec: u32,
    pub nsec: u32,
    /// 0 = Sunday.
    pub wday: u32,
    /// 0-based day of the year.
    pub yday: u32,
    /// Seconds east of UTC.
    pub offset: i32,
}

/// `ns` since the epoch broken down with `offset`.
pub fn tm_at(ns: i64, offset: i32) -> Tm {
    let secs = ns.div_euclid(NS) + i64::from(offset);
    let nsec = ns.rem_euclid(NS) as u32;
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let (year, mon, day) = civil_from_days(days);
    Tm {
        year,
        mon,
        day,
        hour: (rem / 3600) as u32,
        min: (rem / 60 % 60) as u32,
        sec: (rem % 60) as u32,
        nsec,
        wday: (days + 4).rem_euclid(7) as u32,
        yday: (days - days_from_civil(year, 1, 1)) as u32,
        offset,
    }
}

/// Local time of `ns` in the command's zone.
pub fn local(ctx: &UtilCtx<'_>, ns: i64) -> Tm {
    tm_at(ns, ctx.utc_offset(ns.div_euclid(NS)))
}

/// Seconds since the epoch of a wall-clock time; `offset` given, or the local zone's.
fn epoch_of(ctx: &UtilCtx<'_>, wall: i64, offset: Option<i32>) -> i64 {
    match offset {
        Some(o) => wall - i64::from(o),
        None => {
            let guess = wall - i64::from(ctx.utc_offset(wall));
            wall - i64::from(ctx.utc_offset(guess))
        }
    }
}

fn wall_secs(y: i64, m: u32, d: u32, h: u32, mi: u32, s: u32) -> i64 {
    days_from_civil(y, m, d) * 86400 + i64::from(h * 3600 + mi * 60 + s)
}

/// `touch -t [[CC]YY]MMDDhhmm[.ss]`, local time: ns since the epoch.
pub fn parse_stamp(ctx: &UtilCtx<'_>, s: &[u8], now_ns: i64) -> Option<i64> {
    let (main, secs) = match s.iter().position(|&b| b == b'.') {
        Some(i) => (&s[..i], Some(&s[i + 1..])),
        None => (s, None),
    };
    if !main.iter().all(u8::is_ascii_digit)
        || secs.is_some_and(|x| x.len() != 2 || !x.iter().all(u8::is_ascii_digit))
    {
        return None;
    }
    let num = |b: &[u8]| -> u32 {
        std::str::from_utf8(b)
            .ok()
            .and_then(|t| t.parse().ok())
            .unwrap_or(99)
    };
    let (year, rest) = match main.len() {
        8 => (local(ctx, now_ns).year, main),
        10 => {
            let yy = i64::from(num(&main[..2]));
            (if yy >= 69 { 1900 + yy } else { 2000 + yy }, &main[2..])
        }
        12 => (i64::from(num(&main[..4])), &main[4..]),
        _ => return None,
    };
    let (mo, d, h, mi) = (
        num(&rest[..2]),
        num(&rest[2..4]),
        num(&rest[4..6]),
        num(&rest[6..8]),
    );
    let sec = secs.map_or(0, num);
    if !(1..=12).contains(&mo)
        || d == 0
        || d > days_in_month(year, mo)
        || h > 23
        || mi > 59
        || sec > 60
    {
        return None;
    }
    let wall = wall_secs(year, mo, d, h, mi, sec.min(59));
    Some(epoch_of(ctx, wall, None) * NS)
}

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

fn month_named(w: &str) -> Option<u32> {
    if w.len() < 3 {
        return None;
    }
    let w = if w == "sept" { "sep" } else { w };
    MONTHS
        .iter()
        .position(|m| m.starts_with(w) && (w.len() == 3 || *m == w))
        .map(|i| i as u32 + 1)
}

/// A relative unit: (months, seconds) per unit.
fn unit(w: &str) -> Option<(i64, i64)> {
    let w = w.strip_suffix('s').filter(|s| !s.is_empty()).unwrap_or(w);
    Some(match w {
        "year" => (12, 0),
        "month" => (1, 0),
        "fortnight" => (0, 14 * 86400),
        "week" => (0, 7 * 86400),
        "day" => (0, 86400),
        "hour" => (0, 3600),
        "minute" | "min" => (0, 60),
        "second" | "sec" => (0, 1),
        _ => return None,
    })
}

#[derive(Default)]
struct Parsed {
    date: Option<(i64, u32, u32)>,
    time: Option<(u32, u32, u32, u32)>,
    zone: Option<i32>,
    months: i64,
    days: i64,
    secs: i64,
}

struct Lexer<'a> {
    s: &'a [u8],
    i: usize,
}

impl Lexer<'_> {
    fn skip_space(&mut self) {
        while self.i < self.s.len()
            && (self.s[self.i].is_ascii_whitespace() || self.s[self.i] == b',')
        {
            self.i += 1;
        }
    }
    fn digits(&mut self) -> Option<(u32, usize)> {
        let start = self.i;
        while self.i < self.s.len() && self.s[self.i].is_ascii_digit() {
            self.i += 1;
        }
        let n = &self.s[start..self.i];
        if n.is_empty() || n.len() > 9 {
            self.i = start;
            return None;
        }
        Some((std::str::from_utf8(n).ok()?.parse().ok()?, n.len()))
    }
    fn eat(&mut self, c: u8) -> bool {
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn word(&mut self) -> String {
        let start = self.i;
        while self.i < self.s.len() && self.s[self.i].is_ascii_alphabetic() {
            self.i += 1;
        }
        String::from_utf8_lossy(&self.s[start..self.i]).to_ascii_lowercase()
    }
    fn peek_word(&self) -> String {
        let mut j = self.i;
        while j < self.s.len() && self.s[j].is_ascii_whitespace() {
            j += 1;
        }
        let start = j;
        while j < self.s.len() && self.s[j].is_ascii_alphabetic() {
            j += 1;
        }
        String::from_utf8_lossy(&self.s[start..j]).to_ascii_lowercase()
    }
}

/// A numeric zone right after a time or alone: `+0200`, `-05:30`, `+3`.
fn zone_number(lx: &mut Lexer<'_>) -> Option<i32> {
    let save = lx.i;
    let sign = match lx.s.get(lx.i) {
        Some(b'+') => 1,
        Some(b'-') => -1,
        _ => return None,
    };
    lx.i += 1;
    let Some((n, len)) = lx.digits() else {
        lx.i = save;
        return None;
    };
    let (h, m) = if lx.eat(b':') {
        let Some((m, 2)) = lx.digits() else {
            lx.i = save;
            return None;
        };
        (n, m)
    } else if len <= 2 {
        (n, 0)
    } else if len == 4 {
        (n / 100, n % 100)
    } else {
        lx.i = save;
        return None;
    };
    Some(sign * (h as i32 * 3600 + m as i32 * 60))
}

/// A time `HH:MM[:SS[.frac]]` at the cursor, with `am`/`pm` and a zone after it.
fn time_of_day(lx: &mut Lexer<'_>, p: &mut Parsed, h: u32) -> Option<()> {
    let (mi, _) = lx.digits()?;
    let (mut sec, mut nsec) = (0, 0);
    if lx.eat(b':') {
        sec = lx.digits()?.0;
        if lx.eat(b'.') || lx.eat(b',') {
            let start = lx.i;
            lx.digits()?;
            let frac = std::str::from_utf8(&lx.s[start..lx.i]).ok()?;
            let mut f = frac.to_string();
            f.truncate(9);
            while f.len() < 9 {
                f.push('0');
            }
            nsec = f.parse().ok()?;
        }
    }
    let mut h = h;
    let w = lx.peek_word();
    if w == "am" || w == "pm" {
        lx.skip_space();
        lx.word();
        if h == 0 || h > 12 {
            return None;
        }
        h = h % 12 + if w == "pm" { 12 } else { 0 };
    }
    if h > 23 || mi > 59 || sec > 60 {
        return None;
    }
    p.time = Some((h, mi, sec.min(59), nsec));
    if lx.eat(b'z') || lx.eat(b'Z') {
        p.zone = Some(0);
    } else if let Some(z) = zone_number(lx) {
        p.zone = Some(z);
    }
    Some(())
}

/// A GNU date string (the subset above): ns since the epoch, relative to `now_ns`.
pub fn parse_date(ctx: &UtilCtx<'_>, s: &[u8], now_ns: i64) -> Option<i64> {
    let text = std::str::from_utf8(s).ok()?.trim();
    if let Some(n) = text.strip_prefix('@') {
        let (neg, n) = match n.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, n.strip_prefix('+').unwrap_or(n)),
        };
        let (whole, frac) = n.split_once(['.', ',']).unwrap_or((n, ""));
        if whole.is_empty()
            || !whole.bytes().all(|b| b.is_ascii_digit())
            || !frac.bytes().all(|b| b.is_ascii_digit())
        {
            return None;
        }
        let mut f = frac.to_string();
        f.truncate(9);
        while f.len() < 9 {
            f.push('0');
        }
        let v = whole.parse::<i64>().ok()?.checked_mul(NS)? + f.parse::<i64>().ok()?;
        return Some(if neg { -v } else { v });
    }
    let mut p = Parsed::default();
    let mut lx = Lexer {
        s: text.as_bytes(),
        i: 0,
    };
    // The last relative item, for `ago`.
    let mut last_rel: Option<(i64, i64, i64)> = None;
    loop {
        lx.skip_space();
        if lx.i >= lx.s.len() {
            break;
        }
        let c = lx.s[lx.i];
        if c.is_ascii_digit()
            || ((c == b'+' || c == b'-') && lx.s.get(lx.i + 1).is_some_and(u8::is_ascii_digit))
        {
            // A zone after a date and time: `2024-01-01 10:00 +0200`.
            if (c == b'+' || c == b'-') && p.time.is_some() && p.zone.is_none() {
                let save = lx.i;
                if let Some(z) = zone_number(&mut lx)
                    && unit(&lx.peek_word()).is_none()
                {
                    p.zone = Some(z);
                    continue;
                }
                lx.i = save;
            }
            let sign = if lx.eat(b'-') {
                -1
            } else {
                lx.eat(b'+');
                1
            };
            let start = lx.i;
            let (n, len) = lx.digits()?;
            if sign == 1 && lx.s.get(lx.i) == Some(&b'-') && len == 4 && c != b'+' {
                // ISO 8601 date.
                lx.i += 1;
                let (m, _) = lx.digits()?;
                lx.eat(b'-').then_some(())?;
                let (d, _) = lx.digits()?;
                let y = i64::from(n);
                if !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) {
                    return None;
                }
                p.date = Some((y, m, d));
                if lx.eat(b'T') || lx.eat(b't') {
                    let (h, _) = lx.digits()?;
                    lx.eat(b':').then_some(())?;
                    time_of_day(&mut lx, &mut p, h)?;
                }
                continue;
            }
            if sign == 1 && c != b'+' && lx.eat(b':') {
                time_of_day(&mut lx, &mut p, n)?;
                continue;
            }
            if sign == 1 && c != b'+' && lx.eat(b'/') {
                // US date: MM/DD[/YYYY].
                let (d, _) = lx.digits()?;
                let y = if lx.eat(b'/') {
                    let (y, ylen) = lx.digits()?;
                    let y = i64::from(y);
                    if ylen == 2 {
                        if y >= 69 { 1900 + y } else { 2000 + y }
                    } else {
                        y
                    }
                } else {
                    local(ctx, now_ns).year
                };
                if !(1..=12).contains(&n) || d == 0 || d > days_in_month(y, n) {
                    return None;
                }
                p.date = Some((y, n, d));
                continue;
            }
            let w = lx.peek_word();
            if (w == "am" || w == "pm") && sign == 1 && c != b'+' {
                lx.skip_space();
                lx.word();
                if n == 0 || n > 12 {
                    return None;
                }
                p.time = Some((n % 12 + if w == "pm" { 12 } else { 0 }, 0, 0, 0));
                continue;
            }
            if let Some(mon) = month_named(&w).filter(|_| sign == 1 && c != b'+') {
                // `2 Jan 2024`.
                lx.skip_space();
                lx.word();
                let y = year_after(&mut lx).unwrap_or_else(|| local(ctx, now_ns).year);
                p.date = Some((y, mon, n));
                continue;
            }
            if let Some((months, secs)) = unit(&w) {
                lx.skip_space();
                lx.word();
                let k = i64::from(n) * sign;
                let rel = (months * k, 0, secs * k);
                add_rel(&mut p, rel);
                last_rel = Some(rel);
                continue;
            }
            let _ = start;
            return None;
        }
        let w = lx.word();
        if w.is_empty() {
            return None;
        }
        if let Some(mon) = month_named(&w) {
            // `Jan 2 [2024]`.
            lx.skip_space();
            let (d, _) = lx.digits()?;
            let y = year_after(&mut lx).unwrap_or_else(|| local(ctx, now_ns).year);
            p.date = Some((y, mon, d));
            continue;
        }
        let rel = match w.as_str() {
            "now" | "today" => (0, 0, 0),
            "yesterday" => (0, -1, 0),
            "tomorrow" => (0, 1, 0),
            "utc" | "gmt" | "ut" | "z" => {
                p.zone = Some(0);
                continue;
            }
            "ago" => {
                let (m, d, s) = last_rel.take()?;
                add_rel(&mut p, (-2 * m, -2 * d, -2 * s));
                continue;
            }
            "last" | "next" | "this" => {
                lx.skip_space();
                let u = lx.word();
                let (months, secs) = unit(&u)?;
                let k = match w.as_str() {
                    "last" => -1,
                    "next" => 1,
                    _ => 0,
                };
                (months * k, 0, secs * k)
            }
            _ => {
                let (months, secs) = unit(&w)?;
                (months, 0, secs)
            }
        };
        add_rel(&mut p, rel);
        last_rel = Some(rel);
    }
    // Assemble: the date and time given, else now's; then the relative items.
    let now = local(ctx, now_ns);
    let base_off = p.zone;
    let (mut y, mut m, mut d) = p.date.unwrap_or((now.year, now.mon, now.day));
    let (h, mi, s, ns) = match (p.time, p.date) {
        (Some(t), _) => t,
        (None, Some(_)) => (0, 0, 0, 0),
        (None, None) => (now.hour, now.min, now.sec, now.nsec),
    };
    if p.months != 0 {
        let total = y * 12 + i64::from(m - 1) + p.months;
        y = total.div_euclid(12);
        m = (total.rem_euclid(12) + 1) as u32;
    }
    // Day overflow after a month shift rolls into the next month, as GNU's mktime does.
    let day_num = days_from_civil(y, m, 1) + i64::from(d) - 1 + p.days;
    let (yy, mm, dd) = civil_from_days(day_num);
    (y, m, d) = (yy, mm, dd);
    let wall = wall_secs(y, m, d, h, mi, s);
    let secs = epoch_of(ctx, wall, base_off) + p.secs;
    secs.checked_mul(NS)?.checked_add(i64::from(ns))
}

/// A four-digit year after a month and day (`Jan 2 2024`), not a time (`Jan 2 10:00`).
fn year_after(lx: &mut Lexer<'_>) -> Option<i64> {
    let save = lx.i;
    lx.skip_space();
    match lx.digits() {
        Some((y, 4)) if lx.s.get(lx.i) != Some(&b':') => Some(i64::from(y)),
        _ => {
            lx.i = save;
            None
        }
    }
}

fn add_rel(p: &mut Parsed, (m, d, s): (i64, i64, i64)) {
    p.months += m;
    p.days += d;
    p.secs += s;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11017);
        assert_eq!(civil_from_days(11017), (2000, 3, 1));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        let t = tm_at(1_700_000_000 * NS, 0);
        assert_eq!(
            (t.year, t.mon, t.day, t.hour, t.min, t.sec),
            (2023, 11, 14, 22, 13, 20)
        );
        assert_eq!((t.wday, t.yday), (2, 317));
    }
}
