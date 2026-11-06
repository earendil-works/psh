//! `scripts/status`: the scoreboard per layer and area, its history in
//! `tests/scores/status.tsv`, and the hit list: failing cases ranked by how often agents
//! use what they test (TESTING.md "Status and the hit list").
//!
//! A case's weight is the corpus frequency of the rarest construct in its script: a case
//! is only as relevant as its most exotic part, which is usually what it tests. The
//! constructs are found lexically, an approximation until psh's parser can classify
//! scripts; the frequencies are `research/corpus/results.json` (`features` for syntax,
//! `top_names` for command names). Constructs the corpus did not measure and bash
//! builtins outside its top names count as rare.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use crate::case::{Case, Layer};
use crate::groups::Group;
use crate::json::{self, Json};
use crate::report::CaseResult;

/// Weight of a construct agents (almost) never use: never seen, or below the corpus's
/// top-name cutoff.
const RARE: f64 = 1.0;

pub struct Weights {
    features: BTreeMap<String, f64>,
    names: BTreeMap<String, f64>,
    total: f64,
}

impl Weights {
    pub fn load(root: &Path) -> Result<Weights, String> {
        let path = root.join("research/corpus/results.json");
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let j = json::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut features = BTreeMap::new();
        if let Some(Json::Obj(all)) = j.get("features").and_then(|f| f.get("all")) {
            for (k, v) in all {
                features.insert(k.clone(), v.as_f64().unwrap_or(0.0));
            }
        }
        let mut names = BTreeMap::new();
        for entry in j.get("top_names").map(Json::as_arr).unwrap_or(&[]) {
            let e = entry.as_arr();
            if let (Some(name), Some(n)) = (
                e.first().and_then(Json::as_str),
                e.get(1).and_then(Json::as_f64),
            ) {
                names.insert(name.to_string(), n);
            }
        }
        let total = j.get("total").and_then(Json::as_f64).unwrap_or(1.0);
        Ok(Weights {
            features,
            names,
            total,
        })
    }

    /// The case's weight and the construct that sets it. Shell arguments (brush's
    /// invocation cases, beyond our agent layer's own `-c CODE`) count as rare: agents'
    /// commands reach the shell as `-c CODE` only.
    pub fn weigh_case(&self, case: &Case) -> (f64, String) {
        let (w, why) = self.weigh(&case.script);
        if case.layer != Layer::Agent && !case.args.is_empty() && w > RARE {
            return (RARE, "shell arguments".to_string());
        }
        (w, why)
    }

    /// A script's weight and the construct that sets it.
    pub fn weigh(&self, script: &str) -> (f64, String) {
        let mut best = (self.total, "plain commands".to_string());
        let mut consider = |w: f64, what: String| {
            if w < best.0 {
                best = (w, what);
            }
        };
        for f in features(script) {
            match self.features.get(f) {
                Some(&n) => consider(n.max(RARE), f.to_string()),
                None => consider(RARE, f.to_string()),
            }
        }
        for name in command_names(script) {
            if let Some(&n) = self.names.get(&name) {
                consider(n, name);
            } else if BUILTINS.contains(&name.as_str()) && !COVERED.contains(&name.as_str()) {
                consider(RARE, name);
            }
        }
        best
    }
}

/// bash builtins and keywords: a command name among these that the corpus's top names
/// lack is rare. (External programs are left out: test helpers like `argv.py` would
/// otherwise make every case rare.)
const BUILTINS: &[&str] = &[
    "alias",
    "bg",
    "bind",
    "builtin",
    "caller",
    "cd",
    "command",
    "compgen",
    "complete",
    "compopt",
    "coproc",
    "dirs",
    "disown",
    "echo",
    "enable",
    "eval",
    "exec",
    "exit",
    "export",
    "fc",
    "fg",
    "getopts",
    "hash",
    "help",
    "history",
    "jobs",
    "kill",
    "let",
    "local",
    "logout",
    "mapfile",
    "popd",
    "printf",
    "pushd",
    "pwd",
    "read",
    "readarray",
    "readonly",
    "return",
    "select",
    "set",
    "shift",
    "shopt",
    "source",
    "suspend",
    "test",
    "times",
    "trap",
    "type",
    "typeset",
    "ulimit",
    "umask",
    "unalias",
    "unset",
    "wait",
    "declare",
    "break",
    "continue",
    "true",
    "false",
];

