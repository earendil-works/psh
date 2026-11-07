//! Reader for Oils' `.test.sh` spec format (also used for our own L3 agent cases).
//!
//! The rules follow Oils' `test/sh_spec.py` (Apache-2.0, Copyright the Oils authors)
//! exactly, so that the script of each case is byte for byte what Oils runs: notably,
//! lines whose first non-blank character is `#` are dropped everywhere (even inside
//! code), blank lines between cases belong to the preceding case's code, and a case may
//! give its code as `## code: ...`. Oils' expectations (`## stdout:`, `## BUG bash ...`)
//! are parsed only to find where code ends; they are not our oracle.

#[derive(Debug, Clone, PartialEq)]
pub struct OilsCase {
    pub desc: String,
    /// 1-based line of the `####` header.
    pub line: usize,
    pub code: String,
    /// The case's own `## key: value` lines (qualified ones included, without their
    /// qualifier); our L4 cases use `## expect:`.
    pub meta: Vec<(String, String)>,
}

#[derive(Debug, Default)]
pub struct OilsFile {
    /// File-level `## key: value` metadata, in order.
    pub meta: Vec<(String, String)>,
    pub cases: Vec<OilsCase>,
}

#[derive(Debug, Clone, PartialEq)]
enum Tok<'a> {
    Begin(&'a str),
    KeyValue { name: &'a str, value: &'a str },
    Multiline,
    End,
    Plain(&'a str),
    Eof,
}

struct Tokens<'a> {
    lines: Vec<&'a str>,
    /// Index of the next line to read.
    next_line: usize,
    cur: Tok<'a>,
    cur_line: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    /// Blank lines are skipped (between cases).
    Outer,
    /// Blank lines are significant (inside code and multi-line values).
    Raw,
}

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Matches `## [QUALIFIER SHELLS ]NAME: VALUE`; returns (name, value).
fn key_value(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("##")?;
    let after = rest.trim_start();
    if after.len() == rest.len() {
        return None; // `##` must be followed by whitespace
    }
    let try_key = |s: &str| -> Option<(usize, usize)> {
        // [\w-]+ followed by ':'
        let n = s.bytes().take_while(|&b| is_word(b) || b == b'-').count();
        if n > 0 && s.as_bytes().get(n) == Some(&b':') {
            Some((n, n + 1))
        } else {
            None
        }
    };
    // With a qualifier: (OK(-N)?|BUG(-N)?|N-I) SHELLS NAME:
    let qual_len = ["OK", "BUG", "N-I"].iter().find_map(|q| {
        let r = after.strip_prefix(q)?;
        let mut n = q.len();
        if *q != "N-I"
            && r.starts_with('-')
            && r.as_bytes().get(1).is_some_and(|b| b.is_ascii_digit())
        {
            n += 2;
        }
        Some(n)
    });
    if let Some(qn) = qual_len {
        let r = &after[qn..];
        let r2 = r.trim_start();
        if r2.len() < r.len() {
            let sn = r2
                .bytes()
                .take_while(|&b| is_word(b) || b == b'+' || b == b'/')
                .count();
            let r3 = &r2[sn..];
            let r4 = r3.trim_start();
            if sn > 0
                && r4.len() < r3.len()
                && let Some((kn, vn)) = try_key(r4)
            {
                return Some((&r4[..kn], r4[vn..].trim_start()));
            }
        }
    }
    let (kn, vn) = try_key(after)?;
    Some((&after[..kn], after[vn..].trim_start()))
}

fn classify(line: &str, mode: Mode) -> Result<Option<Tok<'_>>, String> {
    if mode == Mode::Outer && line.trim().is_empty() {
        return Ok(None);
    }
    if let Some(desc) = line.strip_prefix("####") {
        return Ok(Some(Tok::Begin(desc.trim())));
    }
    let body = line.trim_end_matches('\n');
    if let Some((name, value)) = key_value(body) {
        return Ok(Some(if name == "STDOUT" || name == "STDERR" {
            if !value.is_empty() {
                return Err(format!("value for {name} must be on the following lines"));
            }
            Tok::Multiline
        } else {
            Tok::KeyValue { name, value }
        }));
    }
    if let Some(rest) = body.strip_prefix("##") {
        let r = rest.trim_start();
        if r.len() < rest.len() && r.starts_with("END") {
            return Ok(Some(Tok::End));
        }
        return Err(format!("invalid ## line {body:?}"));
    }
    if line.trim_start().starts_with('#') {
        return Ok(None); // comments are dropped, even inside code
    }
    Ok(Some(Tok::Plain(line)))
}

impl<'a> Tokens<'a> {
    fn new(source: &'a str) -> Result<Self, String> {
        let lines = source.split_inclusive('\n').collect();
        let mut t = Tokens {
            lines,
            next_line: 0,
            cur: Tok::Eof,
            cur_line: 0,
        };
        t.advance(Mode::Outer)?;
        Ok(t)
    }

