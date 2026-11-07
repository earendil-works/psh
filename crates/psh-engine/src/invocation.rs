//! The shell's command line (`psh -c CMD`, `psh FILE`, a script on stdin), shared by
//! the `psh` binary and the in-process conformance runner so they cannot drift.

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
pub enum Invocation {
    /// `-c COMMAND [NAME [ARGS…]]`
    Command {
        source: Vec<u8>,
        arg0: Option<Vec<u8>>,
        args: Vec<Vec<u8>>,
        options: Vec<Vec<u8>>,
    },
    /// `FILE [ARGS…]`
    File {
        path: Vec<u8>,
        args: Vec<Vec<u8>>,
        options: Vec<Vec<u8>>,
    },
    /// No script operand (or `-s`): the script is stdin.
    Stdin {
        args: Vec<Vec<u8>>,
        options: Vec<Vec<u8>>,
    },
    /// `--help`
    Help,
}

/// Parses the arguments after the program name: `-c`, `-s`, and `set`'s and `shopt`'s
/// options (`-e`, `+x`, `-o pipefail`, `-O extglob`, `--posix`), which come back in
/// `options` for `Script::options` (D1280).
pub fn parse_args(args: &[Vec<u8>]) -> Result<Invocation, String> {
    let mut command = false;
    let mut stdin = false;
    let mut options: Vec<Vec<u8>> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == b"--" || a == b"-" {
            i += 1;
            break;
        }
        if a == b"--help" {
            return Ok(Invocation::Help);
        }
        if a.len() < 2 || (a[0] != b'-' && a[0] != b'+') {
            break;
        }
        if a.starts_with(b"--") {
            match a.as_slice() {
                b"--posix" => return Err("--posix: POSIX mode is not supported".into()),
                b"--norc" | b"--noprofile" | b"--noediting" => {}
                _ => return Err(format!("{}: invalid option", String::from_utf8_lossy(a))),
            }
            i += 1;
            continue;
        }
        let on = a[0] == b'-';
        let mut letters = vec![a[0]];
        for &c in &a[1..] {
            match c {
                b'c' if on => command = true,
                b's' if on => stdin = true,
                b'o' | b'O' => {
                    let Some(name) = args.get(i + 1) else {
                        return Err(format!("-{}: option requires an argument", c as char));
                    };
                    let known = if c == b'o' {
                        crate::options::by_name(name).is_some()
                    } else {
                        std::str::from_utf8(name)
                            .ok()
                            .and_then(crate::options::shopt_index)
                            .is_some()
                    };
                    if c == b'o' && name == b"posix" && on {
                        return Err("-o posix: POSIX mode is not supported".into());
                    }
                    if !known {
                        return Err(format!(
                            "{}: invalid {}option name",
                            String::from_utf8_lossy(name),
                            if c == b'O' { "shell " } else { "" }
                        ));
                    }
                    options.extend([vec![a[0], c], name.clone()]);
                    i += 1;
                }
                c if crate::options::by_letter(c).is_some() => letters.push(c),
                _ => {
                    return Err(format!("{}{}: invalid option", a[0] as char, c as char));
                }
            }
        }
        if letters.len() > 1 {
            options.push(letters);
        }
        i += 1;
    }
    let rest = &args[i..];
    if command {
        let Some(source) = rest.first() else {
            return Err("-c: option requires an argument".into());
        };
        return Ok(Invocation::Command {
            source: source.clone(),
            arg0: rest.get(1).cloned(),
            args: rest.get(2..).map(<[Vec<u8>]>::to_vec).unwrap_or_default(),
            options,
        });
    }
    if stdin || rest.is_empty() {
        // `s` in `$-`.
        options.push(b"-s".to_vec());
        return Ok(Invocation::Stdin {
            args: rest.to_vec(),
            options,
        });
    }
    Ok(Invocation::File {
        path: rest[0].clone(),
        args: rest[1..].to_vec(),
        options,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(a: &[&str]) -> Vec<Vec<u8>> {
        a.iter().map(|s| s.as_bytes().to_vec()).collect()
    }

    #[test]
    fn forms() {
        assert_eq!(
            parse_args(&v(&["-c", "echo", "n", "a"])),
            Ok(Invocation::Command {
                source: b"echo".to_vec(),
                arg0: Some(b"n".to_vec()),
                args: v(&["a"]),
                options: vec![],
            })
        );
        assert_eq!(
            parse_args(&v(&[])),
            Ok(Invocation::Stdin {
                args: vec![],
                options: v(&["-s"])
            })
        );
        assert_eq!(
            parse_args(&v(&["f.sh", "x"])),
            Ok(Invocation::File {
                path: b"f.sh".to_vec(),
                args: v(&["x"]),
                options: vec![],
            })
        );
        assert_eq!(
            parse_args(&v(&["-eu", "-o", "pipefail", "+O", "extglob", "-c", "x"])),
            Ok(Invocation::Command {
                source: b"x".to_vec(),
                arg0: None,
                args: vec![],
                options: v(&["-eu", "-o", "pipefail", "+O", "extglob"]),
            })
        );
        assert!(parse_args(&v(&["-Z"])).is_err());
        assert!(parse_args(&v(&["-o", "bogus", "-c", "x"])).is_err());
        assert!(parse_args(&v(&["-c"])).is_err());
    }
}
