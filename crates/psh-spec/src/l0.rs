//! L0: the parser against `bash -n` (TESTING.md "Test layers").
//!
//! Inputs, each with bash's recorded verdict:
//! - every case script of L1-L3 (`cases/<case id>`; the verdict is the case golden's
//!   `bash-n:` line),
//! - every skeleton of the shape corpus (`shapes/<line>`; `tests/shapes/shapes.txt`,
//!   verdicts in `tests/goldens/shapes/bash-n.txt`),
//! - every snippet of the parser's own `tests/l0/syntax.txt` (`syntax/<name>`; verdicts
//!   in `tests/goldens/l0/bash-n.txt`).
//!
//! An input passes when psh-syntax accepts exactly what bash accepts and, if it parses,
//! printing the tree and parsing the result gives the same tree (compared through
//! `psh_syntax::dump`). The ratchet is `tests/l0/expected-failures.txt`.
//!
//! Verdict files (`bash-n.txt`) list only the inputs bash rejects:
//!
//! ```text
//! # bash -n verdicts for SOURCE (fnv1a64 HASH, N inputs), recorded by ... with VERSION.
//! KEY<TAB>STATUS<TAB>STDERR
//! ```
//!
//! KEY is the 1-based line (shapes) or the snippet's name (syntax). STDERR is escaped
//! like `shapes.txt` (`\\`, `\n`, `\t`). The hash ties the verdicts to the source they
//! were recorded from; a changed source needs `scripts/record-goldens --l0`.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use crate::case::{self, Layer};
use crate::engine::{Engine, RunRequest, Status};
use crate::golden;
use crate::ratchet::Ratchet;
use crate::sandbox::fnv1a64;

pub const RATCHET: &str = "tests/l0/expected-failures.txt";

/// A parse-only input.
pub struct Input {
    /// `shapes/12`, `syntax/a-name`, `cases/oils/alias/3`.
    pub id: String,
    pub script: String,
}

/// Where a parse-only source and its verdicts live.
struct Source {
    name: &'static str,
    path: &'static str,
    verdicts: &'static str,
}

const SHAPES: Source = Source {
    name: "shapes",
    path: "tests/shapes/shapes.txt",
    verdicts: "tests/goldens/shapes/bash-n.txt",
};

const SYNTAX: Source = Source {
    name: "syntax",
    path: "tests/l0/syntax.txt",
    verdicts: "tests/goldens/l0/bash-n.txt",
};

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

/// `shapes.txt`: `COUNT<TAB>SKELETON` per line; the key is the 1-based line.
fn shape_inputs(text: &str) -> Vec<(String, String)> {
    text.lines()
        .enumerate()
        .map(|(i, l)| {
            let skeleton = l.split_once('\t').map_or(l, |(_, s)| s);
            ((i + 1).to_string(), unescape(skeleton))
        })
        .collect()
}

/// `syntax.txt`: `#### NAME` starts a snippet, which runs to its `## status:` line (the
/// format of psh-syntax's `tests/l0.rs`); the key is NAME.
fn syntax_inputs(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut open = false;
    for line in text.lines() {
        if let Some(name) = line.strip_prefix("#### ") {
            out.push((name.to_string(), String::new()));
            open = true;
        } else if line.starts_with("## status:") {
            open = false;
        } else if open && let Some((_, script)) = out.last_mut() {
            script.push_str(line);
            script.push('\n');
        }
    }
    out
}

fn source_inputs(root: &Path, src: &Source) -> Result<(String, Vec<(String, String)>), String> {
    let path = root.join(src.path);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let inputs = match src.name {
        "shapes" => shape_inputs(&text),
        _ => syntax_inputs(&text),
    };
    Ok((text, inputs))
}

/// The rejected inputs of a verdict file, by key, and the hash it was recorded for.
fn read_verdicts(path: &Path) -> Result<(u64, BTreeMap<String, i32>), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let hash = text
        .split("fnv1a64 ")
        .nth(1)
        .and_then(|r| u64::from_str_radix(r.get(..16)?, 16).ok())
        .ok_or_else(|| format!("{}: no source hash in the header", path.display()))?;
    let mut rejected = BTreeMap::new();
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let mut parts = line.splitn(3, '\t');
        let key = parts.next().unwrap_or("");
        let status = parts.next().and_then(|s| s.parse().ok()).unwrap_or(-1);
        rejected.insert(key.to_string(), status);
    }
    Ok((hash, rejected))
}

