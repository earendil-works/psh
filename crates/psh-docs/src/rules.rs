//! The guide's writing rules (docs/guide/README.md), for every page of the guide: the
//! pages SUMMARY.md lists and the front page.
//!
//! - Links to guide pages, their sections and repository files resolve.
//! - Self-contained: no mention or link of ARCH.md, TESTING.md, docs/*.md outside the
//!   guide, `work/`, `research/`, issue numbers (`0004`) or bare goal numbers (`G2`,
//!   "goal 3").
//! - No decision IDs (`D005`), no history phrases ("no longer", "was replaced", ...).
//! - pi is named only on pi.md.
//!
//! Code blocks are exempt; inline code is held to the word rules but not the history
//! phrases. Markdown is read by a small line scanner (fences, code spans, links and
//! headings), which is all the rules need, and which reports line numbers.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The one page that may name pi, the agent psh was built for.
const PI_PAGE: &str = "pi.md";

/// Phrases that tell history. "used to" is handled apart (`tells_history`), since "is
/// used to" describes the present.
const HISTORY: &[&str] = &[
    "no longer",
    "originally",
    "formerly",
    "was replaced",
    "were replaced",
    "has been replaced",
    "have been replaced",
    "was removed",
    "were removed",
    "earlier design",
    "the old design",
];

/// Checks every page SUMMARY.md lists, and `extra` (the front page). Returns the number
/// of pages and the errors.
pub fn check_guide(root: &Path, extra: &[&str]) -> (usize, Vec<String>) {
    let dir = root.join("docs/guide");
    let mut errors = Vec::new();
    let summary = match std::fs::read_to_string(dir.join("SUMMARY.md")) {
        Ok(s) => s,
        Err(e) => return (0, vec![format!("docs/guide/SUMMARY.md: {e}")]),
    };
    let mut sources = BTreeMap::new();
    let extra = extra.iter().map(|p| (0, p.to_string()));
    for (line, page) in summary_pages(&summary).into_iter().chain(extra) {
        match std::fs::read_to_string(dir.join(&page)) {
            Ok(src) => {
                sources.insert(page, src);
            }
            Err(_) => errors.push(format!(
                "docs/guide/SUMMARY.md:{line}: {page} does not exist"
            )),
        }
    }
    let ids: BTreeMap<String, BTreeSet<String>> = sources
        .iter()
        .map(|(p, src)| (p.clone(), heading_ids(src)))
        .collect();
    let exists = |rel: &str| root.join(rel).exists();
    let cx = Ctx {
        pages: &ids,
        exists: &exists,
    };
    for (page, src) in &sources {
        errors.extend(check_page(page, src, &cx));
    }
    (sources.len(), errors)
}

/// The pages of SUMMARY.md with their line numbers: `- [Title](page.md)`; an entry with
/// an empty target is a chapter not written yet.
fn summary_pages(summary: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (i, line) in summary.lines().enumerate() {
        let Some(open) = line.find("](") else {
            continue;
        };
        let rest = &line[open + 2..];
        if let Some(close) = rest.find(')') {
            let target = rest[..close].trim();
            if !target.is_empty() {
                out.push((i + 1, target.to_string()));
            }
        }
    }
    out
}

/// What a page is checked against: the guide's pages with their heading ids, and the
/// repository's files (by path from the repository root).
struct Ctx<'a> {
    pages: &'a BTreeMap<String, BTreeSet<String>>,
    exists: &'a dyn Fn(&str) -> bool,
}

/// A line of Markdown outside code blocks, taken apart.
#[derive(Default)]
struct Line {
    /// The text outside code spans, with link targets removed.
    prose: String,
    /// The contents of code spans.
    code: Vec<String>,
    /// Link targets: inline links and reference definitions.
    targets: Vec<String>,
}

/// The lines outside fenced code blocks, with their 1-based numbers.
fn prose_lines(src: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    for (i, line) in src.lines().enumerate() {
        let t = line.trim_start();
        let indent = line.len() - t.len();
        let run = |c: char| t.chars().take_while(|&x| x == c).count();
        match fence {
            Some((c, n)) => {
                if indent < 4 && run(c) >= n && t.trim_end().chars().all(|x| x == c) {
                    fence = None;
                }
            }
            None => {
                let opener = ['`', '~'].into_iter().find(|&c| run(c) >= 3);
                match opener {
                    Some(c) if indent < 4 => fence = Some((c, run(c))),
                    _ => out.push((i + 1, line)),
                }
            }
        }
    }
    out
}

