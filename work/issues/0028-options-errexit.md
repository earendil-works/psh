# 0028: Shell options, errexit and traps

status: in-progress
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
- [ ] `set` with all options bash supports non-interactively (`-e -u -x -f -n -v -C -a
      -b -h -k -t -o pipefail -o posix …`), `set -o`/`set +o` listing, `$-`,
      `SHELLOPTS`/`BASHOPTS`.
- [ ] errexit with every exemption (conditions of `if`/`while`/`until`, `&&`/`||`
      except the last, `!`, subshells and `$(…)` per `inherit_errexit`, functions
      called in a condition), `set -u` errors with bash's messages, `pipefail`,
      `PIPESTATUS`.
- [ ] `xtrace` (`-x`) with `PS4` and bash's quoting of traced words.
- [ ] `shopt` for the options that change non-interactive behavior (`extglob`,
      `nullglob`, `failglob`, `dotglob`, `globstar`, `nocaseglob`, `nocasematch`,
      `expand_aliases`, `inherit_errexit`, `lastpipe`, `xpg_echo`, …).
- [ ] `trap` with EXIT, ERR, RETURN, DEBUG (non-interactive semantics), signal traps
      through a Host signal hook (coordinate with 0006's signal handling), `trap -p`.
- [ ] Remove 0005's "unsupported feature ends the script with status 2" stopgap for
      everything this issue implements.
- [ ] Ratchets updated; Result records scores.

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

## Follow-ups
- The wasm protocol (psh-wasm `Script`, psh-spec's wasm runner) has no command-line
  options yet: 22 brush cases listed in `expected-failures.wasm.txt` with that reason.
- `set -u` inside arithmetic (`$((x+1))` with x unset) is not checked yet.

## Result
