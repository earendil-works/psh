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

/// The AST pane uses exactly the CLI/WASM parser JSON, including parse errors.
pub fn ast_html(code: &str) -> String {
    let (json, _) =
        crate::parse_json::parse_to_json(code.as_bytes(), psh_syntax::Options::default());
    crate::highlight::highlight("json", &json.to_pretty())
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
    fn ast_includes_structure_and_escapes_source() {
        let h = ast_html("echo \"$name\" | cat > '<script>&'");
        for text in [
            "list", "pipeline", "simple", "word", "param", "redirect", "span",
        ] {
            assert!(h.contains(text), "{text} in {h}");
        }
        assert!(h.contains("tok-string"), "{h}");
        assert!(h.contains("\n  "), "{h}");
        assert!(h.contains("&lt;script&gt;&amp;"), "{h}");
        assert!(!h.contains("<script>"), "{h}");
    }

    #[test]
    fn ast_includes_parse_errors() {
        let h = ast_html("if");
        for text in ["error", "message", "span", "line", "column", "false"] {
            assert!(h.contains(text), "{text} in {h}");
        }
        assert!(!h.contains("\"script\""), "{h}");
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
