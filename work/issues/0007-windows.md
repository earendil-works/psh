# 0007: Windows CI and process layer

status: in-progress
agent: windows
branch: issue/0007-windows
depends-on: part A: 0001; part B: 0006 (`windows-sys` allowed, D024)
touches: .github/ or scripts/windows-*, crates/psh-os/ (windows), crates/psh-spec/ (Windows normalization), tests/process/ (Windows cases)
guide: windows.md (not written yet; list it as a follow-up)
decisions: D1070–D1079

## Goal
Windows as a first-class platform from the start (D011): the gate runs on Windows, and
psh runs externals there natively with no Git Bash (ARCH.md "psh-os", Windows part and
"Windows rules").

## Acceptance criteria
Part A (can start right after 0001; the coordinator may split it out):
- [ ] The gate's legs run on Windows (x64; arm64 if available) on the owner's Parallels
      VM through `prlctl exec` (D027): a script in `scripts/` syncs the tree into the VM,
      runs the legs there under a timeout and reports. GitHub Actions replaces it once
      the repository has a remote (a follow-up then).
- [ ] The spec runner and its normalization table work on Windows against the same
      goldens.

Part B:
- [ ] `CreateProcessW` suspended → Job Object per job (no breakaway) → resume; only the
      stdio handles inherited; no console window flashes.
- [ ] argv quoting by MSVCRT rules; `.bat`/`.cmd` through `cmd.exe` with hardened
      quoting, refusing what cannot be quoted safely.
- [ ] Waits on waiter threads; TERM/INT = Ctrl-Break to the group then
      `TerminateJobObject`; KILL = `TerminateJobObject`; the whole tree dies.
- [ ] Command lookup with PATH/`Path` and PATHEXT, case-insensitive; no cwd search for
      bare names; shebang scripts run by their interpreter, `#!/bin/sh`/`#!/bin/bash` in
      psh.
- [ ] Paths: `C:\x`, `C:/x`, `/c/x` accepted by `cd` and redirections; `/dev/null` →
      `NUL`; long paths.
- [ ] L5 cases on Windows: timeout kills a tree with grandchildren; no orphan after
      cancel; `npm.cmd`-style shims run with arguments; CRLF scripts.

## Notes
- References: pi-env's `daemon/src/sys/windows.rs` (pi-mono; argv quoting, env sorting
  like libuv), Bun Shell and deno_task_shell (spawning, shebangs, BatBadBut), oh-my-pi's
  `sys/spawn/windows.rs` and `sys/job.rs` (MIT). `research/landscape.md` §3 and
  `research/pi-current.md` §4 list the known pain points.

## Log

## Follow-ups

## Result
