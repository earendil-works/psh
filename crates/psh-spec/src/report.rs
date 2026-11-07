//! Comparing a run against its golden, and reporting: the console summary, `--diff`
//! and the Markdown summary per layer and file.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::Item;
use crate::case::{Case, Layer};
use crate::engine::Status;
use crate::golden::{self, Golden};
use crate::groups::Group;
use crate::normalize;
use crate::runner::Observed;
use crate::sandbox::FileEntry;

pub struct CaseResult<'a> {
    pub case: &'a Case,
    pub group: Group,
    pub stdout_ok: bool,
    pub status_ok: bool,
    pub files_ok: bool,
    pub stderr_ok: bool,
    /// Nothing the case started outlived it (0030).
    pub leak_ok: bool,
    /// stdout, status and files match. stderr is scored separately.
    pub pass: bool,
    /// The case could not be checked (no golden, the shell did not start, ...).
    pub error: Option<String>,
    pub expected: Option<Golden>,
    pub actual: Option<Observed>,
}

impl CaseResult<'_> {
    pub fn what_failed(&self) -> String {
        if let Some(e) = &self.error {
            return e.clone();
        }
        let mut parts = Vec::new();
        if !self.stdout_ok {
            parts.push("stdout");
        }
        if !self.status_ok {
            parts.push("status");
        }
        if !self.files_ok {
            parts.push("files");
        }
        if !self.stderr_ok {
            parts.push("stderr");
        }
        if !self.leak_ok {
            parts.push("leak");
        }
        parts.join(" ")
    }
}

/// The name of the shell the goldens were recorded with.
const GOLDEN_SHELL: &str = "bash";

fn collapse_ws(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for word in data
        .split(|b| b.is_ascii_whitespace())
        .filter(|w| !w.is_empty())
    {
        if !out.is_empty() {
            out.push(b' ');
        }
        out.extend_from_slice(word);
    }
    out
}

pub fn compare<'a>(
    item: &Item<'a>,
    golden: Result<&Golden, String>,
    observed: Result<Observed, String>,
    shell: &str,
) -> CaseResult<'a> {
    let mut r = CaseResult {
        case: item.case,
        group: item.group,
        stdout_ok: false,
        status_ok: false,
        files_ok: false,
        stderr_ok: false,
        leak_ok: true,
        pass: false,
        error: None,
        expected: None,
        actual: None,
    };
    let (golden, observed) = match (golden, observed) {
        (Ok(g), Ok(o)) => (g, o),
        (Err(e), _) | (_, Err(e)) => {
            r.error = Some(e);
            return r;
        }
    };
    let norm = |data: &[u8], name: &str| {
        let d = normalize::shell_name(data, name);
        if item.case.ignore_whitespace {
            collapse_ws(&d)
        } else {
            d
        }
    };
    r.stdout_ok = item.case.ignore_stdout
        || norm(&golden.stdout, GOLDEN_SHELL) == norm(&observed.output.stdout, shell);
    r.stderr_ok = norm(&golden.stderr, GOLDEN_SHELL) == norm(&observed.output.stderr, shell);
    r.status_ok = crate::normalize::same_status(&golden.status, &observed.output.status);
    r.files_ok = golden.files == observed.files;
    r.leak_ok = observed.output.leaked.is_empty();
    r.pass = r.stdout_ok && r.status_ok && r.files_ok && r.leak_ok;
    r.expected = Some(golden.clone());
    r.actual = Some(observed);
    r
}

#[derive(Default, Clone, Copy)]
pub struct Counts {
    pub total: usize,
    pub skip: usize,
    pub later: usize,
    pub later_pass: usize,
    pub active: usize,
    pub pass: usize,
    pub stderr_ok: usize,
    pub errors: usize,
}

impl Counts {
    fn add(&mut self, o: &Counts) {
        self.total += o.total;
        self.skip += o.skip;
        self.later += o.later;
        self.later_pass += o.later_pass;
        self.active += o.active;
        self.pass += o.pass;
        self.stderr_ok += o.stderr_ok;
        self.errors += o.errors;
    }
}

/// Counts per (layer, file).
pub fn tally(results: &[CaseResult], items: &[Item]) -> BTreeMap<(Layer, String), Counts> {
    let mut map: BTreeMap<(Layer, String), Counts> = BTreeMap::new();
    for it in items {
        let c = map
            .entry((it.case.layer, it.case.file.clone()))
            .or_default();
        c.total += 1;
        match it.group {
            Group::Skip => c.skip += 1,
            Group::Later => c.later += 1,
            Group::Active => c.active += 1,
        }
    }
    for r in results {
        let c = map.entry((r.case.layer, r.case.file.clone())).or_default();
        if r.error.is_some() {
            c.errors += 1;
        }
        match r.group {
            Group::Active => {
                c.pass += usize::from(r.pass);
                c.stderr_ok += usize::from(r.pass && r.stderr_ok);
            }
            Group::Later => c.later_pass += usize::from(r.pass),
            Group::Skip => {}
        }
    }
    map
}

fn pct(n: usize, d: usize) -> String {
    if d == 0 {
        "-".into()
    } else {
        format!("{:.1}%", 100.0 * n as f64 / d as f64)
    }
}

pub fn layer_counts(results: &[CaseResult], items: &[Item]) -> BTreeMap<Layer, Counts> {
    let mut out: BTreeMap<Layer, Counts> = BTreeMap::new();
    for ((layer, _), c) in tally(results, items) {
        out.entry(layer).or_default().add(&c);
    }
    out
}

