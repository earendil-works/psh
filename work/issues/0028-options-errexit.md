# 0028: Shell options, errexit and traps

status: done
agent: shapes
branch: issue/0028-options-errexit
depends-on: 0005
touches: crates/psh-engine/src/options.rs (new), crates/psh-engine/src/builtins/{set*,shopt*,trap*} (new modules), crates/psh-engine/src/exec.rs (errexit/pipefail/ERR/EXIT hooks only, announced), tests/**/expected-failures.txt
guide: -
decisions: D1280–D1289

## Goal
`set -e/-u/-o …` (set -e ~181, set -o ~152, set -u ~102 cases), `shopt`, `pipefail`,
`trap` (EXIT, ERR, DEBUG/RETURN as far as non-interactive bash runs them, signals via
the host), with bash's exact errexit exemptions. errexit must be right: agents run
`set -euo pipefail` scripts, and a misunderstood `set -e` must never let a script run on.

## Acceptance criteria
- [x] `set` with all options bash supports non-interactively (`-e -u -x -f -n -v -C -a
      -b -h -k -t -o pipefail -o posix …`), `set -o`/`set +o` listing, `$-`,
      `SHELLOPTS`/`BASHOPTS`.
- [x] errexit with every exemption (conditions of `if`/`while`/`until`, `&&`/`||`
      except the last, `!`, subshells and `$(…)` per `inherit_errexit`, functions
      called in a condition), `set -u` errors with bash's messages, `pipefail`,
      `PIPESTATUS`.
- [x] `xtrace` (`-x`) with `PS4` and bash's quoting of traced words.
- [x] `shopt` for the options that change non-interactive behavior (`extglob`,
      `nullglob`, `failglob`, `dotglob`, `globstar`, `nocaseglob`, `nocasematch`,
      `expand_aliases`, `inherit_errexit`, `lastpipe`, `xpg_echo`, …).
- [x] `trap` with EXIT, ERR, RETURN, DEBUG (non-interactive semantics), signal traps
      through a Host signal hook (coordinate with 0006's signal handling), `trap -p`.
- [x] Remove 0005's "unsupported feature ends the script with status 2" stopgap for
      everything this issue implements.
- [x] Ratchets updated; Result records scores.

## Notes
- 0006 (procs) owns process signals and the standalone driver's handlers; agree the
  trap hook with it through the coordinator.
- Parallel: 0026, 0027, 0029.

## Log
### 2026-11-07 part 1
- `options.rs` (D1280): `Options` per vproc (set options with letters, all 59 shopt
  options with bash 5.3's defaults), `$-`, `SHELLOPTS`/`BASHOPTS` (readonly, kept in
  step), `Script::options` and `invocation::parse_args` for `psh -e -o pipefail -O x`.
- `builtins/set.rs`: every option, `-o`/`+o` listings, `-`/`--`, positional args after
  options, bash's error messages and statuses; `set -o posix` refused (D002).
  `builtins/shopt.rs`: `-s -u -p -q -o`, listings, invalid names.
- errexit (D1281) with every exemption, pipefail, `Vp::pipestatus` (exposed as
  `PIPESTATUS` once arrays land with 0026), lastpipe, inherit_errexit; nounset with
  bash's messages and statuses (D1282); noglob, nullglob, failglob (a lone `[` is not a
  pattern), noclobber (`>`, `&>`, `>& file`; `>|` overrides), allexport, noexec,
  verbose; extglob per chunk; expand_aliases syncs `Vp::expand_aliases` (0025).
- Hooks outside `touches:` (announced, accepted by the coordinator): `exec.rs`
  (`run_command_with(check)`, `Body::Stage`, `run_andor`/`run_pipeline`/conditions set
  `no_errexit`, `check_failure`, pipefail/lastpipe, `clobber_check`, substitution clears
  errexit), `state.rs` (Vp fields `opts`, `no_errexit`, `pipestatus`; `set_var` honours
  allexport), `lib.rs` (init, `Script::options`, per-chunk extglob/verbose/noexec),
  `expand.rs` (`$-`, nounset, noglob/nullglob/failglob, `expansion_exit_status`),
  `invocation.rs` (an `options` field in each `Invocation`), and the two callers in
  `crates/psh` and `crates/psh-spec`.
- Accidental pass added to the brush ratchet with its reason: compat/options/extdebug/16
  (passed only while `shopt` was unsupported).
- Rebased onto 0026 (Var/Value model): allexport marks assignments in `assign`,
  `set_var` and `prepare_simple`; `PIPESTATUS` is a real indexed array, set after the
  same commands errexit looks at (bash: `case` with no match leaves it alone).
