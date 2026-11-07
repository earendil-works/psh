# 0032: Engine follow-ups and the hit list

status: in-progress
agent: engine
branch: issue/0032-engine-climb
depends-on: 0026
touches: crates/psh-engine/ (engine internals per ARCH), tests/**/expected-failures*.txt
guide: engine.md (follow-up)
decisions: D1320–D1329, D1940–D1949 (extension)

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

- Hit list, part 2 (expansion, redirections, here-docs, messages): array substring
  bounds with negative lengths and `${scalar[@]:o:l}` (also LINENO); patsub_replacement
  (D1325); nocasematch for `${x/…}`; `"${v:+"x\y"}"` (backslash quotes any char in a
  nested double-quoted operand); `@Q` uses `$'…'` for control chars and `\'` for a
  lone quote; `@k` of scalars, `@A` of arrays without element 0, `${!ref@a}`;
  `${!empty[@]-DEF}`; `"${a[@]+x}"` with no elements is no word; tilde: `~:` ends the
  prefix, `${x:=~}` expands like an operand (quoted: no tilde), `x=${u-~:~}` uses
  assignment rules; GLOBIGNORE; `*/` matches only directories; quoted `-`/`^` in
  bracket expressions; globskipdots; multibyte IFS chars; `{var}>` redirections (D1326);
  `n>&n` of a closed fd is a no-op; `1>&file`; bash's wording of `>&$fd` errors;
  backticks in here-documents keep `\"`; `${ list; }`/`${| list; }` (D1327); message
  `NAME: command substitution: line N:` for backtick parse errors.
- Not engine (route): `[[ ]]` (0027; brush patterns/patterns/* all need it); `$'…'` in
  a double-quoted `${…}` operand (extquote) and the out-of-range io-number word
  (brush redirection/44,45), here-doc line continuations inside `$( '…' )`, here-doc
  parse warnings and line numbers: parser; `wait` for process substitutions: procs;
  many remaining hit-list failures are macOS vs Linux (hexdump, tac, od, /var/root,
  BSD messages) and pass in the container.

- Host info variables (D1328): UID, EUID, PPID, GROUPS, HOSTNAME, OSTYPE, MACHTYPE,
  HOSTTYPE from `Host::info()`.
- `$(…)` follows `Host::strips_crlf()` (builtins' D1292) instead of `cfg!(windows)`
  (D1940, superseding D1324's "Windows builds only").

- Part 4: well-known variables (D1329), one BASH_SUBSHELL level for a `( )` stage.
- Part 5: prefix assignments see the earlier ones while expanding (`A=1 B=$A cmd`);
  unsetting a temporary binding in the function reveals the shadowed value (unless it
  was made local); `$_` loses the export attribute when updated; `${x:}` is a bad
  substitution; `$LINENO` in `for ((…))` is the header's line on every round. A
  readonly `_` is kept silently (bash also prints `_: readonly variable` after every
  command; an await there grew each recursion level past the stack budget, D1059).

## Follow-ups
- BASH and SHELL (HostInfo has no executable path or login shell): later.
- procs (0030, via the coordinator): oils/builtin-trap/31 is quarantined in
  tests/spec.toml [[flaky]]: its 100 ms window for the child to install the USR1 trap
  races under gate load (failed 3 of 5 gate runs, never locally).
- parser: extquote (`"${x:-$'\t'}"` decodes `$'…'`), brush params/53 and 63 line 3.
- procs: `wait` waits for process substitutions and `$!` names the last one (brush
  redirection/15).
- windows agent: drop the `/C:/x` workaround in psh-os/src/windows/path.rs (D1323);
  Windows/JS `home_dir`.
- builtins agent: `read` drops `\r` before the newline on Windows (D1324).
- parser: unquoted `$(` inside an extglob counts parens (brush command_substitution/51).
- `[[ == ]]` extglob/nocasematch once `[[ ]]` runs (0027).

## Result
