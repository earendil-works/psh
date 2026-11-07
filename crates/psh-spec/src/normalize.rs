//! The normalization table: the few legitimate differences between a recording and a
//! test run, and between platforms. Every rule says why. A rule never hides a real
//! difference in behavior.

/// Replaces run-specific paths in output with stable placeholders. Applied to stdout,
/// stderr, file contents and symlink targets, at recording and at test time alike.
pub struct Paths {
    /// (literal, placeholder), longest literal first.
    rules: Vec<(Vec<u8>, &'static str)>,
}

impl Paths {
    /// - `<case>`: the case's working directory (also `$TMP` for Oils cases), a fresh
    ///   temporary directory per case.
    /// - `<home>`: `$HOME`, also per case.
    /// - `<repo>`: `$REPO_ROOT`, the run's copy of the vendored Oils tree.
    /// - `<bin>`: the run's helper directory (first on `PATH`).
    /// - `<run>`: the run's temporary root (anything else under it).
    /// - `<sh>`: the path of the shell binary under test (`$SH`).
    ///
    /// Each path is given in every spelling the shell may print: as created and
    /// canonicalized (macOS's `/var` is a symlink to `/private/var`).
    ///
    /// `literals` are replaced as they are: the host's name (`<host>`), which a few cases
    /// print (prompts, `hostname`) and which differs per machine by nature.
    pub fn new(
        entries: &[(&std::path::Path, &'static str)],
        literals: &[(String, &'static str)],
    ) -> Paths {
        let mut rules: Vec<(Vec<u8>, &'static str)> = literals
            .iter()
            .filter(|(l, _)| !l.is_empty())
            .map(|(l, p)| (l.as_bytes().to_vec(), *p))
            .collect();
        for (path, placeholder) in entries {
            let mut spellings = vec![path.to_path_buf()];
            if let Ok(canon) = std::fs::canonicalize(path) {
                spellings.push(canon);
            }
            for p in spellings {
                let text = p.to_string_lossy().into_owned();
                // On Windows psh writes paths as `C:/x`: the canonical `\\?\C:\x` and the
                // created `C:\x` both have that spelling too (D1914).
                let mut forms = vec![text.clone()];
                if cfg!(windows) {
                    let plain = text.strip_prefix(r"\\?\").unwrap_or(&text).to_string();
                    forms.push(plain.replace('\\', "/"));
                    forms.push(plain);
                }
                for f in forms {
                    let bytes = f.into_bytes();
                    if !bytes.is_empty() && !rules.iter().any(|(l, _)| *l == bytes) {
                        rules.push((bytes, *placeholder));
                    }
                }
            }
        }
        rules.sort_by_key(|a| std::cmp::Reverse(a.0.len()));
        Paths { rules }
    }

    pub fn apply(&self, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(data.len());
        let mut i = 0;
        'outer: while i < data.len() {
            for (lit, ph) in &self.rules {
                if data[i..].starts_with(lit) {
                    out.extend_from_slice(ph.as_bytes());
                    i += lit.len();
                    continue 'outer;
                }
            }
            out.push(data[i]);
            i += 1;
        }
        out
    }
}

/// The shell's name in its own messages (`bash: line 1: ...` vs `psh: line 1: ...`)
/// and as `$0` on a line of its own: the name differs by design, the message must not.
/// Applied to stderr and stdout of both sides at comparison time; goldens keep `bash`.
pub fn shell_name(data: &[u8], name: &str) -> Vec<u8> {
    let name = name.as_bytes();
    let mut out = Vec::with_capacity(data.len());
    for (i, line) in data.split(|&b| b == b'\n').enumerate() {
        if i > 0 {
            out.push(b'\n');
        }
        if line == name {
            out.extend_from_slice(b"<sh>");
        } else if line.starts_with(name) && line[name.len()..].starts_with(b": ") {
            out.extend_from_slice(b"<sh>");
            out.extend_from_slice(&line[name.len()..]);
        } else {
            out.extend_from_slice(line);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn longest_path_wins() {
        let p = Paths::new(
            &[
                (Path::new("/nonexistent/run"), "<run>"),
                (Path::new("/nonexistent/run/1/case"), "<case>"),
            ],
            &[("myhost".to_string(), "<host>")],
        );
        assert_eq!(
            p.apply(b"cd /nonexistent/run/1/case/x; ls /nonexistent/run/bin; myhost"),
            b"cd <case>/x; ls <run>/bin; <host>"
        );
    }

    #[test]
    fn shell_names() {
        assert_eq!(
            shell_name(b"bash: line 1: x: not found\nbash\nbashful: x\n", "bash"),
            b"<sh>: line 1: x: not found\n<sh>\nbashful: x\n"
        );
    }
}

/// Whether a run's status matches the golden's. On Windows no process dies by a signal:
/// the psh binary ends with `128 + n` where bash on Linux died by signal `n`, and a
/// child killed by psh reports 137 (D1914). Elsewhere the statuses must be equal.
pub fn same_status(golden: &crate::engine::Status, got: &crate::engine::Status) -> bool {
    use crate::engine::Status;
    if golden == got {
        return true;
    }
    cfg!(windows) && matches!((golden, got), (Status::Signal(n), Status::Exit(c)) if *c == 128 + *n)
}

#[cfg(test)]
mod status_tests {
    use crate::engine::Status;

    #[test]
    fn statuses() {
        assert!(super::same_status(&Status::Exit(3), &Status::Exit(3)));
        assert!(!super::same_status(&Status::Exit(3), &Status::Exit(4)));
        assert_eq!(
            super::same_status(&Status::Signal(15), &Status::Exit(143)),
            cfg!(windows)
        );
    }
}
