# 0039: Show formatted ASTs in live documentation examples

status: done
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
- [x] AST tab includes the initial example's parsed tree and updates after edits.
- [x] Parse errors appear in the AST tab; old runs cannot overwrite newer results.
- [x] Tests cover rendering, escaping, tab navigation and asynchronous edits.
- [x] Guide documents the tab; targeted checks pass (merge runs the full gate).

## Log
- 2026-11-08: Started in the docs worktree. Reuse the CLI JSON modules for build-time
  output, as the WASM crate does. No parser/ABI changes needed.
- Added a third accessible result tab with indented, highlighted JSON at build time
  and after browser edits. Parsing happens alongside classification, before execution;
  edits mark both pending and retain the existing stale-result protections.
- Reused the CLI JSON modules with existing deser-json/deser-core dependencies
  (D1390); excluded AST text from the search index to avoid indexing duplicate noise.
- Updated ARCH.md, docs/guide/README.md and docs/guide/classify.md. No example output
  or conformance ratchets changed; no engine or WASM ABI changes.
- Validation: 33 psh-docs unit tests, eight editor regression tests, guide check
  (21 pages), site build and formatting pass. Headless Chrome checked the initial AST,
  tab switching, edited pipeline structure and parse errors using the real WASM parser.
- Merged ddc417d1 through scripts/merge-to-main.sh: the full gate passed in 260 s,
  including WASM and native Windows tests. Coordinator reviewed the guide and result.

## Follow-ups

## Result
Every live example has an AST tab beside Output and Classification. It shows the
parser's full JSON result with node types, word expansions and byte spans, or a parse
error, indented and syntax highlighted. It is available before running an example and
updates before edited code starts executing. No known gaps within this scope.
