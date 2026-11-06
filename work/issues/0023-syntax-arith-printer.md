# 0023: Arithmetic parser, bash-format printer, byte-exact sources

status: open
agent: -
branch: issue/0023-syntax-arith-printer
depends-on: 0004
touches: crates/psh-syntax/, crates/psh/ (--parse JSON for the new nodes), tests/l0/
guide: syntax.md (follow-up from 0004)
decisions: D1230–D1239

## Goal
Parser-side pieces the engine needs next (0004 follow-ups), built in psh-syntax while the
engine agent builds the interpreter (0005), so the two don't collide.

## Acceptance criteria
- [ ] **Arithmetic**: a parser for bash arithmetic expressions (`$(( ))`, `(( ))`,
      `for (( ;; ))`, array subscripts, `let`) over *expanded* text, as bash parses it
      after expansion: all operators and precedence, assignment operators, `++`/`--`,
      ternary, comma, bases (`0x`, `0`, `N#`), variable references. An AST with spans,
      exposed as a public API the engine calls at run time, plus error messages
      matching bash's (`syntax error: operand expected (error token is "…")`).
      Evaluation stays in the engine; offer a pure `eval` over an env trait only if it is
      small and the engine agent agrees.
- [ ] **Printer in bash's format**: `declare -f` / `type` output, matching bash 5.3 for
      the Oils/brush cases that print functions (indentation, `;` placement, here-docs).
- [ ] **Bytes**: sources that are not valid UTF-8 parse byte-exactly (spans stay byte
      offsets into the original input); decide and record how the AST carries non-UTF-8
      words.
- [ ] Tests: unit tests, L0 snippets for arithmetic syntax errors where bash reports
      them at parse time, and agreement with recorded goldens for the printer.
- [ ] Stays `#![no_std]` + `alloc`, zero dependencies, wasm32 build.

## Notes
- Coordinate API shapes with the engine agent (0005) through your issue log and reports
  to the coordinator; the engine agent consumes these APIs.
- Clean room: bash's manual describes arithmetic; never its source.

## Log

## Follow-ups

## Result
