# 0006: Unix process layer

status: open
agent: -
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

## Follow-ups

## Result
