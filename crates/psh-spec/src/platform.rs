//! The ratchets of a run, shared and per platform (D1113, D1331).
//!
//! `tests/<layer>/expected-failures.txt`, the shared ratchet, lists psh's own gaps: cases
//! that fail on Linux, where the goldens are recorded. `expected-failures.<platform>.txt`
//! (`macos`, `windows`, `wasm`) lists the cases that fail only on that platform because
//! its programs or OS differ (BSD utilities, a missing `/proc`, no external programs in
//! wasm), each with its reason, and never a case the shared ratchet has. On a platform an
//! active case may fail if either file lists it; Linux has only the shared one.
//!
//! Which file a failure belongs in needs a Linux result: `scripts/spec-linux` records one
//! (`--results`), and `--linux-results` hands it to a run here. With it,
//! `--update-ratchet` moves entries to the file they belong in and keeps shared entries
//! that pass here but fail on Linux; `--add-failures` puts a new case that passes on
//! Linux into the platform file. Without it, both treat this platform like Linux.
//!
//! Cases tagged `[[platform]]` in tests/spec.toml are inherently host-specific: off Linux
//! they run, are reported apart, and are in neither file's checks.
//!
//! In wasm and on Windows (D1914), a case that needed external programs it could not find
//! (the run's notes name the commands it looked for) may fail without being listed: wasm
//! has none, Windows lacks most Unix ones, and listing them would make every native
//! improvement touch the platform file. Their runs never edit the shared ratchet, and a
//! new failure goes to their own file.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use crate::case::Layer;
use crate::groups::Group;
use crate::ratchet::Ratchet;
use crate::report::CaseResult;

pub struct Options {
    pub filtered: bool,
    pub update: bool,
    pub add: bool,
    /// Per case id, whether it passed on Linux (`--linux-results`).
    pub linux: Option<BTreeMap<String, bool>>,
}

/// The platform this run judges: `wasm`, or the OS psh-spec runs on.
pub fn current(wasm: bool) -> &'static str {
    if wasm { "wasm" } else { std::env::consts::OS }
}

pub fn path(layer: Layer, platform: &str) -> String {
    format!("tests/{}/expected-failures.{platform}.txt", layer.name())
}

/// Whether the case is judged here: active, not quarantined, and not host-specific off
/// Linux.
pub fn gated(r: &CaseResult, platform: &str) -> bool {
    r.group == Group::Active
        && r.case.flaky.is_none()
        && (r.case.platform.is_none() || platform == "linux")
}

/// The commands a wasm or Windows run looked for and did not find, by file name.
fn missing_externals(r: &CaseResult) -> Vec<String> {
    let mut seen = Vec::new();
    for m in r.actual.iter().flat_map(|a| a.output.notes.iter()) {
        let name = m.rsplit('/').next().unwrap_or(m).to_string();
        if !seen.contains(&name) {
            seen.push(name);
        }
    }
    seen
}

/// The reason for a new entry of the platform file.
fn reason(r: &CaseResult, platform: &str) -> String {
    if platform == "wasm" || platform == "windows" {
        format!("{platform} only")
    } else {
        format!(
            "fails on {platform} only (passes on Linux): {}",
            r.what_failed()
        )
    }
}

/// The reason of an entry moving from the shared ratchet into the platform's.
fn moved_reason(old: &str, r: &CaseResult, platform: &str) -> String {
    if old.is_empty() {
        reason(r, platform)
    } else if old.contains("passes on Linux") {
        old.to_string()
    } else {
        format!("{old} (passes on Linux)")
    }
}

/// Reads a `--results` file: `id<TAB>pass|fail` lines after a `# psh-spec results:`
/// header that names the platform and commit.
pub fn read_results(
    file: &Path,
    want_platform: &str,
    head: &str,
) -> Result<BTreeMap<String, bool>, String> {
    let text = std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let mut out = BTreeMap::new();
    let mut platform = None;
    for line in text.lines() {
        if let Some(h) = line.strip_prefix("# psh-spec results:") {
            for kv in h.split_whitespace() {
                match kv.split_once('=') {
                    Some(("platform", p)) => platform = Some(p.to_string()),
                    Some(("commit", c)) if !head.is_empty() && c != head => eprintln!(
                        "psh-spec: {} is from commit {c}, this is {head}: rerun scripts/spec-linux if the cases or psh changed",
                        file.display()
                    ),
                    _ => {}
                }
            }
            continue;
        }
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        match line.split_once('\t') {
            Some((id, "pass")) => out.insert(id.to_string(), true),
            Some((id, "fail")) => out.insert(id.to_string(), false),
            _ => return Err(format!("{}: bad line `{line}`", file.display())),
        };
    }
    if platform.as_deref() != Some(want_platform) {
        return Err(format!(
            "{}: not a {want_platform} result (platform {})",
            file.display(),
            platform.as_deref().unwrap_or("unknown")
        ));
    }
    Ok(out)
}

