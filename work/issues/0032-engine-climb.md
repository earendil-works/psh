# 0032: Engine follow-ups and the hit list

status: in-progress
agent: engine
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
- Call stack (D1320): dynamic FUNCNAME/BASH_SOURCE/BASH_LINENO/BASH_ARGV/BASH_ARGC from
  the vproc's frames; extdebug frames for BASH_ARGV/ARGC.
- Patterns (D1321): `MatchOpts` (utf8, extglob, nocase); extglob groups in globbing,
  `case`, `${x#…}`/`${x/…}`; dotglob, globstar, nocaseglob, nocasematch (case) applied.
  `[[ ]]` is not run by the engine yet ("not supported"): whoever lands it uses
  `Pattern::with(pat, vp.match_opts(nocasematch))`. Ratchet: brush
  command_substitution/51 passed only because extglob was unsupported; it needs the
  parser to count parens of an unquoted `$(` inside an extglob (parser follow-up).
- `~user` through `Host::home_dir` (D1322), psh-os Unix via `getpwnam_r` (touches
  psh-os/src/unix/mod.rs, minimal).
- Windows: `normalize` keeps `C:/x` (D1323); `$(…)` drops `\r` before removed newlines
  on Windows only (D1324).
- Cleanups: removed `Core.msg_name`/`Core.dash_c`. Reviewed util.rs and its dispatch
  (lookup after functions/builtins, before PATH): fine. Reviewed D1068: changed vproc
  reads of a materialized pipe to read the OS end straight into the caller's buffer
  instead of a 16 KiB read-ahead (one copy and allocation less per read).

## Follow-ups
- windows agent: drop the `/C:/x` workaround in psh-os/src/windows/path.rs (D1323);
  Windows/JS `home_dir`.
- builtins agent: `read` drops `\r` before the newline on Windows (D1324).
- parser: unquoted `$(` inside an extglob counts parens (brush command_substitution/51).
- `[[ == ]]` extglob/nocasematch once `[[ ]]` runs (0027).

## Result
