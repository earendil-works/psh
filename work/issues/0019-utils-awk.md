# 0019: Utilities batch H: awk

status: done
agent: text
branch: issue/0019-utils-awk
depends-on: 0008
touches: crates/psh-utils/src/awk/, tests/utils/awk/, tests/vendor-awk/ (goawk and onetrueawk tests)
guide: utilities.md (not written yet), utilities reference (not written yet)
decisions: D1190–D1199

## Goal
`awk` (3.5k corpus commands): the largest single utility, with no mature permissive Rust
implementation. Port goawk (MIT) to Rust (D018); F mode until conformant, then S.

## Acceptance criteria
- [ ] POSIX awk plus the gawk-isms agents use: `-F` (regex and multi-character,
      including `[][]`), `-v`, `-f`, `var=value` operands, `length(arr)`,
      `tolower`/`toupper`, `strftime`/`systime`, `printf` with `%*d`, `getline`
      from files and commands, `system()` and `"cmd" | getline` routed through the
      shell, `> "/dev/stderr"`.
- [ ] The onetrueawk `T.*` tests and goawk's tests vendored (check their licenses) and
      run as an L4 leg with a ratchet.
- [ ] Switch to S mode only when both suites pass (coordinator decision, recorded).
- [ ] Each utility registered with its mode; unsupported flags fall through or fail
      clearly (tested per utility).
- [ ] L4 cases in `tests/utils/<name>/` for every flag listed above, recorded against
      GNU and BSD, with the D016 rule named where they conflict.
- [ ] Attribution headers and `NOTICE` entries for ported code.

## Notes
- gawk's source is GPL and off limits (D007); its manual is fine. frawk (MIT/Apache) is
  a performance reference, just-bash's TS awk (Apache) a behavior reference.
- Likely several sessions: land the lexer/parser, then the interpreter, then I/O.

## Log
- Part 0 (coordinator's request, outside this issue's touches): one quoting rule for
  utility messages, D1163 (decision recorded in 0016's range). `common::quote` /
  `common::quote_in` hold it; `fsutil::q` (0015) now calls it. Switched the nine text
  filters and head, tail, sleep, timeout, wc. utils stderr score 322 -> 336 of 339.
- Sources: goawk f69bde0 (MIT) in `vendor/goawk` of the primary checkout; onetrueawk
  72e9b56 (Lucent permission notice, permissive) in `vendor/onetrueawk` (its tests are
  `testdir/`: T.* shell scripts, p.* / t.* programs with .ok outputs). Both may be
  vendored into `tests/vendor-awk/` with their notices.
- Plan:
  - `crates/psh-utils/src/awk/`: `lexer.rs` and `parser.rs` (ported from goawk's lexer
    and parser), `ast.rs`, `resolve.rs` (globals and locals, scalar or array, inferred
    across calls as goawk's resolver does), `compile.rs` (AST to opcodes) and `vm.rs`.
    The VM is one async fn with explicit call frames, so there is no recursion and no
    boxed futures; its loop awaits only at I/O opcodes (getline, print to a stream,
    close, system, the record loop) and calls `ctx.check()` every N instructions. Also
    `value.rs` (num/str/strnum, CONVFMT/OFMT), `io.rs` (record reading with RS: newline,
    "" paragraph mode, single char, regex; FS splitting: " ", one char, regex, ""),
    `printf.rs` and `builtins.rs`.
  - Regexes on `psh-regex` (D1270: POSIX ERE, leftmost-longest, as awk wants). That is a
    new psh-utils -> psh-regex edge (in-repo, zero deps), to be recorded as D1190.
  - Shell commands (`system`, `cmd | getline`, `print | cmd`) need a UtilCtx way to run a
    command string with piped stdin/stdout through psh; asked the coordinator
    (`spawn_shell`, a UtilCtx v3 proposal). Until then those fall through.
  - Mode F (PathFallback) until both suites pass. Natively the spec runner then runs the
    real awk (/usr/bin/awk is on PATH), so the L4 awk cases need a way to force psh's
    awk (a psh-spec or policy switch); to be settled with the coordinator.
  - Land in parts: (1) lexer+parser+resolver with parse tests over the suites' programs,
    (2) VM and values, (3) I/O, getline, printf, builtins, (4) the vendored suites as an
    L4 leg with a ratchet, plus GNU/BSD L4 cases.
- Part 1: `UtilCtx::spawn_shell` (UTIL_API_VERSION 3, D1191, coordinator-approved),
  outside this issue's touches: `crates/psh-engine/src/util.rs` (the API, tests with the
  test host: fresh state, prefix assignments, pipes beyond the pipe capacity, an external
  on the pipes, drop = cancel), `exec.rs` (`run_in_shell`'s reset moved into
  `start_afresh`, shared), `sched.rs` (`spawn_with_id`, to cancel the task on drop), one
  sentence in ARCH.md "Interface". For sed's `e` and find/xargs: `ctx.spawn_shell(script,
  ShellStdio { stdin: ShellIo::Pipe|Inherit, stdout: ... })`, then `child.stdin` /
  `child.stdout` (`Option<UtilFile>`, take/drop to close), `child.wait().await`. psh-utils
  tests come with awk's `system()`.
- Part 2: awk itself, `crates/psh-utils/src/awk/` (F mode, names `awk` and `gawk`):
  lexer, parser, ast, resolver and compiler ported from goawk; the machine (`interp.rs`)
  restructured as one async loop with explicit frames and a key-snapshot stack for
  for-in (nothing recurses); values, printf, regex translation onto psh-regex (awk
  escapes, `\]` in brackets, `[][]`, gawk's `\y \< \>`, intervals), RS (newline, one
  char, paragraph, regex with RT), FS (" ", one char, "", regex), getline from the main
  input, files and commands, `print >`, `>>`, `|`, `/dev/stdout`, `/dev/stderr`,
  `system()`/pipes through `spawn_shell`, close/fflush, gawk's `gensub`, 3-argument
  `match`, `strftime`/`systime`/`mktime`, `length(arr)`, `-i/--include`. Errors as gawk
  words them (syntax errors exit 1 with the excerpt and caret, fatal ones 2 with
  `cmd. line:N` and `(FILENAME=… FNR=…)`). for-in iterates in insertion order.
- Part 2, outside touches (announced): D1192 `UtilPolicy::force_names` +
  `PSH_UTILS_FORCE` (psh-engine util.rs, psh main.rs, psh-spec engine/runner/wasm,
  TESTING.md), so the utils layer tests psh's F/W utilities (utils layer unchanged:
  433/433); psh-classify's awk/gawk entries: `-i`/`--include` take a value (the effect
  table already named them).
- Vendored suites, `tests/vendor-awk/` (README there): onetrueawk's t.*/p.* (225) and
  goawk's interpreter cases (547 of 611; `!gawk`/`+posix` ones left out), expected from
  gawk 5.2.1 in the container and BSD awk on macOS (`record.sh`), run by
  `cargo test -p psh-utils awk::tests::vendored_suites` on the test host with a ratchet
  (fails on regressions and stale lines, `PSH_UPDATE_RATCHET=1`). 767/772 pass; the 5
  left: test-host clock, signal statuses 256+N, one runtime-vs-parse-time error.
  gawk and BSD disagree on 83 cases; psh follows gawk (D016), listed in the README.
- L4: `tests/utils/awk/` (69 cases: basic, functions, io, errors), GNU and BSD recorded,
  8 `## expect: gnu` with reasons; 69/69 pass natively (stdout and stderr), 68/69 on
  wasm (`date` missing there). psh.wasm grew 1.52 MB -> 1.90 MB with awk.
