//! `alias` and `unalias`, as pure functions over a [`psh_syntax::Aliases`] table.
//!
//! The parser expands aliases while it reads each chunk (with the table the shell passes
//! to `Parser::set_aliases`); these builtins only edit and print the table. They return
//! what to write, in order, so the caller keeps stdout and stderr interleaved as bash
//! does:
//!
//! ```ignore
//! async fn alias_builtin(vp: &mut Vp, args: &[Vec<u8>]) -> R {
//!     let out = alias::alias(&mut vp.aliases(), &args[1..]);
//!     for o in out.output {
//!         match o {
//!             Output::Stdout(b) => { write_fd(vp, 1, "alias", &b).await?; }
//!             Output::Error(m) => vp.error(&m).await,        // `NAME: line N: MSG`
//!             Output::Usage(u) => vp.error_raw(u.as_bytes()).await, // no prefix
//!         }
//!     }
//!     Ok(out.status)
//! }
//! ```

use psh_syntax::Aliases;

/// Something to write, in order.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Output {
    /// Bytes for stdout.
    Stdout(Vec<u8>),
    /// A message bash prints with its `NAME: line N: ` prefix.
    Error(String),
    /// A usage line bash prints as is (it ends in a newline).
    Usage(&'static str),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Outcome {
    pub status: i32,
    pub output: Vec<Output>,
}

const ALIAS_USAGE: &str = "alias: usage: alias [-p] [name[=value] ... ]\n";
const UNALIAS_USAGE: &str = "unalias: usage: unalias [-a] name [name ...]\n";

/// `alias [-p] [name[=value] ...]` (the arguments after the command name).
pub(crate) fn alias(table: &mut Aliases, args: &[Vec<u8>]) -> Outcome {
    let mut output = Vec::new();
    let mut status = 0;
    let mut print_all = args.is_empty();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == b"--" {
            i += 1;
            break;
        }
        if a.len() < 2 || a[0] != b'-' {
            break;
        }
        for &c in &a[1..] {
            if c == b'p' {
                print_all = true;
            } else {
                output.push(Output::Error(format!(
                    "alias: -{}: invalid option",
                    char::from(c)
                )));
                output.push(Output::Usage(ALIAS_USAGE));
                return Outcome { status: 2, output };
            }
        }
        i += 1;
    }
    if print_all {
        for (name, value) in table.iter() {
            output.push(Output::Stdout(definition(name, value)));
        }
    }
    for a in &args[i..] {
        match a.iter().position(|&c| c == b'=') {
            Some(eq) if eq > 0 => {
                let (name, value) = (&a[..eq], &a[eq + 1..]);
                if valid_name(name) {
                    table.set(name, value);
                } else {
                    output.push(Output::Error(format!(
                        "alias: `{}': invalid alias name",
                        String::from_utf8_lossy(name)
                    )));
                    status = 1;
                }
            }
            _ => match table.get(a) {
                Some(value) => output.push(Output::Stdout(definition(a, value))),
                None => {
                    output.push(Output::Error(format!(
                        "alias: {}: not found",
                        String::from_utf8_lossy(a)
                    )));
                    status = 1;
                }
            },
        }
    }
    Outcome { status, output }
}

/// `unalias [-a] name ...`.
pub(crate) fn unalias(table: &mut Aliases, args: &[Vec<u8>]) -> Outcome {
    let mut output = Vec::new();
    let mut i = 0;
    let mut all = false;
    while i < args.len() {
        let a = &args[i];
        if a == b"--" {
            i += 1;
            break;
        }
        if a.len() < 2 || a[0] != b'-' {
            break;
        }
        for &c in &a[1..] {
            if c == b'a' {
                all = true;
            } else {
                output.push(Output::Error(format!(
                    "unalias: -{}: invalid option",
                    char::from(c)
                )));
                output.push(Output::Usage(UNALIAS_USAGE));
                return Outcome { status: 2, output };
            }
        }
        i += 1;
    }
    if all {
        table.clear();
        return Outcome { status: 0, output };
    }
    if i == args.len() {
        output.push(Output::Usage(UNALIAS_USAGE));
        return Outcome { status: 2, output };
    }
    let mut status = 0;
    for name in &args[i..] {
        if !table.remove(name) {
            output.push(Output::Error(format!(
                "unalias: {}: not found",
                String::from_utf8_lossy(name)
            )));
            status = 1;
        }
    }
    Outcome { status, output }
}