/// The `--results` file of this run: every active, non-quarantined case.
pub fn render_results(results: &[CaseResult], platform: &str, commit: &str) -> String {
    let mut out = format!("# psh-spec results: platform={platform} commit={commit}\n");
    for r in results {
        if r.group == Group::Active && r.case.flaky.is_none() {
            out.push_str(&format!(
                "{}\t{}\n",
                r.case.id,
                if r.pass { "pass" } else { "fail" }
            ));
        }
    }
    out
}

/// What one layer's check found.
#[derive(Default)]
struct Plan {
    /// Failing, listed nowhere: regressions, or new entries with `--add-failures`.
    new_shared: Vec<String>,
    new_platform: Vec<(String, String)>,
    /// Passing here but failing on Linux, and listed nowhere.
    linux_only: Vec<String>,
    stale_shared: Vec<String>,
    stale_platform: Vec<String>,
    /// Failing here, passing on Linux, listed in the shared ratchet.
    to_platform: Vec<String>,
    /// Failing on Linux, listed in the platform's.
    to_shared: Vec<String>,
    /// Shared entries kept because they fail on Linux though they pass here.
    kept_for_linux: usize,
    /// wasm: failures that needed external programs.
    external: usize,
    /// Failing, not in the shared ratchet, not for missing externals.
    other: usize,
}

fn plan(
    layer: Layer,
    platform: &str,
    results: &[CaseResult],
    shared: &Ratchet,
    plat: &Ratchet,
    opts: &Options,
) -> Plan {
    // wasm and Windows: missing externals may fail, the shared file is not theirs.
    let wasm = platform == "wasm" || platform == "windows";
    let linux = |id: &str| opts.linux.as_ref().and_then(|m| m.get(id).copied());
    let mut p = Plan::default();
    let mut ids: HashSet<&str> = HashSet::new();
    for r in results.iter().filter(|r| r.case.layer == layer) {
        if r.group == Group::Active && r.case.flaky.is_none() {
            // Host-specific cases keep their entries off Linux.
            ids.insert(&r.case.id);
        }
        let id = &r.case.id;
        if !gated(r, platform) {
            // A host-specific case is in no platform file (its shared entry is Linux's).
            if r.case.platform.is_some() && plat.entries.contains_key(id) {
                p.stale_platform.push(id.clone());
            }
            continue;
        }
        let (in_s, in_p) = (
            shared.entries.contains_key(id),
            plat.entries.contains_key(id),
        );
        let on_linux = linux(id);
        // Listed twice: the Linux result says which file keeps it (else the shared one).
        if in_s && in_p {
            if on_linux == Some(true) && !wasm {
                p.stale_shared.push(id.clone());
            } else {
                p.stale_platform.push(id.clone());
            }
        }
        if r.pass {
            if in_p && !in_s {
                p.stale_platform.push(id.clone());
            }
            if in_s && !wasm {
                if on_linux == Some(false) {
                    p.kept_for_linux += 1;
                } else {
                    p.stale_shared.push(id.clone());
                }
            }
            if !in_s && on_linux == Some(false) {
                p.linux_only.push(id.clone());
            }
            continue;
        }
        if wasm && !missing_externals(r).is_empty() {
            p.external += 1;
            if in_p && !in_s {
                p.stale_platform.push(id.clone());
            }
            continue;
        }
        p.other += usize::from(!in_s);
        if in_s {
            if !wasm && !in_p && platform != "linux" && on_linux == Some(true) {
                p.to_platform.push(id.clone());
            }
        } else if in_p {
            if on_linux == Some(false) {
                p.to_shared.push(id.clone());
            }
        } else if wasm || (platform != "linux" && on_linux == Some(true)) {
            p.new_platform.push((id.clone(), reason(r, platform)));
        } else {
            p.new_shared.push(id.clone());
        }
    }
    // Entries of cases that are not active (or no longer exist), when the whole layer ran.
    if !opts.filtered {
        p.stale_platform.extend(
            plat.entries
                .keys()
                .filter(|id| !ids.contains(id.as_str()))
                .cloned(),
        );
        if !wasm {
            p.stale_shared.extend(
                shared
                    .entries
                    .keys()
                    .filter(|id| !ids.contains(id.as_str()))
                    .cloned(),
            );
        }
    }
    p.stale_platform.sort();
    p.stale_platform.dedup();
    p.stale_shared.sort();
    p.stale_shared.dedup();
    p
}

