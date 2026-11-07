//! Platform ratchets (D1113): `tests/<layer>/expected-failures.<platform>.txt` next to the
//! shared ratchet. On a platform, an active case may fail if either file lists it; the
//! platform file lists only cases the shared ratchet does not (they pass natively), with
//! the reason. The first platform is `wasm` (`psh-spec --wasm`). There, a case that
//! needed external programs (the run's notes name the commands it looked for) may fail
//! without being listed: no external program exists in wasm, and listing them would make
//! every native improvement touch the wasm file. What is listed are the other
//! differences, which should be few.

use std::path::Path;

use crate::case::Layer;
use crate::groups::Group;
use crate::ratchet::Ratchet;
use crate::report::CaseResult;

pub struct Options {
    pub filtered: bool,
    pub update: bool,
    pub add: bool,
}

pub fn path(layer: Layer, platform: &str) -> String {
    format!("tests/{}/expected-failures.{platform}.txt", layer.name())
}

/// The reason a case fails on this platform only.
fn reason(r: &CaseResult) -> String {
    let mut missing: Vec<String> = r
        .actual
        .as_ref()
        .map(|a| a.output.notes.clone())
        .unwrap_or_default();
    missing.dedup();
    let mut seen = Vec::new();
    for m in missing {
        let name = m.rsplit('/').next().unwrap_or(&m).to_string();
        if !seen.contains(&name) {
            seen.push(name);
        }
    }
    if seen.is_empty() {
        "wasm only".to_string()
    } else {
        format!("needs external: {}", seen.join(", "))
    }
}

/// Checks (or updates) the platform ratchets; returns the regressions.
pub fn check<'a>(
    root: &Path,
    platform: &str,
    layers: &[Layer],
    results: &'a [CaseResult<'a>],
    opts: &Options,
) -> Result<Vec<&'a CaseResult<'a>>, String> {
    let mut regressions = Vec::new();
    for &layer in layers {
        let shared = Ratchet::parse(
            &std::fs::read_to_string(root.join(layer.ratchet_path())).unwrap_or_default(),
        );
        let file = root.join(path(layer, platform));
        let mut plat = Ratchet::parse(&std::fs::read_to_string(&file).unwrap_or_default());
        let active: Vec<&CaseResult> = results
            .iter()
            .filter(|r| r.case.layer == layer && r.group == Group::Active)
            .collect();
        let mut stale = Vec::new();
        let mut new = Vec::new();
        let (mut only_platform, mut external) = (0, 0);
        for r in &active {
            let in_shared = shared.entries.contains_key(&r.case.id);
            let in_plat = plat.entries.contains_key(&r.case.id);
            let why = reason(r);
            let needs_external = why.starts_with("needs external");
            if in_plat && (r.pass || in_shared || needs_external) {
                stale.push(r.case.id.clone());
            }
            if !r.pass && !in_shared {
                if needs_external {
                    external += 1;
                } else {
                    only_platform += 1;
                    if !in_plat {
                        new.push((*r, why));
                    }
                }
            }
        }
        if !opts.filtered {
            let ids: std::collections::HashSet<&str> =
                active.iter().map(|r| r.case.id.as_str()).collect();
            stale.extend(
                plat.entries
                    .keys()
                    .filter(|id| !ids.contains(id.as_str()))
                    .cloned(),
            );
        }
        println!(
            "{layer} on {platform}: {} cases pass natively but not here: {external} need external programs, {only_platform} other",
            external + only_platform
        );
        let mut changed = false;
        if !stale.is_empty() {
            if opts.update {
                for id in &stale {
                    plat.entries.remove(id);
                }
                changed = true;
                println!(
                    "{layer} on {platform}: removed {} entries from the ratchet",
                    stale.len()
                );
            } else {
                println!(
                    "{layer} on {platform}: {} ratchet entries pass now (or are in the shared ratchet); run with --update-ratchet",
                    stale.len()
                );
            }
        }
        if opts.add && !new.is_empty() {
            for (r, why) in &new {
                plat.entries.insert(r.case.id.clone(), why.clone());
            }
            changed = true;
            println!(
                "{layer} on {platform}: added {} cases to the ratchet",
                new.len()
            );
        } else {
            regressions.extend(new.iter().map(|(r, _)| *r));
        }
        if changed {
            if plat.header.is_empty() {
                plat.header = vec![
                    format!(
                        "# Ratchet for layer {layer} on {platform} (D1113): active cases that pass natively"
                    ),
                    "# (not in expected-failures.txt) but may fail here, each with its reason. Cases"
                        .into(),
                    "# that need external programs are not listed. psh-spec --update-ratchet removes"
                        .into(),
                    "# cases that pass; never add a regression."
                        .into(),
                ];
            }
            if plat.entries.is_empty() {
                let _ = std::fs::remove_file(&file);
            } else {
                std::fs::write(&file, plat.render())
                    .map_err(|e| format!("{}: {e}", file.display()))?;
            }
        }
    }
    Ok(regressions)
}