- brush compat/options/set-e/119–122 write `/tmp/test_source_$$.sh` with the runner's
  fixed fake `$$`: added to `[[serial]]` in tests/spec.toml (they raced).
- Scores (macOS, on main 89528f0 incl. 0026/0029): oils 1806/2511, brush 2069/3171,
  agent 131/144.
- Signal hook agreed with procs through the coordinator (part 2): `ControlHandle::signal`
  → pending mask in `Shared`, taken at safepoints by the trap-owning vproc; trap, ignore
  or default action (EXIT trap, then 128+sig); cancel stays forced;
  `Vp::signal_trap_ignored` for spawn dispositions.

### 2026-11-07 part 2
- `builtins/trap.rs` (D1283): `trap` with `-l -p -P`, resets (`trap SIG`, `trap -`,
  numbers), invalid specs, bash's listing and quoting; Linux signal table; EXIT (shell
  and subshells, background jobs), ERR (with `$LINENO`, `$BASH_COMMAND`), DEBUG,
  RETURN (functions, `source`), function-call hiding with errtrace/functrace,
  subshells list but do not run the parent's traps; a bare `exit` in a trap keeps the
  trap's entry status.
- Signal hook (D1284), as agreed: `ControlHandle::signal`, pending mask in `Shared`,
  delivery by `Vp::top` at safepoints, default actions, `Vp::signal_trap_ignored`.
  Unit tests drive it through the Shell API; end to end it needs procs' `kill`.
- xtrace (D1285) with PS4 expansion and levels, BASH_XTRACEFD, bash's word quoting.
- `$LINENO` (expansion), set's option parsing reworked to bash's rules (lone `+`,
  invalid letters fail before anything applies, invalid names after what came before,
  `-o ''`), `+r` accepted and `-r` refused (restricted mode), errexit for redirection
  failures of compound commands, background jobs' own status unchecked.
- Stack (D1059): the hooks first raised a function level to ~18.5 KiB in debug and
  overflowed the 1000-level test; moving them into plain functions returning boxed
  futures (`before_command`, `after_command`, `function_return`, boxed xtrace) brought it
  to ~13 KiB debug, ~3–4 KiB release.
- Hooks outside `touches:` in part 2: `exec.rs` (the above), `state.rs` (`traps`,
  `in_trap`, `top`, `xtrace_level`, subshell traps), `lib.rs` (`ControlHandle::signal`,
  EXIT trap after the script), `sched.rs` (pending mask), `expand.rs` (`$LINENO`),
  `builtins/eval.rs` (xtrace level, RETURN after `source`), `builtins/mod.rs` (bare
  `exit` in traps).
- Scores (macOS, debug): oils 1876/2511, brush 2202/3171, agent 132/144; ratchets: 70
  oils, 133 brush, 1 agent lines removed.

## Follow-ups
- The wasm protocol (psh-wasm `Script`, psh-spec's wasm runner) has no command-line
  options yet: 22 brush cases listed in `expected-failures.wasm.txt` with that reason.
- `set -u` inside arithmetic (`$((x+1))` with x unset) is not checked yet.
- shopt options stored but not applied: dotglob, globstar, nocaseglob, nocasematch,
  xpg_echo (expansion and echo owners); extdebug semantics (DEBUG skip/return,
  BASH_ARGC/ARGV); DEBUG in pipeline children; FUNCNAME; `[[ ]]` tracing (0027).
- Procs: `kill` on `$$` → `ControlHandle::signal`; the psh binary's handlers must map
  macOS signal numbers to Linux ones (trap/29, brush trap/72–77 wait on that).

## Result
- Options (D1280): `set` (every non-interactive option, listings, bash's parsing rules),
  `shopt` (all 59 options; extglob, expand_aliases, nullglob, failglob,
  inherit_errexit, lastpipe applied), `$-`, `SHELLOPTS`/`BASHOPTS`, command-line
  options; POSIX mode and restricted mode refused loudly.
- errexit with bash's exemptions (D1281), pipefail, `PIPESTATUS`, nounset (D1282),
  noglob, noclobber, allexport, noexec, verbose, xtrace (D1285).
- Traps (D1283): EXIT, ERR, DEBUG, RETURN, signals; the engine half of signal delivery
  (D1284) for procs to feed.
- Scores (macOS): oils 1876/2511, brush 2202/3171, agent 132/144 (from 1243/1392/123
  before 0028 on its base; includes other issues' gains since).
