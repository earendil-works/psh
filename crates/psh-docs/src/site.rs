//! The static site: SUMMARY.md navigation, the page template, assets, and the live
//! examples' files (psh.wasm and js/psh.mjs).

use std::fmt::Write as _;
use std::path::Path;

use crate::render::{self, Page, esc, html_name};
use crate::{Guide, repo};

const STYLE: &str = include_str!("../assets/style.css");
const LIVE: &str = include_str!("../assets/live.js");
const HIGHLIGHT: &str = include_str!("../assets/highlight.js");
const SEARCH: &str = include_str!("../assets/search.js");
const PAGE: &str = include_str!("../assets/page.js");
/// Elanor's published files, vendored by scripts/elanor-update.sh.
const ELANOR_DIR: &str = "crates/psh-docs/assets/elanor";
const ICON_SEARCH: &str = include_str!("../assets/elanor/icons/search.svg");
const ICON_DARK: &str = include_str!("../assets/elanor/icons/dark-mode.svg");
const ICON_LIGHT: &str = include_str!("../assets/elanor/icons/light-mode.svg");
const ICON_MENU: &str = include_str!("../assets/elanor/icons/menu.svg");

/// The site's front page (`index.html`): a short summary of psh, not a chapter.
pub const LANDING: &str = "index.md";

/// One line of SUMMARY.md: `# Part`, `- [Title](page.md)`, or a draft `- [Title]()`.
pub struct Entry {
    pub depth: usize,
    pub title: String,
    pub path: Option<String>,
    pub part: bool,
}

pub fn summary(s: &str) -> Vec<Entry> {
    let mut v = Vec::new();
    for line in s.lines() {
        if let Some(p) = line.strip_prefix("# ") {
            v.push(Entry {
                depth: 0,
                title: p.trim().into(),
                path: None,
                part: true,
            });
        } else if let Some(i) = line.find("- [") {
            let Some((title, rest)) = line[i + 3..].split_once("](") else {
                continue;
            };
            let path = rest.trim_end().trim_end_matches(')');
            v.push(Entry {
                depth: i / 2,
                title: title.into(),
                path: (!path.is_empty()).then(|| path.into()),
                part: false,
            });
        }
    }
    v
}

/// Renders the guide's pages: (source name, page).
pub fn render_all(g: &mut Guide) -> Vec<(String, Page)> {
    let pages = g.pages.keys().cloned().collect();
    let rev = source_rev(&repo());
    let mut out = Vec::new();
    for (name, src) in &g.pages {
        let mut env = render::Env {
            pages: &pages,
            rev: &rev,
            runs: &mut g.runs,
            errors: &mut g.errors,
        };
        let page = render::render(name, src, &mut env);
        out.push((name.clone(), page));
    }
    out
}

/// The commit source links point at: `$GITHUB_SHA` when a CI job builds the site, else
/// the checkout's HEAD, else `main`.
fn source_rev(repo: &Path) -> String {
    if let Ok(sha) = std::env::var("GITHUB_SHA")
        && !sha.is_empty()
    {
        return sha;
    }
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "main".to_string())
}

pub fn build(g: &mut Guide, out: &Path, wasm: &Path) -> Result<(), String> {
    let mut rendered = render_all(g);
    // The front page only summarizes the chapters: search finds them instead.
    let landing = rendered
        .iter()
        .position(|(n, _)| n == LANDING)
        .map(|i| rendered.remove(i));
    let rev = source_rev(&repo());
    std::fs::create_dir_all(out.join("psh")).map_err(|e| format!("{}: {e}", out.display()))?;
    let order: Vec<&Entry> = g.summary.iter().filter(|e| e.path.is_some()).collect();
    let write = |name: &str, text: &str| {
        std::fs::write(out.join(name), text).map_err(|e| format!("{name}: {e}"))
    };
    for (name, page) in rendered.iter().chain(&landing) {
        let pos = order
            .iter()
            .position(|e| e.path.as_deref() == Some(name.as_str()));
        write(
            &html_name(name),
            &template(&g.summary, name, page, pos.map(|i| (i, &order[..])), &rev),
        )?;
    }
    write("style.css", STYLE)?;
    write("page.js", PAGE)?;
    copy_dir(&repo().join(ELANOR_DIR), &out.join("elanor"))?;
    write("live.js", LIVE)?;
    write("highlight.js", HIGHLIGHT)?;
    write("search.js", SEARCH)?;
    write("search.json", &crate::search::index(&rendered, html_name))?;
    // The live examples: the shell module and its glue.
    let psh = out.join("psh");
    std::fs::copy(repo().join("js/psh.mjs"), psh.join("psh.mjs"))
        .map_err(|e| format!("js/psh.mjs: {e}"))?;
    if wasm.exists() {
        std::fs::copy(wasm, psh.join("psh.wasm"))
            .map_err(|e| format!("{}: {e}", wasm.display()))?;
    } else {
        eprintln!(
            "psh-docs: {} is missing (scripts/build-wasm): the live examples will show their build-time output only",
            wasm.display()
        );
    }
    println!(
        "psh-docs: {} pages in {}",
        rendered.len() + landing.iter().len(),
        out.display()
    );
    Ok(())
}