/// Checks (and with `update`/`add` edits) the ratchets of the run's layers; returns the
/// regressions.
pub fn check<'a>(
    root: &Path,
    platform: &str,
    layers: &[Layer],
    results: &'a [CaseResult<'a>],
    opts: &Options,
) -> Result<Vec<&'a CaseResult<'a>>, String> {
    let by_id: BTreeMap<&str, &CaseResult> =
        results.iter().map(|r| (r.case.id.as_str(), r)).collect();
    let mut regressions = Vec::new();
    for &layer in layers {
        let shared_file = root.join(layer.ratchet_path());
        let mut shared = Ratchet::parse(&std::fs::read_to_string(&shared_file).unwrap_or_default());
        let plat_file = root.join(path(layer, platform));
        let mut plat = if platform == "linux" {
            Ratchet::default()
        } else {
            Ratchet::parse(&std::fs::read_to_string(&plat_file).unwrap_or_default())
        };
        let p = plan(layer, platform, results, &shared, &plat, opts);
        let (mut shared_changed, mut plat_changed) = (false, false);

        if platform == "wasm" || platform == "windows" {
            println!(
                "{layer} on {platform}: {} failures not in the shared ratchet: {} need external programs, {} other",
                p.external + p.other,
                p.external,
                p.other
            );
        } else if platform != "linux" && !plat.entries.is_empty() {
            println!(
                "{layer}: {} entries in the shared ratchet, {} only for {platform}",
                shared.entries.len(),
                plat.entries.len()
            );
        }
        if p.kept_for_linux > 0 {
            println!(
                "{layer}: {} shared entries pass here but fail on Linux (kept)",
                p.kept_for_linux
            );
        }
        let moves = p.to_platform.len() + p.to_shared.len();
        if moves > 0 {
            if opts.update {
                for id in &p.to_platform {
                    let old = shared.entries.remove(id).unwrap_or_default();
                    plat.entries
                        .insert(id.clone(), moved_reason(&old, by_id[id.as_str()], platform));
                }
                for id in &p.to_shared {
                    let old = plat.entries.remove(id).unwrap_or_default();
                    shared.entries.insert(id.clone(), old);
                }
                (shared_changed, plat_changed) = (true, true);
                println!(
                    "{layer}: moved {} entries to the {platform} ratchet (they pass on Linux), {} to the shared one (they fail there)",
                    p.to_platform.len(),
                    p.to_shared.len()
                );
            } else {
                println!(
                    "{layer}: {} entries belong in the other ratchet file by the Linux results; run with --update-ratchet",
                    moves
                );
            }
        }
        for (stale, ratchet, changed, what) in [
            (
                &p.stale_shared,
                &mut shared,
                &mut shared_changed,
                String::new(),
            ),
            (
                &p.stale_platform,
                &mut plat,
                &mut plat_changed,
                format!(" {platform}"),
            ),
        ] {
            if stale.is_empty() {
                continue;
            }
            if opts.update {
                for id in stale {
                    ratchet.entries.remove(id);
                }
                *changed = true;
                println!(
                    "{layer}: removed {} stale entries from the{what} ratchet",
                    stale.len()
                );
            } else {
                println!(
                    "{layer}: {} {}ratchet entries pass now (or are not active, or are also shared); run with --update-ratchet",
                    stale.len(),
                    if what.is_empty() {
                        String::new()
                    } else {
                        format!("{platform} ")
                    }
                );
                for id in stale.iter().take(10) {
                    println!("  {id}");
                }
            }
        }
        if opts.add {
            for id in &p.new_shared {
                shared.entries.insert(id.clone(), String::new());
            }
            for (id, why) in &p.new_platform {
                plat.entries.insert(id.clone(), why.clone());
            }
            if !p.new_shared.is_empty() {
                shared_changed = true;
                println!(
                    "{layer}: added {} failing cases to the ratchet",
                    p.new_shared.len()
                );
            }
            if !p.new_platform.is_empty() {
                plat_changed = true;
                println!(
                    "{layer}: added {} failing cases to the {platform} ratchet",
                    p.new_platform.len()
                );
            }
        } else {
            regressions.extend(p.new_shared.iter().map(|id| by_id[id.as_str()]));
            regressions.extend(p.new_platform.iter().map(|(id, _)| by_id[id.as_str()]));
        }
        if !p.linux_only.is_empty() {
            println!(
                "{layer}: {} cases fail on Linux but are in no ratchet (they pass here):",
                p.linux_only.len()
            );
            for id in p.linux_only.iter().take(20) {
                println!("  {id}");
            }
            regressions.extend(p.linux_only.iter().map(|id| by_id[id.as_str()]));
        }
        if shared_changed {
            if shared.header.is_empty() {
                shared.header = vec![
                    format!("# Ratchet for layer {layer}: active cases that may fail (TESTING.md \"Ratchets\")."),
                    "# One case id per line, optionally `id: reason`. psh-spec --update-ratchet removes".into(),
                    "# cases that pass; never add a line to hide a regression.".into(),
                ];
            }
            std::fs::write(&shared_file, shared.render())
                .map_err(|e| format!("{}: {e}", shared_file.display()))?;
        }
        if plat_changed {
            if plat.header.is_empty() {
                plat.header = vec![
                    format!(
                        "# Ratchet for layer {layer} on {platform} (D1113, D1331): active cases that may fail here"
                    ),
                    "# but pass on Linux (they are not in expected-failures.txt), each with its reason."
                        .into(),
                    "# psh-spec --update-ratchet removes cases that pass; never add a regression."
                        .into(),
                ];
            }
            if plat.entries.is_empty() {
                let _ = std::fs::remove_file(&plat_file);
            } else {
                std::fs::write(&plat_file, plat.render())
                    .map_err(|e| format!("{}: {e}", plat_file.display()))?;
            }
        }
    }
    Ok(regressions)
}

