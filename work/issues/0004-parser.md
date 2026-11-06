# 0004: Parser: psh-syntax from brush-parser, hand-written

status: in-progress
agent: parser
branch: issue/0004-parser
depends-on: 0001 (0002 for the bash -n verdicts of L0)
touches: crates/psh-syntax/, crates/psh/ (the --parse flag), NOTICE, tests/l0/
guide: syntax.md (not written yet; list it as a follow-up)
decisions: D1040–D1049

## Goal
The bash parser psh, the classifier and every host use (ARCH.md "psh-syntax", D008,
D013): copied from brush-parser as a starting point, with peg and winnow replaced by a
hand-written lexer and recursive-descent parser, byte spans on every node, a printer,
`#![no_std]` + `alloc` with zero dependencies (D023), building for
`wasm32-unknown-unknown`. Climbs L0 to ~100%.

## Acceptance criteria
- [ ] AST covering bash 5.3 syntax: lists, pipelines (`|`, `|&`, `!`, `time`), `&&`
      `||` `;` `&`, simple commands with assignments and redirections (all forms,
      including `{var}>`, `&>`, `<<<`, here-docs with `-` and quoted delimiters),
      compound commands (`{}`, `()`, `if`, `for` both forms, `while`, `until`, `case`
      with `;;`/`;&`/`;;&`, `select`, `[[ ]]`, `(( ))`), functions, coprocs, words with
      their parts (quotes, `$'…'`, `$"…"`, parameter expansions with all operators,
      `$(…)`, backticks, `$((…))`, process substitution, brace expansion, extglob).
- [ ] Every node has a `u32` byte span into the source; a line table gives line and
      column on demand. A test checks spans for every node kind.
- [ ] Printer; `parse(print(ast)) == ast` on every L0 input.
- [ ] Parse errors carry a span and a message; the parse/no-parse verdict agrees with
      `bash -n` on L0 (ratchet in `tests/l0/expected-failures.txt`).
- [ ] `psh --parse [-c CMD | FILE]` prints the AST as JSON with spans, written with
      `deser-json` (D029) in the `psh` crate (or a module outside `psh-syntax`), so the
      parser stays dependency-free and no_std.
- [ ] `#![no_std]` + `alloc`, zero dependencies, MSRV 1.96 (D025); builds for
      `wasm32-unknown-unknown` (gate); copied files carry
      brush's copyright header and are listed in `NOTICE`.
- [ ] A benchmark note in the Result: parse time for the whole shape corpus or L1+L2,
      and the wasm size of the parser alone (stripped, gzipped).

## Notes
- brush checkout: `/tmp/ls/brush` at `943553433ae0` (or clone `github.com/reubeno/brush`
  and check out that commit into `vendor/`). brush-parser already has
  `SourcePosition { index, line, column }`; we want compact byte spans instead.
- Never consult GNU bash's `parse.y` or oh-my-pi's bash engine (D007). The bash manual
  is fine.
- Alias expansion is not a parse-time feature here (ARCH.md); keep the AST alias-free.
- Decide and record (in your range) the AST representation: owned tree vs arena of
  nodes with indices. The classifier and the engine both walk it.
- Can start on brush's own parser tests and the L1/L2 scripts before 0002's verdicts
  exist.

## Log

### 2026-11-06: part 1, psh-syntax
- Design (D1040–D1046): arena AST (`Ast` owns `Arc<str>`; lists/commands/here-docs by id;
  words inline; literal parts are spans), one-cursor recursive descent without a token
  stream, deferred checks as in bash (backticks, arithmetic, bad `${}`, here-doc bodies),
  brace expansion as structure, extglob as an option, chunked `Parser` for the engine,
  nesting limit 100. brush contributed the AST vocabulary (`ast.rs`, attributed); its
  tokenizer + winnow word parser did not fit (no part spans, no nested parse), so the
  lexer/parser are new.
