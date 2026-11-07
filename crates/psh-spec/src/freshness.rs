//! Built artifacts psh-spec runs but does not build itself must not be older than their
//! sources: the psh binary next to psh-spec (cases that run `$SH`, or need a process
//! group of their own, run in it) and, for `--wasm`, `target/wasm/psh.wasm`. A stale one
//! makes old behavior look like regressions, so psh-spec refuses to run and says how to
//! rebuild.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The crates the psh binary is built from.
pub const PSH_CRATES: &[&str] = &["psh", "psh-engine", "psh-syntax", "psh-os", "psh-classify"];
/// The crates psh.wasm is built from.
pub const WASM_CRATES: &[&str] = &["psh-wasm", "psh-engine", "psh-syntax", "psh-classify"];

/// The newest file among the crates' manifests and sources, and the workspace manifest
/// and lock file.
fn newest_source(root: &Path, crates: &[&str]) -> Option<(SystemTime, PathBuf)> {
    let mut newest: Option<(SystemTime, PathBuf)> = None;
    let mut consider = |p: &Path| {
        if let Ok(t) = std::fs::metadata(p).and_then(|m| m.modified())
            && newest.as_ref().is_none_or(|(n, _)| t > *n)
        {
            newest = Some((t, p.to_path_buf()));
        }
    };
    consider(&root.join("Cargo.toml"));
    consider(&root.join("Cargo.lock"));
    for c in crates {
        let dir = root.join("crates").join(c);
        consider(&dir.join("Cargo.toml"));
        consider(&dir.join("build.rs"));
        walk(&dir.join("src"), &mut consider);
    }
    newest
}

fn walk(dir: &Path, f: &mut impl FnMut(&Path)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, f);
        } else {
            f(&p);
        }
    }
}

/// Fails when `artifact` is missing or older than the newest of its sources.
pub fn check(root: &Path, artifact: &Path, crates: &[&str], fix: &str) -> Result<(), String> {
    let built = std::fs::metadata(artifact)
        .and_then(|m| m.modified())
        .map_err(|_| format!("{} is missing; build it: {fix}", artifact.display()))?;
    if let Some((src, path)) = newest_source(root, crates)
        && src > built
    {
        let rel = path.strip_prefix(root).unwrap_or(&path);
        return Err(format!(
            "{} is older than its sources ({} changed since); results would show old behavior.\nRebuild it: {fix}",
            artifact.display(),
            rel.display()
        ));
    }
    Ok(())
}

/// The cargo command that builds the psh binary next to `exe` (psh-spec itself).
pub fn psh_fix(exe: &Path) -> String {
    let profile = exe
        .parent()
        .and_then(Path::file_name)
        .and_then(|n| n.to_str())
        .unwrap_or("debug");
    match profile {
        "debug" => "cargo build -p psh".to_string(),
        "release" => "cargo build --release -p psh".to_string(),
        other => format!("cargo build --profile {other} -p psh"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_and_fresh() {
        let dir = std::env::temp_dir().join(format!("psh-spec-fresh-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("crates/a/src")).unwrap();
        let artifact = dir.join("bin");
        std::fs::write(&artifact, b"old").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(dir.join("crates/a/src/lib.rs"), b"new").unwrap();
        let err = check(&dir, &artifact, &["a"], "make it").unwrap_err();
        assert!(
            err.contains("crates/a/src/lib.rs") && err.contains("make it"),
            "{err}"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&artifact, b"new").unwrap();
        assert!(check(&dir, &artifact, &["a"], "make it").is_ok());
        assert!(
            check(&dir, &dir.join("missing"), &["a"], "x")
                .unwrap_err()
                .contains("missing")
        );
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            psh_fix(Path::new("/t/release/psh-spec")),
            "cargo build --release -p psh"
        );
    }
}
