# 0019: Utilities batch H: awk

status: in-progress
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

## Follow-ups

## Result
