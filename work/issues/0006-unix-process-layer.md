# 0006: Unix process layer

status: in-progress
agent: procs
branch: issue/0006-unix-process-layer
depends-on: 0005 (`libc` allowed, D024)
touches: crates/psh-os/ (unix), crates/psh/ (driver, signals), crates/psh-engine/ (Host trait changes, announced), tests/process/
guide: processes.md (not written yet; list it as a follow-up)
decisions: D1060–D1069

## Goal
The real Unix host for macOS, Linux and Android (ARCH.md "psh-os", Unix part), built to
the pitfalls in `research/execution-core.md` §5, with the L5 process tests.

## Acceptance criteria
- [ ] Spawn via `posix_spawn` (own process group or session per external, SETSIGDEF incl.
      SIGPIPE, empty mask, CLOEXEC_DEFAULT on macOS, addchdir_np where available), with
      the `fork`+`exec` fallback where `posix_spawn` cannot express the spec.
- [ ] Exit waits on pooled waiter threads (`waitid` with `WNOWAIT`); no SIGCHLD handler;
      no pid reuse while a group may still be signalled.
- [ ] Engine↔external pumps (v1: bounded helper threads); external↔external pipes passed
      directly. No `O_NONBLOCK` on inherited or shared fds. FIFO opens that may block run
      off the engine thread.
- [ ] Abandon = TERM to the group, KILL after the grace period, background reap;
      detached (`nohup`-style) = released. A command completes only on exit *and* EOF;
      output from grandchildren holding the pipe is drained with bounds.
- [ ] Standalone `psh`: handlers only set atomic bits and wake the driver; children get
      default dispositions. Embedded mode installs no handlers.
- [ ] L5 tests in `tests/process/` (all under timeouts, each with a leak check): cancel
      mid-pipeline leaves no process; a here-doc past 64 KiB into an external; `yes |
      head -1` terminates (SIGPIPE); background `&` + `wait`; `$!`; `kill` of a group;
      output after the shell's own exit is collected; signal exit codes `128+n`.
- [ ] L1–L3 ratchets do not regress; scores in the Result.

## Notes
- oh-my-pi's OS host (`crates/pi-shell/src/sys`, `src/process`) is MIT and may be read
  for the recipe; its `src/bash` may not (D007).
- macOS: `poll` misses FIFO writer close; `waitid(WEXITED)` reports stops.

## Log

### 2026-11-07 part 1: psh-os, the Unix host
- Plan agreed with the coordinator: part 1 needs no Host trait change; part 2 adds
  `Host::pipe` + direct external↔external pipes and spawn options (API in this log before
  landing); part 3 is the standalone signals and the job builtins (`wait`, `kill`, `jobs`,
  real `$!`, maybe `disown`) in their own builtin modules, after 0005 part 2 lands.
- New crate `crates/psh-os` (deps: psh-engine, libc on unix; `unsafe` with a SAFETY
  comment per block, `clippy::undocumented_unsafe_blocks = deny`):
  - `lifecycle.rs` (platform-neutral, for 0007 too): exit status, waker, who reaps
    (engine / released / abandoned), unit-tested.
  - `timer.rs`: one timer thread for `Host::sleep`.
  - `unix/spawn.rs`: posix_spawn (SETSID, SETSIGDEF over every catchable signal, empty
    mask, CLOEXEC_DEFAULT on macOS, addchdir_np via dlsym on glibc) or fork+execve
    (musl, Android, old glibc; tested on macOS by forcing it); source fds lifted above
    every target before dup2; ETXTBSY retry; ENOEXEC → `/bin/sh FILE` (D1063); pooled
    `waitid(WEXITED|WNOWAIT)` waiters; abandon = TERM+CONT, KILL to the group after the
    grace, reap; release; `leftovers()` for leak checks (D1060, D1062).
  - `unix/ready.rs` + `unix/file.rs`: `UnixFile`, the readiness thread (D1061): private
    pipe ends O_NONBLOCK, shared fds checked with a zero-timeout poll/select, writes to
    shared streams capped at PIPE_BUF; several wakers per fd; `select` on macOS.
  - `unix/mod.rs`: `UnixHost` (`UnixConfig { placement, grace, pid }`), fs ops with a real
    `access` (`faccessat(AT_EACCESS)`), FIFO opens on a helper thread (a dropped open
    opens the other end non-blocking so the thread ends), `terminal_foreground()`, `kill()`.
- `psh` and psh-spec use psh-os on Unix (std host stays for Windows until 0007). The
  standalone `psh` uses `Placement::Inherit` when stdin is a terminal whose foreground
  group is psh's, else `Session`.
