# 0029: Core shell builtins

status: in-progress
agent: builtins
branch: issue/0029-core-builtins
depends-on: 0005
touches: crates/psh-engine/src/builtins/ (new modules: read, eval, source, command, type, hash, printf, echo, umask, times, …), crates/psh-engine/src/builtins/mod.rs (registration lines only), tests/**/expected-failures.txt
guide: -
decisions: D1290–D1299 (used: D1290, D1291)

## Goal
The shell's own builtins beyond 0005's basics, so psh never falls back to host
`/usr/bin` wrappers for them (macOS ships wrappers for `read`, `command`, `type`, …
that pass ~200 cases by accident; this issue replaces them with real builtins and the
cases must pass for real).

## Acceptance criteria
- [ ] `read` (all options: `-r -a -d -n -N -p -s -t -u`, IFS splitting, timeouts through
      the Host timer, `REPLY`), `eval`, `source`/`.` (with args, PATH search as bash),
      `command` (`-v -V -p`), `type` (all options; functions via the bash-format printer
      from psh-syntax), `hash`, `builtin`, `enable` (listing), `printf` complete
      (`%q`, `%b`, `%(fmt)T`, `-v`), `echo` (`-n -e -E`, `xpg_echo`), `umask`, `times`,
      `caller`, `exec` (with and without a command; fd manipulation), `exit` details,
      `help` (minimal).
- [ ] Every bash builtin name psh does not implement yet fails with a clear
      "not supported yet" error instead of running a host binary of the same name.
- [ ] Ratchets updated; the Result records scores and the cases that only passed via
      host wrappers before.

## Notes
- Arrays (`read -a`, `mapfile`) need 0026's `Var`; coordinate through the coordinator,
  or land the scalar forms first.
- `alias`/`unalias` are written by the parser agent (0025); `wait`/`kill`/`jobs` by the
  procs agent (0006 part 3); `set`/`shopt`/`trap` by 0028; `declare` family by 0026.

## Log

### 2026-11-07 part 1
- Measured first: stubbing every unimplemented bash builtin name lost 276 cases that
  passed through macOS `/usr/bin` wrappers (list in `work/scratch/0029/wrapper-passes.txt`;
  read, command, type, umask, ulimit, kill, wait, jobs, trap, alias, hash, shopt, ...).
