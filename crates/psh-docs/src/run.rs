//! The live examples at build time: each one runs in psh, in a fresh shell on the
//! engine's in-memory test host, with every utility in-process (sandbox mode), the way
//! psh.wasm runs it in the page (assets/live.js). Both start in an empty `/work` with the
//! same environment, so an example that makes its own files prints the same in both.

use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use psh_engine::testhost::TestHost;
use psh_engine::{HostFile, Script, Shell, Step};

/// The working directory and environment of a live example (live.js uses the same).
pub const CWD: &str = "/work";
pub const ENV: &[(&str, &str)] = &[("HOME", "/work"), ("PATH", "/usr/bin:/bin")];

/// How long an example may run before it counts as endless (live.js stops one after as
/// long).
const LIMIT: Duration = Duration::from_secs(5);

/// What an example printed: runs of bytes, in order (`true`: on stderr).
type Chunks = Vec<(bool, Vec<u8>)>;

/// What an example printed, in order, and its exit status.
#[derive(Debug, Default)]
pub struct Run {
    pub chunks: Chunks,
    pub status: i32,
}

impl Run {
    /// The text for the page and the Markdown's `gen:run` region: stdout and stderr as
    /// they interleaved, and a last line with the status when it is not 0.
    pub fn text(&self) -> String {
        let mut s = String::new();
        for (_, b) in &self.chunks {
            s.push_str(&String::from_utf8_lossy(b));
        }
        if self.status != 0 {
            if !s.is_empty() && !s.ends_with('\n') {
                s.push('\n');
            }
            s.push_str(&format!("(exit status {})\n", self.status));
        }
        s
    }
}

/// stdout or stderr of an example: appends to the shared transcript.
struct Sink {
    log: Rc<RefCell<Chunks>>,
    err: bool,
}

impl HostFile for Sink {
    fn poll_read(&self, _: &mut Context<'_>, _: &mut [u8]) -> Poll<io::Result<usize>> {
        Poll::Ready(Err(io::Error::from_raw_os_error(9)))
    }

    fn poll_write(&self, _: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let mut log = self.log.borrow_mut();
        match log.last_mut() {
            Some((e, b)) if *e == self.err => b.extend_from_slice(buf),
            _ => log.push((self.err, buf.to_vec())),
        }
        Poll::Ready(Ok(buf.len()))
    }
}

pub fn run(code: &str) -> Result<Run, String> {
    run_for(code, LIMIT)
}

fn run_for(code: &str, limit: Duration) -> Result<Run, String> {
    let host = Rc::new(TestHost::new());
    host.fs.mkdir_all(CWD);
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut cfg = host.config();
    cfg.utilities = psh_utils::policy(true);
    cfg.cwd = CWD.as_bytes().to_vec();
    cfg.env = ENV
        .iter()
        .map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec()))
        .collect();
    cfg.stdin = None;
    cfg.stdout = Some(Rc::new(Sink {
        log: log.clone(),
        err: false,
    }));
    cfg.stderr = Some(Rc::new(Sink {
        log: log.clone(),
        err: true,
    }));
    let mut shell = Shell::new(cfg);
    shell.start(Script::command(code));
    let t0 = Instant::now();
    while t0.elapsed() < limit {
        match shell.step(1000) {
            Step::Done(status) => {
                drop(shell);
                let chunks = log.take();
                return Ok(Run { chunks, status });
            }
            Step::Yield | Step::Blocked => {}
            Step::Idle => return Err("the shell went idle without a status".into()),
        }
    }
    Err(format!(
        "did not finish in {} s (an endless loop?)",
        limit.as_secs_f32()
    ))
}

/// The AST pane: `psh_syntax::outline` as a tree of rows, or the parse error. live.js
/// builds the same from `psh.parse(src, { outline: true })` and adds what needs
/// JavaScript: folding, the link between rows and the code, and the JSON view (whose
/// buttons show only then).
pub fn ast_html(code: &str) -> String {
    let body = match psh_syntax::parse(code) {
        Ok(ast) => tree_html(&psh_syntax::outline(&ast)),
        Err(e) => {
            let lc = psh_syntax::LineIndex::new(code).line_col(code, e.span.start);
            error_html(&e.message, lc.line, lc.column)
        }
    };
    // The search index skips the pane up to the marker (search.rs).
    format!("{}{body}<!-- /ast -->", view_bar())
}

fn view_bar() -> &'static str {
    "<div class=\"ast-bar\"><button data-view=\"tree\" aria-pressed=\"true\">Tree</button>\
     <button data-view=\"json\" aria-pressed=\"false\">JSON</button></div>"
}

fn error_html(message: &str, line: u32, column: u32) -> String {
    format!(
        "<p class=\"live-error\">Does not parse: {} (line {line}, column {column})</p>",
        crate::highlight::esc(message)
    )
}

