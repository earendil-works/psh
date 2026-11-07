# 0030: Process follow-ups: run-in-shell, host info, signal robustness

status: in-progress
agent: procs
branch: issue/0030-process-followups
depends-on: 0006
touches: crates/psh-os/, crates/psh-engine/src/{host.rs,exec.rs (run_external only),jobs.rs}, crates/psh-spec/ (leak check), tests/process/
guide: processes.md
decisions: D1300–D1309
extension: -

## Goal
Close the process-layer follow-ups from 0006 and the coordinator's queue.

## Acceptance criteria
- [ ] **Run in shell (D1079)**: `pub struct RunInShell;` marker in host.rs; spawn returns it
      (Unix: ENOEXEC instead of D1063's /bin/sh fallback; Windows: `#!/bin/sh`/`#!/bin/bash`
      and files with neither interpreter line nor executable extension — the windows agent
      switches its spawn once the engine side is in). The engine runs `spec.program` as a
      psh script in a subshell vproc (argv[1..] as `$@`, `$0` = argv[0], same env/cwd/fds),
      reusing the existing script runner. Decision + tests.
- [ ] **Host info**: an additive Host op for UID, EUID, PPID, HOSTNAME, OSTYPE, MACHTYPE,
      HOSTTYPE, GROUPS (sensible defaults; per-platform values as bash reports them;
      propose Windows values). The engine agent wires the variables.
- [ ] **Signal robustness**: the flaky binary cases (brush kill/5, kill/6, signals/13,
      oils background/7) pass 200× under `-j 12` load or are understood and fixed;
      blocking reads give way to fatal signals; the foreground "Terminated" stderr report
      as bash prints it non-interactively.
- [ ] **psh-spec per-case leak check**: after each case run through the binary or
      in-process, no process from that case survives (fails the case otherwise).

## Log

### 2026-11-07 part 1: RunInShell (D1300)
- host.rs: `RunInShell` marker (`error()`, `is()`). Engine `run_external_with`: on it,
  `run_in_shell` reads the file through the host, refuses binaries (bash's NUL test),
  and runs it in a fresh subshell via `run_script` (now `pub(crate)`). `Vp::msg_name`
  uses `$0` (was the Core's), syntax-error and `${x?}` status wording use `vp.opts.dash_c`,
  getopts messages `$0`. psh-os returns `RunInShell` for ENOEXEC (no more `/bin/sh`).
  Test host: a file of its fs is a script. Tests: engine `jobs_tests` (args, fresh
  state, messages, binary refusal), L5 (`./s` without `#!`).
- Found: `trap '' SIG` never reached children (`signal_trap_ignored` was unwired) and
  psh-os's `sig_ignore` skipped the Linux→OS mapping (USR1 = 30 on macOS); both fixed,
  L5 test. +3 brush trap/signal cases.
- Windows (for the windows agent): return `RunInShell::error()` from `spawn` for the
  D1079 cases; nothing else needed.

### 2026-11-07 part 2: Host::info (D1301)
- host.rs: `HostInfo` + `Host::info()` (default for OS-less hosts). psh-os: getuid,
  geteuid, getgroups, getppid (embedded: the embedding process), gethostname, uname
  release for darwin; unit test against `id`, `hostname`, `uname -r`, `ps` (macOS and
  Linux). Windows values proposed in D1301. The engine agent wires the variables.

### 2026-11-07 fix: a trap ran twice for one signal (D1302)
- Cause: the handler set the pending bit and the watcher thread then called
  `ControlHandle::signal`, setting it again; if the shell took the first bit in
  between, the trap ran twice. The watcher now only wakes (`ControlHandle::wake`).
  30/30 alone and 60/60 under 12 parallel runs × 4 threads.
- Note: running the standalone tests from a backgrounded shell job (`cmd &` in an
  interactive shell) makes `int_and_hup_too` fail: the job starts with SIGINT ignored and
  psh keeps it ignored, as bash does. Not a psh bug; run them in the foreground or via
  `xargs -P`.
- `int_and_hup_too` in other agents' gates ("psh did not exit"): their gates run where
  SIGINT (or SIGHUP) is ignored, and psh rightly keeps such a signal ignored; the test
  now skips a signal the test process ignores. Stress: the standalone suite 100× (10 in
  parallel, 4 threads each, with the L5 process suite running alongside): 100/100.
- oils/process-sub/5 and /7 under `-j 8` load (reported by infra): not reproduced in ~90
  loaded runs; need a diff when it recurs.

### 2026-11-07 part 3a: signal dispositions in tests (D1304), the Terminated report (D1303)
- Coordinator: `ignored_signals_and_umask_take_effect` failed in text's gate (its gate ran
  with SIGINT ignored). Sweep: every test that spawns externals or psh expected default
  dispositions. Fix at the source instead of per test: `UnixConfig::inherit_ignored`
  (off by default, on in the psh binary), and psh-spec plus the standalone tests reset
  the ignored asynchronous signals before starting anything. Verified with everything
  run under `perl -e '$SIG{...}="IGNORE"…; exec'` for INT TERM HUP QUIT USR1 USR2 ALRM,
  from a backgrounded job and under `nohup`: all pass. No change to timeout.sh needed.
- D1303: the "Terminated" / "line N: PID Killed" report after a foreground simple
  command; `wait`'s report pads the pid to 5 columns. L5 test.

## Follow-ups

## Result
