//! psh-spec: the conformance runner (TESTING.md "The runner").
//!
//! It runs the cases of three layers (Oils spec files, brush's YAML cases, our agent
//! cases) against goldens recorded from bash 5.3, keeps a ratchet of expected failures
//! per layer, and records the goldens itself (`--record`, through
//! `scripts/record-goldens`). No dependencies.

mod ast_features;
mod case;
mod engine;
mod freshness;
mod golden;
mod groups;
mod helpers;
mod json;
mod l0;
mod normalize;
mod oils;
mod platform;
mod ratchet;
mod report;
mod runner;
mod sandbox;
mod status;
mod wasm;
mod yaml;

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use case::{Case, Layer};
use engine::{Engine, External, InProcess};
use golden::Golden;
use groups::{Group, Groups};
use ratchet::Ratchet;
use runner::{Observed, RunDir, Runner};

const USAGE: &str = "\
usage: psh-spec [OPTIONS] [FILTER...]

Runs conformance cases against their bash 5.3 goldens (TESTING.md).
FILTERs select cases: a case id prefix (oils/alias, oils/alias/3, brush/compat) or a
glob on case ids (oils/var-*/*). Without filters, every case of the selected layers.

Engines:
  (default)           psh in-process
  --wasm              psh.wasm under node through js/psh.mjs (scripts/build-wasm first);
                      checked against the shared ratchets plus
                      tests/<layer>/expected-failures.wasm.txt
  --shell PATH        a shell binary as a subprocess (e.g. bash for the self-check);
                      the ratchets apply only when it is psh

Modes:
  --record            record goldens with --shell (scripts/record-goldens does this)
  --bsd               with --record: the BSD goldens of L4 (macOS, scripts/record-goldens --bsd)
  --list              list the selected cases with their group
  --status            the scoreboard per layer and area (scripts/status); appends a
                      line per layer to tests/scores/status.tsv (unless --no-append,
                      filters or another shell)
  --hitlist           with --status: failing cases ranked by corpus weight (-n N rows)
  --diff-goldens A B  compare two golden directories per case (determinism check)
  --record-l0         record `bash -n` verdicts with --shell for the shape corpus and
                      tests/l0/syntax.txt (L0) into <goldens>/{shapes,l0}/bash-n.txt
  --l0                L0: psh-syntax against bash -n on every case script, shape and
                      snippet, plus print/reparse; ratchet tests/l0/expected-failures.txt
  (default)           compare against the goldens and check the ratchets

Options:
  --layer L           oils, brush or agent (repeatable; default all)
  --goldens DIR       golden directory (default tests/goldens)
  --root DIR          repository root (default: found from the current directory)
  -j N                parallel cases (default 6)
  --timeout SECS      per-case timeout (default: 10 s Oils/agent, 15 s brush)
  --diff              show what differs for failing cases
  --all-groups        also run `skip` cases (they never count)
  --update-ratchet    remove ratchet entries of cases that pass
  --add-failures      add failing active cases missing from the ratchet (new cases only)
  --summary FILE      write a Markdown summary per layer and file
  --keep              keep the run directory (printed) for inspection
";

struct Options {
    root: Option<PathBuf>,
    layers: Vec<Layer>,
    filters: Vec<String>,
    shell: Option<PathBuf>,
    record: bool,
    list: bool,
    goldens: Option<PathBuf>,
    jobs: usize,
    timeout: Option<Duration>,
    diff: bool,
    all_groups: bool,
    update_ratchet: bool,
    add_failures: bool,
    summary: Option<PathBuf>,
    keep: bool,
    diff_goldens: Option<(PathBuf, PathBuf)>,
    record_shapes: bool,
    wasm: bool,
    bsd: bool,
    l0: bool,
    status: bool,
    hitlist: bool,
    rows: usize,
    no_append: bool,
}

fn parse_args(args: Vec<String>) -> Result<Options, String> {
    let mut o = Options {
        root: None,
        layers: Vec::new(),
        filters: Vec::new(),
        shell: None,
        record: false,
        list: false,
        goldens: None,
        jobs: 6,
        timeout: None,
        diff: false,
        all_groups: false,
        update_ratchet: false,
        add_failures: false,
        summary: None,
        keep: false,
        diff_goldens: None,
        record_shapes: false,
        wasm: false,
        bsd: false,
        l0: false,
        status: false,
        hitlist: false,
        rows: 40,
        no_append: false,
    };
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        let mut value = |name: &str| it.next().ok_or(format!("{name} needs a value"));
        match a.as_str() {
            "-h" | "--help" => return Err(String::new()),
            "--root" => o.root = Some(value("--root")?.into()),
            "--layer" => {
                let v = value("--layer")?;
                o.layers
                    .push(Layer::parse(&v).ok_or(format!("unknown layer `{v}`"))?);
            }
            "--shell" => o.shell = Some(value("--shell")?.into()),
            "--record" => o.record = true,
            "--list" => o.list = true,
            "--goldens" => o.goldens = Some(value("--goldens")?.into()),
            "-j" | "--jobs" => {
                o.jobs = value("-j")?.parse().map_err(|_| "-j needs a number")?;
            }
            "--timeout" => {
                let secs: f64 = value("--timeout")?
                    .parse()
                    .map_err(|_| "--timeout needs seconds")?;
                o.timeout = Some(Duration::from_secs_f64(secs));
            }
            "--diff" => o.diff = true,
            "--all-groups" => o.all_groups = true,
            "--update-ratchet" => o.update_ratchet = true,
            "--add-failures" => o.add_failures = true,
            "--summary" => o.summary = Some(value("--summary")?.into()),
            "--keep" => o.keep = true,
            "--record-l0" => o.record_shapes = true,
            "--l0" => o.l0 = true,
            "--wasm" => o.wasm = true,
            "--bsd" => o.bsd = true,
            "--status" => o.status = true,
            "--hitlist" => {
                o.status = true;
                o.hitlist = true;
            }
            "--no-append" => o.no_append = true,
            "-n" => o.rows = value("-n")?.parse().map_err(|_| "-n needs a number")?,
            "--diff-goldens" => {
                let a = value("--diff-goldens")?;
                let b = it.next().ok_or("--diff-goldens needs two directories")?;
                o.diff_goldens = Some((a.into(), b.into()));
            }
            s if s.starts_with('-') && s.len() > 1 => return Err(format!("unknown option {s}")),
            _ => o.filters.push(a),
        }
    }
    if o.bsd {
        // Only L4 has BSD goldens.
        o.layers = vec![Layer::Utils];
    }
    if o.layers.is_empty() {
        o.layers = Layer::ALL.to_vec();
    }
    if (o.record || o.record_shapes) && o.shell.is_none() {
        return Err("--record and --record-l0 need --shell PATH".into());
    }
    Ok(o)
}

fn find_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join("tests/spec.toml").is_file() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn selected(case: &Case, filters: &[String]) -> bool {
    filters.is_empty()
        || filters.iter().any(|f| {
            if f.contains('*') || f.contains('?') {
                groups::glob(f, &case.id)
            } else {
                case.id == *f
                    || case
                        .id
                        .starts_with(&format!("{}/", f.trim_end_matches('/')))
            }
        })
}

fn main() -> ExitCode {
    if let Some(argv0) = std::env::args_os().next()
        && let Some(status) = helpers::dispatch(&argv0, &helpers::args_bytes())
    {
        return ExitCode::from(status.clamp(0, 255) as u8);
    }
    let opts = match parse_args(std::env::args().skip(1).collect()) {
        Ok(o) => o,
        Err(e) if e.is_empty() => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            eprintln!("psh-spec: {e}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(opts) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("psh-spec: {e}");
            ExitCode::from(2)
        }
    }
}

/// Everything a run needs to know about one case.
pub struct Item<'a> {
    pub case: &'a Case,
    pub group: Group,
    pub reason: String,
}

fn golden_path(goldens: &Path, layer: Layer, file: &str) -> PathBuf {
    goldens.join(layer.name()).join(format!("{file}.golden"))
}

/// The BSD recordings of L4 (macOS, `--record --bsd`), next to the GNU ones.
const BSD_SET: &str = "utils-bsd";

fn bsd_golden_path(goldens: &Path, file: &str) -> PathBuf {
    goldens.join(BSD_SET).join(format!("{file}.golden"))
}

fn load_goldens(path: &Path) -> Result<Vec<Golden>, String> {
    std::fs::read_to_string(path)
        .map_err(|e| format!("{}: {e}", path.display()))
        .and_then(|t| golden::parse_file(&t).map_err(|e| format!("{}: {e}", path.display())))
}

/// The golden of a case in a loaded set, checked against the case's name.
fn golden_of<'g>(set: &'g Result<Vec<Golden>, String>, case: &Case) -> Result<&'g Golden, String> {
    let gs = set.as_ref().map_err(Clone::clone)?;
    let g = gs
        .iter()
        .find(|g| g.index == case.index)
        .ok_or_else(|| "no golden for this case".to_string())?;
    if g.name == case.name.replace('\n', " ") {
        Ok(g)
    } else {
        Err(format!("stale golden (recorded as {:?})", g.name))
    }
}

fn run(opts: Options) -> Result<ExitCode, String> {
    if let Some((a, b)) = &opts.diff_goldens {
        return diff_goldens(a, b);
    }
    let root = match &opts.root {
        Some(r) => r.clone(),
        None => find_root().ok_or("no tests/spec.toml above the current directory; use --root")?,
    };
    let goldens_dir = opts
        .goldens
        .clone()
        .unwrap_or_else(|| root.join("tests/goldens"));
    if opts.record_shapes {
        let shell = resolve_shell(opts.shell.as_ref().unwrap())?;
        return l0::record(&root, &goldens_dir, &External::new(shell), opts.jobs);
    }
    if opts.l0 {
        return l0::main(
            &root,
            &opts.filters,
            opts.jobs,
            opts.diff,
            opts.update_ratchet,
            opts.add_failures,
        );
    }
    let groups_text = std::fs::read_to_string(root.join("tests/spec.toml"))
        .map_err(|e| format!("tests/spec.toml: {e}"))?;
    let groups = Groups::parse(&groups_text)?;

    let mut cases = Vec::new();
    for &layer in &opts.layers {
        cases.extend(case::load_layer(&root, layer)?);
    }
    cases.retain(|c| selected(c, &opts.filters));
    for c in &mut cases {
        c.serial = groups.is_serial(c);
    }
    let items: Vec<Item> = cases
        .iter()
        .map(|c| {
            let rule = groups.rule_for(c);
            Item {
                case: c,
                group: rule.map_or(Group::Active, |r| r.group),
                reason: rule.map(|r| r.reason.clone()).unwrap_or_default(),
            }
        })
        .collect();

    if opts.list {
        for it in &items {
            println!("{}\t{}\t{}", it.case.id, it.group.name(), it.case.name);
        }
        return Ok(ExitCode::SUCCESS);
    }

    let engine: Box<dyn Engine> = match &opts.shell {
        Some(path) => Box::new(External::new(resolve_shell(path)?)),
        None => {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let psh = exe.with_file_name(format!("psh{}", std::env::consts::EXE_SUFFIX));
            // Cases that run `$SH` (or must not share the runner's process group) run in
            // this binary: a stale one shows old behavior as regressions.
            freshness::check(
                &root,
                &psh,
                freshness::PSH_CRATES,
                &freshness::psh_fix(&exe),
            )?;
            if opts.wasm {
                freshness::check(
                    &root,
                    &root.join("target/wasm/psh.wasm"),
                    freshness::WASM_CRATES,
                    "scripts/build-wasm",
                )?;
                Box::new(wasm::WasmEngine::new(&root, psh)?)
            } else {
                Box::new(InProcess::new(psh))
            }
        }
    };
    let dir = RunDir::create(&root, opts.keep)?;
    if opts.keep {
        eprintln!("psh-spec: run directory {}", dir.root.display());
    }
    let runner = Runner {
        engine: engine.as_ref(),
        dir: &dir,
        timeout: opts.timeout,
        parse_check: opts.record,
        bsd: opts.bsd,
    };

    if opts.record {
        return record(&opts, &root, &goldens_dir, &runner, &items);
    }

    let to_run: Vec<&Item> = items
        .iter()
        .filter(|it| opts.all_groups || it.group != Group::Skip)
        .collect();
    // Golden sets by (set, file): a layer's own, and for L4 also the BSD one.
    let mut golden_cache: HashMap<(String, String), Result<Vec<Golden>, String>> = HashMap::new();
    for it in &to_run {
        let c = it.case;
        if c.expect != case::Dialect::Bsd {
            golden_cache
                .entry((c.layer.name().to_string(), c.file.clone()))
                .or_insert_with(|| load_goldens(&golden_path(&goldens_dir, c.layer, &c.file)));
        }
        if c.expect != case::Dialect::Gnu {
            golden_cache
                .entry((BSD_SET.to_string(), c.file.clone()))
                .or_insert_with(|| load_goldens(&bsd_golden_path(&goldens_dir, &c.file)));
        }
    }
    let observed = runner::parallel_map(&to_run, opts.jobs, |n, it| runner.run(it.case, n));
    let mut results = Vec::new();
    for (it, obs) in to_run.iter().zip(observed) {
        let c = it.case;
        let own = || {
            golden_of(
                &golden_cache[&(c.layer.name().to_string(), c.file.clone())],
                c,
            )
        };
        let bsd = || golden_of(&golden_cache[&(BSD_SET.to_string(), c.file.clone())], c);
        let golden = match c.expect {
            case::Dialect::Gnu => own(),
            case::Dialect::Bsd => bsd(),
            // Both recordings must agree on what decides a pass (D016).
            case::Dialect::Both => match (own(), bsd()) {
                (Ok(g), Ok(b)) if g.stdout == b.stdout && g.status == b.status && g.files == b.files => Ok(g),
                (Ok(_), Ok(_)) => Err("the GNU and BSD goldens disagree: declare `## expect: gnu` or `## expect: bsd` (and the D016 rule)".to_string()),
                (Err(e), _) | (_, Err(e)) => Err(e),
            },
        };
        results.push(report::compare(it, golden, obs, engine.name()));
    }

    if opts.status {
        print!("{}", status::render_status(&results, &items, &opts.layers));
        let append = !opts.no_append && opts.filters.is_empty() && engine.name() == "psh";
        print!("{}", status::l0_section(&root, opts.jobs, append)?);
        if append {
            status::append_history(&root, &results, &items, &opts.layers)?;
            println!("\n(appended to tests/scores/status.tsv)");
        }
        if opts.hitlist {
            let weights = status::Weights::load(&root)?;
            println!("\n{}", status::hitlist(&results, &weights, opts.rows));
        }
        return Ok(ExitCode::SUCCESS);
    }

    // Ratchets: per layer, over the cases this run covered. They describe psh; another
    // shell (the bash self-check) is only compared and summarized.
    let mut regressions = Vec::new();
    let filtered = !opts.filters.is_empty();
    let ratchet_layers: &[Layer] = if engine.name() == "psh" && !opts.wasm {
        &opts.layers
    } else {
        &[]
    };
    for &layer in ratchet_layers {
        let path = root.join(layer.ratchet_path());
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let mut ratchet = Ratchet::parse(&text);
        let layer_results: Vec<&report::CaseResult> = results
            .iter()
            .filter(|r| r.case.layer == layer && r.group == Group::Active)
            .collect();
        let mut stale = Vec::new();
        for r in &layer_results {
            let listed = ratchet.entries.contains_key(&r.case.id);
            if !r.pass && !listed {
                regressions.push(*r);
            }
            if r.pass && listed {
                stale.push(r.case.id.clone());
            }
        }
        // Entries for cases that are not active (or no longer exist) are stale too, when
        // the whole layer ran.
        if !filtered {
            let active: std::collections::HashSet<&str> =
                layer_results.iter().map(|r| r.case.id.as_str()).collect();
            for id in ratchet.entries.keys() {
                if !active.contains(id.as_str()) {
                    stale.push(id.clone());
                }
            }
        }
        let mut changed = false;
        if opts.update_ratchet && !stale.is_empty() {
            for id in &stale {
                ratchet.entries.remove(id);
            }
            changed = true;
            println!(
                "{layer}: removed {} passing entries from the ratchet",
                stale.len()
            );
        } else if !stale.is_empty() {
            println!(
                "{layer}: {} ratchet entries pass now (or are not active); run with --update-ratchet",
                stale.len()
            );
            for id in stale.iter().take(10) {
                println!("  {id}");
            }
        }
        if opts.add_failures {
            let mut added = 0;
            for r in &layer_results {
                if !r.pass && !ratchet.entries.contains_key(&r.case.id) {
                    ratchet.entries.insert(r.case.id.clone(), String::new());
                    added += 1;
                }
            }
            if added > 0 {
                changed = true;
                println!("{layer}: added {added} failing cases to the ratchet");
            }
            regressions.retain(|r| r.case.layer != layer);
        }
        if changed {
            if ratchet.header.is_empty() {
                ratchet.header = vec![
                    format!("# Ratchet for layer {layer}: active cases that may fail (TESTING.md \"Ratchets\")."),
                    "# One case id per line, optionally `id: reason`. psh-spec --update-ratchet removes".into(),
                    "# cases that pass; never add a line to hide a regression.".into(),
                ];
            }
            std::fs::write(&path, ratchet.render())
                .map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }

    if opts.wasm {
        regressions.extend(platform::check(
            &root,
            "wasm",
            &opts.layers,
            &results,
            &platform::Options {
                filtered,
                update: opts.update_ratchet,
                add: opts.add_failures,
            },
        )?);
    }
    report::print_summary(&results, &items, &opts.layers);
    if opts.diff {
        report::print_diffs(&results);
    }
    if let Some(path) = &opts.summary {
        std::fs::write(path, report::markdown(&results, &items, &opts.layers))
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let broken: Vec<&report::CaseResult> = results.iter().filter(|r| r.error.is_some()).collect();
    if !regressions.is_empty() {
        println!(
            "\n{} regressions (failing, not in the ratchet):",
            regressions.len()
        );
        for r in regressions.iter().take(50) {
            println!("  {}  [{}]  {}", r.case.id, r.what_failed(), r.case.name);
        }
        if regressions.len() > 50 {
            println!("  ... and {} more", regressions.len() - 50);
        }
    }
    if !broken.is_empty() {
        println!("\n{} cases could not be checked:", broken.len());
        for r in broken.iter().take(20) {
            println!("  {}: {}", r.case.id, r.error.as_deref().unwrap_or(""));
        }
    }
    Ok(if regressions.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn resolve_shell(path: &Path) -> Result<PathBuf, String> {
    if path.components().count() > 1 {
        return std::fs::canonicalize(path).map_err(|e| format!("{}: {e}", path.display()));
    }
    let paths = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&paths) {
        let p = dir.join(path);
        if p.is_file() {
            return Ok(p);
        }
    }
    Err(format!("{} not found on PATH", path.display()))
}

fn record(
    opts: &Options,
    root: &Path,
    goldens_dir: &Path,
    runner: &Runner,
    items: &[Item],
) -> Result<ExitCode, String> {
    let version = std::process::Command::new(runner.engine.binary())
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
    let observed: Vec<Result<Observed, String>> =
        runner::parallel_map(items, opts.jobs, |n, it| runner.run(it.case, n));
    let mut by_file: BTreeMap<(Layer, String), Vec<(&Case, Observed)>> = BTreeMap::new();
    let mut errors = 0;
    for (it, obs) in items.iter().zip(observed) {
        match obs {
            Ok(o) => by_file
                .entry((it.case.layer, it.case.file.clone()))
                .or_default()
                .push((it.case, o)),
            Err(e) => {
                eprintln!("{}: {e}", it.case.id);
                errors += 1;
            }
        }
    }
    let mut written = 0;
    for ((layer, file), recorded) in by_file {
        let path = if opts.bsd {
            bsd_golden_path(goldens_dir, &file)
        } else {
            golden_path(goldens_dir, layer, &file)
        };
        // A filtered recording replaces only its cases in an existing file.
        let mut goldens: BTreeMap<usize, Golden> = if opts.filters.is_empty() {
            BTreeMap::new()
        } else {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|t| golden::parse_file(&t).ok())
                .unwrap_or_default()
                .into_iter()
                .map(|g| (g.index, g))
                .collect()
        };
        for (case, o) in recorded {
            let (parse_status, parse_stderr) = o.parse.unwrap_or((0, Vec::new()));
            goldens.insert(
                case.index,
                Golden {
                    index: case.index,
                    name: case.name.replace('\n', " "),
                    status: o.output.status,
                    stdout: o.output.stdout,
                    stderr: o.output.stderr,
                    files: o.files,
                    parse_status,
                    parse_stderr,
                },
            );
        }
        let source = case::source_path(Path::new(""), layer, &file);
        let header = format!(
            "Goldens for {} (layer {layer}), recorded by scripts/record-goldens with\n{version}.\nDo not edit; re-record instead (TESTING.md \"Cases and goldens\").",
            source.display()
        );
        let goldens: Vec<Golden> = goldens.into_values().collect();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, golden::write_file(&header, &goldens))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        written += 1;
    }
    let _ = root;
    println!(
        "recorded {} cases into {written} golden files under {}",
        items.len() - errors,
        goldens_dir.display()
    );
    Ok(if errors == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn golden_files(dir: &Path, prefix: &str, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let rel = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        if e.path().is_dir() {
            golden_files(&e.path(), &rel, out);
        } else if let Some(stem) = rel.strip_suffix(".golden") {
            out.push(stem.to_string());
        }
    }
}

/// Compares two golden directories case by case: which cases differ, and whether only
/// in stderr (which does not decide a pass) or in what decides a pass.
fn diff_goldens(a: &Path, b: &Path) -> Result<ExitCode, String> {
    let mut files = Vec::new();
    golden_files(a, "", &mut files);
    golden_files(b, "", &mut files);
    files.sort();
    files.dedup();
    let load = |dir: &Path, f: &str| -> Result<Vec<Golden>, String> {
        match std::fs::read_to_string(dir.join(format!("{f}.golden"))) {
            Ok(t) => golden::parse_file(&t).map_err(|e| format!("{f}: {e}")),
            Err(_) => Ok(Vec::new()),
        }
    };
    let (mut pass_diff, mut stderr_diff, mut parse_diff) = (0, 0, 0);
    for f in &files {
        let ga = load(a, f)?;
        let gb = load(b, f)?;
        let indexes: std::collections::BTreeSet<usize> =
            ga.iter().chain(gb.iter()).map(|g| g.index).collect();
        for i in indexes {
            let x = ga.iter().find(|g| g.index == i);
            let y = gb.iter().find(|g| g.index == i);
            let what = match (x, y) {
                (Some(x), Some(y)) => {
                    if x.stdout != y.stdout || x.status != y.status || x.files != y.files {
                        pass_diff += 1;
                        "differs"
                    } else if x.stderr != y.stderr {
                        stderr_diff += 1;
                        "stderr"
                    } else if x.parse_status != y.parse_status || x.parse_stderr != y.parse_stderr {
                        parse_diff += 1;
                        "bash-n"
                    } else {
                        continue;
                    }
                }
                _ => {
                    pass_diff += 1;
                    "missing"
                }
            };
            println!("{what}\t{f}/{i}");
        }
    }
    println!(
        "{pass_diff} cases differ in stdout/status/files, {stderr_diff} only in stderr, {parse_diff} only in bash -n"
    );
    Ok(if pass_diff + stderr_diff + parse_diff == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}