/// Splits a line into prose, code spans and link targets.
fn parse_line(line: &str) -> Line {
    let mut l = Line::default();
    // A reference definition: `[label]: target`.
    let t = line.trim_start();
    if t.starts_with('[')
        && let Some(close) = t.find("]:")
    {
        let target = t[close + 2..].split_whitespace().next().unwrap_or("");
        l.targets.push(target.to_string());
        l.prose = t[..close + 1].to_string();
        return l;
    }
    // Code spans: a run of n backticks up to the next run of exactly n.
    let b = line.as_bytes();
    let mut text = String::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'`' {
            let n = b[i..].iter().take_while(|&&c| c == b'`').count();
            let start = i + n;
            let mut j = start;
            let mut end = None;
            while j < b.len() {
                if b[j] == b'`' {
                    let m = b[j..].iter().take_while(|&&c| c == b'`').count();
                    if m == n {
                        end = Some(j);
                        break;
                    }
                    j += m;
                } else {
                    j += 1;
                }
            }
            match end {
                Some(e) => {
                    l.code.push(line[start..e].to_string());
                    // Keep the span's place, so a link around it (`[`x`](y)`) stays whole.
                    text.push('\u{1}');
                    i = e + n;
                }
                None => {
                    text.push_str(&line[i..start]);
                    i = start;
                }
            }
        } else {
            let c = line[i..].chars().next().expect("char boundary");
            text.push(c);
            i += c.len_utf8();
        }
    }
    // Inline links: `](target)`, with balanced parentheses inside the target.
    let mut prose = String::new();
    let mut rest = text.as_str();
    while let Some(k) = rest.find("](") {
        prose.push_str(&rest[..k + 1]);
        let after = &rest[k + 2..];
        let mut depth = 0;
        let mut close = None;
        for (j, c) in after.char_indices() {
            match c {
                '(' => depth += 1,
                ')' if depth == 0 => {
                    close = Some(j);
                    break;
                }
                ')' => depth -= 1,
                _ => {}
            }
        }
        let Some(close) = close else {
            prose.push_str("](");
            rest = after;
            continue;
        };
        let target = after[..close].trim().trim_start_matches('<');
        let target = target.split_whitespace().next().unwrap_or("");
        l.targets.push(target.trim_end_matches('>').to_string());
        prose.push(' ');
        rest = &after[close + 1..];
    }
    prose.push_str(rest);
    l.prose = prose.replace('\u{1}', " ");
    l
}

/// The ids mdBook gives a page's headings: what `page.md#id` may point at.
pub fn heading_ids(src: &str) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for (_, line) in prose_lines(src) {
        let t = line.trim_start();
        let level = t.chars().take_while(|&c| c == '#').count();
        if !(1..=6).contains(&level) || !(t[level..].starts_with(' ') || t.len() == level) {
            continue;
        }
        let text = heading_text(t[level..].trim().trim_end_matches('#'));
        let base = slug(&text);
        let mut id = base.clone();
        let mut n = 1;
        while ids.contains(&id) {
            id = format!("{base}-{n}");
            n += 1;
        }
        ids.insert(id);
    }
    ids
}

/// A heading's visible text: code spans unwrapped, link targets and emphasis dropped.
fn heading_text(heading: &str) -> String {
    let mut out = String::new();
    let mut skip_target = false;
    let mut depth = 0;
    let mut prev = '\0';
    for c in heading.chars() {
        if skip_target {
            match c {
                '(' => depth += 1,
                ')' if depth == 0 => skip_target = false,
                ')' => depth -= 1,
                _ => {}
            }
        } else if c == '(' && prev == ']' {
            skip_target = true;
            depth = 0;
        } else if !matches!(c, '`' | '*' | '[' | ']') {
            out.push(c);
        }
        prev = c;
    }
    out
}

/// mdBook's heading ids: alphanumerics, `_` and `-` kept (ASCII lower-cased),
/// whitespace becomes `-`, everything else is dropped.
pub fn slug(text: &str) -> String {
    text.trim()
        .chars()
        .filter_map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                Some(c.to_ascii_lowercase())
            } else if c.is_whitespace() {
                Some('-')
            } else {
                None
            }
        })
        .collect()
}

/// All rule violations of one page, as `docs/guide/PAGE:LINE: message`.
fn check_page(page: &str, src: &str, cx: &Ctx) -> Vec<String> {
    let mut errors = Vec::new();
    for (n, raw) in prose_lines(src) {
        let line = parse_line(raw);
        let mut err = |m: String| errors.push(format!("docs/guide/{page}:{n}: {m}"));
        check_words(&line.prose, page, cx, &mut err);
        check_prose(&line.prose, &mut err);
        for c in &line.code {
            check_words(c, page, cx, &mut err);
        }
        for t in &line.targets {
            check_target(t, page, cx, &mut err);
        }
    }
    errors
}

