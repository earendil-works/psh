# 0007: Windows CI and process layer

status: in-progress
agent: windows
branch: issue/0007-windows
depends-on: part A: 0001; part B: 0006 (`windows-sys` allowed, D024)
touches: .github/ or scripts/windows-*, crates/psh-os/ (windows), crates/psh-spec/ (Windows normalization), tests/process/ (Windows cases)
guide: windows.md (not written yet; list it as a follow-up)
decisions: D1070–D1079, D1910–D1919

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

### 2026-11-07 Part B design (draft; no code until 0006 lands its Host additions)
Read: psh-os on main (`lifecycle.rs`, `timer.rs`, `unix/`), the Host v1 contract (0005
log), the 0006 plan, research/execution-core.md §5, oh-my-pi `sys/spawn/windows.rs` and
`sys/job.rs` (MIT), and pi-env `daemon/src/sys/windows.rs`. Wherever possible the plan
mirrors the Unix host.

**Crate.** `crates/psh-os/src/windows/` (`mod.rs` WindowsHost + fs ops, `spawn.rs`,
`job.rs`, `cmdline.rs` argv/batch quoting, `file.rs` WinFile, `poller.rs`, `path.rs`,
`shebang.rs`). The dependency is `windows-sys = "=0.61.2"`, the version pi-env pins,
under `[target.'cfg(windows)'.dependencies]`, with only the features used. The `deps`
leg gets the Windows set. Wherever an ntdll call is needed
(`NtQueryInformationFile`, later `NtSuspendProcess`), it goes through windows-sys `Wdk_*`
features if they cover it, otherwise `raw-dylib`. `lifecycle.rs` and `timer.rs` are
shared as they are. On Windows, "reap" means closing the process handle: while we hold
it, the pid cannot be recycled, which gives the same guarantee as `WNOWAIT`.

**WindowsHost** (`WindowsConfig { placement, grace, pid }`):
- `Placement::Session`, the default (agents, embedding): every external gets
  `CREATE_NEW_PROCESS_GROUP` plus a Job Object of its own. `Inherit`, for the standalone
  psh in a console: no new group, so the console's Ctrl-C reaches the children as it
  would with cmd. Each external still gets its job, so kills reach the whole tree.
- Console windows: when the host has no console (a daemon or GUI embedder),
  externals get `CREATE_NO_WINDOW`, so nothing flashes. Without a shared console,
  ctrl-break cannot be delivered, so TERM falls back to killing the job (as oh-my-pi
  does). The host never allocates or changes the process's console: an embedder's
  process-wide state stays untouched.
- `leftovers()` lists the jobs that still have active processes
  (`JobObjectBasicAccountingInformation`), for the L5 leak checks, same as Unix.

**Spawn.** `CreateProcessW` with `CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT |
EXTENDED_STARTUPINFO_PRESENT` (+ group/no-window as above), then assign to the job, then
`ResumeThread`, so nothing the child starts escapes the job. The job is created without
breakaway. Only the stdio handles are inherited (`PROC_THREAD_ATTRIBUTE_HANDLE_LIST`,
deduplicated), so concurrent spawns never leak each other's pipe ends.
- Proposed: the job gets `KILL_ON_JOB_CLOSE`, and `release()` clears it
  (`SetInformationJobObject`) before closing. Then a crashed or killed psh takes its
  unreleased externals with it. Unix cannot offer that, so it would be a decision.
