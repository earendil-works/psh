//! Generated regions of the guide's Markdown. A live example (```` ```sh live ````) is
//! followed by its output, as psh prints it, for readers of the plain Markdown:
//!
//! ````markdown
//! ```sh live
//! printf 'b\na\n' | sort
//! ```
//! <!-- gen:run -->
//! ```text
//! a
//! b
//! ```
//! <!-- /gen -->
//! ````
//!
//! `regen` writes a missing region; a region whose output changed is rewritten only by
//! `regen --examples`, and fails `check` until then, so that a change in psh's behavior
//! is seen by someone who checks that the page still says the right thing.

use std::collections::HashMap;

use crate::run::Run;

pub const OPEN: &str = "<!-- gen:run -->";
pub const CLOSE: &str = "<!-- /gen -->";

/// The live examples' runs, by code: each runs once per build.
#[derive(Default)]
pub struct Runs {
    cache: HashMap<String, Result<Run, String>>,
}

impl Runs {
    pub fn get(&mut self, code: &str) -> Result<&Run, String> {
        self.cache
            .entry(code.to_string())
            .or_insert_with(|| crate::run::run(code))
            .as_ref()
            .map_err(Clone::clone)
    }
}

pub struct Regen {
    /// The page with its regions written (changed outputs only when accepted).
    pub text: String,
    /// Live examples whose region is missing (written into `text`).
    pub missing: usize,
    /// Live examples whose output differs from their region.
    pub changed: usize,
    pub errors: Vec<String>,
}

/// The page's regions, regenerated. `accept`: also rewrite changed outputs.
pub fn regen(page: &str, src: &str, runs: &mut Runs, accept: bool) -> Regen {
    let lines: Vec<&str> = src.split_inclusive('\n').collect();
    let mut out = String::new();
    let mut r = Regen {
        text: String::new(),
        missing: 0,
        changed: 0,
        errors: Vec::new(),
    };
    let mut i = 0;
    let mut fence: Option<(char, usize)> = None;
    while i < lines.len() {
        let line = lines[i];
        out.push_str(line);
        i += 1;
        let t = line.trim_start();
        let run = |c: char| t.chars().take_while(|&x| x == c).count();
        if let Some((c, n)) = fence {
            if run(c) >= n && t.trim_end().chars().all(|x| x == c) {
                fence = None;
            }
            continue;
        }
        let Some(c) = ['`', '~'].into_iter().find(|&c| run(c) >= 3) else {
            continue;
        };
        let n = run(c);
        if !crate::render::is_live(t[n..].trim()) {
            fence = Some((c, n));
            continue;
        }
        // A live example: its code up to the closing fence.
        let mut code = String::new();
        while i < lines.len() {
            let l = lines[i];
            let lt = l.trim_start();
            let m = lt.chars().take_while(|&x| x == c).count();
            out.push_str(l);
            i += 1;
            if m >= n && lt.trim_end().chars().all(|x| x == c) {
                break;
            }
            code.push_str(l.trim_end_matches(['\n', '\r']));
            code.push('\n');
        }
        let new = match runs.get(&code) {
            Ok(run) => region(&run.text()),
            Err(e) => {
                r.errors
                    .push(format!("docs/guide/{page}: live example at line {i}: {e}"));
                continue;
            }
        };
        // Its region, if it has one: the next line opens it.
        let has = lines.get(i).is_some_and(|l| l.trim_end() == OPEN);
        if !has {
            r.missing += 1;
            out.push_str(OPEN);
            out.push('\n');
            out.push_str(&new);
            out.push_str(CLOSE);
            out.push('\n');
            continue;
        }
        let Some(end) = (i + 1..lines.len()).find(|&k| lines[k].trim_end() == CLOSE) else {
            r.errors.push(format!(
                "docs/guide/{page}:{}: {OPEN} without {CLOSE}",
                i + 1
            ));
            continue;
        };
        let old: String = lines[i + 1..end].concat();
        out.push_str(lines[i]);
        if old == new {
            out.push_str(&old);
        } else {
            r.changed += 1;
            out.push_str(if accept { &new } else { &old });
        }
        out.push_str(lines[end]);
        i = end + 1;
    }
    r.text = out;
    r
}

/// A region's body: the output in a `text` block, fenced longer than any backtick run
/// in it.
fn region(text: &str) -> String {
    let longest = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat((longest + 1).max(3));
    let mut s = format!("{fence}text\n{text}");
    if !text.is_empty() && !text.ends_with('\n') {
        s.push('\n');
    }
    s.push_str(&fence);
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_are_written_and_kept() {
        let mut runs = Runs::default();
        let md = "# T\n\n```sh live\necho hi\n```\n\nafter\n";
        let r = regen("p.md", md, &mut runs, false);
        assert_eq!(r.missing, 1);
        let want = "# T\n\n```sh live\necho hi\n```\n<!-- gen:run -->\n```text\nhi\n```\n<!-- /gen -->\n\nafter\n";
        assert_eq!(r.text, want);
        let again = regen("p.md", want, &mut runs, false);
        assert_eq!((again.missing, again.changed), (0, 0));
        assert_eq!(again.text, want);

        let stale = want.replace("```text\nhi\n", "```text\nho\n");
        let kept = regen("p.md", &stale, &mut runs, false);
        assert_eq!((kept.changed, kept.text.as_str()), (1, stale.as_str()));
        let accepted = regen("p.md", &stale, &mut runs, true);
        assert_eq!(accepted.text, want);
    }

    #[test]
    fn other_blocks_are_left_alone() {
        let mut runs = Runs::default();
        let md = "````\n```sh live\nnot run\n```\n````\n```sh\necho no\n```\n";
        let r = regen("p.md", md, &mut runs, true);
        assert_eq!((r.missing, r.text.as_str()), (0, md));
    }

    #[test]
    fn output_with_backticks() {
        assert_eq!(region("a ``` b"), "````text\na ``` b\n````\n");
        assert_eq!(region(""), "```text\n```\n");
    }
}