/// Builtins whose use the corpus measures as a syntax feature instead (`declaration`).
const COVERED: &[&str] = &["declare", "local", "export", "readonly", "typeset"];

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Whether `word` occurs in `s` as a whole word.
fn has_word(s: &str, word: &str) -> bool {
    let b = s.as_bytes();
    let mut from = 0;
    while let Some(i) = s[from..].find(word) {
        let start = from + i;
        let end = start + word.len();
        let before = start == 0 || !is_word_byte(b[start - 1]);
        let after = end == b.len() || !is_word_byte(b[end]);
        if before && after {
            return true;
        }
        from = start + 1;
    }
    false
}

/// The corpus features a script shows, found lexically (an approximation of the corpus
/// analysis, which used tree-sitter), plus unmeasured rare constructs.
pub fn features(s: &str) -> Vec<&'static str> {
    let mut f = Vec::new();
    let mut add = |cond: bool, name: &'static str| {
        if cond {
            f.push(name);
        }
    };
    let b = s.as_bytes();
    let at = |i: usize| b.get(i).copied().unwrap_or(0);
    let mut pipe = false;
    let mut background = false;
    let mut redir_in = false;
    let mut redir_out = false;
    for (i, &c) in b.iter().enumerate() {
        match c {
            b'|' if at(i + 1) != b'|' && (i == 0 || at(i - 1) != b'|') => pipe = true,
            b'&' if at(i + 1) != b'&'
                && at(i + 1) != b'>'
                && (i == 0 || !matches!(at(i - 1), b'&' | b'>' | b'<' | b'|')) =>
            {
                background = true
            }
            b'<' if !matches!(at(i + 1), b'<' | b'(' | b'&' | b'>')
                && (i == 0 || !matches!(at(i - 1), b'<' | b'$')) =>
            {
                redir_in = true
            }
            b'>' if !matches!(at(i + 1), b'>' | b'&' | b'(')
                && (i == 0 || !matches!(at(i - 1), b'>' | b'&' | b'-' | b'=' | b'<')) =>
            {
                redir_out = true
            }
            _ => {}
        }
    }
    add(pipe, "pipe");
    add(s.contains("&&"), "and");
    add(s.contains("||"), "or");
    add(s.replace(";;", "").contains(';'), "semicolon");
    add(background, "background");
    add(s.contains(">&") || s.contains("<&"), "redir_fd_dup");
    add(s.contains("/dev/null"), "redir_devnull");
    add(
        s.contains("2>/dev/null") || s.contains("2> /dev/null"),
        "redir_stderr_devnull",
    );
    add(s.contains(">>"), "redir_append");
    add(redir_in, "redir_in");
    add(
        redir_out || s.contains(">>") || s.contains("&>"),
        "redir_to_file",
    );
    add(s.contains("&>"), "redir_amp_out");
    let heredoc = s.replace("<<<", "").contains("<<");
    let heredoc_quoted = [
        "<<'", "<<\"", "<<-'", "<<-\"", "<< '", "<< \"", "<<\\", "<<-\\",
    ]
    .iter()
    .any(|p| s.contains(p));
    add(heredoc, "heredoc");
    add(heredoc_quoted, "heredoc_quoted");
    add(heredoc && !heredoc_quoted, "heredoc_unquoted");
    add(s.contains("<<-"), "heredoc_dash");
    add(s.contains("<<<"), "herestring");
    add(s.contains('\''), "single_quoted");
    add(s.contains('"'), "double_quoted");
    add(s.contains("$'"), "ansi_c_string");
    add(s.replace("$((", "").contains("$("), "cmd_subst");
    add(s.contains('`'), "backtick");
    add(s.contains("$(("), "arith_expansion");
    add(s.contains("<(") || s.contains(">("), "proc_subst");
    add(s.contains("${"), "param_expansion");
    let op = s.split("${").skip(1).any(|rest| {
        let inner = rest.split('}').next().unwrap_or("");
        let body = inner.trim_start_matches(['#', '!']);
        let name_len = body
            .bytes()
            .take_while(|&c| is_word_byte(c) || c == b'@' || c == b'*')
            .count();
        let after = &body[name_len..];
        let after = if after.starts_with('[') {
            after.split_once(']').map_or("", |x| x.1)
        } else {
            after
        };
        !after.is_empty() || inner.starts_with('#') || inner.starts_with('!')
    });
    add(op, "param_expansion_op");
    add(
        s.split("${").skip(1).any(|r| {
            let n = r.bytes().take_while(|&c| is_word_byte(c)).count();
            r.as_bytes().get(n) == Some(&b'[')
        }),
        "array_ref",
    );
    add(
        s.split('$').skip(1).any(|r| {
            r.bytes().next().is_some_and(|c| {
                is_word_byte(c) || matches!(c, b'?' | b'@' | b'*' | b'#' | b'!' | b'$' | b'-')
            })
        }),
        "var_ref",
    );
    add(has_word(s, "for") && !s.contains("for (("), "for");
    add(s.contains("for (("), "for_c");
    add(has_word(s, "while") || has_word(s, "until"), "while");
    add(has_word(s, "until"), "until");
    add(has_word(s, "if"), "if");
    add(has_word(s, "case") && has_word(s, "esac"), "case");
    add(s.contains("()") || has_word(s, "function"), "function");
    add(s.contains("[["), "dbl_bracket");
    add(s.contains("[ ") && !s.contains("[[ "), "single_bracket");
    add(s.replace("$((", "").contains("(("), "arith_cmd");
    add(s.contains("=("), "array");
    add(
        ["declare", "local", "export", "readonly", "typeset"]
            .iter()
            .any(|w| has_word(s, w)),
        "declaration",
    );
    add(
        s.lines().any(|l| l.trim_start().starts_with("! ")),
        "negation",
    );
    add(
        s.split('{').skip(1).any(|r| {
            let inner = r.split('}').next().unwrap_or("");
            !inner.contains(' ') && (inner.contains(',') || inner.contains(".."))
        }),
        "brace_expansion",
    );
    add(
        s.lines().any(|l| {
            l.split_whitespace()
                .any(|w| !w.starts_with(['\'', '"', '#']) && (w.contains('*') || w.contains('?')))
        }),
        "glob_word",
    );
    add(
        s.lines().any(|l| l.trim_start().starts_with('#')) || s.contains(" #"),
        "comment",
    );
    add(s.contains("\\\n"), "line_continuation");
    add(s.contains("set -e") || s.contains("errexit"), "set_e");
    add(s.contains("set -u") || s.contains("nounset"), "set_u");
    add(s.contains("pipefail"), "set_pipefail");
    add(s.contains("set -x") || s.contains("xtrace"), "set_x");
    let mut assignment = false;
    let mut env_prefix = false;
    for line in s.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        if let Some(first) = words.first() {
            let is_assign = first
                .split_once('=')
                .is_some_and(|(n, _)| !n.is_empty() && n.bytes().all(is_word_byte));
            if is_assign {
                if words.len() > 1
                    && !first.ends_with(';')
                    && !words[1].starts_with([';', '&', '|'])
                {
                    env_prefix = true;
                } else {
                    assignment = true;
                }
            }
        }
    }
    add(assignment, "assignment");
    add(env_prefix, "env_prefix");
    add(s.trim_start().starts_with("cd "), "cd_prefix");
    add(
        s.lines()
            .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
            .count()
            > 1,
        "multi_statement",
    );
    // Not measured by the corpus: rare by assumption.
    add(has_word(s, "coproc"), "coproc");
    add(has_word(s, "select"), "select");
    add(s.contains("${!"), "indirect_expansion");
    add(
        s.contains("declare -n") || s.contains("local -n") || s.contains("typeset -n"),
        "nameref",
    );
    add(
        ["@(", "!(", "+(", "*(", "?("].iter().any(|p| s.contains(p)) && s.contains("extglob"),
        "extglob",
    );
    add(s.contains("=~"), "regex_match");
    add(
        s.contains("set -o posix") || s.contains("POSIXLY_CORRECT"),
        "posix_mode",
    );
    f
}

