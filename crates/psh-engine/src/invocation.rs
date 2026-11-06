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
    },
    /// `FILE [ARGS…]`
    File { path: Vec<u8>, args: Vec<Vec<u8>> },
    /// No script operand (or `-s`): the script is stdin.
    Stdin { args: Vec<Vec<u8>> },
    /// `--help`
    Help,
}

/// Parses the arguments after the program name. Options other than `-c` and `-s` are
/// not supported yet and are an error (`set`'s options come with `set`).
pub fn parse_args(args: &[Vec<u8>]) -> Result<Invocation, String> {
    let mut command = false;
    let mut stdin = false;
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
            return Err(format!("{}: invalid option", String::from_utf8_lossy(a)));
        }
        for &c in &a[1..] {
            match c {
                b'c' if a[0] == b'-' => command = true,
                b's' if a[0] == b'-' => stdin = true,
                _ => {
                    return Err(format!(
                        "{}{}: option not supported yet",
                        a[0] as char, c as char
                    ));
                }
            }
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
        });
    }
    if stdin || rest.is_empty() {
        return Ok(Invocation::Stdin {
            args: rest.to_vec(),
        });
    }
    Ok(Invocation::File {
        path: rest[0].clone(),
        args: rest[1..].to_vec(),
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
                args: v(&["a"])
            })
        );
        assert_eq!(parse_args(&v(&[])), Ok(Invocation::Stdin { args: vec![] }));
        assert_eq!(
            parse_args(&v(&["f.sh", "x"])),
            Ok(Invocation::File {
                path: b"f.sh".to_vec(),
                args: v(&["x"])
            })
        );
        assert!(parse_args(&v(&["-e"])).is_err());
        assert!(parse_args(&v(&["-c"])).is_err());
    }
}
