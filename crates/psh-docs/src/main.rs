//! `psh-docs`: the guide in docs/guide.
//!
//! ```text
//! psh-docs check               the writing rules, live examples' output unchanged, every
//!                              page renders
//! psh-docs regen [--examples]  write missing example output into the pages; --examples
//!                              also rewrites output that changed
//! psh-docs build [--out DIR] [--wasm FILE]
//!                              the static site (default target/docs-site), with live
//!                              examples on psh.wasm (default target/wasm/psh.wasm)
//! ```
//!
//! The guide's pages are listed in docs/guide/SUMMARY.md (mdBook's format); index.md is
//! the site's front page.

mod highlight;
#[path = "../../psh/src/json.rs"]
#[allow(dead_code)]
mod json;
#[path = "../../psh/src/parse_json.rs"]
mod parse_json;
mod regions;
mod render;
mod rules;
mod run;
mod search;
mod site;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: psh-docs check | regen [--examples] | build [--out DIR] [--wasm FILE]";

/// The repository root: two levels above this crate (no `canonicalize`, which gives
/// Windows paths a `\\?\` prefix other programs do not take).
pub fn repo() -> PathBuf {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(Path::parent)
        .unwrap_or(crate_dir)
        .to_path_buf()
}

/// The guide after regeneration (in memory).
pub struct Guide {
    pub summary: Vec<site::Entry>,
    /// Page -> regenerated source.
    pub pages: BTreeMap<String, String>,
    pub runs: regions::Runs,
    /// Pages whose regions are missing (`regen` writes them).
    pub stale: Vec<String>,
    /// Pages with an example whose output changed (`regen --examples` accepts it).
    pub changed: Vec<String>,
    pub errors: Vec<String>,
}

fn load(accept: bool) -> Guide {
    let dir = repo().join("docs/guide");
    let mut g = Guide {
        summary: Vec::new(),
        pages: BTreeMap::new(),
        runs: regions::Runs::default(),
        stale: Vec::new(),
        changed: Vec::new(),
        errors: Vec::new(),
    };
    let summary = match std::fs::read_to_string(dir.join("SUMMARY.md")) {
        Ok(s) => s,
        Err(e) => {
            g.errors.push(format!("docs/guide/SUMMARY.md: {e}"));
            return g;
        }
    };
    g.summary = site::summary(&summary);
    let names: Vec<String> = g
        .summary
        .iter()
        .filter_map(|e| e.path.clone())
        .chain([site::LANDING.to_string()])
        .collect();
    for p in names {
        // A missing page is reported by the writing rules.
        let Ok(old) = std::fs::read_to_string(dir.join(&p)) else {
            continue;
        };
        let r = regions::regen(&p, &old, &mut g.runs, accept);
        g.errors.extend(r.errors);
        if r.text != old {
            g.stale.push(p.clone());
        }
        if r.changed > 0 && !accept {
            g.changed.push(p.clone());
        }
        g.pages.insert(p, r.text);
    }
    g
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let flag = |name: &str| args.iter().any(|a| a == name);
    let opt = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    match cmd.as_str() {
        "check" => {
            let (n, rule_errors) = rules::check_guide(&repo(), &[site::LANDING]);
            let mut g = load(false);
            let _ = site::render_all(&mut g);
            let mut bad = false;
            for e in &rule_errors {
                eprintln!("psh-docs: {e}");
                bad = true;
            }
            if !rule_errors.is_empty() {
                eprintln!("  the writing rules are in docs/guide/README.md");
            }
            for s in &g.stale {
                eprintln!("psh-docs: an example has no output region: docs/guide/{s}");
                bad = true;
            }
            if !g.stale.is_empty() {
                eprintln!("  run `cargo run -p psh-docs -- regen`");
            }
            for p in &g.changed {
                eprintln!("psh-docs: an example's output changed: docs/guide/{p}");
                bad = true;
            }
            if !g.changed.is_empty() {
                eprintln!(
                    "  if the new output is right, accept it with `cargo run -p psh-docs -- regen --examples`\n  \
                     and check that the page's text still says the right thing"
                );
            }
            for e in &g.errors {
                eprintln!("psh-docs: {e}");
                bad = true;
            }
            if bad {
                return ExitCode::FAILURE;
            }
            println!("psh-docs: {n} pages ok");
        }
        "regen" => {
            let g = load(flag("--examples"));
            let dir = repo().join("docs/guide");
            for p in &g.stale {
                std::fs::write(dir.join(p), &g.pages[p]).expect("write");
                println!("psh-docs: rewrote docs/guide/{p}");
            }
            for p in &g.changed {
                println!(
                    "psh-docs: kept the old example output in docs/guide/{p} (accept with --examples)"
                );
            }
            if !g.errors.is_empty() {
                for e in &g.errors {
                    eprintln!("psh-docs: {e}");
                }
                return ExitCode::FAILURE;
            }
        }
        "build" => {
            let out = opt("--out")
                .map(PathBuf::from)
                .unwrap_or_else(|| repo().join("target/docs-site"));
            let wasm = opt("--wasm")
                .map(PathBuf::from)
                .unwrap_or_else(|| repo().join("target/wasm/psh.wasm"));
            let mut g = load(false);
            if let Err(e) = site::build(&mut g, &out, &wasm) {
                eprintln!("psh-docs: {e}");
                return ExitCode::FAILURE;
            }
            for e in &g.errors {
                eprintln!("psh-docs: {e}");
            }
            if !g.errors.is_empty() {
                return ExitCode::FAILURE;
            }
        }
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}