/// Keywords after which a command follows.
const KEYWORDS: &[&str] = &["then", "do", "else", "elif", "time", "if", "while", "until"];

/// Words in command position: after the start of a line or `;`, `&`, `|`, `(`, `{`,
/// `!`, `then`, `do`, `else`, skipping assignments.
pub fn command_names(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in s.lines() {
        let mut expect = true;
        for raw in line.split_whitespace() {
            if raw.starts_with('#') {
                break;
            }
            let w = raw.trim_start_matches(['(', '{', '!']);
            if expect && !w.is_empty() {
                let is_assignment = w
                    .split_once('=')
                    .is_some_and(|(n, _)| !n.is_empty() && n.bytes().all(is_word_byte));
                if !is_assignment {
                    let name: String = w
                        .chars()
                        .take_while(|c| !matches!(c, ';' | '&' | '|' | ')' | '}' | '<' | '>'))
                        .collect();
                    if !name.is_empty()
                        && !KEYWORDS.contains(&name.as_str())
                        && !matches!(name.as_str(), "fi" | "done" | "esac" | "}")
                    {
                        out.push(name);
                        expect = false;
                    }
                }
            }
            if raw.ends_with(';')
                || raw.ends_with('&')
                || raw.ends_with('|')
                || raw == "&&"
                || raw == "||"
                || raw == "|"
                || KEYWORDS.contains(&raw)
                || matches!(raw, "{" | "(" | "!")
            {
                expect = true;
            }
        }
    }
    out
}

