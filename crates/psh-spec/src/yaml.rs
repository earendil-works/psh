//! The subset of YAML that brush's test case files use: block mappings and sequences,
//! literal block scalars (`|`, `|-`, `|+`), plain, single- and double-quoted scalars,
//! single-line flow sequences and comments. Scalars stay strings; callers interpret
//! `true`, numbers and the like. Anything outside the subset is an error, not a guess.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Yaml {
    Null,
    Str(String),
    Seq(Vec<Yaml>),
    Map(Vec<(String, Yaml)>),
}

impl Yaml {
    pub fn get(&self, key: &str) -> Option<&Yaml> {
        match self {
            Yaml::Map(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Yaml::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self.as_str()? {
            "true" | "True" | "yes" => Some(true),
            "false" | "False" | "no" => Some(false),
            _ => None,
        }
    }

    pub fn as_seq(&self) -> &[Yaml] {
        match self {
            Yaml::Seq(items) => items,
            _ => &[],
        }
    }
}

#[derive(Debug)]
pub struct YamlError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for YamlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

struct Line<'a> {
    /// 1-based line number.
    num: usize,
    indent: usize,
    /// The line without its indentation (and without the newline).
    text: &'a str,
    /// The whole line, for block scalars.
    raw: &'a str,
}

pub fn parse(source: &str) -> Result<Yaml, YamlError> {
    let lines: Vec<Line> = source
        .split('\n')
        .enumerate()
        .map(|(i, raw)| {
            let raw = raw.strip_suffix('\r').unwrap_or(raw);
            let text = raw.trim_start_matches(' ');
            Line {
                num: i + 1,
                indent: raw.len() - text.len(),
                text,
                raw,
            }
        })
        .collect();
    let mut p = Parser { lines, pos: 0 };
    p.skip_blank();
    if p.pos >= p.lines.len() {
        return Ok(Yaml::Null);
    }
    let indent = p.lines[p.pos].indent;
    let value = p.block(indent)?;
    p.skip_blank();
    if p.pos < p.lines.len() {
        return Err(p.error("unexpected content after the document"));
    }
    Ok(value)
}

struct Parser<'a> {
    lines: Vec<Line<'a>>,
    pos: usize,
}

fn is_blank(text: &str) -> bool {
    text.is_empty() || text.starts_with('#')
}

