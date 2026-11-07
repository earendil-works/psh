# 0032: Engine follow-ups and the hit list

status: open
agent: -
branch: issue/0032-engine-climb
depends-on: 0026
touches: crates/psh-engine/ (engine internals per ARCH), tests/**/expected-failures*.txt
guide: engine.md (follow-up)
decisions: D1320–D1329

## Goal
Collect the engine's open items and then climb the hit list for engine-owned areas.

## Acceptance criteria
- [ ] Call stack: FUNCNAME, BASH_SOURCE, BASH_LINENO, BASH_ARGV/BASH_ARGC (extdebug as far
      as non-interactive bash does) — brush callstack cases.
- [ ] Extglob matching (`@( ) *( ) +( ) ?( ) !( )`) in pathname expansion, `case`,
      `${x#pat}`-style operators and `[[ == ]]` (coordinate the `[[ ]]` side with the
      parser agent's 0027 via the coordinator), plus the stored-but-unapplied shopt
      options from 0028: dotglob, globstar, nocaseglob, nocasematch.
- [ ] Windows items: `normalize` keeps `C:/x` intact (drop the host's `/C:/x`
      workaround with the windows agent); `$(…)` strips a trailing `\r` before removed
      newlines (decide all-platforms vs Windows; `read` follows the same rule, builtins
      agent).
- [ ] `~user` via the Host (procs' Host info op, or a small additive `home_dir(user)`).
- [ ] Cleanups: remove the write-only `Core.msg_name`/`Core.dash_c`; review infra's
      0008 engine changes (util.rs, dispatch) and procs' D1068 pipe materialization after
      the fact and fix anything you object to.
- [ ] Then: `scripts/status --hitlist`, take the engine-owned areas from the top
      (expansion, word splitting, quoting, redirections, here-docs, parse-error messages),
      in parts; route areas owned by others to the coordinator.

## Log

## Follow-ups

## Result
