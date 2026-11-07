# 0030: Process follow-ups: run-in-shell, host info, signal robustness

status: open
agent: -
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

## Follow-ups

## Result