/// The word rules, for prose and inline code alike.
fn check_words(text: &str, page: &str, cx: &Ctx, err: &mut impl FnMut(String)) {
    let seps = |c: char| c.is_whitespace() || "()[]{},;\"'`“”‘’<>|*!?".contains(c);
    for word in text.split(seps) {
        let word = word.trim_end_matches(['.', ':']);
        let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
        if word.len() == 4 && word.starts_with('0') && digits(word) {
            err(format!(
                "issue number {word}: the guide cannot point at issues; say what was found"
            ));
        } else if let Some(d) = word.strip_prefix('D')
            && (3..=4).contains(&d.len())
            && digits(d)
        {
            err(format!(
                "decision ID {word}: the guide cites no decisions; explain the reason in words"
            ));
        } else if let Some(g) = word.strip_prefix('G')
            && (1..=2).contains(&g.len())
            && digits(g)
        {
            err(format!(
                "bare goal number {word}: name the goal and link its section of goals.md"
            ));
        } else if outside_doc(word, cx) {
            err(format!(
                "mentions {word}, outside the guide: say what the page needs in its own words"
            ));
        }
    }
    // pi by name: whole words, so neither `pipe` nor `pipeline`.
    if page != PI_PAGE
        && text
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|w| w == "pi" || w == "Pi")
    {
        err(format!(
            "names pi: only {PI_PAGE} does; describe hosts in general (\"an agent host\")"
        ));
    }
}

/// Whether a word names a document outside the guide.
fn outside_doc(word: &str, cx: &Ctx) -> bool {
    let w = word.trim_start_matches("./");
    matches!(w, "ARCH" | "TESTING")
        || w.starts_with("work/")
        || w.starts_with("research/")
        || (w.starts_with("docs/") && !w.starts_with("docs/guide/"))
        || (w.ends_with(".md") && !cx.pages.contains_key(w.trim_start_matches("docs/guide/")))
}

/// The prose-only rules: history phrases and goals by number.
fn check_prose(text: &str, err: &mut impl FnMut(String)) {
    let lower = text.to_lowercase();
    for h in HISTORY {
        if lower.contains(h) {
            err(format!("\"{h}\" tells history: describe psh as it is"));
        }
    }
    if tells_history(&lower) {
        err("\"used to\" tells history: describe psh as it is".to_string());
    }
    let words: Vec<&str> = lower.split_whitespace().collect();
    for w in words.windows(2) {
        let n = w[1].trim_start_matches('#');
        if (w[0] == "goal" || w[0] == "goals") && n.starts_with(|c: char| c.is_ascii_digit()) {
            err(format!(
                "goal by number (\"{} {}\"): name the goal and link its section of goals.md",
                w[0], w[1]
            ));
        }
    }
}

/// "used to" as in "psh used to ...", not "is used to ..." (present passive).
fn tells_history(lower: &str) -> bool {
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    words.windows(2).enumerate().any(|(i, w)| {
        w == ["used", "to"]
            && !matches!(
                i.checked_sub(1).map(|k| words[k]),
                Some("is" | "are" | "be" | "been" | "being" | "get" | "gets" | "got")
            )
    })
}

/// A link target: a guide page and section, a repository file, or an external URL.
fn check_target(target: &str, page: &str, cx: &Ctx, err: &mut impl FnMut(String)) {
    if target.is_empty() {
        err("empty link target".to_string());
        return;
    }
    if ["http://", "https://", "mailto:"]
        .iter()
        .any(|p| target.starts_with(p))
    {
        return;
    }
    let (path, frag) = target.split_once('#').unwrap_or((target, ""));
    if path.is_empty() {
        section(page, frag, cx, err);
        return;
    }
    // Resolve against docs/guide to a path from the repository root.
    let mut parts: Vec<&str> = vec!["docs", "guide"];
    for c in path.split('/') {
        match c {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    err(format!("link to {target} leaves the repository"));
                    return;
                }
            }
            c => parts.push(c),
        }
    }
    let rel = parts.join("/");
    if let Some(p) = rel.strip_prefix("docs/guide/")
        && !p.contains('/')
        && p.ends_with(".md")
    {
        if cx.pages.contains_key(p) {
            if !frag.is_empty() {
                section(p, frag, cx, err);
            }
        } else {
            err(format!(
                "link to {target}, which is not a page of the guide (SUMMARY.md)"
            ));
        }
    } else if matches!(rel.as_str(), "ARCH.md" | "TESTING.md")
        || rel.starts_with("work/")
        || rel.starts_with("research/")
        || (rel.starts_with("docs/") && !rel.starts_with("docs/guide/"))
    {
        err(format!(
            "link to {target}, outside the guide: say what the page needs in its own words"
        ));
    } else if !(cx.exists)(&rel) {
        err(format!("link to {target}: {rel} does not exist"));
    }
}

