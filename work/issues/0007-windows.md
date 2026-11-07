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
- 2026-11-07 (gate reliability; branch issue/0007-windows-race). Two failures reported by
  infra. (1) "PrlJob_GetRetCode": the VM was suspended under a run. Causes: concurrent
  runs, and old-style runs that could not see newer ones. (2) LNK1104 on test-x64.
  Nothing had survived the Job Object. The holder was Windows on arm64's **XtaCache
  service** (xtac64se.exe): it opens x64 executables that ran under emulation and keeps
  them open while it builds its translation cache, for minutes under load. Reproduced
  with two parallel x64 test loops; Restart Manager (RmGetList) named the holder. Also
  seen: with 3-4 gates in parallel on the 4-vCPU VM, a run took 40 minutes, and an
  emulated psh.exe needed 3-4s to start, which broke the cli tests' 10s deadlines.
  Fix (D1072): runs take turns (a queue with a bounded wait), one shared checkout
  `C:\psh-gate\shared`, kill-and-wait of the whole job after every leg (a leaked test
  process fails the leg), a wait for held executables before test legs plus one retry on
  LNK1104/os error 5, waits for transitional VM states, and one retry on prlctl's own
  failure (255). Gate timeouts were raised to cover the queue: 99-windows 7500s,
  merge-to-main's default PSH_GATE_TIMEOUT 10800s (outside `touches:`, minimal). Tested:
  a single run (cold shared checkout 14 min, warm full run 5 min, warm `test` 34s); A
  running, B queued behind it (A did not suspend), and C started while B suspended the
  VM (C waited, then woke it). Not done: disabling XtaCache (the owner's decision) and
  a cargo runner that runs copies (psh.exe would still be held). The old per-worktree
  directories in C:\psh-gate are now unused and can be deleted.
- 2026-11-07 (x64 cli timeouts). infra's test-x64 failures (cli.rs, 10s deadline per
  psh spawn, 4 times, 27 min leg) all date from 01:00-02:00, when 3-4 gates built in
  the VM at once. Measured with the VM to myself (taking run.lock): x64 psh.exe starts
  in 30-60ms, a freshly linked one in 0.5-0.8s on its first run; the x64 cli tests take
  0.4s, 4.7s with 12 spinning processes on the 4 vCPUs. Defender (MsMpEng) had used 2.5
  hours of CPU in the VM so far and about 10s per x64 relink-and-test. **Changed in the
  VM:** a Defender exclusion for C:\psh-gate (`Add-MpPreference -ExclusionPath
  C:\psh-gate`, as SYSTEM): relink-and-test 23s -> 12s, first start of a fresh x64 exe
  -> 80-150ms. test-x64 stays whole (D1073). The windows leg's inputs gained
  `.gitattributes` (it decides the line endings of the VM's checkout).

## Follow-ups
- Delete the old per-worktree checkouts in the VM (C:\psh-gate\<worktree>) once no
  old-style run is left.
- If the owner agrees: disable XtaCache in the VM (D1072); then the wait and retry are a
  rarely used fallback.
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