impl Parser<'_> {
    fn error(&self, message: &str) -> YamlError {
        let line = self.lines.get(self.pos).map_or(self.lines.len(), |l| l.num);
        YamlError {
            line,
            message: message.to_string(),
        }
    }

    fn skip_blank(&mut self) {
        while self.pos < self.lines.len() && is_blank(self.lines[self.pos].text) {
            self.pos += 1;
        }
    }

    /// A block node whose first line is at `self.pos` with exactly `indent`.
    fn block(&mut self, indent: usize) -> Result<Yaml, YamlError> {
        let text = self.lines[self.pos].text;
        if text.starts_with('[') {
            return self.inline_value(text, indent);
        }
        if text == "-" || text.starts_with("- ") {
            self.sequence(indent)
        } else {
            self.mapping(indent, None)
        }
    }

    fn sequence(&mut self, indent: usize) -> Result<Yaml, YamlError> {
        let mut items = Vec::new();
        loop {
            self.skip_blank();
            let Some(line) = self.lines.get(self.pos) else {
                break;
            };
            if line.indent != indent || !(line.text == "-" || line.text.starts_with("- ")) {
                if line.indent > indent {
                    return Err(self.error("bad indentation in a sequence"));
                }
                break;
            }
            let rest = line.text[1..].trim_start_matches(' ');
            let item_indent = indent + (line.text.len() - rest.len());
            if is_blank(rest) {
                // The item is a block on the following lines.
                self.pos += 1;
                self.skip_blank();
                match self.lines.get(self.pos) {
                    Some(next) if next.indent > indent => items.push(self.block(next.indent)?),
                    _ => items.push(Yaml::Null),
                }
            } else if rest.starts_with("- ") {
                return Err(self.error("nested inline sequences are not supported"));
            } else if find_key(rest).is_some() {
                // `- key: value` starts a mapping whose keys align with `key`.
                items.push(self.mapping(item_indent, Some(rest))?);
            } else {
                let value = self.inline_value(rest, indent)?;
                items.push(value);
            }
        }
        Ok(Yaml::Seq(items))
    }

    /// A mapping at `indent`. `first` is the text of a first entry that shares its line
    /// with a sequence dash.
    fn mapping(&mut self, indent: usize, first: Option<&str>) -> Result<Yaml, YamlError> {
        let mut entries: Vec<(String, Yaml)> = Vec::new();
        let mut first = first;
        loop {
            let text = match first.take() {
                Some(text) => text,
                None => {
                    self.skip_blank();
                    let Some(line) = self.lines.get(self.pos) else {
                        break;
                    };
                    if line.indent < indent {
                        break;
                    }
                    if line.indent > indent {
                        return Err(self.error("bad indentation in a mapping"));
                    }
                    if line.text.starts_with("- ") || line.text == "-" {
                        break;
                    }
                    line.text
                }
            };
            let Some((key, rest)) = find_key(text) else {
                return Err(self.error("expected `key: value`"));
            };
            let key = unquote_key(key);
            if entries.iter().any(|(k, _)| *k == key) {
                return Err(self.error(&format!("duplicate key `{key}`")));
            }
            let rest = rest.trim_start_matches(' ');
            let value = if is_blank(rest) {
                self.pos += 1;
                self.skip_blank();
                match self.lines.get(self.pos) {
                    Some(next) if next.indent > indent => self.block(next.indent)?,
                    // A sequence may sit at the same indentation as its key.
                    Some(next)
                        if next.indent == indent
                            && (next.text.starts_with("- ") || next.text == "-") =>
                    {
                        self.sequence(indent)?
                    }
                    _ => Yaml::Null,
                }
            } else {
                self.inline_value(rest, indent)?
            };
            entries.push((key, value));
        }
        Ok(Yaml::Map(entries))
    }

    /// A value that starts on the current line (after `key:` or `- `). Consumes the
    /// line, and for block scalars the lines that belong to it. `parent` is the
    /// indentation of the line's node.
    fn inline_value(&mut self, text: &str, parent: usize) -> Result<Yaml, YamlError> {
        if let Some(header) = text.strip_prefix('|') {
            return self.block_scalar(header, parent);
        }
        if text.starts_with('>') {
            return Err(self.error("folded block scalars are not supported"));
        }
        let value = if text.starts_with('[') {
            // A flow sequence may continue on the following lines.
            let mut joined = text.to_string();
            let mut extra = 0;
            let items = loop {
                match flow_sequence(&joined) {
                    Ok((items, rest)) => {
                        check_trailing(rest).map_err(|m| self.error(&m))?;
                        break items;
                    }
                    Err(m) if m.starts_with("unterminated") => {
                        extra += 1;
                        let Some(next) = self.lines.get(self.pos + extra) else {
                            return Err(self.error(&m));
                        };
                        joined.push(' ');
                        joined.push_str(next.text);
                    }
                    Err(m) => return Err(self.error(&m)),
                }
            };
            self.pos += extra;
            Yaml::Seq(items)
        } else if text.starts_with('{') {
            return Err(self.error("flow mappings are not supported"));
        } else {
            let (value, rest) = scalar(text, false).map_err(|m| self.error(&m))?;
            check_trailing(rest).map_err(|m| self.error(&m))?;
            value
        };
        self.pos += 1;
        Ok(value)
    }

    fn block_scalar(&mut self, header: &str, parent: usize) -> Result<Yaml, YamlError> {
        let header = strip_comment(header).trim();
        let mut chomp = ' ';
        let mut explicit_indent = None;
        for c in header.chars() {
            match c {
                '-' | '+' => chomp = c,
                '1'..='9' => explicit_indent = Some(parent + c as usize - '0' as usize),
                _ => return Err(self.error("bad block scalar header")),
            }
        }
        self.pos += 1;
        let start = self.pos;
        // The content's indentation: explicit, or that of the first non-empty line.
        let indent = explicit_indent.unwrap_or_else(|| {
            self.lines[start..]
                .iter()
                .find(|l| !l.raw.trim().is_empty())
                .map_or(parent + 1, |l| l.indent)
        });
        let mut out = String::new();
        while let Some(line) = self.lines.get(self.pos) {
            if line.raw.trim().is_empty() {
                // Blank lines belong to the scalar if more content follows.
                out.push('\n');
                self.pos += 1;
                continue;
            }
            if line.indent < indent || indent <= parent {
                break;
            }
            out.push_str(&line.raw[indent..]);
            out.push('\n');
            self.pos += 1;
        }
        // Trailing blank lines were consumed; only `|+` keeps their newlines.
        let content_end = out.trim_end_matches('\n').len();
        let value = match chomp {
            '-' => out[..content_end].to_string(),
            '+' => out,
            _ if content_end == 0 => String::new(),
            _ => format!("{}\n", &out[..content_end]),
        };
        Ok(Yaml::Str(value))
    }
}