pub fn print_summary(results: &[CaseResult], items: &[Item], layers: &[Layer]) {
    let counts = layer_counts(results, items);
    for layer in layers {
        let Some(c) = counts.get(layer) else {
            continue;
        };
        println!(
            "{layer}: {}/{} active pass ({}), {} also match stderr; later {}/{}; skip {}{}",
            c.pass,
            c.active,
            pct(c.pass, c.active),
            c.stderr_ok,
            c.later_pass,
            c.later,
            c.skip,
            if c.errors > 0 {
                format!("; {} could not be checked", c.errors)
            } else {
                String::new()
            }
        );
    }
}

/// The quarantined cases of this run (`[[flaky]]`, D1225), so they stay visible: each
/// with whether it passed and why it is there. Empty when none ran.
pub fn flaky(results: &[CaseResult]) -> String {
    let flaky: Vec<&CaseResult> = results.iter().filter(|r| r.case.flaky.is_some()).collect();
    if flaky.is_empty() {
        return String::new();
    }
    let passed = flaky.iter().filter(|r| r.pass).count();
    let mut out = format!(
        "\nflaky (quarantined, not gated): {} cases, {passed} passed this run\n",
        flaky.len()
    );
    for r in flaky {
        out.push_str(&format!(
            "  {} {}: {}\n",
            if r.pass { "pass" } else { "FAIL" },
            r.case.id,
            r.case.flaky.as_deref().unwrap_or("")
        ));
    }
    out
}

pub fn markdown(results: &[CaseResult], items: &[Item], layers: &[Layer]) -> String {
    let files = tally(results, items);
    let counts = layer_counts(results, items);
    let mut out = String::from("# psh-spec summary\n\n");
    out.push_str("| Layer | Active | Pass | % | + stderr | Later (pass) | Skip |\n");
    out.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    for layer in layers {
        if let Some(c) = counts.get(layer) {
            let _ = writeln!(
                out,
                "| {layer} | {} | {} | {} | {} | {} ({}) | {} |",
                c.active,
                c.pass,
                pct(c.pass, c.active),
                c.stderr_ok,
                c.later,
                c.later_pass,
                c.skip
            );
        }
    }
    for layer in layers {
        let _ = write!(
            out,
            "\n## {layer}\n\n| File | Active | Pass | % | + stderr | Later (pass) | Skip |\n| --- | ---: | ---: | ---: | ---: | ---: | ---: |\n"
        );
        for ((l, file), c) in &files {
            if l == layer {
                let _ = writeln!(
                    out,
                    "| {file} | {} | {} | {} | {} | {} ({}) | {} |",
                    c.active,
                    c.pass,
                    pct(c.pass, c.active),
                    c.stderr_ok,
                    c.later,
                    c.later_pass,
                    c.skip
                );
            }
        }
    }
    out
}

fn block(data: &[u8]) -> String {
    let mut s = String::new();
    golden::write_block(&mut s, data);
    s
}

fn files_text(files: &[FileEntry]) -> String {
    let g = Golden {
        index: 0,
        name: String::new(),
        status: Status::Exit(0),
        stdout: Vec::new(),
        stderr: Vec::new(),
        files: files.to_vec(),
        parse_status: 0,
        parse_stderr: Vec::new(),
    };
    golden::write_file("", &[g])
        .lines()
        .skip_while(|l| !l.starts_with("--- "))
        .map(|l| format!("{l}\n"))
        .collect()
}

/// A line diff (`-` expected, `+` actual), by longest common subsequence.
fn line_diff(expected: &str, actual: &str) -> String {
    let a: Vec<&str> = expected.lines().collect();
    let b: Vec<&str> = actual.lines().collect();
    if a.len() * b.len() > 4_000_000 {
        return format!("(too large to diff)\n--- expected\n{expected}--- actual\n{actual}");
    }
    let mut lcs = vec![vec![0u32; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut out = String::new();
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            let _ = writeln!(out, "  {}", a[i]);
            i += 1;
            j += 1;
        } else if j < b.len() && (i == a.len() || lcs[i][j + 1] >= lcs[i + 1][j]) {
            let _ = writeln!(out, "+ {}", b[j]);
            j += 1;
        } else {
            let _ = writeln!(out, "- {}", a[i]);
            i += 1;
        }
    }
    out
}

pub fn print_diffs(results: &[CaseResult]) {
    for r in results {
        if r.pass && r.stderr_ok {
            continue;
        }
        println!("\n=== {} [{}] {}", r.case.id, r.what_failed(), r.case.name);
        let (Some(e), Some(a)) = (&r.expected, &r.actual) else {
            continue;
        };
        if !r.stdout_ok {
            print!(
                "--- stdout\n{}",
                line_diff(&block(&e.stdout), &block(&a.output.stdout))
            );
        }
        if !r.status_ok {
            println!("--- status: expected {}, got {}", e.status, a.output.status);
        }
        if !r.files_ok {
            print!(
                "--- files\n{}",
                line_diff(&files_text(&e.files), &files_text(&a.files))
            );
        }
        if !r.stderr_ok {
            print!(
                "--- stderr\n{}",
                line_diff(&block(&e.stderr), &block(&a.output.stderr))
            );
        }
        if !r.leak_ok {
            println!("--- leaked (killed): {}", a.output.leaked.join(", "));
        }
    }
}