- Part 3: onetrueawk's T.* scripts (31 of 34; README) vendored in
  `tests/vendor-awk/onetrueawk-T/`, run through the psh binary by
  `cargo test -p psh --test onetrueawk` (Unix; `[[test]]` in crates/psh/Cargo.toml,
  outside touches): psh fails only the checks gawk fails, plus 3 ratcheted (`.` and
  invalid UTF-8 in psh-regex). Fixed on the way: `//` in split, glibc's srand(0) = 1,
  gawk's "unexpected newline or end of string", "syntax error" for a bad number.
- Part 3, outside touches (announced): psh-regex gains `Regex::exec_at` / `find_at`
  (search from an offset with the text before it in view of `^`, `\b`, `\B`, `\<`):
  additive, for global substitution (awk's gsub, gensub, FS/RS regexes; sed's `g`
  can use it too). With it and fixes found by running gawk's own suite locally (GPL,
  never vendored; 231 of 237 match real gawk, the rest for-in order, rand, ARGV[0] as
  gawk, a NUL in the source, one `%c` subtlety): gawk's sub/gsub backslash rules, octal
  escapes in a regex as written characters (`/a\52b/` is `a*b`), bracket ranges with
  `]`/`[` ends and collating symbols, FILENAME `-` for stdin, ARGV[0] the name called by.
- Part 4: S mode (D1194, coordinator's decision); release 1M-line sum 0.19 s vs macOS
  awk 0.35 s; agent/oils cases that use awk unchanged.

## Follow-ups
- psh-regex: an option for `.` (and bracket negations) to match bytes outside valid
  UTF-8, as gawk's matcher does (3 T.utf8bad checks).
- `system()`/`close()` of a command killed by a signal: gawk returns 256+N, psh's shell
  status is 128+N (`ShellChild::wait` would need the signal).

## Result
- `awk` and `gawk` (S mode, D1194) in `crates/psh-utils/src/awk/`: a goawk port (lexer,
  parser, resolver, compiler; a non-recursive async stack machine), POSIX awk plus the
  gawk extensions agents use; where gawk and BSD awk differ, gawk (D1193). `system()`,
  `cmd | getline` and `print | cmd` run in psh's shell via `UtilCtx::spawn_shell`
  (UTIL_API_VERSION 3, D1191). Unsupported options fall through to the real awk (an
  error in a sandbox).
- Tests: unit cases (`cargo test -p psh-utils awk::`), the vendored suites against gawk
  with a ratchet (`tests/vendor-awk/`: onetrueawk t.*/p.* and goawk's cases, 767/772;
  onetrueawk's T.* via `cargo test -p psh --test onetrueawk`, Unix, 3 checks ratcheted),
  L4 `tests/utils/awk/` (69 cases, GNU and BSD recorded). `PSH_UTILS_FORCE` (D1192)
  makes the L4 layer test psh's F/W utilities.
- Also landed: psh-regex `exec_at`/`find_at`; classifier `-i/--include` for awk/gawk.
- Speed (release build, 1M lines, `awk '{s+=$2} END{print s}'`): psh 0.19 s, macOS awk
  0.35 s.
- Known gaps: `.` does not match invalid UTF-8 bytes (psh-regex); signal deaths of
  `system()`/`close()` commands are 128+N, not gawk's 256+N; for-in order is insertion
  order; no BEGINFILE/ENDFILE, `@include`, `PROCINFO["sorted_in"]`, `--csv`, `|&`
  coprocesses, IGNORECASE; `%Z` in strftime is UTC or an offset, not a zone name.