/// Records `bash -n` for the shape corpus and the parser's snippets (the case scripts'
/// verdicts are recorded with the cases).
pub fn record(
    root: &Path,
    goldens: &Path,
    engine: &dyn Engine,
    jobs: usize,
) -> Result<ExitCode, String> {
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
    let dir = std::env::temp_dir().join(format!("psh-spec-l0-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let env = vec![
        (
            "PATH".to_string(),
            "/usr/local/bin:/usr/bin:/bin".to_string(),
        ),
        ("LC_ALL".to_string(), "C.UTF-8".to_string()),
    ];
    for src in [&SHAPES, &SYNTAX] {
        if !root.join(src.path).is_file() {
            continue;
        }
        let (text, inputs) = source_inputs(root, src)?;
        let verdicts = crate::runner::parallel_map(&inputs, jobs, |_, (_, script)| {
            engine.run(&RunRequest {
                args: &["-n".to_string()],
                stdin: Some(script.as_bytes()),
                env: &env,
                cwd: &dir,
                timeout: Duration::from_secs(10),
            })
        });
        let mut out = format!(
            "# bash -n verdicts for {} (fnv1a64 {:016x}, {} inputs),\n\
             # recorded by scripts/record-goldens with {version}.\n\
             # KEY<TAB>STATUS<TAB>STDERR for each input bash rejects; unlisted ones parse.\n",
            src.path,
            fnv1a64(text.as_bytes()),
            inputs.len()
        );
        let mut rejected = 0;
        for ((key, _), v) in inputs.iter().zip(verdicts) {
            let v = v.map_err(|e| format!("{} {key}: {e}", src.name))?;
            let status = match v.status {
                Status::Exit(0) => continue,
                Status::Exit(n) => n.to_string(),
                other => other.to_string(),
            };
            rejected += 1;
            out.push_str(&format!("{key}\t{status}\t{}\n", escape(&v.stderr)));
        }
        let rel = src.verdicts.strip_prefix("tests/goldens/").unwrap();
        let target = goldens.join(rel);
        std::fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::write(&target, out).map_err(|e| format!("{}: {e}", target.display()))?;
        println!(
            "recorded bash -n for {} {} inputs ({rejected} rejected) into {}",
            inputs.len(),
            src.name,
            target.display()
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
    Ok(ExitCode::SUCCESS)
}

/// Every L0 input with bash's verdict (`true`: parses).
pub fn inputs(root: &Path) -> Result<Vec<(Input, bool)>, String> {
    let mut out = Vec::new();
    for layer in Layer::ALL {
        let cases = case::load_layer(root, layer)?;
        let mut goldens: BTreeMap<String, BTreeMap<usize, i32>> = BTreeMap::new();
        for c in cases {
            let verdicts = match goldens.get(&c.file) {
                Some(v) => v,
                None => {
                    let path = root
                        .join("tests/goldens")
                        .join(layer.name())
                        .join(format!("{}.golden", c.file));
                    let text = std::fs::read_to_string(&path)
                        .map_err(|e| format!("{}: {e}", path.display()))?;
                    let parsed = golden::parse_file(&text)
                        .map_err(|e| format!("{}: {e}", path.display()))?;
                    goldens.insert(
                        c.file.clone(),
                        parsed
                            .into_iter()
                            .map(|g| (g.index, g.parse_status))
                            .collect(),
                    );
                    &goldens[&c.file]
                }
            };
            let status = *verdicts
                .get(&c.index)
                .ok_or_else(|| format!("{}: no golden", c.id))?;
            out.push((
                Input {
                    id: format!("cases/{}", c.id),
                    script: c.script,
                },
                status == 0,
            ));
        }
    }
    for src in [&SHAPES, &SYNTAX] {
        if !root.join(src.path).is_file() {
            continue;
        }
        let (text, list) = source_inputs(root, src)?;
        let (hash, rejected) = read_verdicts(&root.join(src.verdicts))?;
        if hash != fnv1a64(text.as_bytes()) {
            return Err(format!(
                "{} changed since its verdicts were recorded; run scripts/record-goldens --l0",
                src.path
            ));
        }
        for (key, script) in list {
            let ok = !rejected.contains_key(&key);
            out.push((
                Input {
                    id: format!("{}/{key}", src.name),
                    script,
                },
                ok,
            ));
        }
    }
    Ok(out)
}

pub struct Outcome {
    pub id: String,
    pub bash_ok: bool,
    /// psh-syntax's verdict, or the panic it raised.
    pub psh: Result<(), String>,
    /// For a script that parsed: whether print -> reparse kept the tree.
    pub round_trip: Result<(), String>,
    pub pass: bool,
}

/// Parses one input with psh-syntax and checks the round trip.
pub fn check_one(input: &Input, bash_ok: bool) -> Outcome {
    let run = || -> (Result<(), String>, Result<(), String>) {
        match psh_syntax::parse(&input.script) {
            Err(e) => (Err(format!("{e}")), Ok(())),
            Ok(ast) => {
                let printed = psh_syntax::print(&ast);
                let rt = match psh_syntax::parse(&printed) {
                    Ok(again) if psh_syntax::dump(&again) == psh_syntax::dump(&ast) => Ok(()),
                    Ok(_) => Err(format!("round trip changed the tree; printed:\n{printed}")),
                    Err(e) => Err(format!("printed form does not parse: {e}\n{printed}")),
                };
                (Ok(()), rt)
            }
        }
    };
    let mut panicked = false;
    let (psh, round_trip) = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)) {
        Ok(r) => r,
        Err(p) => {
            panicked = true;
            let msg = p
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            (Err(format!("panic: {msg}")), Err("panic".into()))
        }
    };
    let pass = !panicked && psh.is_ok() == bash_ok && round_trip.is_ok();
    Outcome {
        id: input.id.clone(),
        bash_ok,
        psh,
        round_trip,
        pass,
    }
}

fn selected(id: &str, filters: &[String]) -> bool {
    filters.is_empty()
        || filters.iter().any(|f| {
            let f = f.strip_prefix("l0/").unwrap_or(f);
            id == f || id.starts_with(&format!("{}/", f.trim_end_matches('/')))
        })
}

/// Runs L0 over the (filtered) inputs.
pub fn run_all(
    root: &Path,
    filters: &[String],
    jobs: usize,
) -> Result<(Vec<Input>, Vec<Outcome>), String> {
    let mut all = inputs(root)?;
    all.retain(|(i, _)| selected(&i.id, filters));
    // Parser panics are counted as failures; keep their messages out of the output.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let outcomes = crate::runner::parallel_map(&all, jobs, |_, (input, ok)| check_one(input, *ok));
    std::panic::set_hook(hook);
    Ok((all.into_iter().map(|(i, _)| i).collect(), outcomes))
}

/// Counts per source (`cases`, `shapes`, `syntax`): (inputs, agree, round trip ok, pass).
pub fn counts(outcomes: &[Outcome]) -> BTreeMap<&'static str, [usize; 4]> {
    let mut m: BTreeMap<&'static str, [usize; 4]> = BTreeMap::new();
    for o in outcomes {
        let src = if o.id.starts_with("cases/") {
            "cases"
        } else if o.id.starts_with("shapes/") {
            "shapes"
        } else {
            "syntax"
        };
        let c = m.entry(src).or_default();
        c[0] += 1;
        c[1] += usize::from(o.psh.is_ok() == o.bash_ok);
        c[2] += usize::from(o.round_trip.is_ok());
        c[3] += usize::from(o.pass);
    }
    m
}

