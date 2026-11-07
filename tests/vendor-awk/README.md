# Vendored awk suites

Run by `cargo test -p psh-utils awk::tests::vendored_suites` (the gate's `test` leg, on
every platform): each case runs psh's `awk` on the engine's test host and is compared,
exit status and stdout, with gawk's result.

- `onetrueawk/`: the `t.*` and `p.*` programs and their data from The One True Awk
  (72e9b56, its testdir; Lucent's permission notice in `LICENSE`). `t.*` read
  `test.data`, `p.*` `test.countries`: `awk -f PROG DATA` in a directory with the suite.
- `goawk/cases.txt`: GoAWK's interpreter cases (f69bde0, `interp/interp_test.go`'s
  `interpTests`; MIT, `LICENSE.txt`), converted by `convert.py`: one case per line,
  `program<TAB>stdin`, escaped (`\\ \n \t \xNN`). Cases GoAWK marks `!gawk` or `+posix`
  are left out. Run as `awk -- PROGRAM -`.
- `expected.gnu.txt`: gawk 5.2.1's results in the recording container,
  `expected.bsd.txt`: macOS's `/usr/bin/awk` (onetrueawk 20200816), both from
  `record.sh` (`--bsd` for the second; `record.pl` runs the cases): one line per case,
  `id<TAB>status<TAB>stdout` escaped as above. Re-record only when the cases change.
- `modes.txt`: cases compared otherwise (sorted lines for `for (k in a)` order, status
  only for `rand()`, or held to the BSD result), each with its reason.
- `expected-failures.txt`: the ratchet. The test fails on a case that fails and is not
  listed, and on a listed case that passes; `PSH_UPDATE_RATCHET=1` rewrites the file
  (keeping the reasons). `PSH_AWK_SHOW=1 ... -- --nocapture` prints each failure.

- `onetrueawk-T/`: onetrueawk's self-checking `T.*` scripts and the files they read,
  except T.arnold and T.beebe (they unpack tar files of other suites) and T.gawk
  (adapted from gawk's GPL suite). `cargo test -p psh --test onetrueawk` (Unix: the
  scripts use the system's tools) runs each with the psh binary, `awk=awk` and
  `PSH_UTILS_FORCE=awk`, and an `./echo` like the suite's echo.c. A script prints a
  `BAD:` line per failing check; psh may fail those gawk fails (`expected.T.gnu.txt`,
  `record.sh --T`: they are where gawk and onetrueawk differ), the rest is the ratchet
  `expected-failures.T.txt` (`script<TAB>line`; regressions and stale lines fail,
  `PSH_UPDATE_RATCHET=1` rewrites it). Its three lines: psh-regex's `.` does not match
  bytes that are not valid UTF-8 (as GNU's regexec), gawk's matcher does.

gawk's own test suite is GPL: it may be run for reference, never vendored.

Where gawk and BSD awk disagree (83 of 772 cases) psh follows gawk (GNU on conflict):
syntax errors exit 1 (BSD: 2), multi-character RS is a regex and RT is set, CONVFMT
and OFMT take integer conversions, `close()` returns a command's exit status, `printf
%c` of "" is a NUL, "\x" escapes, constant division by zero is a parse error,
`"cmd" | getline x+1` binds as `("cmd" | getline x) + 1`, and `next`/`nextfile` in a
function are allowed. The rest are `for (k in a)` order and `rand()`.
