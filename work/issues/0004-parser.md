# 0004: Parser: psh-syntax from brush-parser, hand-written

status: open
agent: -
branch: issue/0004-parser
depends-on: 0001 (0002 for the bash -n verdicts of L0)
touches: crates/psh-syntax/, crates/psh/ (the --parse flag), NOTICE, tests/l0/
guide: syntax.md (not written yet; list it as a follow-up)
decisions: D1040–D1049

## Goal
The bash parser psh, the classifier and every host use (ARCH.md "psh-syntax", D008,
D013): copied from brush-parser as a starting point, with peg and winnow replaced by a
hand-written lexer and recursive-descent parser, byte spans on every node, a printer,
zero dependencies, building for `wasm32-unknown-unknown`. Climbs L0 to ~100%.

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
- [ ] `psh --parse [-c CMD | FILE]` prints the AST as JSON with spans (hand-written JSON,
      no serde).
- [ ] Zero dependencies; builds for `wasm32-unknown-unknown` (gate); copied files carry
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

## Follow-ups

## Result