/// Bash refuses `/`, `$`, `` ` ``, `=`, blanks, metacharacters and quotes in alias names.
fn valid_name(name: &[u8]) -> bool {
    !name.iter().any(|c| {
        matches!(
            c,
            b'/' | b'$'
                | b'`'
                | b'='
                | b' '
                | b'\t'
                | b'\n'
                | b'|'
                | b'&'
                | b';'
                | b'('
                | b')'
                | b'<'
                | b'>'
                | b'\\'
                | b'\''
                | b'"'
        )
    })
}

/// `alias name='value'` (with `--` before names that start with `-`), as `alias` prints
/// a definition: the value single-quoted, each `'` as `'\''`.
fn definition(name: &[u8], value: &[u8]) -> Vec<u8> {
    let mut out = b"alias ".to_vec();
    if name.first() == Some(&b'-') {
        out.extend_from_slice(b"-- ");
    }
    out.extend_from_slice(name);
    out.extend_from_slice(b"='");
    for &c in value {
        if c == b'\'' {
            out.extend_from_slice(b"'\\''");
        } else {
            out.push(c);
        }
    }
    out.extend_from_slice(b"'\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Vec<Vec<u8>> {
        a.iter().map(|s| s.as_bytes().to_vec()).collect()
    }

    fn text(o: &Outcome) -> String {
        let mut s = String::new();
        for x in &o.output {
            match x {
                Output::Stdout(b) => s.push_str(&String::from_utf8_lossy(b)),
                Output::Error(m) => {
                    s.push_str("E: ");
                    s.push_str(m);
                    s.push('\n');
                }
                Output::Usage(u) => s.push_str(u),
            }
        }
        s
    }

    // Expected texts are bash 5.3's (recording container).
    #[test]
    fn define_and_print() {
        let mut t = Aliases::new();
        let o = alias(&mut t, &args(&["b=x y", "a=it's", "c="]));
        assert_eq!(
            o,
            Outcome {
                status: 0,
                output: vec![]
            }
        );
        let o = alias(&mut t, &[]);
        assert_eq!(text(&o), "alias a='it'\\''s'\nalias b='x y'\nalias c=''\n");
        let o = alias(&mut t, &args(&["-p", "a"]));
        assert_eq!(
            text(&o),
            "alias a='it'\\''s'\nalias b='x y'\nalias c=''\nalias a='it'\\''s'\n"
        );
        let o = alias(&mut t, &args(&["a", "nope"]));
        assert_eq!(o.status, 1);
        assert_eq!(text(&o), "alias a='it'\\''s'\nE: alias: nope: not found\n");
        let o = alias(&mut t, &args(&["--", "-p=1"]));
        assert_eq!(o.status, 0);
        assert_eq!(
            text(&alias(&mut t, &args(&["--", "-p"]))),
            "alias -- -p='1'\n"
        );
        let o = alias(&mut t, &args(&["a=b=c", "é=1", "{=2"]));
        assert_eq!(o.status, 0);
        assert_eq!(t.get(b"a"), Some(&b"b=c"[..]));
    }

    #[test]
    fn errors() {
        let mut t = Aliases::new();
        let o = alias(&mut t, &args(&["a/b=x", "a b=x", "$x=1"]));
        assert_eq!(o.status, 1);
        assert_eq!(
            text(&o),
            "E: alias: `a/b': invalid alias name\nE: alias: `a b': invalid alias name\nE: alias: `$x': invalid alias name\n"
        );
        let o = alias(&mut t, &args(&["-x"]));
        assert_eq!(o.status, 2);
        assert_eq!(
            text(&o),
            format!("E: alias: -x: invalid option\n{ALIAS_USAGE}")
        );
        let o = alias(&mut t, &args(&["=x"]));
        assert_eq!(
            (o.status, text(&o)),
            (1, "E: alias: =x: not found\n".into())
        );
        let o = unalias(&mut t, &[]);
        assert_eq!((o.status, text(&o)), (2, UNALIAS_USAGE.into()));
        let o = unalias(&mut t, &args(&["nope"]));
        assert_eq!(
            (o.status, text(&o)),
            (1, "E: unalias: nope: not found\n".into())
        );
        let o = unalias(&mut t, &args(&["-x"]));
        assert_eq!(o.status, 2);
        alias(&mut t, &args(&["a=1", "b=2"]));
        let o = unalias(&mut t, &args(&["a", "a"]));
        assert_eq!(
            (o.status, text(&o)),
            (1, "E: unalias: a: not found\n".into())
        );
        assert_eq!(unalias(&mut t, &args(&["-a"])).status, 0);
        assert!(t.is_empty());
    }
}