- L5: `tests/process/` (15 tests, integration test of psh-os, D1064): yes|head SIGPIPE,
  200k lines through an external pipeline, a 200 KB here-doc into `cat`/`wc`, here-string,
  128+n codes, cancel mid-pipeline (one stage ignoring TERM) and in `$(…)` with no
  leftover, output after the shell's exit collected, a grandchild holding a `$(…)` pipe,
  fd 3 and `2>&1 |` into externals, FIFO between a background vproc and `cat`, the
  shell's stdin, 100 short pipelines, a check that the leak check sees a leftover.
  Passing on macOS and in Linux (rust:1.96-trixie in Docker); psh-os unit tests too.
- Gate: `deps` knows psh-os (`libc psh-engine psh-syntax`) and psh-spec's new deps;
  `fmt` inputs include `tests/process/`. NOTICE lists oh-my-pi's pi-shell sys layer (ready.rs
  and spawn.rs follow it closely).
- Ratchet: oils/here-doc/3 and /4 pass now (removed). Scores (macOS): oils 733/2511,
  brush 720/3171, agent 100/144. Linux in Docker, before → after: oils 717 → 725,
  brush 670 → 670, agent 108 → 109. psh-spec time unchanged (~25 s, -j 6).
- Flake seen once, not ours: oils/builtin-cd/10 and /13 share `/tmp/oil-spec-test/pwd`
  and /13 `rmdir`s it, so they race under `-j`.
- Docs: ARCH.md (hosts table, psh-os Unix), TESTING.md (L5 row, psh-spec host). Guide:
  none (processes.md is a follow-up).

### 2026-11-07 part 2 proposal: Host additions (not landed; for the coordinator and engine)
All additive; every new trait method has a default, so the test host and the std host
need no change unless they want the feature.

```rust
// host.rs
pub trait Host {
    // ...
    /// A new OS pipe: (read end, write end). Both are files of this host: the engine
    /// may read or write them like any file and pass them to externals as
    /// `ChildFd::File`. Hosts without OS pipes keep the default.
    fn pipe(&self) -> io::Result<(Rc<dyn HostFile>, Rc<dyn HostFile>)> {
        Err(io::ErrorKind::Unsupported.into())
    }
    /// `kill(2)` for the `kill` builtin's numeric pids (negative: a process group).
    fn kill(&self, pid: i32, sig: Signal) -> io::Result<()> {
        let _ = (pid, sig);
        Err(io::ErrorKind::Unsupported.into())
    }
}

pub trait ExtProc {
    // ...
    /// The OS process id (`$!`, `jobs -p`), where there is one.
    fn pid(&self) -> Option<u32> { None }
}

#[derive(Default)]                 // new: the engine writes `..SpawnSpec::default()`
pub struct SpawnSpec {
    // ... program, argv, env, cwd, fds as now
    /// Signals the child starts with ignored (bash: SIGINT and SIGQUIT in `&` jobs;
    /// `trap '' SIG`). Every other signal starts at its default.
    pub sig_ignore: Vec<Signal>,
    /// The child's umask; `None` keeps the process's (the `umask` builtin sets it).
    pub umask: Option<u32>,
}
```

Engine change (pipe.rs, exec.rs `run_external` only): an engine pipe whose buffer is empty
when an external is spawned on one of its ends is *materialized*: the engine asks
`host.pipe()` once, keeps both OS ends in the pipe object, and hands the external the OS
end as `ChildFd::File`. From then on vproc reads drain the in-memory buffer first and
then read the OS read end; vproc writes go to the OS write end (so order is kept: the
buffer was empty when the OS pipe took over). The engine drops the OS read end when its
last `PipeReader` goes and the OS write end when its last `PipeWriter` goes; the kernel
counts the externals' copies. Waiting readers are woken at materialization so they
re-poll the OS end. Not materialized: pipes with buffered data (here-docs, a vproc wrote
first and nobody read yet) and hosts without `pipe()`: those keep today's pumps. Effect:
`a | b | c` of externals passes OS pipes directly with no pump and no engine wakeups
per chunk; `cmd | while read` reads the OS pipe directly.

Host side (psh-os): `pipe()` returns two shared `UnixFile`s (blocking, readiness-checked);
`sig_ignore` or a `umask` different from the process's take the fork path (`posix_spawn`
can express neither); `pid()`; `kill()`.

