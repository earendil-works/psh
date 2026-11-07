//! jq's destructuring alternative `E as P1 ?// P2 | BODY`, which jaq's parser lacks,
//! rewritten into what it means before the program is parsed (D1181):
//!
//! ```text
//! E as $alt | . as $dot | null as $v1 | ... |
//!   try ($alt as P1 | BODY) catch ($dot | $alt as P2 | BODY)
//! ```
//!
//! Every variable of every pattern is bound (null where its pattern did not match); an
//! error in the body of any but the last alternative tries the next one, keeping what
//! was already output, as jq does. Written for psh after jq's manual.

/// One token: its kind and its byte range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Str,
    Var,
    Ident,
    Punct,
    Other,
}

#[derive(Clone, Copy, Debug)]
struct Tok {
    kind: Kind,
    start: usize,
    end: usize,
}

/// The end of a string literal starting at `i` (its quote), past interpolations.
fn string_end(b: &[u8], mut i: usize) -> usize {
    i += 1;
    while i < b.len() {
        match b[i] {
            b'"' => return i + 1,
            b'\\' if b.get(i + 1) == Some(&b'(') => {
                // An interpolation: up to its closing parenthesis.
                i += 2;
                let mut depth = 1;
                while i < b.len() && depth > 0 {
                    match b[i] {
                        b'"' => {
                            i = string_end(b, i);
                            continue;
                        }
                        b'(' => depth += 1,
                        b')' => depth -= 1,
                        _ => {}
                    }
                    i += 1;
                }
            }
            b'\\' => i += 2,
            _ => i += 1,
        }
    }
    b.len()
}

fn tokens(src: &str) -> Vec<Tok> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let start = i;
        let kind = if c.is_ascii_whitespace() {
            i += 1;
            continue;
        } else if c == b'#' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        } else if c == b'"' {
            i = string_end(b, i);
            Kind::Str
        } else if c == b'$' {
            i += 1;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b':') {
                i += 1;
            }
            // `$name:` ends before the colon unless it is `$a::b`.
            while i > start + 1 && b[i - 1] == b':' && !src[start..i].contains("::") {
                i -= 1;
            }
            Kind::Var
        } else if c.is_ascii_alphabetic() || c == b'_' || c == b'@' {
            i += 1;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b':') {
                i += 1;
            }
            Kind::Ident
        } else if c.is_ascii_digit() || (c == b'.' && b.get(i + 1).is_some_and(u8::is_ascii_digit))
        {
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'.') {
                i += 1;
            }
            Kind::Other
        } else if src[i..].starts_with("?//") {
            i += 3;
            Kind::Punct
        } else {
            i += 1;
            Kind::Punct
        };
        out.push(Tok {
            kind,
            start,
            end: i,
        });
    }
    out
}

fn text<'s>(src: &'s str, t: &Tok) -> &'s str {
    &src[t.start..t.end]
}

