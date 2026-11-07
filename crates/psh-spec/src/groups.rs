//! `tests/spec.toml`: which group each case is in (TESTING.md "Groups").
//!
//! ```toml
//! [[skip]]
//! match = ["oils/interactive/*", "brush/brush/**"]   # globs on case ids
//! reason = "interactive features (D002)"
//!
//! [[later]]
//! tag = "pty"            # or a tag: pty, interactive, a file's `## tags:`
//! reason = "..."
//! ```
//!
//! `[[serial]]` tables (same keys) are not a group: they mark cases that must not run
//! concurrently with each other, in this run or another psh-spec run on the machine,
//! because they use fixed paths outside their directory.
//!
//! `[[flaky]]` tables (same keys, `reason` required) are not a group either: they
//! quarantine cases that pass or fail by timing (D1225). Such a case runs as usual and
//! is reported, but never fails the run and is never in a ratchet.
//!
//! `[[platform]]` tables (same keys, `reason` required) are not a group either: they tag
//! cases that are inherently host-specific (`/dev/full`, `/proc`, the `/bin` layout, a
//! case-insensitive file system, signals a system lacks). Goldens come from Linux, so
//! such a case is gated only there; on other platforms it runs and is reported apart,
//! never in a ratchet (D1331).
//!
//! Rules are tried in file order; the first that matches decides. A rule may be
//! `[[active]]` to carve an exception out of a later, broader rule. Unmatched cases
//! are active. Only this subset of TOML is accepted: arrays of tables, and string or
//! string-array values.

use crate::case::Case;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Group {
    Active,
    Later,
    Skip,
}

impl Group {
    pub fn name(self) -> &'static str {
        match self {
            Group::Active => "active",
            Group::Later => "later",
            Group::Skip => "skip",
        }
    }
}

impl Rule {
    fn matches(&self, case: &Case) -> bool {
        self.patterns.iter().any(|p| glob(p, &case.id))
            || self.tags.iter().any(|t| case.tags.contains(t))
    }
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub group: Group,
    pub patterns: Vec<String>,
    pub tags: Vec<String>,
    pub reason: String,
    /// Line of the rule's header, for messages.
    pub line: usize,
}

#[derive(Debug, Default)]
pub struct Groups {
    pub rules: Vec<Rule>,
    /// `[[serial]]` rules; their `group` field is unused.
    pub serial: Vec<Rule>,
    /// `[[flaky]]` rules; their `group` field is unused.
    pub flaky: Vec<Rule>,
    /// `[[platform]]` rules; their `group` field is unused.
    pub platform: Vec<Rule>,
}