- argv: MSVCRT quoting (std's `make_command_line` rules, argv[0] included). Programs
  that parse their own command line (cmd.exe, msiexec) get exactly this quoting; there is
  no raw-argument escape hatch in v1.
- `.bat`/`.cmd`: run through `%SystemRoot%\System32\cmd.exe /e:ON /v:OFF /d /c` with std's
  hardened batch quoting (BatBadBut, CVE-2024-24576). Arguments it cannot quote safely
  are refused with an `InvalidInput` error and a clear message. The code is copied from
  std or oh-my-pi `cmdline.rs` (both MIT/Apache), with attribution in NOTICE.
- Environment: a `CREATE_UNICODE_ENVIRONMENT` block, sorted case-insensitively as
  Windows requires. Of case-insensitive duplicates the last wins. Names containing `=`
  (other than the leading one of `=C:`) or NUL are an error.
- cwd: `C:/x` becomes `C:\x`. CreateProcess cannot take a cwd longer than MAX_PATH; that
  is a clear error, not a silent fallback.
- fds > 2 (`exec 3>f; cmd`) cannot reach a native program: they are ignored, not an
  error. Recorded as a decision.
- Exit status: `GetExitCodeProcess` gives `Exited(code)`. After our
  `TerminateJobObject` it is reported as `Signaled(KILL)` (137); after our ctrl-break,
  `STATUS_CONTROL_C_EXIT` is reported as `Signaled(the signal sent)`. Crash NTSTATUS
  codes map to signals: access violation and stack overflow → SEGV (139), illegal
  instruction → ILL, other `0xC…` codes → ABRT. A plain `code & 0xff` would be
  misleading (an access violation would become 5). Recorded as a decision.

**Wait and signals.** One waiter thread per external (64 KiB stack) on
`WaitForMultipleObjects(process, wake event)`, with the pending KILL deadline as the
timeout, feeding `lifecycle::Tracker`. TERM/INT/HUP/QUIT send `CTRL_BREAK_EVENT` to the
external's group (Session placement); if it cannot be delivered (no shared console, or
Inherit placement), the job is terminated at once. TERM also schedules a KILL after the
grace period. KILL is `TerminateJobObject`. Abandon is TERM, then
`TerminateJobObject` after the grace period, then the handle closes; the whole tree dies.
Stop/continue (`NtSuspendProcess`/`NtResumeProcess` on the job's members) arrive with
0006 part 3's job builtins.

**Pipes and files** (the "never block the engine thread" rule):
- Engine ↔ external (`ChildFd::Pipe`), v1 as ARCH.md says: an anonymous pipe plus one
  helper thread per engine end. A reader thread does blocking `ReadFile` into a bounded
  buffer and wakes the waker; a writer thread drains a bounded buffer. Dropping our end
  calls `CancelSynchronousIo` on the thread, then closes the handle, so the child sees
  EOF or a broken pipe. Overlapped named pipes on the thread pool can replace the threads
  later without changing the engine.
- External ↔ external: an anonymous `CreatePipe`, both ends passed directly (0006's
  planned `Host::pipe()`; the ends are `ChildFd::File`).
- psh's own inherited stdio, and FIFOs (named pipes): a thread blocked in `ReadFile` on a
  shared handle would steal bytes meant for a child (`read line; cat`). So shared
  handles are never read speculatively. A poller thread (the counterpart of the Unix
  readiness thread) checks `PeekNamedPipe` for pipes, `WaitForMultipleObjects` +
  `PeekConsoleInput` for consoles, and reads only what the engine asked for, when it is
  there. Pipes are polled at a short interval, since anonymous pipes have no readiness
  event. Writes to a shared pipe first ask `NtQueryInformationFile(FilePipeLocalInformation)`
  for `WriteQuotaAvailable` and write at most that, the counterpart of the PIPE_BUF cap
  on Unix. Regular files are read and written synchronously. Consoles are written with
  `WriteFile`; console output is UTF-8 if the console code page is 65001, otherwise
  decoded (open: see below).

**Paths and lookup:**
- Engine paths are bytes, `/`-separated, `C:/x` (Host contract). The host converts them
  to UTF-16 (invalid UTF-8 is `InvalidInput`), `\`-separated. It also accepts `/c/x`
  and `C:\x`. std adds `\\?\` for long paths in `std::fs`; the host's own `CreateFileW`
  calls do the same. `/dev/null` and `ChildFd::Null` become `NUL`.
- `stat`: kind and size from std. mtime from the last write time. dev and ino from
  `GetFileInformationByHandle` (volume serial and file index, for `-ef` and `cd -P` loop
  detection). uid and gid are 0. mode is synthesized: directories 0755; files 0644, plus
  0111 when the extension is in PATHEXT or the file starts with `#!`; read-only
  attribute removes the write bits.
- `access`: Read means the file can be opened. Write: not read-only. Execute:
  directories, files with a PATHEXT extension, files with a shebang (ARCH: "decide by
  extension"). App Execution Aliases (0-byte `IO_REPARSE_TAG_APPEXECLINK` files in
  WindowsApps, like `python.exe` or `winget.exe`) count as executable files, which std's
  metadata does not do.
- `resolve(name, PATH, cwd)`: entries separated by `;`, names compared case-insensitively
  (the file system already is). For each entry: if `name` already ends in a PATHEXT
  extension, try it as is; otherwise try `name` plus each PATHEXT extension in order, and
  then the bare name only if it starts with `#!` (so `node_modules/.bin/tsc` with
  `tsc.cmd` beside it resolves to `tsc.cmd`, as cmd would). Explicit empty or `.` entries
  mean the cwd (POSIX). There is never an implicit cwd search, unlike cmd.exe.
- Shebangs (in `spawn`, for a program without `.exe`/`.com` that starts with `#!`):
  `/usr/bin/env X [args]` resolves X on PATH. `/bin/X`, `/usr/bin/X` and
  `/usr/local/bin/X` resolve the basename X on PATH. `sh` and `bash` (direct or via env)
  run in psh itself: the host reports this the way Unix reports ENOEXEC, and the engine
  runs the file as a psh script. Other absolute interpreters are tried as written.

**Questions for the engine** (0005/0006; I'll raise them when part B starts, not now):
1. "Run this file in psh": the Unix host falls back to `/bin/sh FILE` on ENOEXEC
   (D1063). Windows has no `/bin/sh`, and an embedder's `current_exe()` is not psh. The
   engine needs a distinct spawn result (an `io::Error` with a marker payload, or an
   enum) to run the file as a psh script itself. The same path serves `#!/bin/sh`.
2. PATH on Windows: the OS variable is `Path` and `;`-separated. The engine should
   import it as `PATH` (environment names are case-insensitive for the OS). Its
   separator is a policy question for the owner: native `;` (consistent with children,
   ARCH.md today), or bash's `:` with `/c/x` entries, which agent-written
   `PATH="$HOME/bin:$PATH"` assumes. Converting at the boundary, as Git Bash does, is
   possible, but `C:` makes `:` ambiguous. I lean to `;`, plus a rewrite of a
   `:`-separated value that contains only `/x/...` or `C:/...` entries when it is
   exported. To be decided with the owner.
3. `$HOME`, `/tmp`, `$TMPDIR` defaults on Windows (engine or embedder).
4. Output decoding of native programs (ANSI/OEM code page; ARCH's open question): I
   propose the host decodes per external output pipe, when the console or the active
   code page is not UTF-8, with `encoding_rs` (already in the tree, D020). Undecided.

**Tests.** L5 runs on Windows too. `tests/process/harness.rs` picks the host by cfg. The
cases need programs that exist on both systems, so I propose a small test helper binary in
the workspace with modes: `sleep N`, `cat`, `echo-args` (prints its argv, one per line,
for quoting tests), `spawn-grandchild`, `ignore-break`, `exit N`, `crash`. Windows-specific
cases: a timeout kills a tree with grandchildren; no orphan after cancel; a `.cmd` shim
with arguments containing spaces, quotes, `&`, `%`, `^` (and refusal of unquotable
ones); PATHEXT lookup and the `#!` rule; `/dev/null`, `C:\x`, `C:/x`, `/c/x` in `cd`
and redirections; CRLF scripts; a 64+ KiB here-doc into an external; psh's stdin shared
between `read` and a child. They run in the gate's `test` and `test-x64` legs.

### 2026-11-07 Part B, part 1: the Windows host
- `crates/psh-os/src/windows/`: `WindowsHost` (`WindowsConfig { placement, grace, pid,
  leftovers, decode }`, `kill_all`, `killer()`/`RunKiller`, `groups()`, `leftovers()`,
  `settle`), mirroring the Unix host and D1067's leftovers policy. `Leftovers` moved to
  `psh_os::Leftovers` (re-exported from `unix`, so nothing changes there).
  - `spawn.rs` (D1074): suspended `CreateProcessW` → own Job Object (kill-on-close for
    the run's jobs) → resume; stdio-only handle list; `Placement::Group` (new console
    group) or `Inherit` (standalone); `CREATE_NO_WINDOW` without a console; batch files
    through cmd.exe; `#!` interpreters from the child's PATH; waiter thread per external
    on the lifecycle `Tracker`; ctrl-break or `TerminateJobObject`; abandon escalation;
    crash codes as signals; `Host::kill` by pid (run's externals via their job, others
    `TerminateProcess`), refusing the embedder's own pid.
  - `cmdline.rs`: MSVCRT and BatBadBut quoting after std (NOTICE).
  - `file.rs` + `poller.rs` (D1077): `WinFile`; pump pipes as private overlapped named
    pipes with a helper thread each (cancel event + `CancelIoEx`); shared pipes read via
    `PeekNamedPipe` and a poller thread, written synchronously; optional ANSI decoding of
    pumped child output (D1078).
  - `path.rs` (D1075, D1076): `C:/x`, `/c/x`, `/dev/null` → `NUL`, `/tmp` → temp dir;
    `\\?\` stripped from canonical paths; PATH split on `;` and on `:` except after a
    drive letter; `resolve` with PATHEXT, `#!` bare names, App Execution Aliases;
    `stat` with volume serial + file index as dev/ino, synthesized modes;
    `shell_env` (Path → PATH, HOME, TMPDIR).
- `psh` (standalone: `Inherit`, `Leftovers::Keep`, `shell_env`) and psh-spec's
  in-process engine use `WindowsHost` on Windows; the std host is no longer used there.
- Tests: unit tests (quoting, env block, exit codes, paths, PATH splitting, shebangs,
  pumps, shared pipes, decoder) and host tests in `windows/tests.rs` (exit codes and
  output, a 160 KB input pipe through findstr, abandon kills a tree with a grandchild,
  KILL/TERM/`Host::kill`, release + `kill_all`, disown/detach leave the run, batch file
  arguments with spaces, `&` and `%` and refusal of line breaks, lookup + `#!/usr/bin/env`
  into a `.cmd` interpreter, files, `/c/x` = `C:/x`, `/dev/null`, `Host::pipe`). All
  pass on arm64 and x64 in the VM; an end-to-end run of `psh.exe` with cmd, findstr,
  ping, pipelines, here-docs and `$(...)` works.
- Found on the way: `$(cmd /c echo sub)` keeps the `\r` of CRLF output (ARCH's "Windows
  rules" says `$(…)` and `read` strip it): engine work, reported to the coordinator.
- Docs: ARCH.md (hosts table, psh-os Windows, Windows rules), TESTING.md (psh-spec host),
  NOTICE (std, oh-my-pi windows spawn). Guide: none (windows.md is a follow-up).

### 2026-11-07 Part B, part 2: L5 on Windows, console events, Host::path_entries
- `tests/process/windows.rs` (integration test `process-windows` of psh-os): its own
  small harness over `WindowsHost` (the Unix one is procs' and Unix-only), with
  `psh-test-helper` (`tests/process/helper.rs`, a `[[bin]]` of psh-os: sleep, cat, yes,
  args, exit, crash, tree, stubborn, bytes) as `$H`. 16 cases: exit codes and a crash
  (134), a 300 KB external pipeline, `yes` into a reader that stops, a 150 KB here-doc,
  cancel of a tree with a grandchild and of a pipeline with a break-ignoring stage (no
  leftovers), background jobs killed at the end, `Leftovers::Keep`, an npm-style `.cmd`
  shim with `a b`, `x&y`, `100%`, a quote, argv quoting, the shell's stdin shared with an
  external, `/c/`, `C:/` and `/dev/null` paths, CRLF scripts, command not found, 50 short
  externals, system programs through the real PATH. Pass on arm64 and x64.
- `psh_os::windows::signals`: console ctrl events (Ctrl-C INT, Ctrl-Break TERM, close/
  logoff/shutdown HUP) cancel the standalone shell's run; it settles and exits 128+n.
  Test in `crates/psh/tests/cli.rs` (Windows): a ctrl-break to psh's group ends it with
  143 and no leftovers.
- The host maps `/C:/x` (the engine's `normalize` puts a slash before a drive) to
  `C:\x`; the engine should keep `C:/x` as it is (follow-up for the engine).
- 0029 made the engine split PATH on `:` itself (type, hash, source), which broke every
  PATH lookup on Windows. With the coordinator's OK: `Host::path_entries` (default `:`;
  `WindowsHost` per D1076; the test host can play `;`), used by the three call sites;
  engine unit test and the Windows L5 case above (D1910). Other `:` splits in the engine
  are not PATH lookups (tilde expansion in assignments, getopts).

## Follow-ups
- Engine: strip the `\r` of CRLF in `$(…)` and `read` on Windows (ARCH "Windows rules").
- Engine: the "run this file in psh" hook (D1079).
- Engine: `normalize` turns `C:/x` into `/C:/x` (`cd`, `pwd` show it); keep the drive.
- Engine (routed to builtins): `PATH=/x cmd` ignores the prefix PATH for the lookup.
- One L5 harness for both platforms once procs' part 3 has settled `tests/process/`.
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
