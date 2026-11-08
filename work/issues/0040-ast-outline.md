# 0040: A readable, interactive AST outline for live examples

status: in-progress
agent: coordinator
branch: issue/0040-ast-outline
depends-on: 0039
touches: crates/psh-syntax/src/outline.rs, crates/psh-syntax/src/lib.rs, crates/psh/src/ (cli, main, parse_json), crates/psh/tests/cli.rs, crates/psh-wasm/, js/psh.mjs, crates/psh-docs/, docs/decisions.md, ARCH.md, docs/guide/README.md, docs/guide/classify.md, docs/guide/cli.md
guide: docs/guide/README.md, docs/guide/classify.md, docs/guide/cli.md
decisions: D1400–D1409

## Goal
The AST tab (0039) shows the full `psh --parse` JSON: ~300 lines for a one-line loop,
most of it wrapper nodes, empty fields and word boilerplate. Replace it with an outline
of the tree (owner's choice, option C): one row per meaningful node, wrappers with a
single child collapsed, literal words as one row, labelled with kind, the attributes
the source does not show, and the source text. Hovering a row highlights its span in
the editor; moving the caret in the editor selects the deepest row there; rows fold.
The raw JSON stays available behind a toggle.

## Acceptance criteria
- [ ] `psh_syntax::outline(&Ast)` yields rows (depth, role, kind, info, source excerpt,
      span) for every construct; unit tests cover the collapse rules and each kind.
- [ ] `psh --parse --tree` prints the outline as indented text.
- [ ] psh.wasm returns the outline rows (a `psh_parse` flag; `psh.parse(src, {outline})`).
- [ ] The docs AST tab shows the outline at build time and after edits, with hover →
      editor highlight, caret → row, folding, and a JSON toggle; parse errors as before.
- [ ] Tests: psh-docs rendering, the editor regression script, CLI and wasm tests.
- [ ] Guide pages updated.

## Notes
- No cargo feature for the outline: psh-syntax stays dependency-free, and unused code is
  dropped at link time, as with `dump`.

## Log

## Follow-ups

## Result