fn section(page: &str, frag: &str, cx: &Ctx, err: &mut impl FnMut(String)) {
    if !cx.pages.get(page).is_some_and(|ids| ids.contains(frag)) {
        err(format!("link to {page}#{frag}: no such section"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOALS: &str = "# Goals\n\n## The goals\n\n### Agent-first\n\n### Bash's behavior\n\n\
                         ### `fast` *code*\n\n## Same\n\n## Same\n";

    fn errors(page: &str, src: &str) -> Vec<String> {
        let mut pages = BTreeMap::new();
        pages.insert("goals.md".to_string(), heading_ids(GOALS));
        pages.insert("pi.md".to_string(), BTreeSet::new());
        pages.insert(page.to_string(), heading_ids(src));
        let exists = |rel: &str| ["crates/psh-syntax/src/lib.rs", "crates"].contains(&rel);
        let cx = Ctx {
            pages: &pages,
            exists: &exists,
        };
        check_page(page, src, &cx)
            .into_iter()
            .map(|e| e.split_once(": ").expect("prefix").1.to_string())
            .collect()
    }

    fn short(errs: &[String]) -> Vec<&str> {
        errs.iter()
            .map(|e| e.split([':', ',']).next().expect("text"))
            .collect()
    }

    #[test]
    fn heading_ids_follow_mdbook() {
        let ids = heading_ids(GOALS);
        let want = [
            "goals",
            "the-goals",
            "agent-first",
            "bashs-behavior",
            "fast-code",
            "same",
            "same-1",
        ];
        assert_eq!(ids, want.iter().map(|s| s.to_string()).collect());
        assert!(heading_ids("```\n# not a heading\n```\n#nospace\n").is_empty());
    }

    #[test]
    fn links_resolve() {
        let md = "# T\n\n[a](goals.md#agent-first), [b](goals.md#bashs-behavior), [c](#t), \
                  [d](../../crates/psh-syntax/src/lib.rs), [e](https://example.com/x(y)), \
                  [`code`](../../crates/)\n\n[ref]: goals.md\n";
        assert_eq!(errors("p.md", md), Vec::<String>::new());
        let md = "# T\n\n[a](goals.md#portable) [b](nope.md) [c](#missing) \
                  [d](../../crates/nope.rs) [e](README.md) [f](../../../../x)\n";
        assert_eq!(
            short(&errors("p.md", md)),
            [
                "link to goals.md#portable",
                "link to nope.md",
                "link to p.md#missing",
                "link to ../../crates/nope.rs",
                "link to README.md",
                "link to ../../../../x leaves the repository",
            ]
        );
    }

    #[test]
    fn self_contained() {
        let md = "# T\n\nSee ARCH.md, `TESTING.md`, [x](../../ARCH.md), [y](../decisions.md), \
                  docs/host.md and work/issues/x and research/corpus.md. Issue 0004, G2 and \
                  goal 3.\n\n[r]: ../../work/BOARD.md\n";
        assert_eq!(
            short(&errors("p.md", md)),
            [
                "mentions ARCH.md",
                "mentions docs/host.md",
                "mentions work/issues/x",
                "mentions research/corpus.md",
                "issue number 0004",
                "bare goal number G2",
                "goal by number (\"goal 3.\")",
                "mentions TESTING.md",
                "link to ../../ARCH.md",
                "link to ../decisions.md",
                "link to ../../work/BOARD.md",
            ]
        );
    }

    #[test]
    fn no_decisions_no_history_pi_on_its_page() {
        let md = "# T\n\nAs D1050 says (`D016`), this was replaced. It no longer \
                  runs in pi or pi's adapter, and psh used to fork.\n\n\
                  A pipe is used to connect a pipeline; so is code:\n\n```sh\n# pi, D1050, no longer\n```\n";
        assert_eq!(
            short(&errors("p.md", md)),
            [
                "decision ID D1050",
                "names pi",
                "\"no longer\" tells history",
                "\"was replaced\" tells history",
                "\"used to\" tells history",
                "decision ID D016",
            ]
        );
        assert_eq!(
            errors("pi.md", "# pi\n\npi's adapter, `pi/vm.ts`.\n"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn code_spans_and_fences() {
        let md =
            "# T\n\n``a ` D123 ``, `unclosed\n\n~~~~\n```\nD123\n```\n~~~~\nok\n    ```\nD124\n";
        assert_eq!(
            short(&errors("p.md", md)),
            ["decision ID D123", "decision ID D124"]
        );
    }

    #[test]
    fn summary_entries() {
        let s = "# Part\n\n- [A](a.md)\n- [Later]()\n  - [B](sub/b.md)\n";
        assert_eq!(
            summary_pages(s),
            [(3, "a.md".to_string()), (5, "sub/b.md".to_string())]
        );
    }
}
