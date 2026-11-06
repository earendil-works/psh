//! Helpers over words for the engine and the classifier.

use alloc::string::String;

use crate::ast::{Ast, Part, Word, WordPart};

/// The body of a backquoted command substitution (the text of a [`Part::Backtick`],
/// with its backquotes), ready to parse: inside backquotes a backslash quotes only `$`,
/// `` ` `` and `\` (and `"` when the substitution is inside double quotes).
pub fn backtick_body(text: &str, in_double_quotes: bool) -> String {
    let inner = text
        .strip_prefix('`')
        .and_then(|t| t.strip_suffix('`'))
        .unwrap_or(text);
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\'
            && let Some(&n) = chars.peek()
            && (matches!(n, '$' | '`' | '\\') || (in_double_quotes && n == '"'))
        {
            out.push(n);
            chars.next();
            continue;
        }
        out.push(c);
    }
    out
}

/// The value of a word that has no expansions at all (only literal text, escapes and
/// quotes without expansions inside), after quote removal; `None` if it has any
/// expansion, brace expansion or extglob. Glob characters in unquoted text are kept as
/// they are, so callers that care must check for them.
pub fn literal_text(ast: &Ast, word: &Word) -> Option<String> {
    let mut out = String::new();
    if collect(ast, &word.parts, &mut out) {
        Some(out)
    } else {
        None
    }
}

fn collect(ast: &Ast, parts: &[WordPart], out: &mut String) -> bool {
    for p in parts {
        let text = ast.text(p.span);
        match &p.kind {
            Part::Lit => out.push_str(text),
            Part::Escaped => out.push_str(&text[1..]),
            Part::Single => out.push_str(&text[1..text.len() - 1]),
            Part::Double(inner) | Part::Locale(inner) => {
                if !collect(ast, inner, out) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backtick_unescape() {
        assert_eq!(
            backtick_body(r"`echo \$x \`date\` \a`", false),
            r"echo $x `date` \a"
        );
        assert_eq!(backtick_body(r#"`echo \"x\"`"#, false), r#"echo \"x\""#);
        assert_eq!(backtick_body(r#"`echo \"x\"`"#, true), r#"echo "x""#);
    }

    #[test]
    fn literal() {
        let ast = crate::parse(r#"echo a\ b'c'"d" "$x""#).unwrap();
        let crate::CommandKind::Simple(s) = &ast.cmd(crate::CmdId(0)).kind else {
            panic!()
        };
        assert_eq!(literal_text(&ast, &s.words[1]).as_deref(), Some("a bcd"));
        assert_eq!(literal_text(&ast, &s.words[2]), None);
    }
}