- New modules: `read.rs` (all options but `-a`, which needs 0026's arrays; byte-at-a-time
  reads, `-t` through `Host::sleep`, `TMOUT`, `-n/-N` counting characters in UTF-8,
  bash's splitting incl. the "one field + one trailing separator" rule), `eval.rs`
  (`eval`, `source`/`.` with args, `-p PATH`, PATH then cwd search, `/dev/stdin`, `return`,
  messages named after the sourced file; `run_text` runs text chunk by chunk with
  aliases, like `run_script`), `command.rs` (`command -pvV`, `type -afptP` with functions
  through `print_function_bash`, aliases when `expand_aliases`, `builtin`, `exec -cla`,
  `enable [-anps]` incl. disabling, `times` (zeros: no clock in the Host)), `hash.rs`.
- Shared engine files (reported to the coordinator before landing):
  - `expand.rs`: an empty `""` word made no field (`f a ""` gave `$#`=1): one line.
  - `exec.rs`: prefix assignments of every builtin are temporary and exported (D1290);
    `exec` is a real builtin (the special case in `run_argv` is gone; `run_simple`'s
    "exec without a command keeps its redirections" stays); `run_external_with` +
    `ExtOpts` (argv0, empty env, search path) for `exec`/`command -p`; lookups through
    the hash table (D1291); spawning a directory says "Is a directory" (126);
    `builtins::get` (honours `enable -n`) instead of `find`; `dev_fd` and
    `DEFAULT_PATH` pub(crate).
  - `state.rs`: `Vp::sourced` (stack of sourced files; `return` and message names),
    `Vp::hash`, `Vp::disabled`; `Vp::msg_name()` used by `error()`. `lib.rs` inits.
  - `tests.rs`: `run`/`run_setup`/`check`/`check_status`/`Out` pub(crate), for
    `builtins/core_tests.rs`.
  - `alias.rs` (parser's module): `definition` pub(crate) for `command -v`.
- Not yet stubbed: the "not supported yet" fallback for other bash builtin names is
  written but off; turning it on now would lose ~190 wrapper passes on names other
  agents own (kill, wait, jobs, trap, shopt, declare, mapfile, ulimit, ...). Plan: turn
  it on in a later part, coordinated with them.
- Accidental passes added to the ratchet with reasons: oils/builtin-read/54 (`read -a`),
  brush exec/1 (passed because bare `exec` ended the script), brush signals/21 (the
  `/usr/bin/command` wrapper was `$PPID`).
- Scores (macOS, before → after): oils 1111 → 1251/2511, brush 1066 → 1240/3171,
  agent 115 → 120/144.

### 2026-11-07 part 2
- Landed part 1 as ad6235d..ebd9b1a (gate: oils 1251, brush 1238, agent 120).
- Every bash builtin name psh lacks now fails with "NAME: not supported yet" (status 2,
  the script stops) instead of running `/usr/bin/NAME`. Coordinator's call: all 181
  wrapper passes lost to it are in the ratchets as "passed via macOS /usr/bin wrapper"
  (shopt 48, trap 29, wait 28, ulimit 24+, declare/typeset 26, kill 16, fg, pushd,
  dirs); owners delete those lines as they implement their builtins.
- printf: `%q` worked nowhere (`q` was eaten as a length modifier); `-vNAME`; `%b` takes
  `\nnn` without the 0 (new `Dialect::PercentB` in escape.rs); integer range errors
  ("Numerical result out of range", clamped like strtoimax/strtoumax, `%u -1`);
  `%#.0f`/`%#.0e`; bash's wording of format errors; `%(fmt)T` with a new
  `strftime.rs`: TZif files (RFC 8536, v1-v4) from `/usr/share/zoneinfo` etc. through
  the Host, POSIX `TZ` rules (`EST5EDT,M3.2.0,M11.1.0`, `<+0330>-3:30`), an exported
  `TZ` only, else `/etc/localtime`, else UTC; C-locale strftime with glibc's flags and
  widths; bash's 128-byte limit. "Now" (-1, -2, no argument) is `SystemTime::now()`
  outside wasm until the Host has a clock (proposed).
- `caller`: shared-file change announced to the coordinator: `Vp::calls: Vec<Call>`
  (function name or `source`, the file of the code, the calling line; replaces part 1's
  `Vp::sourced`), pushed by `call_function` (exec.rs) and by `source`; `Func::source`
  records the defining file (`define_function`). Output follows bash's
  FUNCNAME/BASH_SOURCE/BASH_LINENO; no `main` frame yet (a script file's top level
  says status 1 where bash prints `0 NULL`).
- `umask` (octal, symbolic incl. `a=u` copies of the original value, `X`, `-S`, `-p`):
  `Vp::umask: Option<u32>` passed to children through `SpawnSpec::umask`. Files the
  engine itself creates do not see it yet: Host proposal sent to the coordinator
  (`Host::umask()` for the initial value, `OpenOptions::mode` with fchmod on create;
  plus `Host::now()`).
- `help [-dms]` (bash's synopses, psh's own one-line descriptions, no bash help text),
  `logout` (not a login shell), `history` (empty, arguments checked), `builtin -x`
  is an invalid option, `exit 1 2` abandons the rest of the line (status 2).
- Scores (macOS, ratchet run): oils 1215/2511, brush 1150/3171, agent 119/144 — lower
  than part 1 only because of the stubs; without them oils 1279, brush 1263, agent 120.

### 2026-11-07 part 3
- Part 2 landed at dd4ad01 (squashed after repeated rebases onto 0026, 0028, 0006).
  Most wrapper-pass ratchet lines are gone again: those builtins landed. Still stubbed:
  ulimit, dirs/pushd/popd, bg/fg, suspend, bind/complete/compgen/compopt, fc.
- `read -a` (arrays from 0026: all fields, a trailing separator makes no empty field;
  readonly and associative targets refused before reading), and every `read` assignment
  goes through `Vp::assign` (readonly, `-i`, `-l`/`-u`, namerefs). Counts and fds take
  `+N` and blanks, `-t +1` too.
- `echo` honours `shopt -s xpg_echo` (`Shopt::XpgEcho` added to options.rs: one
  variant, 0028's file).
- Scores (macOS): oils 1980/2511, brush 2419/3171, agent 133/144.

## Follow-ups
- `read` reads a byte at a time: a regular file could be read in blocks and the rest
  given back with a Host seek.
- Host: `umask()`, `OpenOptions::mode`, `now()` (proposal in part 2's log).
- `BASH_SOURCE`/`FUNCNAME`/`BASH_LINENO` arrays from `Vp::calls` once arrays exist; a
  `main` frame for script files.
- `run_text` (builtins/eval.rs) and lib.rs `run_script` are two chunk loops; factor one
  out with the shapes agent (per-chunk options).
- `NAME --help` for builtins (bash prints its long help, status 2).

## Result
