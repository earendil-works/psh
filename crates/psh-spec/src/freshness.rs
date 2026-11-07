//! Built artifacts psh-spec runs but does not build itself must not be older than their
//! sources: the psh binary next to psh-spec (cases that run `$SH`, or need a process
//! group of their own, run in it) and, for `--wasm`, `target/wasm/psh.wasm`. A stale one
//! makes old behavior look like regressions, so psh-spec refuses to run and says how to
//! rebuild.
//!
//! The sources are the ones cargo's dep-info file (`psh.d` next to the binary) lists, so
//! a file compiled only on another platform (`psh-os/src/windows/`) does not make the
//! binary look stale; without one, every file of the crates counts.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The crates the psh binary is built from.
pub const PSH_CRATES: &[&str] = &[
    "psh",
    "psh-engine",
    "psh-syntax",
    "psh-os",
    "psh-classify",
    "psh-utils",
];
/// The crates psh.wasm is built from.
pub const WASM_CRATES: &[&str] = &[
    "psh-wasm",
    "psh-engine",
    "psh-syntax",
    "psh-classify",
    "psh-utils",
];
/// Cargo's dep-info file of psh.wasm (scripts/build-wasm's target directory).
pub const WASM_DEP_INFO: &str = "target/wasm/build-psh/wasm32-unknown-unknown/wasm/psh_wasm.d";

/// The files of the workspace a dep-info file lists (`target: a.rs b\ c.rs ...`; cargo
/// escapes spaces in paths with a backslash). Files outside `root` (the registry) are
/// left out: they change only with Cargo.lock.
fn dep_info_sources(root: &Path, dep_info: &Path) -> Option<Vec<PathBuf>> {
    let text = std::fs::read_to_string(dep_info).ok()?;
    let line = text.lines().next()?;
    // The target is the binary's own path; `: ` ends it (a Windows drive letter's `:`
    // is not followed by a space).
    let (_, deps) = line.split_once(": ")?;
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = deps.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&' ') => {
                cur.push(' ');
                chars.next();
            }
            ' ' => {
                if !cur.is_empty() {
                    out.push(PathBuf::from(std::mem::take(&mut cur)));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(PathBuf::from(cur));
    }
    // On Windows the root may be canonical (`\\?\C:\...`) while cargo writes plain
    // paths (`C:\...`): compare without the prefix.
    let plain = |p: &Path| -> PathBuf {
        let s = p.to_string_lossy();
        PathBuf::from(s.strip_prefix(r"\\?\").unwrap_or(&s).to_string())
    };
    let root = plain(root);
    out.retain(|p| plain(p).starts_with(&root));
    (!out.is_empty()).then_some(out)
}

/// The newest source of the artifact: the workspace files its dep-info lists, or
/// without one the crates' manifests and sources and the workspace manifest and lock
/// file.
fn newest_source(root: &Path, dep_info: &Path, crates: &[&str]) -> Option<(SystemTime, PathBuf)> {
    let mut newest: Option<(SystemTime, PathBuf)> = None;
    let mut consider = |p: &Path| {
        if let Ok(t) = std::fs::metadata(p).and_then(|m| m.modified())
            && newest.as_ref().is_none_or(|(n, _)| t > *n)
        {
            newest = Some((t, p.to_path_buf()));
        }
    };
    // With dep-info, exactly the files the build read: no manifests or lock file. An
    // edit cargo does not rebuild for (a test target in a crate's Cargo.toml, another
    // crate's dependency in Cargo.lock) must not make the binary look stale when the
    // advice, `cargo build`, cannot change it; an edit that matters rebuilds it.
    if let Some(files) = dep_info_sources(root, dep_info) {
        for f in files {
            consider(&f);
        }
        return newest;
    }
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

/// Fails when `artifact` is missing or older than the newest of its sources (those
/// `dep_info` lists, else every file of `crates`).
pub fn check(
    root: &Path,
    artifact: &Path,
    dep_info: &Path,
    crates: &[&str],
    fix: &str,
) -> Result<(), String> {
    let built = std::fs::metadata(artifact)
        .and_then(|m| m.modified())
        .map_err(|_| format!("{} is missing; build it: {fix}", artifact.display()))?;
    if let Some((src, path)) = newest_source(root, dep_info, crates)
        && src > built
    {
        let rel = path.strip_prefix(root).unwrap_or(&path);
        // `/`-separated on every platform, like the rest of psh-spec's messages.
        let rel: Vec<String> = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        return Err(format!(
            "{} is older than its sources ({} changed since); results would show old behavior.\nRebuild it: {fix}",
            artifact.display(),
            rel.join("/")
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
        let no_deps = dir.join("bin.d");
        let err = check(&dir, &artifact, &no_deps, &["a"], "make it").unwrap_err();
        assert!(
            err.contains("crates/a/src/lib.rs") && err.contains("make it"),
            "{err}"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&artifact, b"new").unwrap();
        assert!(check(&dir, &artifact, &no_deps, &["a"], "make it").is_ok());
        // With dep-info, only the files it lists count: a newer file of another
        // platform does not make the artifact stale.
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(dir.join("crates/a/src/other os.rs"), b"new").unwrap();
        assert!(check(&dir, &artifact, &no_deps, &["a"], "make it").is_err());
        let lib = dir.join("crates/a/src/lib.rs");
        std::fs::write(
            &no_deps,
            format!(
                "{}: {} /elsewhere/x.rs\n",
                artifact.display(),
                lib.display()
            ),
        )
        .unwrap();
        assert!(check(&dir, &artifact, &no_deps, &["a"], "make it").is_ok());
        // Nor a manifest edit cargo rebuilds nothing for.
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(dir.join("crates/a/Cargo.toml"), b"[package]").unwrap();
        std::fs::write(dir.join("Cargo.lock"), b"").unwrap();
        assert!(check(&dir, &artifact, &no_deps, &["a"], "make it").is_ok());
        std::fs::write(
            &no_deps,
            format!(
                "{}: {} {}\n",
                artifact.display(),
                lib.display(),
                dir.join("crates/a/src/other\\ os.rs").display()
            ),
        )
        .unwrap();
        assert!(check(&dir, &artifact, &no_deps, &["a"], "make it").is_err());
        assert!(
            check(&dir, &dir.join("missing"), &no_deps, &["a"], "x")
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
