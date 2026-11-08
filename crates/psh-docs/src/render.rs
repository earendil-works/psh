//! Markdown to HTML: pulldown-cmark plus what the guide needs on top: heading anchors
//! (the ids the writing rules check links against) and a page outline, links to other
//! pages and to the repository's files, the "In short" and "Trade-off" asides, syntax
//! highlighting, and live examples.
//!
//! Links are checked by the writing rules (rules.rs, with line numbers); here they are
//! only resolved.

use std::collections::BTreeSet;

use pulldown_cmark::{
    CodeBlockKind, CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd, html,
};

pub use crate::highlight::esc;
use crate::highlight::highlight;

/// Where links to repository files point on the site.
pub const REPO_URL: &str = "https://github.com/earendil-works/psh";

pub struct Page {
    pub title: String,
    pub html: String,
    /// (level, id, text) of the h2 and h3 headings.
    pub toc: Vec<(u8, String, String)>,
}

pub struct Env<'a> {
    /// The guide's pages (`engine.md`), for links between them.
    pub pages: &'a BTreeSet<String>,
    /// The commit source links point at (`source_url`).
    pub rev: &'a str,
    /// The live examples' build-time runs.
    pub runs: &'a mut crate::regions::Runs,
    pub errors: &'a mut Vec<String>,
}

pub fn html_name(md: &str) -> String {
    format!("{}.html", md.trim_end_matches(".md"))
}

fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_SMART_PUNCTUATION
        | Options::ENABLE_GFM
}

/// The info string of a live example: ```` ```sh live ````.
pub fn is_live(info: &str) -> bool {
    let mut w = info.split_whitespace();
    matches!(w.next(), Some("sh" | "bash")) && w.any(|w| w == "live")
}