/// For `scripts/status`: per layer, the failing active cases by the file that lists them,
/// and the host-specific cases.
pub fn status_section(
    root: &Path,
    platform: &str,
    layers: &[Layer],
    results: &[CaseResult],
) -> String {
    let mut out = format!(
        "\n## Failures by ratchet ({platform})\n\n| Layer | Failing | Shared | Only {platform} | Unlisted | Host-specific (pass) |\n| --- | ---: | ---: | ---: | ---: | ---: |\n"
    );
    for &layer in layers {
        let shared = Ratchet::parse(
            &std::fs::read_to_string(root.join(layer.ratchet_path())).unwrap_or_default(),
        );
        let plat = Ratchet::parse(
            &std::fs::read_to_string(root.join(path(layer, platform))).unwrap_or_default(),
        );
        let (mut failing, mut in_s, mut in_p, mut host, mut host_pass) = (0, 0, 0, 0, 0);
        for r in results.iter().filter(|r| r.case.layer == layer) {
            if r.group == Group::Active && r.case.flaky.is_none() && !gated(r, platform) {
                host += 1;
                host_pass += usize::from(r.pass);
                continue;
            }
            if !gated(r, platform) || r.pass {
                continue;
            }
            failing += 1;
            if shared.entries.contains_key(&r.case.id) {
                in_s += 1;
            } else if plat.entries.contains_key(&r.case.id) {
                in_p += 1;
            }
        }
        out.push_str(&format!(
            "| {layer} | {failing} | {in_s} | {in_p} | {} | {host} ({host_pass}) |\n",
            failing - in_s - in_p
        ));
    }
    out
}

