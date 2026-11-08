# 0040: A readable, interactive AST outline for live examples

status: done
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
- [x] `psh_syntax::outline(&Ast)` yields rows (depth, role, kind, info, source excerpt,
      span) for every construct; unit tests cover the collapse rules and each kind.
- [x] `psh --parse --tree` prints the outline as indented text.
- [x] psh.wasm returns the outline rows (a `psh_parse` flag; `psh.parse(src, {outline})`).
- [x] The docs AST tab shows the outline at build time and after edits, with hover →
      editor highlight, caret → row, folding, and a JSON toggle; parse errors as before.
- [x] Tests: psh-docs rendering, the editor regression script, CLI and wasm tests.
- [x] Guide pages updated.

## Notes
- No cargo feature for the outline: psh-syntax stays dependency-free, and unused code is
  dropped at link time, as with `dump`.

## Log
- 2026-11-08: Owner picked option C (interactive outline) and asked for it in
  psh-syntax, unsure about a flag. Decided against a cargo feature (D1400): the module
  needs only `alloc`, unused code is dropped at link time; the exported wasm parser
  grows ~20 KB (390 → 411 KB).
- `psh_syntax::outline`/`outline_text` (outline.rs): collapse rules, roles, info, a
  60-char one-line excerpt; backtick bodies parsed separately get the backtick's span.
  Unit tests cover the collapse rules, each kind and nesting/span invariants
  (here-doc bodies are the one place a child's span lies outside its parent's).
- `psh --parse --tree` (text; errors on stderr; excludes `--compact`); CLI tests.
- `parse_json::outline_to_json`, `psh_parse` flag 4, `psh.parse(src, { outline })`;
  wasm unit test and smoke test.
- Docs: run.rs renders the rows (no JSON at build time any more, so psh-docs dropped
  the json/parse_json path modules and its direct deser deps); live.js renders the same
  from wasm, marks a row's span in a transparent layer over the code, selects the
  deepest row at the caret (after the reader put the caret in the code), folds rows
  (twisty, ARIA-tree-like arrow keys on the focused pane), and offers a Tree/JSON view
  switch (JSON from a shared analyzer instance). Search index skips the pane up to an
  end marker. Node editor tests updated; headless Chrome checked caret → row, hover →
  mark, folding and reveal, keys, JSON view and parse errors (work/scratch/0040).
- Guide: README.md, classify.md, cli.md (`--tree` with an example), wasm.md (outline
  option; module sizes were stale and are current now). ARCH.md updated.

## Follow-ups

## Result
The AST tab of every live example shows an outline of the syntax tree instead of the
raw JSON: one row per node (kind, role, operators/names, source excerpt), wrappers of
one child collapsed. Hovering a row marks its code in the editor, the caret in the code
selects the deepest row, rows fold, arrow keys move on the focused pane, and a JSON
button shows the full `psh --parse` output. The same outline is `psh_syntax::outline`,
`psh --parse --tree`, and `psh.parse(src, { outline: true })` in wasm. Known gaps: no
default folding for very large trees; rows are plain divs, not an ARIA tree widget.
