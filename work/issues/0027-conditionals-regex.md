# 0027: `[[ ]]`, `test`, and a POSIX regex engine

status: in-progress
agent: parser
branch: issue/0027-conditionals-regex
depends-on: 0005
touches: crates/psh-regex/ (new), crates/psh-engine/src/builtins/test.rs, crates/psh-engine/src/cond.rs (new), crates/psh-engine/src/exec.rs (only the `[[ ]]` dispatch arm), Cargo.toml (workspace member), scripts/deps-allowlist.txt, tests/**/expected-failures.txt
guide: -
decisions: D1270–D1279

## Goal
Conditional expressions (`[[ ]]` ~219 cases, plus the `test`/`[` long tail) and the
regex engine they need. bash's `=~` uses POSIX extended regular expressions with
leftmost-longest matching and back-references in BREs; the `regex` crate has neither
(research/builtins.md §5), so psh gets its own small engine, which later also serves
`sed`/`grep` for the cases the ripgrep crates cannot do (D020's fall-through list).

## Acceptance criteria
- [ ] `psh-regex`: zero dependencies, `#![no_std]` + `alloc`, wasm32. POSIX ERE and BRE
      (bracket expressions with classes, equivalence/collating as far as a UTF-8/C
      locale needs, intervals, anchors, alternation, groups, back-references in BRE and
      as GNU extension in ERE), leftmost-longest match with submatch positions as
      POSIX specifies; byte and UTF-8 modes. Linear-time for patterns without
      back-references (no exponential backtracking: e.g. a Pike VM / NFA simulation with
      longest-match tracking); bounded backtracking only for back-references.
      Differential tests against GNU (container) and macOS regex on generated patterns.
- [ ] `[[ ]]`: all unary/binary operators, `==`/`!=` with patterns (extglob), `=~` with
      `BASH_REMATCH`, quoting rules for the right-hand side, `&&`/`||`/`!`/`( )`,
      arithmetic comparisons, `-v`, `-R`, file tests through the Host.
- [ ] `test`/`[` complete (the POSIX argument-count rules, `-a`/`-o`, all file tests).
- [ ] Ratchets updated; Result records scores.

## Notes
- Clean room: POSIX and the bash manual describe the semantics; never read GNU or BSD
  regex/bash source. Russ Cox's articles on regex implementation are fine to read.
- Parallel issues: 0026 (arrays), 0028 (options), 0029 (builtins) also touch
  psh-engine; stay in your listed files and announce anything else first.

## Log

### 2026-11-07: part 1, psh-regex
- Oracle: `work/scratch/0027/oracle/rx.c` (calls `regcomp`/`regexec`, hex in/out, a
  2 s alarm per match because glibc's back-reference matching can hang) built into a
  scratch image `psh-rx-oracle` (Debian stable, same base as the recording image), and
  with macOS's libc. Drivers: `q.py` (ask the oracle), `d.py` (diff against
  `examples/rxcheck`), `gen.py` (random patterns: ERE/BRE, back-references, UTF-8,
  icase, newline), `runall.sh`, `mkgolden.py` (writes `crates/psh-regex/tests/gnu.txt`).
- GNU semantics found (D1270): leftmost-longest overall, groups as an in-order greedy
  backtracker finds them within the bounds; `((a)|b)*` keeps group 2 from an earlier
  iteration; `(a*)*` on `aa` gives `(0,2)` (only the first iteration may be empty);
  repeated anchors are an error in ERE, `*`/`\{\}` after another operator in BRE;
  back-references only to groups in the same alternative; unmatched `)` is literal in
  ERE; invalid UTF-8 bytes match nothing but themselves; C.UTF-8 classes.
- Random differential runs: ~24k cases over all variants agree except ~0.1% (which
  alternative or iteration owns a group when assertions such as `\b` or bounded group
  repetitions are involved: `(\b)*`, `(a*){2,3}`). Recorded: 4,053 cases with 4 known
  differences. Deliberate deviation: non-ASCII ranges (D1271).
- Linear: `(a*)*b`, `(a|aa)*$`, `(x+x+)+y` over 20k chars in milliseconds (test).

## Follow-ups

## Result
