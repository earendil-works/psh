//! `set`: options and positional parameters (D1280).

use crate::options::{self, SET_OPTS, SetOpt};
use crate::state::{R, Vp};

use super::{out, s, shell_quote};

const USAGE: &str = "set: usage: set [-abefhkmnptuvxBCEHPT] [-o option-name] [--] [-] [arg ...]";

pub(crate) async fn set(vp: &mut Vp, args: &[Vec<u8>]) -> R {
    if args.len() == 1 {
        return list_vars(vp).await;
    }
    let mut i = 1;
    let mut positional = false;
    while i < args.len() {
        let a = &args[i];
        if a == b"--" {
            i += 1;
            positional = true;
            break;
        }
        if a == b"-" {
            // Turns off -x and -v and ends the options.
            set_opt(vp, SetOpt::Xtrace, false);
            set_opt(vp, SetOpt::Verbose, false);
            i += 1;
            positional = i < args.len();
            break;
        }
        let on = match a.first() {
            Some(b'-') => true,
            Some(b'+') => false,
            _ => break,
        };
        if a.len() == 1 {
            break;
        }
        for &c in &a[1..] {
            if c == b'o' {
                match args.get(i + 1) {
                    Some(name) if !name.starts_with(b"-") && !name.starts_with(b"+") => {
                        i += 1;
                        match options::by_name(name) {
                            Some(SetOpt::Posix) if on => return refuse_posix(vp).await,
                            Some(opt) => set_opt(vp, opt, on),
                            None => {
                                vp.error(&format!("set: {}: invalid option name", s(name)))
                                    .await;
                                return Ok(2);
                            }
                        }
                    }
                    _ => {
                        let st = list_options(vp, on).await?;
                        if st != 0 {
                            return Ok(st);
                        }
                    }
                }
                continue;
            }
            match options::by_letter(c) {
                Some(opt) => set_opt(vp, opt, on),
                None => {
                    vp.error(&format!(
                        "set: {}{}: invalid option",
                        if on { '-' } else { '+' },
                        c as char
                    ))
                    .await;
                    vp.error_raw(format!("{USAGE}\n").as_bytes()).await;
                    return Ok(2);
                }
            }
        }
        i += 1;
    }
    if positional || i < args.len() {
        vp.args = args[i..].to_vec();
    }
    Ok(0)
}

/// POSIX mode is out of scope (D002): turning it on fails loudly, like any feature psh
/// does not have, rather than running on with bash semantics (D1280).
pub(crate) async fn refuse_posix(vp: &Vp) -> R {
    crate::expand::unsupported(vp, "set -o posix").await
}

/// Turns an option on or off and keeps `SHELLOPTS` in step.
pub(crate) fn set_opt(vp: &mut Vp, opt: SetOpt, on: bool) {
    vp.opts.set(opt, on);
    options::sync(vp);
}

/// `set -o` (`on`) or `set +o`.
pub(crate) async fn list_options(vp: &Vp, on: bool) -> R {
    let mut buf = Vec::new();
    for (opt, name, ..) in SET_OPTS {
        let state = vp.opts.get(*opt);
        if on {
            buf.extend_from_slice(
                format!("{name:<15}\t{}\n", if state { "on" } else { "off" }).as_bytes(),
            );
        } else {
            buf.extend_from_slice(
                format!("set {}o {name}\n", if state { '-' } else { '+' }).as_bytes(),
            );
        }
    }
    out(vp, "set", &buf).await
}

/// `set` without arguments: every variable with a value, sorted, quoted for re-input.
async fn list_vars(vp: &Vp) -> R {
    let mut vars: Vec<_> = vp.vars.iter().filter(|(_, v)| v.get().is_some()).collect();
    vars.sort_by(|a, b| a.0.cmp(b.0));
    let mut buf = Vec::new();
    for (n, v) in vars {
        buf.extend_from_slice(n.as_bytes());
        buf.push(b'=');
        buf.extend_from_slice(&shell_quote(v.get().unwrap_or_default()));
        buf.push(b'\n');
    }
    out(vp, "set", &buf).await
}

