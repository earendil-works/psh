# 0029: Core shell builtins

status: done
agent: builtins
branch: issue/0029-core-builtins
depends-on: 0005
touches: crates/psh-engine/src/builtins/ (new modules: read, eval, source, command, type, hash, printf, echo, umask, times, …), crates/psh-engine/src/builtins/mod.rs (registration lines only), tests/**/expected-failures.txt
guide: -
decisions: D1290–D1299 (used: D1290–D1293)

## Goal
The shell's own builtins beyond 0005's basics, so psh never falls back to host
`/usr/bin` wrappers for them (macOS ships wrappers for `read`, `command`, `type`, …
that pass ~200 cases by accident; this issue replaces them with real builtins and the
cases must pass for real).

## Acceptance criteria
- [x] `read` (all options: `-r -a -d -n -N -p -s -t -u`, IFS splitting, timeouts through
      the Host timer, `REPLY`), `eval`, `source`/`.` (with args, PATH search as bash),
      `command` (`-v -V -p`), `type` (all options; functions via the bash-format printer
      from psh-syntax), `hash`, `builtin`, `enable` (listing), `printf` complete
      (`%q`, `%b`, `%(fmt)T`, `-v`), `echo` (`-n -e -E`, `xpg_echo`), `umask`, `times`,
      `caller`, `exec` (with and without a command; fd manipulation), `exit` details,
      `help` (minimal).
- [x] Every bash builtin name psh does not implement yet fails with a clear
      "not supported yet" error instead of running a host binary of the same name.
- [x] Ratchets updated; the Result records scores and the cases that only passed via
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

### 2026-11-07 part 4
- Part 3 landed at 6effec4.
- `PATH=dir cmd`: the command is looked up in that PATH (one entry at a time through
  `Host::path_entries`), past the hash table and without filling it, as bash
  (exec.rs `run_external_with`: 5 lines). Tests: core_tests (hash table untouched) and
  a real-OS test in `crates/psh/tests/cli.rs` (Unix: a `#!/bin/sh` file).
- umask now starts from `Host::umask()` and reaches files the shell creates itself:
  `builtins::umask::creation_mode` sets `OpenOptions::mode` (D1069) for redirections
  (exec.rs `open_to`) and `UtilCtx::open` (util.rs, one line), only when the shell has
  its own mask. printf `%(…)T` already used `Host::now()`.
- CRLF (coordinator's rule, D1292): `Host::strips_crlf()` (default false, test host
  switch `set_strips_crlf`); `read` drops one unescaped `\r` before the newline for
  whole lines only (no -d/-n/-N), with or without -r. The Windows host and `$(…)` are
  the windows and engine agents' (they were told).
- printf: `-v NAME[SUB]` through `assign_elem`/`assign` (attributes, readonly errors),
  `%Q` (precision cuts before quoting), `%#q`/`%#Q` (single quotes, bash 5.3), `%q`
  quotes `~` after `:`/`=` and `#` only first; `-v` without a name prints the usage.
- `dirs`, `pushd`, `popd` (new `dirs.rs`; `Vp::dirstack`; `+N`/`-N`, `-n`, `-c -l -p
  -v`, `~` for HOME; `DIRSTACK` as a dynamic array: one `Dynamic::Dirstack` variant in
  vars.rs, refreshed in state.rs).
- `ulimit` (D1293, coordinator-approved Host change): `Host::rlimit`,
  `SpawnSpec::rlimits` in host.rs; psh-os unix `rlimit.rs` (getrlimit; setrlimit in the
  fork path of spawn, which `rlimits` now forces, like the umask), test in psh-os
  tests.rs; the test host reports a few fixed limits. Engine `ulimit.rs`: bash's table
  and labels, -S/-H/-a, values attached or separate, `unlimited`/`soft`/`hard`, an
  operand as the last resource's value (or -f), errors as bash. The shell itself is not
  limited (documented gap). Windows and the JS host have no limits: `ulimit` rejects
  every resource there (wasm ratchet lines added with that reason).
- Scores (macOS): oils 2083/2511, brush 2537/3171, agent 140/144, utils 73/73.

## Follow-ups
- `read` reads a byte at a time: a regular file could be read in blocks and the rest
  given back with a Host seek.
- Windows: the directory stack cases fail there (windows ratchet lines, not yet
  investigated); ulimit has no limits on Windows.
