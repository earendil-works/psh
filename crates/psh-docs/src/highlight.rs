//! A small syntax highlighter for the guide's code blocks: shell (`sh`, `bash`), Rust,
//! JavaScript and JSON. It marks the roles of Elanor's code theme (comment, punctuation,
//! keyword, function, variable, string, number, type) and style.css colors them with the
//! theme's tokens. assets/highlight.js is the shell half again, for the live examples'
//! editor; a test runs it in node and compares.
//!
//! Shell is read the way a reader sees it, not the way psh parses it: the command name
//! of each simple command is a function, `NAME=` and `$x`/`${…}` are variables, quotes
//! and here-document bodies are strings, operators are punctuation. A block whose lines
//! start with `$ ` is a session: those lines are commands, the others their output.

/// The highlighted HTML of `code`.
pub fn highlight(lang: &str, code: &str) -> String {
    match lang {
        "sh" | "bash" | "shell" => html(&sh_tokens(code)),
        "rust" | "rs" | "js" | "javascript" | "ts" | "typescript" | "json" => c_like(lang, code),
        _ => esc(code),
    }
}

pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn html(tokens: &[(Option<&'static str>, String)]) -> String {
    let mut out = String::new();
    for (role, text) in tokens {
        match role {
            Some(r) => {
                out.push_str("<span class=\"tok-");
                out.push_str(r);
                out.push_str("\">");
                out.push_str(&esc(text));
                out.push_str("</span>");
            }
            None => out.push_str(&esc(text)),
        }
    }
    out
}

// ---- Shell ----------------------------------------------------------------------------

const SH_KW: &[&str] = &[
    "if", "then", "else", "elif", "fi", "for", "while", "until", "do", "done", "case", "esac",
    "in", "function", "select", "time", "!", "{", "}", "[[", "]]", "coproc",
];
/// Keywords after which the next word is a command name again.
const SH_KW_CMD: &[&str] = &[
    "if", "then", "else", "elif", "while", "until", "do", "!", "{", "time", "coproc", "function",
];
/// Longest first.
const SH_OPS: &[&str] = &[
    ";;&", "<<<", "<<-", "&>>", ";;", ";&", "||", "&&", "|&", ">>", "<<", "&>", ">&", "<&", "<>",
    ">|", "<(", ">(", "|", "&", ";", "<", ">", "(", ")",
];

/// Characters that end a plain word.
fn sh_special(c: char) -> bool {
    c.is_ascii_whitespace() || "|&;<>()'\"$`\\".contains(c)
}

/// The tokens of a shell block; joined, their texts are `code` again.
pub fn sh_tokens(code: &str) -> Vec<(Option<&'static str>, String)> {
    let session = code.lines().any(|l| l.starts_with("$ "));
    if !session {
        return Sh::new(code).run();
    }
    let mut out = Vec::new();
    for line in code.split_inclusive('\n') {
        if let Some(cmd) = line.strip_prefix("$ ") {
            out.push((Some("punctuation"), "$".to_string()));
            out.push((None, " ".to_string()));
            out.extend(Sh::new(cmd).run());
        } else {
            out.push((None, line.to_string()));
        }
    }
    out
}

struct Sh {
    c: Vec<char>,
    i: usize,
    out: Vec<(Option<&'static str>, String)>,
    /// The next word is a command name.
    cmd: bool,
    /// The next word is a loop variable (`for NAME`).
    loop_var: bool,
    /// An `in` is the keyword (`for NAME in`, `case WORD in`).
    expect_in: bool,
    /// Here-documents whose bodies start at the next newline: (delimiter, strip tabs).
    heredocs: Vec<(String, bool)>,
    /// The next word is a here-document delimiter (`<<-` strips tabs).
    delim_next: Option<bool>,
}

impl Sh {
    fn new(code: &str) -> Sh {
        Sh {
            c: code.chars().collect(),
            i: 0,
            out: Vec::new(),
            cmd: true,
            loop_var: false,
            expect_in: false,
            heredocs: Vec::new(),
            delim_next: None,
        }
    }

    fn text(&self, a: usize, b: usize) -> String {
        self.c[a..b].iter().collect()
    }

    fn push(&mut self, role: Option<&'static str>, a: usize, b: usize) {
        let t = self.text(a, b);
        self.out.push((role, t));
    }

    /// The token starting at `i` starts a word (not glued to the one before).
    fn word_start(&self) -> bool {
        self.i == 0 || {
            let p = self.c[self.i - 1];
            p.is_ascii_whitespace() || "|&;<>()".contains(p)
        }
    }

    fn run(mut self) -> Vec<(Option<&'static str>, String)> {
        while self.i < self.c.len() {
            let start = self.i;
            let ch = self.c[start];
            let at_word = self.word_start();
            if ch == '\n' {
                self.i += 1;
                self.push(None, start, self.i);
                self.cmd = true;
                self.expect_in = false;
                self.bodies();
            } else if ch == ' ' || ch == '\t' || ch == '\r' {
                while self.i < self.c.len() && matches!(self.c[self.i], ' ' | '\t' | '\r') {
                    self.i += 1;
                }
                self.push(None, start, self.i);
            } else if ch == '#' && at_word {
                while self.i < self.c.len() && self.c[self.i] != '\n' {
                    self.i += 1;
                }
                self.push(Some("comment"), start, self.i);
            } else if ch == '\\' {
                self.i = (self.i + 2).min(self.c.len());
                self.push(None, start, self.i);
                if at_word {
                    self.cmd = false;
                }
            } else if ch == '\'' || (ch == '$' && self.c.get(start + 1) == Some(&'\'')) {
                self.single(ch == '$');
                self.after_word(at_word);
            } else if ch == '"' {
                self.double();
                self.after_word(at_word);
            } else if ch == '$' {
                self.dollar(at_word);
            } else if ch == '`' {
                self.i += 1;
                self.push(Some("punctuation"), start, self.i);
                self.cmd = true;
            } else if let Some(op) = self.op() {
                self.i += op.chars().count();
                self.push(Some("punctuation"), start, self.i);
                match op {
                    "<<" => self.delim_next = Some(false),
                    "<<-" => self.delim_next = Some(true),
                    "|" | "||" | "&&" | "&" | ";" | ";;" | ";&" | ";;&" | "|&" | "(" | ")"
                    | "<(" | ">(" => {
                        self.cmd = true;
                        self.expect_in = false;
                    }
                    _ => {}
                }
            } else {
                self.word(at_word);
            }
        }
        self.out
    }

    fn op(&self) -> Option<&'static str> {
        SH_OPS.iter().copied().find(|op| {
            op.chars()
                .enumerate()
                .all(|(k, o)| self.c.get(self.i + k) == Some(&o))
        })
    }

    /// A quote or expansion started a word in command position: that word is the name.
    fn after_word(&mut self, at_word: bool) {
        if at_word {
            if let Some(strip) = self.delim_next.take() {
                let d = self.out.last().map(|t| t.1.clone()).unwrap_or_default();
                let d = d.trim_matches(|c| c == '\'' || c == '"').to_string();
                self.heredocs.push((d, strip));
            }
            self.cmd = false;
            self.loop_var = false;
        }
    }

    /// `'…'`, or `$'…'` (with backslash escapes).
    fn single(&mut self, ansi: bool) {
        let start = self.i;
        self.i += if ansi { 2 } else { 1 };
        while self.i < self.c.len() && self.c[self.i] != '\'' {
            if ansi && self.c[self.i] == '\\' {
                self.i += 1;
            }
            self.i += 1;
        }
        self.i = (self.i + 1).min(self.c.len());
        self.push(Some("string"), start, self.i);
    }

    /// `"…"`: a string, with its `$name` and `${…}` expansions as variables.
    fn double(&mut self) {
        let mut seg = self.i;
        self.i += 1;
        while self.i < self.c.len() && self.c[self.i] != '"' {
            match self.c[self.i] {
                '\\' => self.i += 2,
                '$' => {
                    let (a, b) = (self.i, self.var_end(self.i));
                    if b > a + 1 {
                        self.push(Some("string"), seg, a);
                        self.push(Some("variable"), a, b);
                        seg = b;
                        self.i = b;
                    } else {
                        self.i += 1;
                    }
                }
                _ => self.i += 1,
            }
        }
        self.i = (self.i + 1).min(self.c.len());
        if self.i > seg {
            self.push(Some("string"), seg, self.i);
        }
    }

    /// Where the variable expansion at `a` (a `$`) ends: `$name`, `$1`, `$?`, `${…}`.
    /// `a + 1` if it is none of those.
    fn var_end(&self, a: usize) -> usize {
        let c = &self.c;
        match c.get(a + 1) {
            Some('{') => {
                let mut depth = 0;
                let mut j = a + 1;
                while j < c.len() {
                    match c[j] {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                return j + 1;
                            }
                        }
                        '\n' => return j,
                        _ => {}
                    }
                    j += 1;
                }
                j
            }
            Some(&x) if x.is_ascii_alphabetic() || x == '_' => {
                let mut j = a + 1;
                while j < c.len() && (c[j].is_ascii_alphanumeric() || c[j] == '_') {
                    j += 1;
                }
                j
            }
            Some(&x) if x.is_ascii_digit() || "?@*#$!-".contains(x) => a + 2,
            _ => a + 1,
        }
    }

    /// `$…` outside quotes: `$(` and `$((` open a command or arithmetic, the rest are
    /// variables.
    fn dollar(&mut self, at_word: bool) {
        let start = self.i;
        if self.c.get(start + 1) == Some(&'(') {
            let arith = self.c.get(start + 2) == Some(&'(');
            self.i += if arith { 3 } else { 2 };
            self.push(Some("punctuation"), start, self.i);
            self.cmd = !arith;
            return;
        }
        let end = self.var_end(start);
        self.i = end;
        self.push(
            if end > start + 1 {
                Some("variable")
            } else {
                None
            },
            start,
            end,
        );
        self.after_word(at_word);
    }

    fn word(&mut self, at_word: bool) {
        let start = self.i;
        while self.i < self.c.len() && !sh_special(self.c[self.i]) {
            self.i += 1;
        }
        if self.i == start {
            // A character no rule takes (cannot happen: every special one has a rule).
            self.i += 1;
        }
        let w = self.text(start, self.i);
        if !at_word {
            self.push(None, start, self.i);
            return;
        }
        if let Some(strip) = self.delim_next.take() {
            self.push(Some("string"), start, self.i);
            self.heredocs.push((w, strip));
            self.cmd = false;
            return;
        }
        if self.loop_var {
            self.loop_var = false;
            self.expect_in = true;
            self.push(Some("variable"), start, self.i);
            return;
        }
        let kw = SH_KW.contains(&w.as_str());
        if (kw && self.cmd) || w == "]]" || (w == "in" && self.expect_in) {
            self.push(Some("keyword"), start, self.i);
            self.cmd = SH_KW_CMD.contains(&w.as_str());
            self.expect_in = w == "case";
            self.loop_var = matches!(w.as_str(), "for" | "select");
            return;
        }
        if self.cmd {
            // `NAME=value` and `NAME+=value` before the command name.
            if let Some(eq) = w.find('=') {
                let name = w[..eq].trim_end_matches('+');
                let ident = name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                    && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
                if ident {
                    let k = w[..=eq].chars().count();
                    self.push(Some("variable"), start, start + k);
                    if start + k < self.i {
                        self.push(None, start + k, self.i);
                    }
                    return;
                }
            }
            self.cmd = false;
            self.push(Some("function"), start, self.i);
            return;
        }
        let digits = w.chars().all(|c| c.is_ascii_digit());
        self.push(if digits { Some("number") } else { None }, start, self.i);
    }

    /// After a newline: the bodies of the here-documents that started on the line.
    fn bodies(&mut self) {
        for (delim, strip) in std::mem::take(&mut self.heredocs) {
            let start = self.i;
            loop {
                if self.i >= self.c.len() {
                    break;
                }
                let line_start = self.i;
                while self.i < self.c.len() && self.c[self.i] != '\n' {
                    self.i += 1;
                }
                let line = self.text(line_start, self.i);
                if self.i < self.c.len() {
                    self.i += 1;
                }
                let line = if strip {
                    line.trim_start_matches('\t')
                } else {
                    &line
                };
                if line == delim {
                    break;
                }
            }
            if self.i > start {
                self.push(Some("string"), start, self.i);
            }
        }
    }
}