impl Groups {
    pub fn parse(text: &str) -> Result<Groups, String> {
        let mut rules: Vec<Rule> = Vec::new();
        // Indexes into `rules` of the `[[serial]]`, `[[flaky]]` and `[[platform]]` tables
        // (kind 0, 1, 2), split off at the end.
        let mut split: Vec<(usize, u8)> = Vec::new();
        let mut lines = text.lines().enumerate().peekable();
        while let Some((n, raw)) = lines.next() {
            let err = |m: &str| format!("spec.toml line {}: {m}", n + 1);
            let line = strip_comment(raw).trim();
            if line.is_empty() {
                continue;
            }
            if let Some(name) = line.strip_prefix("[[").and_then(|l| l.strip_suffix("]]")) {
                match name.trim() {
                    "serial" => split.push((rules.len(), 0)),
                    "flaky" => split.push((rules.len(), 1)),
                    "platform" => split.push((rules.len(), 2)),
                    _ => {}
                }
                let group = match name.trim() {
                    "serial" | "flaky" | "platform" => Group::Active,
                    "skip" => Group::Skip,
                    "later" => Group::Later,
                    "active" => Group::Active,
                    other => return Err(err(&format!("unknown group `{other}`"))),
                };
                rules.push(Rule {
                    group,
                    patterns: Vec::new(),
                    tags: Vec::new(),
                    reason: String::new(),
                    line: n + 1,
                });
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| err("expected key = value"))?;
            let rule = rules
                .last_mut()
                .ok_or_else(|| err("key outside a [[group]]"))?;
            let mut value = value.trim().to_string();
            // Arrays may span lines.
            if value.starts_with('[') {
                while !strip_comment(&value).trim_end().ends_with(']') {
                    let (_, more) = lines.next().ok_or_else(|| err("unterminated array"))?;
                    value.push(' ');
                    value.push_str(strip_comment(more).trim());
                }
            }
            let values = parse_value(strip_comment(&value).trim()).map_err(|m| err(&m))?;
            match key.trim() {
                "match" => rule.patterns.extend(values),
                "tag" => rule.tags.extend(values),
                "reason" => rule.reason = values.join(" "),
                other => return Err(err(&format!("unknown key `{other}`"))),
            }
        }
        for r in &rules {
            if r.reason.trim().is_empty() {
                return Err(format!(
                    "spec.toml line {}: every rule needs a reason",
                    r.line
                ));
            }
            if r.patterns.is_empty() && r.tags.is_empty() {
                return Err(format!("spec.toml line {}: rule matches nothing", r.line));
            }
        }
        let (mut serial, mut flaky, mut platform) = (Vec::new(), Vec::new(), Vec::new());
        for (i, kind) in split.into_iter().rev() {
            let r = rules.remove(i);
            match kind {
                0 => serial.push(r),
                1 => flaky.push(r),
                _ => platform.push(r),
            }
        }
        serial.reverse();
        flaky.reverse();
        platform.reverse();
        Ok(Groups {
            rules,
            serial,
            flaky,
            platform,
        })
    }

    /// The rule that decides a case's group, if any.
    pub fn rule_for(&self, case: &Case) -> Option<&Rule> {
        self.rules.iter().find(|r| r.matches(case))
    }

    /// Whether a `[[serial]]` rule matches the case.
    pub fn is_serial(&self, case: &Case) -> bool {
        self.serial.iter().any(|r| r.matches(case))
    }

    /// The reason of the `[[platform]]` rule that matches the case, if any.
    pub fn platform_reason(&self, case: &Case) -> Option<&str> {
        self.platform
            .iter()
            .find(|r| r.matches(case))
            .map(|r| r.reason.as_str())
    }

    /// The reason of the `[[flaky]]` rule that matches the case, if any.
    pub fn flaky_reason(&self, case: &Case) -> Option<&str> {
        self.flaky
            .iter()
            .find(|r| r.matches(case))
            .map(|r| r.reason.as_str())
    }
}

/// A `#` outside a string starts a comment.
fn strip_comment(line: &str) -> &str {
    let mut in_str: Option<char> = None;
    let mut escaped = false;
    for (i, c) in line.char_indices() {
        match in_str {
            Some(q) => {
                if escaped {
                    escaped = false;
                } else if c == '\\' && q == '"' {
                    escaped = true;
                } else if c == q {
                    in_str = None;
                }
            }
            None if c == '"' || c == '\'' => in_str = Some(c),
            None if c == '#' => return &line[..i],
            None => {}
        }
    }
    line
}

fn parse_string(s: &str) -> Result<(String, &str), String> {
    if let Some(body) = s.strip_prefix('\'') {
        let end = body.find('\'').ok_or("unterminated string")?;
        return Ok((body[..end].to_string(), &body[end + 1..]));
    }
    let body = s.strip_prefix('"').ok_or("expected a string")?;
    let mut out = String::new();
    let mut chars = body.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return Ok((out, &body[i + 1..])),
            '\\' => match chars.next().map(|(_, e)| e) {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                _ => return Err("unsupported escape".into()),
            },
            _ => out.push(c),
        }
    }
    Err("unterminated string".into())
}

