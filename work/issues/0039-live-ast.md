# 0039: Show formatted ASTs in live documentation examples

status: in-progress
agent: coordinator
branch: issue/0039-live-ast
depends-on: 0036, 0038
touches: crates/psh-docs/, Cargo.lock, scripts/deps-allowlist.txt, docs/decisions.md, ARCH.md, docs/guide/README.md, docs/guide/classify.md
guide: docs/guide/README.md, docs/guide/classify.md
decisions: D1390–D1399

## Goal
Add an AST result tab beside Output and Classification in live documentation examples.
Reuse the parser JSON used by the CLI and WASM, with readable indentation and syntax
highlighting, without waiting for execution.

## Acceptance criteria
- [ ] AST tab includes the initial example's parsed tree and updates after edits.
- [ ] Parse errors appear in the AST tab; old runs cannot overwrite newer results.
- [ ] Tests cover rendering, escaping, tab navigation and asynchronous edits.
- [ ] Guide documents the tab; targeted checks and the merge gate pass.

## Log
- 2026-11-08: Started in the docs worktree. Reuse the CLI JSON modules for build-time
  output, as the WASM crate does. No parser/ABI changes needed.

## Follow-ups

## Result
