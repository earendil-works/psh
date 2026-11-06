# 0019: Utilities batch H: awk

status: open
agent: -
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

## Follow-ups

## Result
