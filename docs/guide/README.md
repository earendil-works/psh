# Writing the guide

The guide explains psh to a human reader: what a user or an embedder of psh sees and can
do, and the architecture that was chosen, how it works and, where it helps, why it beats
the obvious alternative. It is not a second ARCH.md: ARCH.md, TESTING.md, the decision
log and `research/` stay the detailed reference for people changing the code, and the
guide says what it needs in its own words.

This file is about the guide; it is not a chapter, so the rules below do not apply to it.

## The rules

These hold for every page listed in `SUMMARY.md` (code blocks are exempt; inline code is
held to the word rules). `psh-docs check` enforces them in the gate (see Tooling below);
what it cannot see, such as a goal called a goal without being on `goals.md`, is on the
writer and the reviewer.

- **Self-contained.** A reader of the guide has the guide and nothing else. Never mention
  or link ARCH.md, TESTING.md, docs/*.md (the decision log included), `research/`,
  `work/`, or issue numbers (`0004`). Write what the page needs, or what was found:
  "in a corpus of about 295,000 agent commands, 47% use a pipe", not "see corpus.md".
- **No decisions.** No decision IDs (`D005`) and no links to the decision log. Explain the
  reason in the page's own words.
- **No history.** Describe psh as it is: no "used to", "was replaced", "originally",
  earlier designs or migration stories. Comparisons with other shells are fine where they
  explain a choice ("unlike a shell built on an async runtime, ...").
- **Built or not.** psh is young. A page may describe designed behavior that is not built
  yet, marked with a `> **Not built yet.** ...` note at the start of the section. When it
  lands, the note goes and the text is checked against the code. Measured numbers come
  only from things that exist.
- **pi on one page.** pi, the agent psh was built for, is named only on `pi.md`: how pi
  and its pi-env daemon use psh, and how another agent host could. Everywhere else,
  describe hosts in general ("an agent host", "the embedder"). This applies to the whole
  word `pi`, not to `pipe` or `pipeline`.
- **Goals by name, and only goals.md's.** Every goal a page refers to or relies on is a
  section of `goals.md`, which says what it means for a user, why psh chose it, and what
  it costs. Refer to one by its name and link its section
  (`[portable](goals.md#portable)`), never by number. An aim that is not on `goals.md`
  is not called a goal elsewhere. A new goal goes onto `goals.md` first.
- **Source as source.** Pages may point at code, as relative links to repository files
  (`[crates/psh-syntax/src/lexer.rs](../../crates/psh-syntax/src/lexer.rs)`). A link to
  a path that does not exist is an error, so link code only once it exists.

`SUMMARY.md` lists the pages in reading order (mdBook's format). A `- [Title]()` entry is
a chapter not written yet.

## How a page is written

- Start with what the reader wants to know, then an `> **In short.** ...` paragraph: the
  whole page in three sentences.
- Pages about using psh (the language, utilities, Windows, the command line, embedding,
  classification) say what a user sees and can do, with examples. Pages about how it
  works go from the problem to the design. Show real commands and real output, not
  paraphrases.
- Give a design choice its cost where it helps: `> **Trade-off.** X over Y. Why ... The
  cost: ...`.
- End with "Where to look" (source links), once there is source to link.
- Plain, direct sentences, in the present tense. No marketing.
- When psh deliberately behaves differently from bash, say so where the behavior is
  described, with the reason.

## Tooling

```sh
cargo run -q -p psh-docs -- check             # the rules, the examples' output, rendering
cargo run -q -p psh-docs -- regen             # write missing example output into the pages
cargo run -q -p psh-docs -- regen --examples  # also accept example output that changed
cargo run -q -p psh-docs -- build             # the site: target/docs-site/
```

`check` checks every page in `SUMMARY.md` and the front page, `index.md`; the gate runs
it. It reports `docs/guide/PAGE:LINE: message` for:

- a link to a page that is not in `SUMMARY.md`, to a section (`#id`, as mdBook makes ids
  from headings) that does not exist, or to a repository file that does not exist;
- a mention or link of ARCH.md, TESTING.md, docs/*.md outside the guide, `work/` or
  `research/`, and any `.md` name that is not a guide page;
- issue numbers (`0004`), decision IDs (`D005`), bare goal numbers (`G2`, "goal 3");
- history phrases ("no longer", "used to", "was replaced", "originally", ...); "is used
  to" is fine;
- `pi` as a whole word on any page but `pi.md`.

Fenced code blocks are skipped; inline code is held to the word rules but not to the
history phrases. A false positive is fixed in the tool, not worked around in the text.

`check` also renders every page and runs every live example (below): an example whose
output differs from the one its page records fails until `regen --examples` accepts the
new output, and whoever accepts it checks that the text around it still says the right
thing.

## Live examples

A block marked ```` ```sh live ```` is a live example. On the site it is an editor: the
reader can change the script and run it (Run, Ctrl/Cmd-Enter, or by itself 600 ms after
the last edit) in psh.wasm, in the page. Its tabs show the output, stdout and stderr as
they came with stderr in red and the exit status when it is not 0, and the
classification of the script: the commands it would run, their categories, and the
paths it reads and writes. The AST tab shows the parser's syntax tree as an outline,
one row per node with its kind and source text (wrappers with a single child left out),
or the parse error for invalid input. Pointing at a row marks the code it covers in the
editor, the caret in the editor selects the deepest row under it, rows fold, and the
JSON button shows the full `psh --parse` output with byte spans.
Classification and the AST update before execution starts, without waiting for a slow
script to finish or reach the five-second execution limit. Editing marks both as
pending until the new text is parsed.

Each one runs in a fresh shell in an empty `/work` directory (also `HOME`) with
`PATH=/usr/bin:/bin`, over an in-memory file system, with every utility in-process and
no external programs: an example makes the files it needs, and `git` or `cargo` are
"command not found". The build runs it the same way in psh itself, on the engine's
in-memory host, and writes the output after the block for readers of the Markdown:

````markdown
```sh live
printf 'b\na\n' | sort
```
<!-- gen:run -->
```text
a
b
```
<!-- /gen -->
````

`regen` writes this region; never edit it by hand. A failed example ends with an
`(exit status N)` line. The wasm leg of the gate runs every live example in psh.wasm
through `js/psh.mjs` and fails if it prints anything else, so the output a reader sees
before running an example is the output they get when they run it. Pick examples whose
output does not depend on the clock, process ids or the machine.

## The site

`build` renders every page with the navigation of `SUMMARY.md`, an outline of the page,
previous and next links, a search box (`/` focuses it; `search.json` is searched in the
browser, there is no server), light and dark, and syntax highlighting for shell, Rust,
JavaScript and JSON. A shell block whose lines start with `$ ` is a session: those lines
are commands, the others their output. Links to repository files go to GitHub at the
commit the site is built from. `> **In short.**`, `> **Trade-off.**` and
`> **Not built yet.**` quotes become callouts. It copies `target/wasm/psh.wasm` (from
`scripts/build-wasm`; `--wasm FILE` for another) and `js/psh.mjs` for the live examples;
without the module the examples show their recorded output only, as they do without
JavaScript or from `file://`. Serve the directory over HTTP to try them:
`python3 -m http.server -d target/docs-site`.

The design is Elanor, Earendil's design system: its tokens, fonts, icons and logos are
vendored in `crates/psh-docs/assets/elanor/` by `scripts/elanor-update.sh`, so a build
needs no network. The page template is in `crates/psh-docs/src/site.rs`, the styles in
`crates/psh-docs/assets/style.css`, and the editor in `crates/psh-docs/assets/live.js`.