/// Splits `key: rest` (or `key:` at the end of the line). Keys are plain words or
/// quoted strings.
fn find_key(text: &str) -> Option<(&str, &str)> {
    let end = if text.starts_with('"') || text.starts_with('\'') {
        let quote = text.as_bytes()[0];
        let close = text[1..].bytes().position(|b| b == quote)? + 1;
        close + 1
    } else {
        text.find(':')?
    };
    let after = &text[end..];
    let rest = after.strip_prefix(':')?;
    if !(rest.is_empty() || rest.starts_with(' ')) {
        return None;
    }
    let key = &text[..end];
    if key.is_empty() || key.starts_with('#') || key.starts_with('[') || key.contains(" #") {
        return None;
    }
    Some((key, rest))
}

fn unquote_key(key: &str) -> String {
    match scalar(key, false) {
        Ok((Yaml::Str(s), "")) => s,
        _ => key.to_string(),
    }
}

fn strip_comment(text: &str) -> &str {
    if text.starts_with('#') {
        return "";
    }
    match text.find(" #") {
        Some(i) => &text[..i],
        None => text,
    }
}

fn check_trailing(rest: &str) -> Result<(), String> {
    let rest = rest.trim_start_matches(' ');
    if rest.is_empty() || rest.starts_with('#') {
        Ok(())
    } else {
        Err(format!("unexpected `{rest}` after a value"))
    }
}

/// One scalar at the start of `text`; returns it and the rest of the line. In flow
/// context, plain scalars end at `,` and `]`.
fn scalar(text: &str, flow: bool) -> Result<(Yaml, &str), String> {
    if let Some(body) = text.strip_prefix('\'') {
        let mut out = String::new();
        let mut chars = body.char_indices();
        while let Some((i, c)) = chars.next() {
            if c == '\'' {
                if body[i + 1..].starts_with('\'') {
                    out.push('\'');
                    chars.next();
                } else {
                    return Ok((Yaml::Str(out), &body[i + 1..]));
                }
            } else {
                out.push(c);
            }
        }
        return Err("unterminated single-quoted string".into());
    }
    if let Some(body) = text.strip_prefix('"') {
        let mut out = String::new();
        let mut chars = body.char_indices();
        while let Some((i, c)) = chars.next() {
            match c {
                '"' => return Ok((Yaml::Str(out), &body[i + 1..])),
                '\\' => {
                    let (_, e) = chars.next().ok_or("unterminated escape")?;
                    match e {
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        'r' => out.push('\r'),
                        '0' => out.push('\0'),
                        'a' => out.push('\x07'),
                        'b' => out.push('\x08'),
                        'e' => out.push('\x1b'),
                        'f' => out.push('\x0c'),
                        'v' => out.push('\x0b'),
                        ' ' => out.push(' '),
                        '/' => out.push('/'),
                        '\\' => out.push('\\'),
                        '"' => out.push('"'),
                        'x' | 'u' | 'U' => {
                            let n = match e {
                                'x' => 2,
                                'u' => 4,
                                _ => 8,
                            };
                            let mut code = 0u32;
                            for _ in 0..n {
                                let (_, h) = chars.next().ok_or("short escape")?;
                                code = code * 16 + h.to_digit(16).ok_or("bad hex escape")?;
                            }
                            out.push(char::from_u32(code).ok_or("bad code point")?);
                        }
                        other => return Err(format!("unsupported escape \\{other}")),
                    }
                }
                _ => out.push(c),
            }
        }
        return Err("unterminated double-quoted string".into());
    }
    // A plain scalar: up to a comment (or, in flow context, `,` / `]`).
    let mut end = text.len();
    let bytes = text.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'#' && i > 0 && bytes[i - 1] == b' ' {
            end = i;
            break;
        }
        if flow && (b == b',' || b == b']') {
            end = i;
            break;
        }
    }
    let value = text[..end].trim_end();
    let value = if value == "~" || value == "null" {
        Yaml::Null
    } else {
        Yaml::Str(value.to_string())
    };
    Ok((value, &text[end..]))
}

