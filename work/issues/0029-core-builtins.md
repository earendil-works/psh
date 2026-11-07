# 0029: Core shell builtins

status: in-progress
agent: builtins
branch: issue/0029-core-builtins
depends-on: 0005
touches: crates/psh-engine/src/builtins/ (new modules: read, eval, source, command, type, hash, printf, echo, umask, times, …), crates/psh-engine/src/builtins/mod.rs (registration lines only), tests/**/expected-failures.txt
guide: -
decisions: D1290–D1299

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

## Follow-ups

## Result
