# 0041: `bash` and `sh` in a command run in psh

status: in-progress
agent: shells
branch: issue/0041-nested-shells
depends-on: -
touches: crates/psh-engine/src/ (exec.rs command lookup, invocation.rs, lib.rs ShellConfig, builtins type/command/hash, a reader for scripts on stdin), crates/psh/src/main.rs (stdin mode), crates/psh-spec/ (config only if needed), tests/ (new cases), docs/decisions.md, ARCH.md, docs/guide/
guide: docs/guide/processes.md, docs/guide/differences.md, docs/guide/engine.md, docs/guide/cli.md, docs/guide/windows.md
decisions: D1410–D1419 (D1410 is the coordinator's design decision; use D1411–D1419)

## Goal
Implement D1410: a command named `bash` or `sh` (no slash) runs in psh instead of
leaving it for the `bash` on `PATH`. Read D1410 first; it fixes the scope (bare names
only, shebangs untouched, real program for paths and for options psh lacks).

Part 1: `-c`, `FILE` and the options. Part 2: a script on stdin (`| bash`, `bash <<EOF`,
`bash -s`), read one complete command at a time, which also fixes the standalone `psh`
(it reads all of stdin before it runs anything, so a `read` in the script sees EOF
instead of the script's next line).

Why: in 314k bash tool calls of the owner's sessions a bare `bash`/`sh` appears in about
1,400 (`FILE` 733, `-c` 313, `-n` 333; `-c` often under `xargs`/`find -exec`), stdin
scripts in about 3. Today these run bash 3.2 on macOS and fail on Windows and in wasm.

## Acceptance criteria
Part 1
- [ ] Lookup: functions, builtins, then `bash`/`sh`, then utilities and `PATH`; works
      without a utility policy (bare engine, wasm); through `env`, `xargs`, `find -exec`,
      `timeout`, `nohup`, `time` (`UtilCtx::spawn`); `exec bash -c …` in place.
- [ ] A path (`/bin/bash`, `./bash`, `command -p bash`, a hashed path) runs the program;
      on Windows `/bin/sh`, `/bin/bash`, `/usr/bin/sh`, `/usr/bin/bash` run psh.
- [ ] Starts afresh (reuse `exec::start_afresh` and the D1300 path): exported variables
      only, no functions/aliases/traps/options; descriptors and cwd shared; `SHLVL+1`,
      `$$`/`BASHPID` its own, `$PPID` the caller's `$$`, `BASH_SUBSHELL` 0; `exit`
      ends only the nested shell; `set -e` and traps do not leak either way.
- [ ] Command line through `invocation::parse_args`: `-c STR [ARG0 ARGS…]`,
      `FILE [ARGS…]`, `-n`, `-e -u -x -v -f -o/+o -O/+O`, combined letters (`-euc`,
      `-lc`), `--norc`/`--noprofile`; `-l`/`--login` accepted, no profile read;
      `--version` prints psh's version. Error wording as bash's (`bash: FILE: No such file
      or directory`, 127; syntax errors under `-n` with `FILE: line N:`).
- [ ] Unsupported options (`-i`, `--rcfile`, `--init-file`, `-r`, `--restricted`,
      `--posix`, `-D`) and, until part 2, no script operand / `-s` run the real program
      from `PATH` if present, else status 2 with a message naming psh.
- [ ] Off switch in `ShellConfig` (and an env var for the psh binary, like
      `PSH_UTILS_OFF`): both names go to `PATH` again.
- [ ] `type bash`, `type -t`, `command -v`, `which`, `hash` report it like a utility
      (D1083/D1200).
- [ ] Cancellation: `timeout 1 bash -c 'sleep 10'` ends in time; killing the run ends
      nested shells and their externals.
- [ ] Spec cases (goldens from the container) for all of the above, including
      `xargs -I{} sh -c '…' _ {}`, `find … -exec bash -c '…' _ {} \;`, `bash -c` with
      `$0`/`$@`, exported vs plain variables, nested `bash -c "bash -c …"`, `bash -n` on
      good and bad files. Windows leg passes the same cases.

Part 2
- [ ] A shell whose script is stdin (bare `bash`/`sh`, `-s ARGS`, the standalone `psh`
      without `-c`/FILE) reads and runs one complete command at a time (the parser's
      `incomplete` errors tell when to read more; here-doc bodies included), leaving the
      rest of stdin to the commands: `printf 'read x\nhello\necho got $x\n' | bash`
      prints `got hello`, as does the same through a here-doc and a file redirect.
      Pipes are read without overshooting (bash reads them byte by byte; seekable
      input may read ahead and seek back).
- [ ] Syntax errors stop at the failing command as bash's do (earlier commands ran).
- [ ] Spec cases: `| bash`, `bash <<'EOF'`, `bash -s a b <<EOF`, `bash < file`,
      `curl … | sh` shape with a local file, `cat script | psh`.

Both parts
- [ ] ARCH.md command lookup bullet, and the guide pages in `guide:` (processes,
      differences, engine, cli; windows for the path names); D1411+ for what you decide.
- [ ] Ratchets updated if cases newly pass.

## Notes
- Reuse, don't duplicate: `exec.rs` `run_in_shell`/`start_afresh` (D1300),
  `util.rs` `UtilCtx::spawn_shell` (D1191), `invocation.rs` `parse_args` (shared with
  the psh binary and psh-spec). `-n` is noexec via `set`'s letters already.
- `sh` runs in bash mode (POSIX mode is refused, D1280); note it on differences.md.
- Fall-through to the real program must find a program on `PATH` that is not psh
  itself (the psh binary may be installed as `bash` somewhere; skip it or document).
- The classifier already parses literal `bash -c` strings; nothing to change there.
  Leave shebang handling alone (D1410).
- Oracle probes use `/opt/homebrew/bin/bash` by path; that must keep running bash.

## Log
- 2026-11-08 (shells), part 1:
  - Lookup: `util::lookup` returns an engine-internal entry (`util/nested.rs`) for a bare
    `bash`/`sh` (on Windows also `/bin/sh`, `/bin/bash`, `/usr/bin/sh`, `/usr/bin/bash`)
    after hashed names and before slashes and the registry, so `command`, `exec`,
    wrappers (`run_wrapped`), `type`, `command -v` and `which` all see it, with or
    without a policy. `util::run_with` starts it through `run_external_with`
    (`ExtOpts::shell`).
  - Found: `start_afresh` (D1300) dropped every unexported variable, so a script psh ran
    itself had an empty `IFS`, no `$RANDOM`, `PPID`, `BASH_VERSION`. It now rebuilds the
    variables from the exported ones via `lib::initial_vars` (factored out of
    `Shell::new`), plus `SHELLOPTS`/`BASHOPTS` import (D1412).
  - First version ran the nested shell inline; the full run showed 6 brush regressions
    (`sh -c 'trap … TERM' &` then `kill $!`/`kill $(cat pidfile)`). With the
    coordinator: a nested shell is now a virtual process (D1411): an `ExtProc` over its
    own task, with its own `$$` and pending-signal mask (`Vp::sigs`, replacing the
    scheduler's mask at the ~10 signal sites), a registry of nested shells by `$$` for
    `kill`, abandon = TERM to it and its foreground wait, task dropped after 2 s. Also
    `kill <the shell's pid>` from a nested shell goes to the shell's mask (wasm and
    embedders: never to the embedding process).
  - `exec -a/-l` name a nested shell's `$0`; `exec bash` keeps `$$`, `PPID`, `SHLVL`.
    bash's exec of a last command (`-c`, `( )`, `$( )`) keeps `SHLVL` too, with odd rules
    (probed: `( bash … )` keeps, `( exec bash … )` counts); psh always counts (D1412).
  - utils/env/basic/7 (`env PATH=/nonexistent sh -c true` → 127) changed to `uname`
    (neither a utility nor a nested shell), with the coordinator's OK; the golden did not
    change (still 127). That `env PATH=/nonexistent sh -c 'echo ok'` works now is tested
    in the engine (`nested::tests::runs_without_a_policy_or_path`) and the psh binary
    (`tests/cli.rs nested_shells`), since bash's golden says 127 there.
  - Ratchets: `brush/compat/builtins/jobs/12` (`/proc/$$` of a bare `sh`) in the shared
    file, reason D1410 (coordinator's OK). wasm: `brush/compat/signals/12`, `/22` were
    "needs external programs" there; they now reach the existing wasm limitation (a shell
    cannot die by a signal, D1903) and are in `expected-failures.wasm.txt` (OK'd).
    `scripts/spec-linux --update` also wanted to move ulimit/pushd entries from the shared
    file to the macOS one, which would have broken the wasm leg; I left those as they
    were (unrelated to this issue).
  - Windows ratchet: `oils/builtin-umask/10`, `/11` (no group/other bits) and
    `oils/sh-options-bash/4` (`$SH` is `psh.exe`, the case's `sed 's/.*sh /sh /'` misses
    it) left the shared file as passing on Linux and macOS, so they went into
    `expected-failures.windows.txt`; `sh-options-bash/5` (needed bash) passes there now.
    The process tests (`tests/process/unix.rs`, `standalone.rs`) start `/bin/sh` by path,
    since they need a real process; `tests/cli.rs` `prefix_path_is_used_for_lookup` uses
    `uname` instead of `sh`.
  - psh-spec: psh's side also maps a leading `bash:` to `<sh>` (a nested bash names
    itself `bash`, as the recorded one does).
  - New cases: `tests/agent/nested-shells.test.sh` (19 cases, goldens recorded); engine
    tests in `util/nested.rs`; `tests/cli.rs` `nested_shells`,
    `timeout_sends_term_to_a_nested_shell`.
  - Docs: ARCH.md (command lookup, nested shells, control), guide processes.md (new
    section), differences.md, engine.md, cli.md, windows.md.

- 2026-11-08 (shells), D1413 (owner's design change, superseding D1411's `$$`): a
  nested shell has no process id. `pidscan.rs` scans a `-c` string or FILE (parsed;
  `$(…)`, here-docs, backquotes and literal `bash -c '…'` included) for `$$`/`${$…}`/
  `BASHPID`; a hit runs the first real program on `PATH` that is not psh itself
  (`ShellConfig::own_program`, from `current_exe` in the binary). Otherwise `$$` and
  `BASHPID` in a nested shell, and `BASHPID` in any subshell, are expansion errors
  (`$$: this shell runs inside psh and has no process id`). `Vp::shell_pid`/`bashpid`
  are `Option`s; the `Core` registry of nested shells for `kill` is gone; `$!` and jobs
  unchanged (job ids from `next_job_id`). A fall-through program a signal ended ends the
  nested shell with that signal, reported with its real pid.
  - Ratchets: `jobs/12` passes on Linux now (real sh), its line is gone; wasm
    `signals/22` passes there now (line gone). New shared lines with D1413's reason:
    oils `vars-special/13`, brush `signals/38`, `/42` (`BASHPID` in subshells).
  - Spec cases: the fall-through path is `agent/nested-shells/19` (`kill -0 $$`,
    `ps -p $$`, a nested `$PPID`), and cases 4, 15, 16 now run their `$$` parts under
    the real bash. The two error paths (no real bash; `BASHPID` in a subshell) cannot
    match a bash golden, so they are tested in the engine
    (`nested::tests::no_process_id`, `pid_uses_run_the_real_program`, `pidscan` tests)
    and the binary (`tests/cli.rs nested_shells_have_no_pid`) instead. The case-4 line
    comparing `$$` with `$BASHPID` was dropped: under fall-through on macOS it runs
    bash 3.2, which has no `BASHPID`.

## Follow-ups
- psh's `timeout` drops its child at the deadline and returns at once; GNU timeout
  signals and then waits for it. With a nested shell that traps TERM, its trap output
  comes after timeout's status (`UtilChild` has no `signal`).
- `set +B` / `bash +B` do not turn brace expansion off (pre-existing).
- `psh -c 'if'` says `-c: line 1:` where bash says `-c: line 2:` (pre-existing).
- `psh --utils` does not list `bash`/`sh`.
- wasm: psh-spec could compare statuses like on Windows (`Signal(n)` ≈ `Exit(128+n)`),
  which would empty the "no process to die by it" wasm entries.
- `scripts/spec-linux --update` moves shared entries that fail on macOS and wasm into the
  macOS file only (ulimit cases), which breaks the wasm leg.

## Result