pub fn render(page: &str, md: &str, env: &mut Env) -> Page {
    let events: Vec<Event> = Parser::new_ext(md, options()).collect();
    let mut out: Vec<Event> = Vec::new();
    let mut toc = Vec::new();
    let mut title = String::new();
    let mut ids = BTreeSet::new();
    let mut live_n = 0;
    let mut i = 0;
    while i < events.len() {
        match &events[i] {
            Event::Start(Tag::Heading { level, .. }) => {
                let lvl = *level as u8;
                let mut j = i + 1;
                let mut text = String::new();
                while !matches!(events[j], Event::End(TagEnd::Heading(_))) {
                    if let Event::Text(t) | Event::Code(t) = &events[j] {
                        text.push_str(t);
                    }
                    j += 1;
                }
                // The ids mdBook makes, as the writing rules check links against them.
                let base = crate::rules::slug(&text);
                let mut id = base.clone();
                let mut n = 1;
                while ids.contains(&id) {
                    id = format!("{base}-{n}");
                    n += 1;
                }
                ids.insert(id.clone());
                if *level == HeadingLevel::H1 && title.is_empty() {
                    title = text.clone();
                } else if (2..=3).contains(&lvl) {
                    toc.push((lvl, id.clone(), text.clone()));
                }
                let mut inner = String::new();
                html::push_html(&mut inner, events[i + 1..j].iter().cloned());
                out.push(Event::Html(CowStr::from(format!(
                    "<h{lvl} id=\"{id}\"><a class=\"anchor\" href=\"#{id}\">{inner}</a></h{lvl}>\n"
                ))));
                i = j + 1;
                continue;
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let info = match kind {
                    CodeBlockKind::Fenced(s) => s.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                let mut j = i + 1;
                let mut code = String::new();
                while !matches!(events[j], Event::End(TagEnd::CodeBlock)) {
                    if let Event::Text(t) = &events[j] {
                        code.push_str(t);
                    }
                    j += 1;
                }
                let lang = info.split_whitespace().next().unwrap_or("");
                let live = is_live(&info);
                let hl = highlight(lang, &code);
                let html = if live {
                    live_n += 1;
                    live_block(page, live_n, &hl, &code, env)
                } else {
                    format!("<pre class=\"code\"><code class=\"lang-{lang}\">{hl}</code></pre>\n")
                };
                out.push(Event::Html(CowStr::from(html)));
                i = j + 1;
                // The `gen:run` region right after a live block is its output for readers
                // of the plain Markdown; the example shows it already. (HTML blocks come
                // as Start(HtmlBlock), Html..., End(HtmlBlock).)
                if live {
                    let has = |e: Option<&Event>, s: &str| matches!(e, Some(Event::Html(h)) if h.contains(s));
                    if matches!(events.get(i), Some(Event::Start(Tag::HtmlBlock)))
                        && has(events.get(i + 1), "<!-- gen:run")
                    {
                        while i < events.len() && !has(events.get(i), "<!-- /gen -->") {
                            i += 1;
                        }
                        i += 1;
                        if matches!(events.get(i), Some(Event::End(TagEnd::HtmlBlock))) {
                            i += 1;
                        }
                    }
                }
                continue;
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => {
                let dest = resolve_link(dest_url, env);
                out.push(Event::Start(Tag::Link {
                    link_type: *link_type,
                    dest_url: CowStr::from(dest),
                    title: title.clone(),
                    id: id.clone(),
                }));
                i += 1;
                continue;
            }
            _ => {}
        }
        out.push(events[i].clone());
        i += 1;
    }
    let mut body = String::new();
    html::push_html(&mut body, out.into_iter());
    // Source links are marked, so that readers can tell them from pages.
    let body = body.replace(
        &format!("<a href=\"{REPO_URL}/"),
        &format!("<a class=\"source\" href=\"{REPO_URL}/"),
    );
    // "> **Trade-off.** ...", "> **In short.** ..." and "> **Not built yet.** ...": plain
    // quotes on GitHub, asides here.
    let body = body
        .replace(
            "<blockquote>\n<p><strong>Trade-off",
            "<blockquote class=\"tradeoff\">\n<p><strong>Trade-off",
        )
        .replace(
            "<blockquote>\n<p><strong>In short",
            "<blockquote class=\"short\">\n<p><strong>In short",
        )
        .replace(
            "<blockquote>\n<p><strong>Not built yet",
            "<blockquote class=\"unbuilt\">\n<p><strong>Not built yet",
        );
    if title.is_empty() {
        env.errors
            .push(format!("docs/guide/{page}: no title (# heading)"));
    }
    Page {
        title,
        html: body,
        toc,
    }
}

/// A live example. The markup is the page without JavaScript: the highlighted code and
/// the output from the build. live.js turns the code into an editor (a textarea over
/// this `pre`, which keeps showing the highlighting), shows Run and Reset, runs the
/// code in psh.wasm and makes the tabs work.
fn live_block(page: &str, n: usize, hl: &str, code: &str, env: &mut Env) -> String {
    let (out, classify) = match env.runs.get(code) {
        Ok(r) => (output_html(r), crate::run::classification_html(code)),
        Err(e) => {
            env.errors
                .push(format!("docs/guide/{page}: live example {n}: {e}"));
            (String::new(), String::new())
        }
    };
    let tab = |id: &str, label: &str, selected: bool| {
        format!(
            "<button role=\"tab\" id=\"live-{n}-tab-{id}\" data-tab=\"{id}\" aria-controls=\"live-{n}-{id}\" \
             aria-selected=\"{selected}\" tabindex=\"{}\"{}>{label}</button>",
            if selected { 0 } else { -1 },
            // Without JavaScript the tabs cannot switch: only Output shows.
            if selected { "" } else { " hidden" }
        )
    };
    let pane = |tag: &str, id: &str, hidden: bool, body: &str| {
        format!(
            "<{tag} class=\"live-pane\" data-pane=\"{id}\" id=\"live-{n}-{id}\" role=\"tabpanel\" \
             aria-labelledby=\"live-{n}-tab-{id}\" tabindex=\"0\"{}>{body}</{tag}>\n",
            if hidden { " hidden" } else { "" }
        )
    };
    format!(
        "<div class=\"live\" id=\"live-{n}\">\n\
         <div class=\"live-bar\"><span class=\"live-badge\">live</span>\
         <span class=\"live-note\">output from the build</span>\
         <button class=\"live-reset\" hidden>Reset</button>\
         <button class=\"live-run\" hidden>Run</button></div>\n\
         <div class=\"live-editor\"><pre class=\"code\"><code class=\"lang-sh\">{hl}</code></pre></div>\n\
         <div class=\"live-tabs\" role=\"tablist\" aria-label=\"Results\">{}{}{}\
         <span class=\"live-status\" role=\"status\"></span></div>\n\
         {}{}{}</div>\n",
        tab("out", "Output", true),
        tab("classify", "Classification", false),
        tab("ast", "AST", false),
        pane("pre", "out", false, &out),
        pane("div", "classify", true, &classify),
        pane("div", "ast", true, &crate::run::ast_html(code)),
    )
}

/// The Output pane: stdout and stderr as they interleaved, stderr marked, and the exit
/// status when it is not 0 (live.js writes the same).
pub fn output_html(r: &crate::run::Run) -> String {
    let mut h = String::new();
    let mut nl = true;
    for (err, b) in &r.chunks {
        let t = String::from_utf8_lossy(b);
        nl = t.ends_with('\n');
        let t = esc(&t);
        if *err {
            h.push_str(&format!("<span class=\"live-stderr\">{t}</span>"));
        } else {
            h.push_str(&t);
        }
    }
    if r.status != 0 {
        if !nl {
            h.push('\n');
        }
        h.push_str(&format!(
            "<span class=\"live-exit\">(exit status {})</span>",
            r.status
        ));
    }
    if h.is_empty() {
        h.push_str("<span class=\"muted\">(no output)</span>");
    }
    h
}

/// A link to another page becomes its `.html`; a relative link to a repository file goes
/// to the file on GitHub.
fn resolve_link(dest: &str, env: &Env) -> String {
    if dest.contains("://") || dest.starts_with("mailto:") || dest.starts_with('#') {
        return dest.to_string();
    }
    let (path, frag) = match dest.split_once('#') {
        Some((p, f)) => (p, format!("#{f}")),
        None => (dest, String::new()),
    };
    if env.pages.contains(path) {
        return format!("{}{frag}", html_name(path));
    }
    // Relative to docs/guide.
    let mut parts: Vec<&str> = vec!["docs", "guide"];
    for c in path.split('/') {
        match c {
            ".." => {
                parts.pop();
            }
            "." | "" => {}
            c => parts.push(c),
        }
    }
    let rel = parts.join("/");
    let dir = path.ends_with('/') || crate::repo().join(&rel).is_dir();
    source_url(&rel, dir, &frag, env.rev)
}

/// The one place source links are made: a repository path on GitHub, at the commit the
/// site is built from, so that the code a page points at is the code it describes.
pub fn source_url(rel: &str, dir: bool, frag: &str, rev: &str) -> String {
    let kind = if dir { "tree" } else { "blob" };
    format!("{REPO_URL}/{kind}/{rev}/{rel}{frag}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_md(md: &str) -> (Page, Vec<String>) {
        let pages: BTreeSet<String> = ["engine.md".to_string(), "p.md".to_string()].into();
        let mut runs = crate::regions::Runs::default();
        let mut errors = Vec::new();
        let p = render(
            "p.md",
            md,
            &mut Env {
                pages: &pages,
                rev: "main",
                runs: &mut runs,
                errors: &mut errors,
            },
        );
        (p, errors)
    }

    #[test]
    fn links_and_headings() {
        let md = "# Title\n\nSee [the engine](engine.md#steps), [code](../../crates/psh-engine/src/lib.rs), \
                  [dir](../../crates/), [x](https://example.com).\n\n## Sub `code`\n\n## Sub `code`\n";
        let (p, errors) = render_md(md);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(p.title, "Title");
        assert!(p.html.contains("href=\"engine.html#steps\""));
        assert!(p.html.contains(&format!(
            "<a class=\"source\" href=\"{REPO_URL}/blob/main/crates/psh-engine/src/lib.rs\""
        )));
        assert!(
            p.html
                .contains(&format!("href=\"{REPO_URL}/tree/main/crates\""))
        );
        assert_eq!(
            p.toc,
            [
                (2, "sub-code".to_string(), "Sub code".to_string()),
                (2, "sub-code-1".to_string(), "Sub code".to_string())
            ]
        );
    }

    #[test]
    fn asides() {
        let md = "# T\n\n> **Trade-off.** x\n\n> **In short.** y\n\n> **Not built yet.** z\n";
        let (p, _) = render_md(md);
        assert!(p.html.contains("<blockquote class=\"tradeoff\">"));
        assert!(p.html.contains("<blockquote class=\"short\">"));
        assert!(p.html.contains("<blockquote class=\"unbuilt\">"));
    }

    #[test]
    fn live_blocks_carry_their_output() {
        let md = "# T\n\n```sh live\necho hi; echo err >&2; exit 2\n```\n<!-- gen:run -->\n```text\nhi\n```\n<!-- /gen -->\n\nafter\n";
        let (p, errors) = render_md(md);
        assert!(errors.is_empty(), "{errors:?}");
        for want in [
            "<div class=\"live-editor\"><pre class=\"code\"><code class=\"lang-sh\"><span class=\"tok-function\">echo</span>",
            "data-pane=\"out\" id=\"live-1-out\" role=\"tabpanel\" aria-labelledby=\"live-1-tab-out\" tabindex=\"0\">hi\n<span class=\"live-stderr\">err\n</span><span class=\"live-exit\">(exit status 2)</span></pre>",
            "data-pane=\"classify\"",
            "data-tab=\"ast\" aria-controls=\"live-1-ast\"",
            "data-pane=\"ast\" id=\"live-1-ast\" role=\"tabpanel\" aria-labelledby=\"live-1-tab-ast\" tabindex=\"0\" hidden>",
            "<span class=\"ast-kind\">simple</span>",
        ] {
            assert!(p.html.contains(want), "{want} in {}", p.html);
        }
        // The region's copy of the output is not rendered a second time.
        assert!(!p.html.contains("lang-text"), "{}", p.html);
        assert!(p.html.contains("<p>after</p>"));
    }
}
