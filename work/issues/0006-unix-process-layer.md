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

## Follow-ups
- guide: `processes.md` (placement, cancel and abandonment, leak checks).
- Engine: run files without `#!` in psh itself (bash does), then drop D1063's fallback.
- psh-spec: a leak check per case with `UnixHost::leftovers()` (released background
  jobs excluded).
- oils/builtin-cd/10 vs /13 race on a shared /tmp path under `-j` (test infra).
- Linux: pidfd parking instead of waiter threads (ARCH "later").

## Result