pub fn render_status(results: &[CaseResult], items: &[crate::Item], layers: &[Layer]) -> String {
    crate::report::markdown(results, items, layers)
}

/// Appends one line per layer to `tests/scores/status.tsv`.
pub fn append_history(
    root: &Path,
    results: &[CaseResult],
    items: &[crate::Item],
    layers: &[Layer],
) -> Result<(), String> {
    let path = root.join("tests/scores/status.tsv");
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let mut text = std::fs::read_to_string(&path).unwrap_or_default();
    if text.is_empty() {
        text.push_str(
            "# date (UTC)\tcommit\tlayer\tactive\tpass\tpass+stderr\tlater\tlater_pass\tskip\n",
        );
    }
    let (date, commit) = stamp(root);
    let counts = crate::report::layer_counts(results, items);
    for layer in layers {
        if let Some(c) = counts.get(layer) {
            let _ = writeln!(
                text,
                "{date}\t{commit}\t{layer}\t{}\t{}\t{}\t{}\t{}\t{}",
                c.active, c.pass, c.stderr_ok, c.later, c.later_pass, c.skip
            );
        }
    }
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The hit list: failing active cases by weight, and areas (source files) by the summed
/// weight of their failing cases.
pub fn hitlist(results: &[CaseResult], weights: &Weights, n: usize) -> String {
    let mut cases: Vec<(f64, &Case, String)> = results
        .iter()
        .filter(|r| r.group == Group::Active && !r.pass)
        .map(|r| {
            let (w, why) = weights.weigh_case(r.case);
            (w, r.case, why)
        })
        .collect();
    cases.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.id.cmp(&b.1.id)));
    let mut areas: BTreeMap<String, (f64, usize)> = BTreeMap::new();
    for (w, c, _) in &cases {
        let e = areas.entry(format!("{}/{}", c.layer, c.file)).or_default();
        e.0 += w;
        e.1 += 1;
    }
    let mut areas: Vec<(String, f64, usize)> =
        areas.into_iter().map(|(k, (w, n))| (k, w, n)).collect();
    areas.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut out = String::new();
    let _ = writeln!(
        out,
        "Hit list: {} failing active cases. Weight = corpus frequency of the rarest construct in the case (of {} commands).\n",
        cases.len(),
        weights.total
    );
    let _ = writeln!(out, "Areas (summed weight of failing cases):\n");
    let _ = writeln!(out, "{:>12}  {:>5}  area", "weight", "fails");
    for (area, w, n) in areas.iter().take(n) {
        let _ = writeln!(out, "{w:>12.0}  {n:>5}  {area}");
    }
    let _ = writeln!(out, "\nCases:\n");
    let _ = writeln!(
        out,
        "{:>8}  {:<36}  {:<22}  name",
        "weight", "case", "rarest construct"
    );
    for (w, c, why) in cases.iter().take(n) {
        let _ = writeln!(out, "{w:>8.0}  {:<36}  {:<22}  {}", c.id, why, c.name);
    }
    out
}