- Probing without 0002: split Oils `spec/*.test.sh` (by `####`) and brush YAML `stdin:`
  blocks into 7389 scripts, plus the 26234 shape skeletons from the shapes worktree, and
  ~200 hand-written tricky snippets; recorded `/opt/homebrew/bin/bash -n` verdicts
  (scratch: `work/scratch/0004/{split.py,verdicts.sh,compare.sh}`).
- **L0 agreement with `bash -n`: 33819/33819 (100%)** (Oils 2.7k + brush 4.7k cases, 26234
  shapes, 196 own snippets). All 32.5k parsing inputs round-trip print → reparse with the
  same `dump`. Error messages: the first line equals bash's for ~95% of the scripts that
  fail; the rest are `[[ ]]` second lines and here-doc warnings.
- bash facts found by probing (all in the code and tests): `name=(…)` arguments are
  accepted after `declare typeset local export readonly alias let eval` only; backtick
  bodies, `$((…))` contents and `${x@Z}` are not checked by `bash -n`; unquoted here-doc
  body errors are runtime errors; outer here-doc bodies start after the command line even
  when a `$(…)` on it spans lines; `${ …; }` (5.3) is a command list; newlines are allowed
  in `[[ ]]` except between a lone word and its operator; line continuations may split
  operators (`&\⏎&`, `$\⏎(`); `a=(1 2` gives status 1, not 2.
- `tests/l0/syntax.txt`: the hand-written snippets with bash's verdicts, checked by
  `crates/psh-syntax/tests/l0.rs` (verdict + round trip). Unit tests in `src/tests.rs`
  (structure via `dump`, spans of every node kind, errors and their spans, here-docs,
  chunks, nesting on a 1 MiB stack).
- Outside `touches:`: `crates/psh-engine/src/lib.rs` (the stub matched `Ok(())`; now
  `Ok(_)`), ARCH.md psh-syntax bullets (arenas, deferred checks, chunks), NOTICE.
- Benchmarks (release, M-series Mac): shape corpus (26234 inputs, 2.3 MB) ≈ 53 ms per
  pass (~43 MB/s); L1+L2 scripts (7389, 1 MB) ≈ 16 ms. wasm32 cdylib exporting
  parse+print+dump: 128 KB / 43 KB gzipped (opt-level 3), 81 KB / 32 KB gzipped (opt-level
  z), stripped, LTO.
- L0 leg: verdicts for the vendored L1/L2 scripts come from 0002's recorder; the shape
  corpus verdicts need recording too (0002 or 0003). The ratchet file
  `tests/l0/expected-failures.txt` would start empty. To agree with infra.

### 2026-11-06: part 2, `psh --parse`
- Coordinator: 0002 (infra) records container `bash -n` verdicts (Oils/brush scripts,
  shapes, `tests/l0/syntax.txt`) and builds the L0 leg + ratchet in psh-spec.
- `tests/l0/syntax.txt` format (stable, for infra): UTF-8, LF. Lines starting with `# `
  before the first case are a header. A case starts with `#### NAME` (NAME is a number
  today; treat it as an opaque id); the script is every following line up to the
  `## status: N` line, each line terminated by `\n` (so the script always ends with a
  newline, which is how the verdicts were recorded); `## status: N` is `bash -n`'s exit
  status (0 = parses; one case has 1); an optional `## message: TEXT` follows for
  failures: bash's first stderr line without `FILE: line N: `. Blank lines between cases
  are not part of any script. No script line starts with `#### ` or `## `. To re-record,
  run `bash -n` on each script and rewrite `status`/`message`.
- `psh --parse [--compact] [-O extglob] [-c CMD | FILE]` (stdin without either): JSON on
  stdout via deser-json in the `psh` crate (`src/json.rs`: a small JSON tree with a deser
  `Serialize` impl; `src/parse_json.rs`: the mapping, its shape documented at the top;
  D1047). Status 0 parsed, 2 syntax error (the JSON then holds the error). Pretty output
  inlines short leaf arrays (spans). Smoke-checked over all 33.8k L0 inputs: no crashes.
- deser-json added with `default-features = false, features = ["std"]` (+ `deser-core`,
  its own dependency, for the trait): the build pulls deser-core, deser-json, itoa, zmij.

## Follow-ups

## Result
