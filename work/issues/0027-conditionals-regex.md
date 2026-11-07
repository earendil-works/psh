# 0027: `[[ ]]`, `test`, and a POSIX regex engine

status: done
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
- [x] `psh-regex`: zero dependencies, `#![no_std]` + `alloc`, wasm32. POSIX ERE and BRE
      (bracket expressions with classes, equivalence/collating as far as a UTF-8/C
      locale needs, intervals, anchors, alternation, groups, back-references in BRE and
      as GNU extension in ERE), leftmost-longest match with submatch positions as
      POSIX specifies; byte and UTF-8 modes. Linear-time for patterns without
      back-references (no exponential backtracking: e.g. a Pike VM / NFA simulation with
      longest-match tracking); bounded backtracking only for back-references.
      Differential tests against GNU (container) and macOS regex on generated patterns.
- [x] `[[x]]`: all unary/binary operators, `==`/`!=` with patterns (extglob), `=~` with
      `BASH_REMATCH`, quoting rules for the right-hand side, `&&`/`||`/`!`/`( )`,
      arithmetic comparisons, `-v`, `-R`, file tests through the Host.
- [x] `test`/`[` complete (the POSIX argument-count rules, `-a`/`-o`, all file tests).
- [x] Ratchets updated; Result records scores.

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

### 2026-11-07: part 2, `[[ ]]` and `test`
- `crates/psh-engine/src/cond.rs` (D1272), dispatched from `run_compound` (so
  redirections on `[[ ]]` work); `mod cond` in lib.rs; `builtins::test` is
  `pub(crate)` for its `unary`/`binary`. psh-engine depends on psh-regex (allowlist
  lines cite D1270). ARCH.md's crate table lists psh-regex.
- Probed in the container: statuses (`!`/`||` after an error), the escaped chars of
  quoted right sides (trace lines show them), `BASH_REMATCH` (local, readonly,
  associative, after a failed match), `-R` (a nameref with a value), `-v` through a
  nameref to an element (false), `-t x` (status 2, "integer expected"), tilde prefixes
  (`~*` stays literal).
- `test`: `-o` (set options), `-R`, `-O`/`-G` (against the host's euid/groups), `-t`
  errors. `-N` stays false: the Host reports no access times.
- Parser: the right side of `=~` may start with `|`; an operator there is "syntax
  error in conditional expression: unexpected token"; two-char operators (`<>`, `>>`,
  `;;`, `<&` …) are one token in `[[ ]]` messages, with bash's "near" quirk (`;;` →
  `;`, `<&` → `&`). L0 still 100%.
- psh-regex: `[`, `[^` at the end is ERR 2 (GNU), not 7.
- Wasm: `brush/compat/extended_tests/11` now passes natively but needs `/bin/sh` and
  `/etc/os-release`, which the JS host lacks (the same reason as `builtins/test/2`).

### 2026-11-08: `-v` with arithmetic subscripts
- Asked for via the coordinator (engine agent): `[[ -v a[i+1] ]]` and `test -v
  'a[i+1]'` evaluate the subscript arithmetically unless the array is associative
  (probed: also through namerefs and for scalars, `s[i-1]`). `builtins::test` now takes
  `&mut Vp` (arithmetic can assign), `is_set` evaluates with `arith::eval` first.
  An arithmetic error is reported as `test`/`[[`'s error (status 2), where bash
  abandons the command line.

## Follow-ups
- `[[ -v ref[N] ]]` with `ref` a nameref to a whole array (brush marks it a known
  failure too); `-N` needs access times from the Host.
- The 4 known psh-regex differences from GNU (D1270): which iteration or alternative
  owns a group with `(\b)*`-style assertions and bounded group repetitions.

## Result
- `crates/psh-regex`: POSIX ERE/BRE with GNU semantics (`Regex::new(pat, Options)`,
  `exec`/`find`/`is_match`), linear without back-references, 4,057 recorded GNU cases
  as a test (D1270, D1271). Ready for `sed`/`grep` fall-throughs.
- `[[ ]]` complete (D1272) and the `test` long tail. Scores (native, after rebasing
  onto main with `touch -d`): oils 2094 → 2220 of 2511 active, brush 2560 → 2700 of
  3171, agent 140 → 141 of 144; 270 ratchet entries removed. Remaining `[[`/`test`
  failures: oils `builtin-bracket/31` (`touch` with no operand's message), the
  follow-ups below.