- Windows: `strips_crlf` true in WindowsHost (windows agent); `$(…)` CRLF (engine).
- `ulimit`: the shell's own in-process work is not limited (D1293); a Windows
  implementation (job objects) if agents need it.
- `times`: real user/system times need a Host query (getrusage); zeros today.
- `PATH=x builtin …` (e.g. `PATH=x command ls`) sees the temporary PATH as the shell's
  and starts a fresh hash table; bash keeps its table.
- `BASH_SOURCE`/`FUNCNAME`/`BASH_LINENO` arrays from `Vp::calls` once arrays exist; a
  `main` frame for script files.
- `run_text` (builtins/eval.rs) and lib.rs `run_script` are two chunk loops; factor one
  out with the shapes agent (per-chunk options).
- `NAME --help` for builtins (bash prints its long help, status 2).

## Result
Landed in four parts (ad6235d..ebd9b1a, ..dd4ad01, ..6effec4, and part 4).
- Builtins in `crates/psh-engine/src/builtins/`: `read.rs` (every option incl. `-a`,
  timeouts on `Host::sleep`, `TMOUT`, bash's IFS rules, attributes via `Vp::assign`,
  CRLF per D1292), `eval.rs` (`eval`, `source`/`.` with args, `-p`, PATH then cwd,
  `/dev/stdin`, `return`, messages named after the file), `command.rs` (`command
  -pvV`, `type -afptP` with bash's function layout and aliases, `builtin`, `exec -cla`,
  `enable [-anps]` incl. turning builtins off, `times` (zeros), `caller`), `hash.rs`
  (the table, D1291; `PATH=… cmd` bypasses it), `printf.rs` completed (`%q %Q %#q %b`,
  `%(fmt)T` with `strftime.rs`: TZif and POSIX TZ, C-locale strftime; `-v NAME[SUB]`;
  integer ranges), `echo` with `xpg_echo`, `umask.rs` (octal/symbolic; children and
  the shell's own creates), `ulimit.rs` (D1293), `dirs.rs` (`dirs`/`pushd`/`popd`,
  `DIRSTACK`), `help.rs` (`help -dms` with psh's own descriptions, `logout`,
  `history`).
- Every other bash builtin name (`bg`, `fg`, `suspend`, `bind`, `complete`, `compgen`,
  `compopt`, `fc`) fails with "NAME: not supported yet" (status 2) instead of running a
  `/usr/bin` program; `exit 1 2` abandons the line; prefix assignments on builtins are
  temporary and exported (D1290).
- Engine changes others build on: `Vp::calls` (function/source frames for `caller`,
  later FUNCNAME/BASH_SOURCE), `Vp::hash`, `Vp::disabled`, `Vp::umask`,
  `Vp::rlimits`, `Vp::dirstack`; `exec::run_external_with` + `ExtOpts`;
  `builtins::get` (honours `enable -n`); Host: `strips_crlf`, `rlimit`,
  `SpawnSpec::rlimits` (plus the D1069 items procs added for this issue).
- Wrapper passes: measured at the start, 276 cases passed only through macOS `/usr/bin`
  sh wrappers (list: `work/scratch/0029/wrapper-passes.txt`; read, command, type,
  umask, ulimit, kill, wait, jobs, trap, alias, hash, shopt, …). Those of names other
  issues owned were ratcheted as "passed via macOS /usr/bin wrapper" when the stub
  landed and have since been freed by those issues; this issue's own all pass for real.
- Scores (macOS): oils 1111 → 2083/2511, brush 1066 → 2537/3171, agent 115 → 140/144
  (most of the rise from 0026/0028/0006 landing in parallel; this issue's areas:
  builtin-read 61/64, compat read 83/83, eval-source 21/23, dot 14/16, eval 14/14,
  command 33/46, type 15/27 + builtin-type-bash 25/31 (the rest: /usr/bin vs /bin and
  machine paths), hash 17/22, printf 59/63 + 78/83, caller 6/6, enable 7/7, exit 17/17,
  umask 22/24 + 5/5, builtin-dirs 18/18, pushd_popd_dirs 16/18, ulimit (machine values
  differ from the container's)).
- Known gaps: see Follow-ups (the shell is not limited by `ulimit`, `times` zeros,
  `NAME --help`, no `main` caller frame, byte-at-a-time `read`).