/// L0's scoreboard for `scripts/status`, and its line in `tests/scores/status.tsv`
/// (layer `l0`: inputs as active, passing inputs as pass).
pub fn l0_section(root: &Path, jobs: usize, append: bool) -> Result<String, String> {
    let (_, outcomes) = crate::l0::run_all(root, &[], jobs)?;
    let mut out = String::from(
        "\n## l0\n\n| Source | Inputs | Agree with bash -n | Round trip | Pass |\n| --- | ---: | ---: | ---: | ---: |\n",
    );
    let mut total = [0usize; 4];
    for (src, c) in crate::l0::counts(&outcomes) {
        let _ = writeln!(out, "| {src} | {} | {} | {} | {} |", c[0], c[1], c[2], c[3]);
        for i in 0..4 {
            total[i] += c[i];
        }
    }
    if append {
        let path = root.join("tests/scores/status.tsv");
        let mut text = std::fs::read_to_string(&path).unwrap_or_default();
        let (date, commit) = stamp(root);
        let _ = writeln!(
            text,
            "{date}\t{commit}\tl0\t{}\t{}\t-\t0\t0\t0",
            total[0], total[3]
        );
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(out)
}

fn stamp(root: &Path) -> (String, String) {
    let commit = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(root)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let date = std::process::Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    (date, commit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexical_features() {
        let f = features("cd /x && cat <<'EOF' | grep a 2>/dev/null\nbody\nEOF\n");
        for want in [
            "cd_prefix",
            "and",
            "heredoc",
            "heredoc_quoted",
            "pipe",
            "redir_devnull",
            "redir_stderr_devnull",
        ] {
            assert!(f.contains(&want), "{want} missing from {f:?}");
        }
        assert!(!f.contains(&"heredoc_unquoted"));
        assert!(!f.contains(&"background"));
        let f = features("x=${a:-b} ${#a} ${a[0]} $((1+2)) $(echo) sleep 1 &\n");
        for want in [
            "param_expansion_op",
            "array_ref",
            "arith_expansion",
            "cmd_subst",
            "background",
        ] {
            assert!(f.contains(&want), "{want} missing from {f:?}");
        }
        assert!(!features("echo ${a}").contains(&"param_expansion_op"));
        assert!(features("select x in a b; do :; done").contains(&"select"));
        assert!(features("x=1\n").contains(&"assignment"));
        assert!(features("FOO=1 cmd\n").contains(&"env_prefix"));
    }

    #[test]
    fn command_positions() {
        assert_eq!(
            command_names("FOO=1 echo hi | grep x && { trap 'x' EXIT; }\nif true; then read a; fi"),
            vec!["echo", "grep", "trap", "true", "read"]
        );
    }
}