fn flow_sequence(text: &str) -> Result<(Vec<Yaml>, &str), String> {
    let mut rest = text[1..].trim_start();
    let mut items = Vec::new();
    if let Some(r) = rest.strip_prefix(']') {
        return Ok((items, r));
    }
    loop {
        if let Some(r) = rest.strip_prefix(']') {
            // `[a, b,]`: a trailing comma.
            return Ok((items, r));
        }
        if rest.is_empty() {
            return Err("unterminated flow sequence".into());
        }
        if rest.starts_with('[') {
            let (inner, r) = flow_sequence(rest)?;
            items.push(Yaml::Seq(inner));
            rest = r;
        } else {
            let (value, r) = scalar(rest, true)?;
            items.push(value);
            rest = r;
        }
        rest = rest.trim_start();
        if let Some(r) = rest.strip_prefix(',') {
            rest = r.trim_start();
        } else if let Some(r) = rest.strip_prefix(']') {
            return Ok((items, r));
        } else {
            return Err("unterminated flow sequence".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Yaml {
        Yaml::Str(v.into())
    }

    #[test]
    fn mappings_sequences_and_scalars() {
        let doc = "\
name: \"Arrays\"
incompatible_configs: [\"sh\", 'x', 1]
# a comment
cases:
  - name: one # trailing comment
    ignore_stderr: true
    stdin: |
      a=1

      echo \"$a\"

  - name: 'two''s'
    args:
      - -c
      - echo hi
    env:
      HOME: /x
cases2:
- plain
";
        let y = parse(doc).unwrap();
        assert_eq!(y.get("name"), Some(&s("Arrays")));
        assert_eq!(
            y.get("incompatible_configs").unwrap().as_seq(),
            &[s("sh"), s("x"), s("1")]
        );
        let cases = y.get("cases").unwrap().as_seq();
        assert_eq!(cases.len(), 2);
        assert_eq!(cases[0].get("name"), Some(&s("one")));
        assert_eq!(cases[0].get("ignore_stderr").unwrap().as_bool(), Some(true));
        assert_eq!(cases[0].get("stdin"), Some(&s("a=1\n\necho \"$a\"\n")));
        assert_eq!(cases[1].get("name"), Some(&s("two's")));
        assert_eq!(
            cases[1].get("args").unwrap().as_seq(),
            &[s("-c"), s("echo hi")]
        );
        assert_eq!(cases[1].get("env").unwrap().get("HOME"), Some(&s("/x")));
        assert_eq!(y.get("cases2").unwrap().as_seq(), &[s("plain")]);
    }

    #[test]
    fn block_scalar_chomping() {
        let y = parse("a: |-\n  x\n  y\n\nb: |+\n  x\n\nc: |\n  x\n").unwrap();
        assert_eq!(y.get("a"), Some(&s("x\ny")));
        assert_eq!(y.get("b"), Some(&s("x\n\n")));
        assert_eq!(y.get("c"), Some(&s("x\n")));
    }

    #[test]
    fn block_scalar_keeps_deeper_indentation_and_hashes() {
        let y = parse("a: |\n  if x; then\n    # not a comment\n  fi\nb: 1\n").unwrap();
        assert_eq!(y.get("a"), Some(&s("if x; then\n  # not a comment\nfi\n")));
        assert_eq!(y.get("b"), Some(&s("1")));
    }

    #[test]
    fn double_quoted_escapes() {
        let y = parse(r#"a: "x\ty\n\\\"\x41""#).unwrap();
        assert_eq!(y.get("a"), Some(&s("x\ty\n\\\"A")));
    }

    #[test]
    fn plain_scalars_with_quotes_inside() {
        let y = parse("stdin: builtin echo \"Hello world\"\nn: '\\#'\n").unwrap();
        assert_eq!(y.get("stdin"), Some(&s("builtin echo \"Hello world\"")));
        assert_eq!(y.get("n"), Some(&s("\\#")));
    }

    #[test]
    fn multi_line_flow_sequences() {
        let y = parse("a:\n  [\n    \"-c\",\n    \"x, y\",\n  ]\nb: [1,\n  2]\n").unwrap();
        assert_eq!(y.get("a").unwrap().as_seq(), &[s("-c"), s("x, y")]);
        assert_eq!(y.get("b").unwrap().as_seq(), &[s("1"), s("2")]);
    }

    #[test]
    fn errors_have_lines() {
        let err = parse("a: 1\n  b: 2\n").unwrap_err();
        assert_eq!(err.line, 2);
        assert!(parse("a: >\n  x\n").is_err());
        assert!(parse("a: 1\na: 2\n").is_err());
    }
}
