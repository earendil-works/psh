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
- [x] The gate's legs run on Windows (x64; arm64 if available) on the owner's Parallels
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
- 2026-11-06 (part A). VM "Windows 11" (Parallels, Windows 11 25H2 build 26200 on
  arm64, 4 vCPUs). Already there: Visual Studio Community 2022 17.14 with MSVC 14.44
  (arm64, x64, x86 targets), Windows SDK 10.0.26100.0, Git for Windows 2.55, winget.
  **Installed:** rustup via `winget install --id Rustlang.Rustup -e --architecture arm64`
  (rustup 1.29.3, `C:\Users\mitsuhiko\.rustup`, `.cargo`), which brought stable 1.99.1
  (aarch64-pc-windows-msvc, default profile); then target `x86_64-pc-windows-msvc` and
  toolchain 1.96 (minimal). The runner installs the host's exact rustc version and the
  MSRV on first use (`--profile minimal -c clippy,rustfmt`), so toolchains accumulate
  there over time. Nothing else changed in the VM besides `C:\psh-gate\` (checkouts,
  logs, pid files).
- Findings: `prlctl exec --current-user` runs as the logged-in user (without it, as
  SYSTEM, without winget). prlctl quotes arguments that contain spaces but passes
  embedded `"` through raw, so the script refuses paths with spaces or quotes; guest
  exit codes propagate. Killing `prlctl exec` on the host does **not** kill the guest
  process: hence the Job Object and the `-Kill` cleanup (D1070). Windows PowerShell 5.1
  turns redirected native stderr into error records, so native commands run through
  `cmd /c ... 2>&1`. x64 binaries run under emulation without any setup, so `test-x64`
  is a real leg. Git over the share needs `-c safe.directory=*`.
- Verified: a failing fmt fails the leg (exit 1) with the diff; a test that hangs with a
  `start /b` grandchild hits the leg timeout (124) and leaves no process in the VM; a
  TERM to the host script (as gate.sh's timeout sends) kills the runner and its tree
  and suspends the VM again; two concurrent runs from different worktrees: only the last
  one out suspends, and a VM that was running beforehand stays running.
- Timings now (tiny workspace): ~5s resume, ~2s fetch, legs 3-13s warm; a fresh boot is
  about a minute. Cold build of a new worktree's checkout ~30s.
- Decisions D1070 (legs, sync, runner) and D1071 (default on, last; `PSH_GATE_WINDOWS=0`
  escape hatch, logged; VM refcount), agreed with the coordinator. Updated TESTING.md
  ("The gate", "Windows"), outside `touches:` as the coordinator asked; ARCH.md's D027
  paragraph is still accurate. Guide: none affected (windows.md not written yet).

## Follow-ups
- Part A, second box: the spec runner and its normalization table on Windows, once 0002
  lands (a `spec` leg in windows-gate.ps1 that runs the same ratchets against the
  goldens).
- GitHub Actions replaces the VM once there is a remote (D027).
- The windows leg runs on every merge; once legs are cached by their inputs (TESTING.md),
  a docs-only change should skip it too.
- `docs/guide/windows.md` (not written yet).

## Result
Part A (first box): `scripts/windows-gate.sh [LEG...]` runs `fmt`, `clippy`, `test`,
`test-x64`, `msrv` on the Parallels VM against the worktree as it is (uncommitted changes
included); `scripts/gate.sh` runs it as its last leg, `windows`. Known gaps: the VM must
have a logged-in user (`--current-user`); paths with spaces cannot be passed through
prlctl; the spec ratchets are not run on Windows yet (waits for 0002).