/// The tokens of one pattern from `i`: a variable, or a bracketed array or object
/// pattern. Returns the index after it.
fn pattern_end(src: &str, toks: &[Tok], i: usize) -> Option<usize> {
    let t = toks.get(i)?;
    match (t.kind, text(src, t)) {
        (Kind::Var, _) => Some(i + 1),
        (Kind::Punct, "[" | "{") => {
            let mut depth = 0;
            for (k, t) in toks.iter().enumerate().skip(i) {
                if t.kind == Kind::Punct {
                    match text(src, t) {
                        "[" | "{" | "(" => depth += 1,
                        "]" | "}" | ")" => {
                            depth -= 1;
                            if depth == 0 {
                                return Some(k + 1);
                            }
                        }
                        _ => {}
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// The variables a pattern binds: the `$name`s outside parenthesized key expressions.
fn pattern_vars(src: &str, toks: &[Tok], out: &mut Vec<String>) {
    let mut parens = 0;
    for t in toks {
        match (t.kind, text(src, t)) {
            (Kind::Punct, "(") => parens += 1,
            (Kind::Punct, ")") => parens -= 1,
            (Kind::Var, v) if parens == 0 && v != "$__loc__" && !out.iter().any(|x| x == v) => {
                out.push(v.to_string());
            }
            _ => {}
        }
    }
}

/// Where the body starting at token `i` ends: before an unbalanced closing bracket, a
/// `;` that ends no `def` of the body, or an `if`'s `then`/`elif`/`else`/`end`.
fn body_end(src: &str, toks: &[Tok], i: usize) -> usize {
    let (mut depth, mut ifs, mut defs) = (0i32, 0i32, 0i32);
    for (k, t) in toks.iter().enumerate().skip(i) {
        let s = text(src, t);
        match (t.kind, s) {
            (Kind::Punct, "(" | "[" | "{") => depth += 1,
            (Kind::Punct, ")" | "]" | "}") => {
                if depth == 0 {
                    return t.start;
                }
                depth -= 1;
            }
            (Kind::Punct, ";") if depth == 0 => {
                if defs == 0 {
                    return t.start;
                }
                defs -= 1;
            }
            (Kind::Ident, "def") if depth == 0 => defs += 1,
            (Kind::Ident, "if") => ifs += 1,
            (Kind::Ident, "end") => {
                if ifs == 0 && depth == 0 {
                    return t.start;
                }
                ifs -= 1;
            }
            (Kind::Ident, "then" | "elif" | "else") if ifs == 0 && depth == 0 => {
                return t.start;
            }
            _ => {}
        }
        let _ = k;
    }
    src.len()
}

/// Rewrites the first `as P1 ?// P2 ...| BODY`; `None` if there is none.
fn rewrite_one(src: &str, n: usize) -> Option<String> {
    let toks = tokens(src);
    for (i, t) in toks.iter().enumerate() {
        if t.kind != Kind::Ident || text(src, t) != "as" {
            continue;
        }
        let mut pats: Vec<(usize, usize)> = Vec::new();
        let mut k = i + 1;
        while let Some(e) = pattern_end(src, &toks, k) {
            pats.push((k, e));
            k = e;
            match toks.get(k) {
                Some(t) if t.kind == Kind::Punct && text(src, t) == "?//" => k += 1,
                _ => break,
            }
        }
        if pats.len() < 2 {
            continue;
        }
        let Some(bar) = toks.get(k) else { continue };
        if text(src, bar) != "|" {
            continue;
        }
        let body_start = toks.get(k + 1).map_or(src.len(), |t| t.start);
        let end = body_end(src, &toks, k + 1);
        let body = &src[body_start..end];
        let mut vars = Vec::new();
        for (a, b) in &pats {
            pattern_vars(src, &toks[*a..*b], &mut vars);
        }
        let pat_text = |(a, b): (usize, usize)| &src[toks[a].start..toks[b - 1].end];
        let alt = format!("$__alt{n}");
        let dot = format!("$__dot{n}");
        let mut chain = String::new();
        for (idx, p) in pats.iter().enumerate() {
            let branch = format!("({dot} | {alt} as {} | {body})", pat_text(*p));
            if idx + 1 == pats.len() {
                chain.push_str(&branch);
            } else {
                chain.push_str(&format!("try {branch} catch ("));
            }
        }
        for _ in 1..pats.len() {
            chain.push(')');
        }
        let nulls: String = vars.iter().map(|v| format!("null as {v} | ")).collect();
        let mut out = String::new();
        out.push_str(&src[..t.start]);
        out.push_str(&format!("as {alt} | (. as {dot} | {nulls}{chain})"));
        out.push_str(&src[end..]);
        return Some(out);
    }
    None
}

/// The program with every destructuring alternative rewritten; `None` if it has none.
pub fn rewrite(src: &str) -> Option<String> {
    if !src.contains("?//") {
        return None;
    }
    let mut cur = src.to_string();
    let mut changed = false;
    for n in 0..64 {
        match rewrite_one(&cur, n) {
            Some(next) => {
                cur = next;
                changed = true;
            }
            None => break,
        }
    }
    changed.then_some(cur)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_alternatives() {
        let r = rewrite(".[] as [$a] ?// $a | $a").unwrap();
        assert_eq!(
            r,
            ".[] as $__alt0 | (. as $__dot0 | null as $a | try ($__dot0 | $__alt0 as [$a] | $a) catch (($__dot0 | $__alt0 as $a | $a)))"
        );
        // The body ends at the enclosing bracket.
        let r = rewrite("[.[] as {a:$x} ?// [$x] | $x], 1").unwrap();
        assert!(r.ends_with("| $x)))], 1"), "{r}");
        assert!(rewrite(".a // .b").is_none());
    }
}
