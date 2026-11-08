# 0041: `bash` and `sh` in a command run in psh

status: open
agent: -
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

## Follow-ups

## Result