/// Copies the vendored Elanor files (tokens, fonts and their licenses, icons, logos).
fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
    let rd = std::fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))?;
    for e in rd.flatten() {
        let (src, dst) = (e.path(), to.join(e.file_name()));
        if src.is_dir() {
            copy_dir(&src, &dst)?;
        } else {
            std::fs::copy(&src, &dst).map_err(|e| format!("{}: {e}", src.display()))?;
        }
    }
    Ok(())
}

/// An Elanor icon inline, so that it takes the text color (`.icon` in style.css).
fn icon(svg: &str, class: &str) -> String {
    let at = svg.find("<svg").unwrap_or(0);
    svg[at..].trim_end().replacen(
        "<svg ",
        &format!("<svg class=\"icon {class}\" aria-hidden=\"true\" focusable=\"false\" "),
        1,
    )
}

fn template(
    entries: &[Entry],
    current: &str,
    page: &Page,
    pos: Option<(usize, &[&Entry])>,
    rev: &str,
) -> String {
    let landing = current == LANDING;
    let mut nav = String::from("<ol>");
    for e in entries.iter().filter(|_| !landing) {
        if e.part {
            let _ = write!(nav, "<li class=\"part\">{}</li>", esc(&e.title));
            continue;
        }
        let cls = format!("d{}", e.depth);
        match &e.path {
            Some(p) => {
                let cur = if p == current {
                    " aria-current=\"page\""
                } else {
                    ""
                };
                let _ = write!(
                    nav,
                    "<li class=\"{cls}\"><a href=\"{}\"{cur}>{}</a></li>",
                    html_name(p),
                    esc(&e.title)
                );
            }
            None => {
                let _ = write!(nav, "<li class=\"{cls} draft\">{}</li>", esc(&e.title));
            }
        }
    }
    nav.push_str("</ol>");
    let mut toc = String::new();
    if !page.toc.is_empty() {
        toc.push_str("<p class=\"toc-title\">On this page</p><ol>");
        for (lvl, id, text) in page.toc.iter().take(80) {
            let _ = write!(
                toc,
                "<li class=\"h{lvl}\"><a href=\"#{id}\">{}</a></li>",
                esc(text)
            );
        }
        toc.push_str("</ol>");
    }
    let mut pager = String::new();
    if let Some((i, order)) = pos {
        pager.push_str("<nav class=\"pager\">");
        let link = |class: &str, label: &str, e: &Entry| {
            format!(
                "<a class=\"{class}\" href=\"{}\"><small>{label}</small>{}</a>",
                html_name(e.path.as_deref().unwrap_or_default()),
                esc(&e.title)
            )
        };
        if i > 0 {
            pager.push_str(&link("prev", "Previous", order[i - 1]));
        }
        if let Some(e) = order.get(i + 1) {
            pager.push_str(&link("next", "Next", e));
        }
        pager.push_str("</nav>");
    }
    // The page's Markdown, at the commit the site was built from, like every other source
    // link.
    let edit = render::source_url(&format!("docs/guide/{current}"), false, "", rev);
    let title = if landing {
        "psh: a portable bash for the commands agents write".to_string()
    } else {
        format!("{} · The psh guide", esc(&page.title))
    };
    let (layout, nav, toc) = if landing {
        ("layout landing", String::new(), String::new())
    } else {
        (
            "layout",
            format!("<nav class=\"book\" aria-label=\"Chapters\">{nav}</nav>"),
            format!("<aside class=\"toc\">{toc}</aside>"),
        )
    };
    format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<link rel="icon" href="elanor/logos/favicon.svg" type="image/svg+xml">
<link rel="stylesheet" href="elanor/tokens.css">
<link rel="stylesheet" href="elanor/fonts/mallos-serif.css">
<link rel="stylesheet" href="elanor/fonts/commit-mono.css">
<link rel="stylesheet" href="style.css">
<script>try{{const s=localStorage.getItem("psh-color-scheme");if(s)document.documentElement.dataset.colorScheme=s}}catch{{}}</script>
</head>
<body>
<a class="skip" href="#main">Skip to content</a>
<header class="top">
{menu}<div class="identity"><a class="mark" href="https://earendil.com" title="Earendil"><img class="on-light" src="elanor/logos/earendil-logo-on-light.svg" alt="Earendil"><img class="on-dark" src="elanor/logos/earendil-logo-on-dark.svg" alt="Earendil"></a><a class="brand" href="index.html"><b>psh</b><span>The psh guide</span></a></div>
<span class="spacer"></span>
<div class="search">{search_icon}<input type="search" id="search" placeholder="Search  /" aria-label="Search the guide" autocomplete="off"><ol id="search-results" hidden></ol></div>
<div class="actions"><a class="icon-button src" href="{edit}" title="This page's source">Source</a><button class="icon-button scheme" type="button" aria-label="Switch between light and dark">{to_dark}{to_light}</button></div>
</header>
<div class="{layout}">
{nav}
<main id="main" tabindex="-1"><div class="page"><article data-text-context="prose">
{body}
</article>{pager}
<footer class="foot"><p>psh is built by <a href="https://earendil.com">Earendil</a>. <a href="{repo}">Source on GitHub</a>.</p></footer></div>
{toc}</main>
</div>
<script type="module" src="page.js"></script>
{live}<script type="module" src="search.js"></script>
</body>
</html>
"##,
        // Wide tables scroll inside their wrapper instead of widening the column.
        body = page
            .html
            .replace("<table>", "<div class=\"table-scroll\"><table>")
            .replace("</table>\n", "</table></div>\n"),
        // Only pages with live examples load their editor (live.js loads psh.wasm itself,
        // on first use).
        live = if page.html.contains("<div class=\"live\"") {
            "<script type=\"module\" src=\"live.js\"></script>\n"
        } else {
            ""
        },
        repo = render::REPO_URL,
        // The front page has no chapter list to open.
        menu = if landing {
            String::new()
        } else {
            format!(
                "<button class=\"icon-button menu-button\" type=\"button\" aria-label=\"Chapters\" aria-expanded=\"false\">{}</button>\n",
                icon(ICON_MENU, "menu-icon")
            )
        },
        search_icon = icon(ICON_SEARCH, "search-icon"),
        to_dark = icon(ICON_DARK, "to-dark"),
        to_light = icon(ICON_LIGHT, "to-light"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every token the site's CSS uses exists in the vendored Elanor tokens (or is the
    /// site's own, set in style.css): an Elanor update that renames one fails here
    /// instead of silently unstyling the site.
    #[test]
    fn tokens_exist() {
        let tokens = std::fs::read_to_string(repo().join(ELANOR_DIR).join("tokens.css")).unwrap();
        for part in STYLE.split("var(").skip(1) {
            let name = part.split([',', ')']).next().unwrap().trim();
            let def = format!("{name}:");
            assert!(
                tokens.contains(&def) || STYLE.contains(&def),
                "style.css uses {name}, which elanor/tokens.css does not define"
            );
        }
    }

    #[test]
    fn icons_inline() {
        let i = icon(ICON_SEARCH, "s");
        assert!(
            i.starts_with("<svg class=\"icon s\" aria-hidden=\"true\""),
            "{i}"
        );
    }

    #[test]
    fn tables_scroll_but_commands_do_not_nest() {
        let page = Page {
            title: "T".into(),
            html: "<table>\n<tr><td>x</td></tr>\n</table>\n<table class=\"commands\"></table>"
                .into(),
            toc: Vec::new(),
        };
        let html = template(&[], "p.md", &page, None, "main");
        assert!(html.contains("<div class=\"table-scroll\"><table>\n<tr><td>x</td></tr>\n</table></div>\n<table class=\"commands\"></table>"), "{html}");
    }

    #[test]
    fn summary_lines() {
        let s = summary("# The tour\n\n- [Engine](engine.md)\n  - [Sub](sub.md)\n- [Draft]()\n");
        assert!(s[0].part && s[0].title == "The tour");
        assert_eq!(s[1].path.as_deref(), Some("engine.md"));
        assert_eq!((s[2].depth, s[2].title.as_str()), (1, "Sub"));
        assert_eq!(s[3].path, None);
    }
}