pub fn summary(outcomes: &[Outcome]) -> String {
    let mut out = String::new();
    for (src, c) in counts(outcomes) {
        out.push_str(&format!(
            "l0/{src}: {}/{} pass ({} agree with bash -n, {} round-trip)\n",
            c[3], c[0], c[1], c[2]
        ));
    }
    out
}

/// `psh-spec --l0`: summary, ratchet check (or update), `--diff` details.
pub fn main(
    root: &Path,
    filters: &[String],
    jobs: usize,
    diff: bool,
    update: bool,
    add: bool,
) -> Result<ExitCode, String> {
    let (inputs, outcomes) = run_all(root, filters, jobs)?;
    print!("{}", summary(&outcomes));
    let path = root.join(RATCHET);
    let mut ratchet = Ratchet::parse(&std::fs::read_to_string(&path).unwrap_or_default());
    let mut regressions = Vec::new();
    let mut stale = Vec::new();
    for o in &outcomes {
        let listed = ratchet.entries.contains_key(&o.id);
        if !o.pass && !listed {
            regressions.push(o);
        }
        if o.pass && listed {
            stale.push(o.id.clone());
        }
    }
    if filters.is_empty() {
        let known: std::collections::HashSet<&str> =
            outcomes.iter().map(|o| o.id.as_str()).collect();
        stale.extend(
            ratchet
                .entries
                .keys()
                .filter(|id| !known.contains(id.as_str()))
                .cloned(),
        );
    }
    let mut changed = false;
    if !stale.is_empty() {
        if update {
            for id in &stale {
                ratchet.entries.remove(id);
            }
            changed = true;
            println!(
                "l0: removed {} passing entries from the ratchet",
                stale.len()
            );
        } else {
            println!(
                "l0: {} ratchet entries pass now (or no longer exist); run with --update-ratchet",
                stale.len()
            );
            for id in stale.iter().take(10) {
                println!("  {id}");
            }
        }
    }
    if add && !regressions.is_empty() {
        for o in &regressions {
            ratchet.entries.insert(o.id.clone(), String::new());
        }
        println!(
            "l0: added {} failing inputs to the ratchet",
            regressions.len()
        );
        regressions.clear();
        changed = true;
    }
    if changed {
        if ratchet.header.is_empty() {
            ratchet.header = vec![
                "# Ratchet for L0 (the parser against bash -n, TESTING.md): inputs that may fail.".into(),
                "# One id per line, optionally `id: reason`. psh-spec --l0 --update-ratchet removes".into(),
                "# inputs that pass; never add a line to hide a regression.".into(),
            ];
        }
        std::fs::write(&path, ratchet.render()).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    if diff {
        let scripts: BTreeMap<&str, &str> = inputs
            .iter()
            .map(|i| (i.id.as_str(), i.script.as_str()))
            .collect();
        for o in outcomes.iter().filter(|o| !o.pass) {
            println!("\n=== {}", o.id);
            println!(
                "bash -n: {}; psh: {}",
                if o.bash_ok { "parses" } else { "rejects" },
                match &o.psh {
                    Ok(()) => "parses".to_string(),
                    Err(e) => format!("rejects: {e}"),
                }
            );
            if let Err(e) = &o.round_trip {
                println!("{e}");
            }
            let script = scripts.get(o.id.as_str()).copied().unwrap_or("");
            let shown: String = script.chars().take(2000).collect();
            println!("--- script\n{shown}");
        }
    }
    if !regressions.is_empty() {
        println!(
            "\n{} L0 regressions (failing, not in the ratchet):",
            regressions.len()
        );
        for o in regressions.iter().take(50) {
            println!("  {}", o.id);
        }
        return Ok(ExitCode::from(1));
    }
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

    #[test]
    fn syntax_snippets() {
        let s = syntax_inputs(
            "# header\n#\n#### 0\necho a\n## status: 0\n\n#### b\nif\n# c\n## status: 2\n## message: x\n",
        );
        assert_eq!(
            s,
            vec![
                ("0".to_string(), "echo a\n".to_string()),
                ("b".to_string(), "if\n# c\n".to_string())
            ]
        );
    }

    #[test]
    fn round_trip_and_verdicts() {
        let ok = Input {
            id: "x".into(),
            script: "echo a | grep b && cd c\n".into(),
        };
        assert!(check_one(&ok, true).pass);
        assert!(!check_one(&ok, false).pass);
        let bad = Input {
            id: "y".into(),
            script: "if then fi\n".into(),
        };
        assert!(check_one(&bad, false).pass);
    }
}