fn parse_value(s: &str) -> Result<Vec<String>, String> {
    if let Some(mut rest) = s.strip_prefix('[') {
        let mut out = Vec::new();
        loop {
            rest = rest.trim_start();
            if let Some(r) = rest.strip_prefix(']') {
                if !r.trim().is_empty() {
                    return Err("text after array".into());
                }
                return Ok(out);
            }
            let (v, r) = parse_string(rest)?;
            out.push(v);
            rest = r.trim_start();
            rest = rest.strip_prefix(',').unwrap_or(rest);
        }
    }
    let (v, rest) = parse_string(s)?;
    if !rest.trim().is_empty() {
        return Err("text after value".into());
    }
    Ok(vec![v])
}

/// `*` matches within one `/`-separated part, `**` across parts, `?` one character.
pub fn glob(pattern: &str, text: &str) -> bool {
    fn go(p: &[u8], t: &[u8]) -> bool {
        match p.first() {
            None => t.is_empty(),
            Some(b'*') if p.get(1) == Some(&b'*') => {
                let rest = &p[2..];
                (0..=t.len()).any(|i| go(rest, &t[i..]))
            }
            Some(b'*') => {
                let rest = &p[1..];
                let mut i = 0;
                loop {
                    if go(rest, &t[i..]) {
                        return true;
                    }
                    if i == t.len() || t[i] == b'/' {
                        return false;
                    }
                    i += 1;
                }
            }
            Some(b'?') => !t.is_empty() && t[0] != b'/' && go(&p[1..], &t[1..]),
            Some(&c) => t.first() == Some(&c) && go(&p[1..], &t[1..]),
        }
    }
    go(pattern.as_bytes(), text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs() {
        assert!(glob("oils/alias/*", "oils/alias/3"));
        assert!(!glob("oils/alias/*", "oils/alias-x/3"));
        assert!(!glob("brush/*", "brush/compat/arrays/1"));
        assert!(glob("brush/**", "brush/compat/arrays/1"));
        assert!(glob("oils/zsh-*/*", "oils/zsh-assoc/0"));
        assert!(glob("oils/alias/1?", "oils/alias/12"));
    }

    #[test]
    fn parse_rules() {
        let g = Groups::parse(
            r#"
# comment
[[active]]
match = "oils/glob/2"  # keep this one
reason = "exception"

[[skip]]
match = [
  "oils/glob/*",   # all of glob
  'oils/x/#1',
]
reason = "because \"reasons\""

[[later]]
tag = "pty"
reason = "needs a terminal"
"#,
        )
        .unwrap();
        assert_eq!(g.rules.len(), 3);
        assert_eq!(g.rules[1].patterns, vec!["oils/glob/*", "oils/x/#1"]);
        assert_eq!(g.rules[1].reason, "because \"reasons\"");
        assert_eq!(g.rules[2].tags, vec!["pty"]);
        assert!(Groups::parse("[[skip]]\nmatch = \"x\"\n").is_err());
        let g = Groups::parse("[[serial]]\nmatch = \"a/*\"\nreason = \"r\"\n[[skip]]\nmatch = \"b\"\nreason = \"r\"\n").unwrap();
        assert_eq!(g.serial.len(), 1);
        let g = Groups::parse(
            "[[flaky]]\nmatch = \"a/1\"\nreason = \"race\"\n[[serial]]\nmatch = \"a/2\"\nreason = \"r\"\n[[skip]]\nmatch = \"b\"\nreason = \"r\"\n",
        )
        .unwrap();
        assert_eq!((g.rules.len(), g.serial.len(), g.flaky.len()), (1, 1, 1));
        assert_eq!(g.flaky[0].reason, "race");
        assert!(Groups::parse("[[flaky]]\nmatch = \"a/1\"\n").is_err());
        let p = Groups::parse(
            "[[platform]]\nmatch = \"a/1\"\nreason = \"/dev/full\"\n[[skip]]\nmatch = \"a/1\"\nreason = \"r\"\n",
        )
        .unwrap();
        assert_eq!((p.rules.len(), p.platform.len()), (1, 1));
        assert_eq!(p.platform[0].reason, "/dev/full");
        assert_eq!(g.rules.len(), 1);
        assert_eq!(g.rules[0].group, Group::Skip);
        assert!(Groups::parse("[[nope]]\n").is_err());
    }
}