// ---- Rust, JavaScript, JSON -----------------------------------------------------------

const JS_KW: &[&str] = &[
    "as",
    "async",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "default",
    "delete",
    "do",
    "else",
    "export",
    "extends",
    "finally",
    "for",
    "from",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "let",
    "new",
    "of",
    "return",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "yield",
];
const JS_CONST: &[&str] = &["true", "false", "null", "undefined", "NaN", "Infinity"];
const RS_KW: &[&str] = &[
    "as", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "type", "unsafe", "use", "where",
    "while",
];
const RS_CONST: &[&str] = &["true", "false"];
const RS_TYPES: &[&str] = &[
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f32",
    "f64", "bool", "char", "str",
];

/// Rust, JavaScript and JSON: one tokenizer with per-language word lists.
fn c_like(lang: &str, code: &str) -> String {
    let rust = matches!(lang, "rust" | "rs");
    let json = lang == "json";
    let (kw, consts, types): (&[&str], &[&str], &[&str]) = if rust {
        (RS_KW, RS_CONST, RS_TYPES)
    } else if json {
        (&[], &["true", "false", "null"], &[])
    } else {
        (JS_KW, JS_CONST, &[])
    };
    let c: Vec<char> = code.chars().collect();
    let mut out: Vec<(Option<&'static str>, String)> = Vec::new();
    let mut i = 0;
    let mut prev_word = String::new();
    let mut punct = String::new();
    let rest = |a: usize, b: usize| c[a..b].iter().collect::<String>();
    while i < c.len() {
        let ch = c[i];
        let comment = !json && ch == '/' && matches!(c.get(i + 1), Some('/' | '*'));
        if !ch.is_whitespace() && !ch.is_alphanumeric() && !"_$\"'`".contains(ch) && !comment {
            punct.push(ch);
            prev_word.clear();
            i += 1;
            continue;
        }
        if !punct.is_empty() {
            out.push((Some("punctuation"), std::mem::take(&mut punct)));
        }
        if comment && c[i + 1] == '/' {
            let mut j = i;
            while j < c.len() && c[j] != '\n' {
                j += 1;
            }
            out.push((Some("comment"), rest(i, j)));
            i = j;
        } else if comment {
            let mut j = i + 2;
            while j + 1 < c.len() && !(c[j] == '*' && c[j + 1] == '/') {
                j += 1;
            }
            j = (j + 2).min(c.len());
            out.push((Some("comment"), rest(i, j)));
            i = j;
        } else if ch == '"' || (ch == '`' && !rust) || (ch == '\'' && !lifetime(rust, &c, i)) {
            let mut j = i + 1;
            while j < c.len() && c[j] != ch {
                if c[j] == '\\' {
                    j += 1;
                }
                j += 1;
            }
            j = (j + 1).min(c.len());
            // A JSON object key is a name, not a value.
            let key = json && c[j..].iter().find(|c| !c.is_whitespace()) == Some(&':');
            out.push((Some(if key { "variable" } else { "string" }), rest(i, j)));
            i = j;
        } else if ch.is_ascii_digit() {
            let mut j = i;
            while j < c.len() && (c[j].is_ascii_alphanumeric() || c[j] == '.' || c[j] == '_') {
                j += 1;
            }
            out.push((Some("number"), rest(i, j)));
            i = j;
        } else if ch.is_alphabetic() || ch == '_' || ch == '$' || ch == '\'' {
            let mut j = i + 1;
            while j < c.len() && (c[j].is_alphanumeric() || c[j] == '_' || c[j] == '$') {
                j += 1;
            }
            let w = rest(i, j);
            let bang = rust && c.get(j) == Some(&'!') && c.get(j + 1) != Some(&'=');
            let role = if kw.contains(&w.as_str()) {
                "keyword"
            } else if consts.contains(&w.as_str()) {
                "number"
            } else if types.contains(&w.as_str())
                || (!json && w.starts_with(|c: char| c.is_ascii_uppercase()))
            {
                "type"
            } else if c.get(j) == Some(&'(')
                || bang
                || matches!(prev_word.as_str(), "fn" | "function")
            {
                "function"
            } else {
                "variable"
            };
            out.push((Some(role), w.clone()));
            if bang {
                out.push((Some("function"), "!".to_string()));
                j += 1;
            }
            prev_word = w;
            i = j;
        } else {
            out.push((None, ch.to_string()));
            i += 1;
        }
    }
    if !punct.is_empty() {
        out.push((Some("punctuation"), punct));
    }
    html(&out)
}

/// `'a` in Rust is a lifetime, not a character literal (`'a'`).
fn lifetime(rust: bool, c: &[char], i: usize) -> bool {
    rust && c.get(i + 1).is_some_and(|c| c.is_alphabetic() || *c == '_')
        && c.get(i + 2) != Some(&'\'')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roles(code: &str) -> Vec<(String, String)> {
        sh_tokens(code)
            .into_iter()
            .filter(|(r, t)| r.is_some() || !t.trim().is_empty())
            .map(|(r, t)| (r.unwrap_or("-").to_string(), t))
            .collect()
    }

    fn pairs(v: &[(&str, &str)]) -> Vec<(String, String)> {
        v.iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    #[test]
    fn tokens_join_to_the_source() {
        for code in [
            "a=1 b+=2 cmd -x \"$a ${b:-c}\" 'q' $'e\\'' 2>&1 | wc -l # c\n",
            "cat <<-EOF >out\n\tbody $x\n\tEOF\necho done\n",
            "$ psh -c 'echo hi'\nhi\n",
            "for f in *.rs; do echo \"$f\"; done; case $x in a) ;; esac",
            "\\\n` x`$((1+2))$(ls)",
        ] {
            let joined: String = sh_tokens(code).into_iter().map(|t| t.1).collect();
            assert_eq!(joined, code);
        }
    }

    #[test]
    fn shell_roles() {
        assert_eq!(
            roles("X=1 sort -u \"$f\" | head -2 >> out.txt # done"),
            pairs(&[
                ("variable", "X="),
                ("-", "1"),
                ("function", "sort"),
                ("-", "-u"),
                ("string", "\""),
                ("variable", "$f"),
                ("string", "\""),
                ("punctuation", "|"),
                ("function", "head"),
                ("-", "-2"),
                ("punctuation", ">>"),
                ("-", "out.txt"),
                ("comment", "# done"),
            ])
        );
        assert_eq!(
            roles("for f in a b; do echo $f; done"),
            pairs(&[
                ("keyword", "for"),
                ("variable", "f"),
                ("keyword", "in"),
                ("-", "a"),
                ("-", "b"),
                ("punctuation", ";"),
                ("keyword", "do"),
                ("function", "echo"),
                ("variable", "$f"),
                ("punctuation", ";"),
                ("keyword", "done"),
            ])
        );
        assert_eq!(
            roles("cat <<'EOF'\nif x\nEOF\nls"),
            pairs(&[
                ("function", "cat"),
                ("punctuation", "<<"),
                ("string", "'EOF'"),
                ("string", "if x\nEOF\n"),
                ("function", "ls"),
            ])
        );
        assert_eq!(
            roles("$ echo hi\nhi there\n"),
            pairs(&[
                ("punctuation", "$"),
                ("function", "echo"),
                ("-", "hi"),
                ("-", "hi there\n"),
            ])
        );
    }

    /// assets/highlight.js gives every shell block of the guide the HTML this file does.
    #[test]
    fn js_port_agrees() {
        use std::io::Write as _;
        use std::process::{Command, Stdio};
        let mut samples: Vec<String> = [
            "a=1 b+=2 cmd -x \"$a ${b:-c}\" 'q' $'e\\'' 2>&1 | wc -l # c\n",
            "cat <<-EOF >out\n\tbody $x\n\tEOF\necho done\n",
            "for f in *.rs; do echo \"$f\"; done; case $x in a) ;; esac",
            "\\\n` x`$((1+2))$(ls) é=1 ü",
            "",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let dir = crate::repo().join("docs/guide");
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let md = std::fs::read_to_string(e.path()).unwrap_or_default();
            let mut block: Option<String> = None;
            for line in md.split_inclusive('\n') {
                match &mut block {
                    Some(b) if line.starts_with("```") => {
                        samples.push(std::mem::take(b));
                        block = None;
                    }
                    Some(b) => b.push_str(line),
                    None if line.starts_with("```sh") || line.starts_with("```bash") => {
                        block = Some(String::new())
                    }
                    None => {}
                }
            }
        }
        let js = crate::repo().join("crates/psh-docs/assets/highlight.js");
        let script = format!(
            "import {{ highlight }} from {:?};\n\
             let s = ''; process.stdin.setEncoding('utf8');\n\
             for await (const c of process.stdin) s += c;\n\
             process.stdout.write(s.split('\\0').map(highlight).join('\\0'));\n",
            format!("file://{}", js.display())
        );
        let child = Command::new("node")
            .args(["--input-type=module", "-e", &script])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn();
        let Ok(mut child) = child else {
            eprintln!("node not found: highlight.js not compared");
            return;
        };
        child
            .stdin
            .take()
            .unwrap()
            .write_all(samples.join("\0").as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success(), "node failed");
        let js_html = String::from_utf8(out.stdout).unwrap();
        let js_html: Vec<&str> = js_html.split('\0').collect();
        assert_eq!(js_html.len(), samples.len());
        for (code, js) in samples.iter().zip(js_html) {
            assert_eq!(
                highlight("sh", code),
                js,
                "highlight.js differs on {code:?}"
            );
        }
    }

    #[test]
    fn c_like_roles() {
        let h = highlight("rust", "fn get<'a>(x: &'a u32) -> char { vec![b'x'] }");
        assert!(h.contains("<span class=\"tok-function\">get</span>"), "{h}");
        assert!(h.contains("<span class=\"tok-type\">u32</span>"), "{h}");
        let h = highlight("json", "{\"ok\": true, \"n\": [1]}");
        assert!(
            h.contains("<span class=\"tok-variable\">&quot;ok&quot;</span>")
                || h.contains("<span class=\"tok-variable\">\"ok\"</span>"),
            "{h}"
        );
        assert!(h.contains("<span class=\"tok-number\">true</span>"), "{h}");
    }
}
