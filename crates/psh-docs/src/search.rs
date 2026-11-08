//! The site's search index (`search.json`): one entry per section (the text from one h1,
//! h2 or h3 to the next) of every rendered page, as plain text. assets/search.js loads it
//! on the first use of the search box and searches it in the browser; there is no server.

use crate::render::Page;

/// Elements whose text is not searched: diagrams, and the live examples' classification
/// pane (a table of the commands that are on the page anyway).
const SKIP: &[(&str, &str)] = &[
    ("<svg", "</svg>"),
    ("<div class=\"live-pane\" data-pane=\"classify\"", "</div>"),
];

/// `[[file, page title, anchor, heading, text], ...]` for `rendered` (source name, page).
pub fn index(rendered: &[(String, Page)], html_name: impl Fn(&str) -> String) -> String {
    let mut out = String::from("[");
    for (name, page) in rendered {
        let file = html_name(name);
        for (anchor, heading, text) in sections(&page.html) {
            if text.is_empty() && heading.is_empty() {
                continue;
            }
            if out.len() > 1 {
                out.push_str(",\n");
            }
            out.push('[');
            for (i, s) in [&file, &page.title, &anchor, &heading, &text]
                .into_iter()
                .enumerate()
            {
                if i > 0 {
                    out.push(',');
                }
                json_str(&mut out, s);
            }
            out.push(']');
        }
    }
    out.push_str("]\n");
    out
}

/// (anchor, heading text, body text) per section of a page's HTML.
fn sections(html: &str) -> Vec<(String, String, String)> {
    let html = skip_elements(html);
    let mut out = Vec::new();
    let mut rest = html.as_str();
    let mut cur = (String::new(), String::new());
    loop {
        let next = ["<h1 ", "<h2 ", "<h3 "]
            .iter()
            .filter_map(|t| rest.find(t))
            .min();
        let body = &rest[..next.unwrap_or(rest.len())];
        out.push((cur.0.clone(), cur.1.clone(), text_of(body)));
        let Some(at) = next else { break };
        rest = &rest[at..];
        let close = rest[..3].replace('<', "</");
        let end_tag = rest.find(&close).unwrap_or(rest.len());
        let open_end = rest.find('>').map_or(rest.len(), |i| i + 1);
        let anchor = rest[..open_end]
            .split("id=\"")
            .nth(1)
            .and_then(|s| s.split('"').next())
            .unwrap_or("")
            .to_string();
        let heading = text_of(&rest[open_end.min(end_tag)..end_tag]);
        cur = (anchor, heading);
        rest = &rest[(end_tag + close.len() + 1).min(rest.len())..];
    }
    // The text before the first heading has no anchor and no heading: fold it into the
    // first section (the page title's).
    if out.len() > 1 && out[0].0.is_empty() && out[0].1.is_empty() {
        let first = out.remove(0);
        out[0].2 = format!("{} {}", first.2, out[0].2).trim().to_string();
    }
    out
}

fn skip_elements(html: &str) -> String {
    let mut s = html.to_string();
    for (open, close) in SKIP {
        while let Some(a) = s.find(open) {
            let b = s[a..].find(close).map_or(s.len(), |i| a + i + close.len());
            s.replace_range(a..b, " ");
        }
    }
    s
}

/// The text of an HTML fragment: tags removed, the common entities decoded, whitespace
/// collapsed.
fn text_of(html: &str) -> String {
    // Block-level tags separate words; inline ones (`<code>`, `<a>`, `<em>`) do not.
    const BLOCK: &[&str] = &[
        "p",
        "li",
        "td",
        "th",
        "tr",
        "br",
        "div",
        "pre",
        "h1",
        "h2",
        "h3",
        "h4",
        "ol",
        "ul",
        "table",
        "blockquote",
        "figure",
        "figcaption",
        "aside",
        "section",
    ];
    let mut t = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(a) = rest.find('<') {
        t.push_str(&rest[..a]);
        let end = rest[a..].find('>').map_or(rest.len(), |i| a + i + 1);
        let name: String = rest[a + 1..end]
            .trim_start_matches('/')
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect();
        if BLOCK.contains(&name.as_str()) {
            t.push(' ');
        }
        rest = &rest[end..];
    }
    t.push_str(rest);
    let t = t
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn json_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_split_at_headings() {
        let html = "<h1 id=\"t\">Title</h1><p>Intro &amp; more</p><svg><text>x</text></svg>\
                    <h2 id=\"a\">A <code>b</code></h2><p>one</p><h3 id=\"c\">C</h3><p>two</p>";
        let s = sections(html);
        assert_eq!(
            s,
            [
                ("t".into(), "Title".into(), "Intro & more".into()),
                ("a".into(), "A b".into(), "one".into()),
                ("c".into(), "C".into(), "two".into()),
            ]
        );
    }

    #[test]
    fn json_escapes() {
        let mut s = String::new();
        json_str(&mut s, "a\"b\\c\n\u{1}");
        assert_eq!(s, r#""a\"b\\c\n\u0001""#);
    }
}