    fn advance(&mut self, mode: Mode) -> Result<(), String> {
        loop {
            let Some(line) = self.lines.get(self.next_line) else {
                self.cur = Tok::Eof;
                self.cur_line = self.next_line + 1;
                return Ok(());
            };
            self.next_line += 1;
            if let Some(tok) =
                classify(line, mode).map_err(|e| format!("line {}: {e}", self.next_line))?
            {
                self.cur = tok;
                self.cur_line = self.next_line;
                return Ok(());
            }
        }
    }
}

/// Consumes contiguous metadata; returns a `## code:` value if one was given.
fn key_values<'a>(
    t: &mut Tokens<'a>,
    meta: &mut Vec<(String, String)>,
) -> Result<Option<&'a str>, String> {
    let mut code = None;
    loop {
        match t.cur {
            Tok::Multiline => loop {
                t.advance(Mode::Raw)?;
                match t.cur {
                    Tok::Plain(_) => {}
                    Tok::End => {
                        t.advance(Mode::Outer)?;
                        break;
                    }
                    _ => break,
                }
            },
            Tok::KeyValue { name, value } => {
                if name == "code" {
                    code = Some(value);
                } else {
                    meta.push((name.to_string(), value.to_string()));
                }
                t.advance(Mode::Outer)?;
            }
            _ => return Ok(code),
        }
    }
}

pub fn parse(source: &str) -> Result<OilsFile, String> {
    let mut t = Tokens::new(source)?;
    let mut file = OilsFile::default();
    while let Tok::KeyValue { name, value } = t.cur {
        file.meta.push((name.to_string(), value.to_string()));
        t.advance(Mode::Outer)?;
    }
    loop {
        let desc = match t.cur {
            Tok::Eof => return Ok(file),
            Tok::Begin(desc) => desc,
            ref other => {
                return Err(format!(
                    "line {}: expected a #### case header, got {other:?}",
                    t.cur_line
                ));
            }
        };
        let line = t.cur_line;
        t.advance(Mode::Outer)?;
        let mut meta = Vec::new();
        let code = if let Some(code) = key_values(&mut t, &mut meta)? {
            code.to_string()
        } else {
            let mut code = String::new();
            let Tok::Plain(_) = t.cur else {
                return Err(format!(
                    "line {}: expected code for case {desc:?}",
                    t.cur_line
                ));
            };
            while let Tok::Plain(l) = t.cur {
                code.push_str(l);
                t.advance(Mode::Raw)?;
            }
            key_values(&mut t, &mut meta)?;
            code
        };
        file.cases.push(OilsCase {
            desc: desc.to_string(),
            line,
            code,
            meta,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cases_and_metadata() {
        let src = "\
## compare_shells: bash dash
## oils_failures_allowed: 1

#### hello and fail
echo hello
# a comment, dropped
echo world

exit 1
## status: 1
## STDOUT:
hello

world
## END
## OK dash/mksh status: 2

#### code in metadata
## code: foo()
## status: 2

#### last
echo last
## N-I bash stdout-json: \"\"
";
        let f = parse(src).unwrap();
        assert_eq!(
            f.meta,
            vec![
                ("compare_shells".into(), "bash dash".into()),
                ("oils_failures_allowed".into(), "1".into())
            ]
        );
        assert_eq!(f.cases.len(), 3);
        assert_eq!(f.cases[0].desc, "hello and fail");
        assert_eq!(f.cases[0].line, 4);
        assert_eq!(f.cases[0].code, "echo hello\necho world\n\nexit 1\n");
        assert_eq!(f.cases[1].code, "foo()");
        assert_eq!(f.cases[2].code, "echo last\n");
    }

    #[test]
    fn trailing_blank_lines_belong_to_the_code() {
        let f = parse("#### a\necho a\n\n\n#### b\necho b").unwrap();
        assert_eq!(f.cases[0].code, "echo a\n\n\n");
        assert_eq!(f.cases[1].code, "echo b");
    }

    #[test]
    fn multiline_value_without_end() {
        let f = parse("#### a\necho a\n## STDOUT:\na\n#### b\necho b\n").unwrap();
        assert_eq!(f.cases.len(), 2);
        assert_eq!(f.cases[1].code, "echo b\n");
    }

    #[test]
    fn invalid_double_hash_line() {
        assert!(parse("#### a\necho a\n##bogus\n").is_err());
    }

    #[test]
    fn qualifier_forms() {
        assert_eq!(
            key_value("## OK-2 bash/zsh stdout: x"),
            Some(("stdout", "x"))
        );
        assert_eq!(key_value("## N-I dash status: 2"), Some(("status", "2")));
        assert_eq!(
            key_value("## stdout-json: \"\""),
            Some(("stdout-json", "\"\""))
        );
        assert_eq!(key_value("## END"), None);
    }
}
