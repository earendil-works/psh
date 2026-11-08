# 0036: The guide as a static site

status: done
agent: coordinator
branch: issue/0036-docs-site
depends-on: 0012, 0011
touches: crates/psh-docs/, docs/guide/, scripts/elanor-update.sh, scripts/deps-allowlist.txt, scripts/gate.d/86-guide.sh, README.md, REPORT.md, Cargo.lock, AGENTS.md, work/BOARD.md
guide: README.md (tooling), introduction.md, status.md, index.md (new)
decisions: D1360–D1369

## Goal
The owner's request: build docs for psh akin to djs's (`~/Development/djs`,
`crates/djs-docs`): `psh-docs build` renders docs/guide into a static site in Elanor's
design (navigation from SUMMARY.md, outline, pager, search, light/dark, syntax
highlighting, source links), with a front page and live examples that run in psh.wasm
in the browser. The other top-level human docs are folded in: REPORT.md goes into the
guide (status.md) and is removed.

## Acceptance criteria
- [x] `cargo run -p psh-docs -- build [--out DIR] [--wasm FILE]` writes the site
      (default target/docs-site) with every SUMMARY.md page and a front page.
- [x] Elanor's vendored files (tokens, fonts, icons, logos) as in djs, with
      `scripts/elanor-update.sh` to refresh them.
- [x] Search over every section, in the browser, no server.
- [x] ```` ```sh live ```` blocks: editable, run in psh.wasm in the page; the build-time
      output (`gen:run` region) and classification come from psh itself, and
      `psh-docs check` fails when an example's output changed (`regen --examples`
      accepts it).
- [x] `psh-docs check` keeps its writing rules and also renders every page.
- [x] REPORT.md folded into the guide and removed; README points at the site.
- [x] Decision recorded for the new dependencies; deps allowlist updated.

## Log

2026-11-08 (coordinator, at the owner's request)
- Modeled on djs-docs (same owner, MIT): template, style.css, page.js, search.js and
  search.rs adapted; Elanor's vendored files copied from djs (same pinned commit) with
  scripts/elanor-update.sh adapted (no colors.tsv: psh has no generated diagrams).
- psh-docs: the old checker is rules.rs (unchanged rules, now also for index.md);
  render.rs (pulldown-cmark), highlight.rs (shell, Rust, JS, JSON; a session mode for
  `$ ` transcripts), run.rs (live examples on TestHost, sandbox utilities, empty /work,
  interleaved stdout/stderr through a custom HostFile, 5 s limit), regions.rs (gen:run),
  site.rs, search.rs. Not ported from djs: path auto-linking, status/chart generation,
  SVG diagrams (psh's guide has none).
- live.js: textarea-over-pre editor as in djs; runs a fresh MemHost + psh.wasm instance
  per run; Output and Classification tabs (the classification table is built in Rust at
  build time and from `psh.classify` JSON in the page, same markup).
- Live examples added: index.md, introduction.md, language.md (two transcript blocks
  became live), utilities.md, classify.md. crates/psh-docs/tools/live-check.mjs checks
  in the wasm-host leg that psh.wasm prints the recorded output (all 6 agree); a
  highlight.rs test compares assets/highlight.js on every shell block (needs node).
- REPORT.md folded into status.md ("What is not built yet", "Known gaps", the gaps
  re-checked where cheap: `time`, `diff <(…) <(…)`, logical `$PWD`) and removed; README,
  BOARD, ARCH (crate table), TESTING (legs), AGENTS (accepting example output) updated.
- Guide pages updated: README.md (tooling, live examples, the site), index.md (new),
  introduction.md, language.md, utilities.md, classify.md, status.md.
- .github/workflows/docs.yml as in djs (deploys once the repository is on GitHub).
- Measured while here: psh.wasm built from this branch is 5.5 MB (1.9 MB gzipped); the
  guide (status.md, wasm.md) still says 4.3 MB (1.5 MB) from the checkpoint commit.

## Follow-ups
- Re-measure status.md and wasm.md numbers (psh.wasm is 5.5 MB / 1.9 MB gzipped now; the
  code table's psh-docs row predates the site).
- More live examples (jobs.md, wasm.md, differences.md), and turning `psh -c` session
  blocks into live ones where they need no external programs.
- REPO_URL (render.rs) assumes github.com/earendil-works/psh: change it if the
  repository lands elsewhere.

## Result
`cargo run -p psh-docs -- build` writes the guide as a site to target/docs-site (front
page index.md, every SUMMARY.md page, search, light/dark, source links); with
target/wasm/psh.wasm present (scripts/build-wasm) its ```` ```sh live ```` examples are
editable and run in the page with Output and Classification tabs. `check` (gate) also
renders every page and fails on changed example output; `regen [--examples]` writes or
accepts it. docs/guide/README.md documents all of it. Known gaps: no path auto-linking
or generated status numbers as djs has; the site is not deployed anywhere yet.