### 2026-11-07 part 2/3 groundwork (no engine logic yet; merge freeze)
- Host trait additions as proposed (coordinator OK, engine agent asked): `Host::pipe`,
  `Host::kill`, `ExtProc::pid`, `SpawnSpec: Default` with `sig_ignore`, `umask`; plus
  `SIGHUP`/`SIGQUIT` constants. exec.rs only gets `..SpawnSpec::default()`. psh-os
  implements all of them: `sig_ignore` beyond what the process ignores and a umask other
  than the process's take the fork path; signals ignored when the host was made stay
  ignored in children (bash's rule), except SIGPIPE and SIGCHLD.
- Standalone signals (D1066): `psh_os::unix::signals` (`catch`, `die_by`,
  `is_ignored`), `psh_os::unix::settle`; the psh driver cancels on INT/TERM/HUP, waits
  for the escalations and dies by the signal.
- Escalation (D1065): KILL to the group at the leader's exit or after the grace period,
  whichever is first (measured: a zombie-only group answers `kill(-pgid, 0)` with 0 on
  Linux, EPERM on macOS, so "only our zombie left" cannot be checked cheaply).
- L5: `tests/process/standalone.rs` (an integration test of the `psh` crate, 5 tests):
  TERM cancels a pipeline and psh dies by TERM without waiting the grace, INT and HUP,
  a TERM-ignoring external is KILLed before psh exits, a signal ignored at start stays
  ignored in psh and its children, exit codes unchanged. psh-os unit tests for
  `sig_ignore`, `umask`, `pipe()` between two externals, `kill`, `pid`. All pass on macOS
  and Linux (Docker).
- More L5 (21 in-process now): a vproc that never reads ends `yes` by SIGPIPE, a 200 KB
  here-doc into `true`, pumps on both sides of brace groups, `exec` of an external,
  cancel while a FIFO open waits, 8 shells at once. 10× on macOS and 6× on Linux clean.
  In Docker, run with `--init`: without a reaping PID 1, killed grandchildren stay
  zombies in their group and the leak check sees them.

- Checked against the engine's Host v1 contract (0005 log): kept HostFuture, HostFile
  (`&self`, Any), drop semantics, never-block, byte paths. `Host::kill` on an embedded
  host (fake `$$`) refuses the embedder's pid, its group, `0` and `-1`. Open against the
  contract: ENOEXEC still falls back to `/bin/sh` in the host (D1063) until the engine
  can run such files; then the host reports it as an error instead. Bounded draining of
  grandchildren: the engine waits for EOF as bash does (also for `$(…)`); the bound
  belongs to whoever drains the shell's final output (psh-spec: its deadline; pi-env:
  its window), and with part 2 externals write to OS pipes directly, so nothing is lost
  with a dropped pump.

### 2026-11-07 part 3a: who survives a run (D1067)
- Coordinator flagged it (0005 found it): released jobs + a session per external =
  orphans nothing reaches. Evaluated the proposal (one group/session per run): a run
  session cannot be joined from outside without a helper process; a run group in the
  embedder's session needs a pinned zombie leader and cannot let `disown` escape. Chosen:
  embedded host = one run, per-external sessions recorded, killed at host drop
  (`Leftovers::Kill`, default; `Keep` for bash's behavior), `kill_all`/`RunKiller`,
  `SpawnSpec::detach` and `ExtProc::disown` (new, additive, default = release) as the
  escapes. Standalone psh: psh's own group always (bash-like), `Keep`, and `die_by`
  signals the group when psh leads it.
- Tests: L5 normal end kills two TERM-ignoring background loops; a finished external's
  grandchild is killed at the end; cancel kills background jobs; the embedder kills the
  run from another thread; `Keep` leaves jobs (checked, then killed); host unit test:
  released and leftover sessions die at drop, detached and disowned survive. Standalone:
  background jobs stay in psh's group and die with a group kill; a grandchild dies with
  psh when only psh's pid gets TERM.
- psh-spec in-process is `Kill` now: brush/compat/background_jobs/0,1,4,5 (bg `touch`
  without `wait`) fail until 0005's `exit_grace` (500 ms in psh-spec) is on main; I
  merge after it, so they must pass then.
- Rebased on main e98333b (0005 part 2, `exit_grace`): no conflicts; the four
  background_jobs cases pass; psh-spec: oils 1111/2511, brush 1066/3171, agent 115/144,
  no regressions. Orphan check (processes with ppid 1, before vs 5 s after a full
  `psh-spec -j 6` run, in-process and binary cases): this branch 0 new; main e98333b
  6 new (4 `sh -c 'trap "exit 0" TERM; …; while :; do sleep 0.05; done'` loops from the
  binary-run cases, 2 `sleep`s).

## Follow-ups
- guide: `processes.md` (placement, cancel and abandonment, leak checks).
- Engine: run files without `#!` in psh itself (bash does), then drop D1063's fallback.
- psh-spec: a leak check per case with `UnixHost::leftovers()` (released background
  jobs excluded).
- oils/builtin-cd/10 vs /13 race on a shared /tmp path under `-j` (test infra).
- Linux: pidfd parking instead of waiter threads (ARCH "later").

## Result
