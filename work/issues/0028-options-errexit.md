# 0028: Shell options, errexit and traps

status: open
agent: -
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

## Follow-ups

## Result
