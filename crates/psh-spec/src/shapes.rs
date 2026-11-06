//! `bash -n` verdicts for the shape corpus (`tests/shapes/shapes.txt`), the parse-only
//! input of L0. Written to `tests/goldens/shapes/bash-n.txt`:
//!
//! ```text
//! # header comments: the shapes file's hash and size, the bash version
//! LINE<TAB>STATUS<TAB>STDERR
//! ```
//!
//! One line per skeleton that bash does not accept (status not 0), by its 1-based line
//! in `shapes.txt`; every skeleton not listed parses. STDERR is escaped like
//! `shapes.txt` (`\\`, `\n`, `\t`).

use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use crate::engine::{Engine, RunRequest, Status};
use crate::sandbox::fnv1a64;

/// Undoes `shapes.txt`'s escaping of a skeleton.
pub fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn escape(data: &[u8]) -> String {
    String::from_utf8_lossy(data)
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

pub fn record(
    root: &Path,
    goldens: &Path,
    engine: &dyn Engine,
    jobs: usize,
) -> Result<ExitCode, String> {
    let path = root.join("tests/shapes/shapes.txt");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let skeletons: Vec<String> = text
        .lines()
        .map(|l| unescape(l.split_once('\t').map_or(l, |(_, s)| s)))
        .collect();
    let dir = std::env::temp_dir().join(format!("psh-spec-shapes-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let env = vec![
        (
            "PATH".to_string(),
            "/usr/local/bin:/usr/bin:/bin".to_string(),
        ),
        ("LC_ALL".to_string(), "C.UTF-8".to_string()),
    ];
    let verdicts = crate::runner::parallel_map(&skeletons, jobs, |_, script| {
        engine.run(&RunRequest {
            args: &["-n".to_string()],
            stdin: Some(script.as_bytes()),
            env: &env,
            cwd: &dir,
            timeout: Duration::from_secs(10),
        })
    });
    let _ = std::fs::remove_dir_all(&dir);
    let version = std::process::Command::new(engine.binary())
        .arg("--version")
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .to_string()
        })
        .unwrap_or_default();
    let mut out = format!(
        "# bash -n verdicts for tests/shapes/shapes.txt (fnv1a64 {:016x}, {} skeletons),\n\
         # recorded by scripts/record-goldens with {version}.\n\
         # LINE<TAB>STATUS<TAB>STDERR for each skeleton bash rejects; unlisted ones parse.\n",
        fnv1a64(text.as_bytes()),
        skeletons.len()
    );
    let mut rejected = 0;
    for (i, v) in verdicts.into_iter().enumerate() {
        let v = v.map_err(|e| format!("line {}: {e}", i + 1))?;
        let status = match v.status {
            Status::Exit(0) => continue,
            Status::Exit(n) => n.to_string(),
            other => other.to_string(),
        };
        rejected += 1;
        out.push_str(&format!("{}\t{status}\t{}\n", i + 1, escape(&v.stderr)));
    }
    let target = goldens.join("shapes/bash-n.txt");
    std::fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&target, out).map_err(|e| format!("{}: {e}", target.display()))?;
    println!(
        "recorded bash -n for {} skeletons ({rejected} rejected) into {}",
        skeletons.len(),
        target.display()
    );
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaping() {
        assert_eq!(unescape(r"a\nb\tc\\n"), "a\nb\tc\\n");
        assert_eq!(escape(b"x\\y\nz"), r"x\\y\nz");
    }
}