/// One row per node, indented by depth (`--depth`), with its byte span for live.js;
/// `data-open` marks a row that has children.
fn tree_html(rows: &[psh_syntax::OutlineRow]) -> String {
    use crate::highlight::esc;
    if rows.is_empty() {
        return "<p class=\"muted\">(no commands)</p>".into();
    }
    let mut h = String::from("<div class=\"ast-tree\">");
    for (i, r) in rows.iter().enumerate() {
        let open = rows.get(i + 1).is_some_and(|n| n.depth > r.depth);
        h.push_str(&format!(
            "<div class=\"ast-row\" data-depth=\"{d}\"{} data-span=\"{} {}\" style=\"--depth:{d}\">\
             <span class=\"ast-twisty\"></span>",
            if open { " data-open=\"true\"" } else { "" },
            r.span.start,
            r.span.end,
            d = r.depth,
        ));
        if !r.role.is_empty() {
            h.push_str(&format!(" <span class=\"ast-role\">{}</span>", esc(r.role)));
        }
        h.push_str(&format!(" <span class=\"ast-kind\">{}</span>", esc(r.kind)));
        if !r.info.is_empty() {
            h.push_str(&format!(
                " <span class=\"ast-info\">{}</span>",
                esc(&r.info)
            ));
        }
        if !r.text.is_empty() {
            h.push_str(&format!(
                " <code class=\"ast-text\">{}</code>",
                esc(&r.text)
            ));
        }
        h.push_str("</div>");
    }
    h.push_str("</div>");
    h
}

/// The Classification pane of a live example: what the script would run, as psh's
/// classifier sees it (live.js builds the same from `psh.classify`).
pub fn classification_html(code: &str) -> String {
    use crate::highlight::esc;
    let c = psh_classify::classify(code);
    let yes_no = |b: bool| if b { "yes" } else { "no" };
    let mut h = format!(
        "<p>Primary category <b>{}</b> · read-only: <b>{}</b> · output cut short: <b>{}</b></p>",
        c.primary().name(),
        yes_no(c.read_only()),
        yes_no(c.truncated)
    );
    if let Some(e) = &c.error {
        h.push_str(&format!(
            "<p class=\"live-error\">Does not parse: {}</p>",
            esc(&e.message)
        ));
        return h;
    }
    let text_of = |source: Option<usize>, s: psh_classify::Span| -> String {
        let t = match source {
            None => code,
            Some(i) => c.sources[i].text.as_str(),
        };
        t.get(s.start as usize..s.end as usize)
            .unwrap_or("")
            .to_string()
    };
    if !c.commands.is_empty() {
        h.push_str("<table class=\"commands\"><thead><tr><th>Command</th><th>Category</th><th>Runs</th></tr></thead><tbody>");
        for x in &c.commands {
            let cats: Vec<&str> = x.categories.iter().map(|k| k.name()).collect();
            h.push_str(&format!(
                "<tr><td><code>{}</code></td><td>{}</td><td>{}</td></tr>",
                esc(&text_of(x.source, x.span)),
                esc(&cats.join(", ")),
                esc(x.via.name())
            ));
        }
        h.push_str("</tbody></table>");
    }
    let paths = |v: &[psh_classify::PathEffect]| -> String {
        v.iter()
            .map(|p| match &p.path {
                Some(p) => format!("<code>{}</code>", esc(p)),
                None => "<i>dynamic</i>".to_string(),
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    if !c.reads.is_empty() {
        h.push_str(&format!("<p>Reads {}</p>", paths(&c.reads)));
    }
    if !c.writes.is_empty() {
        h.push_str(&format!("<p>Writes {}</p>", paths(&c.writes)));
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_in_an_empty_work_directory() {
        let r = run("pwd; printf 'b\\na\\n' > f; sort f; echo oops >&2; exit 3").unwrap();
        assert_eq!(r.text(), "/work\na\nb\noops\n(exit status 3)\n");
        assert_eq!(r.chunks.last().map(|c| c.0), Some(true));
    }

    #[test]
    fn endless_loops_end() {
        assert!(run_for("while :; do :; done", Duration::from_millis(100)).is_err());
    }

    #[test]
    fn ast_is_an_outline_and_escapes_source() {
        let h = ast_html("echo \"$name\" | cat > '<script>&'");
        assert!(h.starts_with("<div class=\"ast-bar\">"), "{h}");
        assert!(h.ends_with("</div><!-- /ast -->"), "{h}");
        assert!(
            h.contains(
                "<div class=\"ast-row\" data-depth=\"0\" data-open=\"true\" data-span=\"0 32\" \
                 style=\"--depth:0\"><span class=\"ast-twisty\"></span> <span class=\"ast-kind\">\
                 pipeline</span> <span class=\"ast-info\">|</span> <code class=\"ast-text\">"
            ),
            "{h}"
        );
        assert!(
            h.contains(
                "<div class=\"ast-row\" data-depth=\"3\" data-span=\"6 11\" style=\"--depth:3\">\
                 <span class=\"ast-twisty\"></span> <span class=\"ast-kind\">param</span> \
                 <span class=\"ast-info\">name</span> <code class=\"ast-text\">$name</code></div>"
            ),
            "{h}"
        );
        assert!(h.contains("&lt;script&gt;&amp;"), "{h}");
        assert!(!h.contains("<script>"), "{h}");
        assert!(ast_html("# nothing").contains("(no commands)"));
    }

    #[test]
    fn ast_includes_parse_errors() {
        let h = ast_html("echo a\nfi");
        assert!(
            h.contains(
                "<p class=\"live-error\">Does not parse: syntax error near unexpected token \
                 `fi' (line 2, column 1)</p>"
            ),
            "{h}"
        );
        assert!(!h.contains("ast-tree"), "{h}");
    }

    #[test]
    fn classification() {
        let h = classification_html("rm -rf build; ls | head -1");
        assert!(h.contains("Primary category <b>mutate</b>"), "{h}");
        assert!(
            h.contains("<td><code>rm -rf build</code></td><td>mutate</td>"),
            "{h}"
        );
        assert!(h.contains("Writes <code>build</code>"), "{h}");
    }
}
