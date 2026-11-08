# 0036: The guide as a static site

status: in-progress
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
- [ ] `cargo run -p psh-docs -- build [--out DIR] [--wasm FILE]` writes the site
      (default target/docs-site) with every SUMMARY.md page and a front page.
- [ ] Elanor's vendored files (tokens, fonts, icons, logos) as in djs, with
      `scripts/elanor-update.sh` to refresh them.
- [ ] Search over every section, in the browser, no server.
- [ ] ```` ```sh live ```` blocks: editable, run in psh.wasm in the page; the build-time
      output (`gen:run` region) and classification come from psh itself, and
      `psh-docs check` fails when an example's output changed (`regen --examples`
      accepts it).
- [ ] `psh-docs check` keeps its writing rules and also renders every page.
- [ ] REPORT.md folded into the guide and removed; README points at the site.
- [ ] Decision recorded for the new dependencies; deps allowlist updated.

## Log

## Follow-ups

## Result