/// The host-specific cases of this run when they are not gated, so they stay visible.
pub fn host_specific(results: &[CaseResult], platform: &str) -> String {
    if platform == "linux" {
        return String::new();
    }
    let cases: Vec<&CaseResult> = results
        .iter()
        .filter(|r| r.group == Group::Active && r.case.flaky.is_none() && r.case.platform.is_some())
        .collect();
    if cases.is_empty() {
        return String::new();
    }
    let passed = cases.iter().filter(|r| r.pass).count();
    let mut out = format!(
        "\nhost-specific (gated on Linux only): {} cases, {passed} pass on {platform}\n",
        cases.len()
    );
    for r in cases {
        out.push_str(&format!(
            "  {} {}: {}\n",
            if r.pass { "pass" } else { "FAIL" },
            r.case.id,
            r.case.platform.as_deref().unwrap_or("")
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::case::Case;

    fn cases(n: usize) -> Vec<Case> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut c = crate::case::load_layer(&root, Layer::Agent).unwrap();
        c.truncate(n);
        c
    }

    fn result(case: &Case, pass: bool) -> CaseResult<'_> {
        CaseResult {
            case,
            group: Group::Active,
            stdout_ok: pass,
            status_ok: true,
            files_ok: true,
            stderr_ok: true,
            leak_ok: true,
            pass,
            error: None,
            expected: None,
            actual: None,
        }
    }

    fn ratchet(ids: &[&Case]) -> Ratchet {
        let mut r = Ratchet::default();
        for c in ids {
            r.entries.insert(c.id.clone(), String::new());
        }
        r
    }

    fn opts(linux: &[(&Case, bool)]) -> Options {
        Options {
            filtered: false,
            update: false,
            add: false,
            linux: (!linux.is_empty())
                .then(|| linux.iter().map(|(c, p)| (c.id.clone(), *p)).collect()),
        }
    }

    #[test]
    fn without_linux_results_this_platform_is_linux() {
        let c = cases(4);
        let results = [
            result(&c[0], false),
            result(&c[1], false),
            result(&c[2], true),
            result(&c[3], true),
        ];
        let (shared, plat) = (ratchet(&[&c[2]]), ratchet(&[&c[1], &c[3]]));
        let p = plan(Layer::Agent, "macos", &results, &shared, &plat, &opts(&[]));
        assert_eq!(p.new_shared, [c[0].id.clone()]);
        assert!(p.new_platform.is_empty() && p.to_platform.is_empty());
        assert_eq!(p.stale_shared, [c[2].id.clone()]);
        assert_eq!(p.stale_platform, [c[3].id.clone()]);
    }

    #[test]
    fn linux_results_decide_the_file() {
        let c = cases(5);
        let results = [
            result(&c[0], false), // shared, passes on Linux: to macos
            result(&c[1], false), // macos, fails on Linux: to shared
            result(&c[2], true),  // shared, fails on Linux: kept
            result(&c[3], true),  // listed nowhere, fails on Linux
            result(&c[4], false), // new, passes on Linux: macos
        ];
        let (shared, plat) = (ratchet(&[&c[0], &c[2]]), ratchet(&[&c[1]]));
        let linux = [
            (&c[0], true),
            (&c[1], false),
            (&c[2], false),
            (&c[3], false),
            (&c[4], true),
        ];
        let p = plan(
            Layer::Agent,
            "macos",
            &results,
            &shared,
            &plat,
            &opts(&linux),
        );
        assert_eq!(p.to_platform, [c[0].id.clone()]);
        assert_eq!(p.to_shared, [c[1].id.clone()]);
        assert_eq!(p.kept_for_linux, 1);
        assert!(p.stale_shared.is_empty());
        assert_eq!(p.linux_only, [c[3].id.clone()]);
        assert_eq!(p.new_platform.len(), 1);
        assert_eq!(p.new_platform[0].0, c[4].id);
        assert!(p.new_shared.is_empty());
    }

    #[test]
    fn host_specific_cases_are_gated_on_linux_only() {
        let mut c = cases(1);
        c[0].platform = Some("/dev/full".into());
        let results = [result(&c[0], false)];
        let (shared, plat) = (ratchet(&[&c[0]]), ratchet(&[&c[0]]));
        let p = plan(
            Layer::Agent,
            "macos",
            &results,
            &shared,
            &Ratchet::default(),
            &opts(&[]),
        );
        assert!(p.new_shared.is_empty() && p.stale_shared.is_empty());
        let p = plan(
            Layer::Agent,
            "macos",
            &results,
            &Ratchet::default(),
            &plat,
            &opts(&[]),
        );
        assert_eq!(p.stale_platform, [c[0].id.clone()]);
        let p = plan(
            Layer::Agent,
            "linux",
            &results,
            &Ratchet::default(),
            &Ratchet::default(),
            &opts(&[]),
        );
        assert_eq!(p.new_shared, [c[0].id.clone()]);
    }

    #[test]
    fn results_round_trip() {
        let c = cases(2);
        let results = [result(&c[0], true), result(&c[1], false)];
        let text = render_results(&results, "linux", "abc1234");
        let dir = std::env::temp_dir().join(format!("psh-spec-results-{}", std::process::id()));
        std::fs::write(&dir, &text).unwrap();
        let m = read_results(&dir, "linux", "abc1234").unwrap();
        assert!(m[&c[0].id]);
        assert!(!m[&c[1].id]);
        assert!(read_results(&dir, "macos", "").is_err());
        let _ = std::fs::remove_file(&dir);
    }
}