/// Applies the options of the command line (`psh -e -o pipefail -O extglob -c …`),
/// already checked by `invocation::parse_args`.
pub(crate) fn apply_invocation_options(vp: &mut Vp, words: &[Vec<u8>]) {
    if words.is_empty() {
        return;
    }
    let mut i = 0;
    while i < words.len() {
        let w = &words[i];
        let on = w.first() == Some(&b'-');
        for &c in &w[1..] {
            match c {
                b'o' | b'O' => {
                    i += 1;
                    let Some(name) = words.get(i) else { continue };
                    if c == b'o' {
                        if let Some(opt) = options::by_name(name) {
                            vp.opts.set(opt, on);
                        }
                    } else if let Some(idx) = std::str::from_utf8(name)
                        .ok()
                        .and_then(options::shopt_index)
                    {
                        vp.opts.set_shopt_at(idx, on);
                    }
                }
                b's' => vp.opts.stdin = true,
                c => {
                    if let Some(opt) = options::by_letter(c) {
                        vp.opts.set(opt, on);
                    }
                }
            }
        }
        i += 1;
    }
    options::sync(vp);
}

#[cfg(test)]
mod tests {
    use crate::run_on_test_host as run;

    fn out(src: &str) -> (i32, String) {
        let (st, out, _) = run(src);
        (st, String::from_utf8_lossy(&out).into_owned())
    }

    #[test]
    fn errexit_exemptions() {
        let cases: &[(&str, i32, &str)] = &[
            ("set -e; f() { false && true; }; f; echo after", 1, ""),
            ("set -e; { false && true; }; echo after", 0, "after\n"),
            ("set -e; (false && true); echo after", 1, ""),
            ("set -e; x=$(false); echo after", 1, ""),
            (
                "set -e; f() { false; echo in; }; f || echo failed; echo after",
                0,
                "in\nafter\n",
            ),
            (
                "set -e; if false; then :; fi; while false; do :; done; echo ok",
                0,
                "ok\n",
            ),
            ("set -e; ! true; echo a; false || false; echo b", 1, "a\n"),
            ("set -e; true | { false; echo x; }; echo after", 1, ""),
            ("set -e; { false; echo x; } | cat; echo after", 0, "after\n"),
            ("set -e; false | true; echo after", 0, "after\n"),
            ("set -eo pipefail; false | true; echo after", 1, ""),
            ("set -e; x=$(false; echo sub); echo $x", 0, "sub\n"),
            (
                "shopt -s inherit_errexit; set -e; x=$(false; echo sub); echo $x",
                1,
                "",
            ),
            ("set -e; ((0)); echo after", 1, ""),
            ("set -e; f() { return 3; }; f; echo after", 3, ""),
        ];
        for (src, st, want) in cases {
            assert_eq!(out(src), (*st, want.to_string()), "{src}");
        }
    }

    #[test]
    fn options_and_listing() {
        assert_eq!(out("set -euo pipefail; echo $-"), (0, "ehuBc\n".into()));
        assert_eq!(
            out("set -o errexit nounset; echo $1 $-"),
            (0, "nounset ehBc\n".into())
        );
        assert_eq!(out("set -x - a b; echo $# $-"), (0, "2 hBc\n".into()));
        assert_eq!(out("set -o bogus; echo $?"), (0, "2\n".into()));
        let (st, o) = out("set +o; shopt -p extglob; shopt -q extglob; echo $?");
        assert_eq!(st, 0);
        assert!(
            o.starts_with("set +o allexport\nset -o braceexpand\n"),
            "{o}"
        );
        assert!(o.ends_with("set +o xtrace\nshopt -u extglob\n1\n"), "{o}");
        assert_eq!(out("SHELLOPTS=x; echo no").0, 1);
        assert_eq!(
            out("set -u; echo ${x-d}; echo $x; echo no"),
            (127, "d\n".into())
        );
        assert_eq!(out("set -eu; echo $x"), (1, String::new()));
        assert!(out("set -a; x=1; env").1.lines().any(|l| l == "x=1"));
        assert!(!out("x=1; env").1.lines().any(|l| l == "x=1"));
    }
}
