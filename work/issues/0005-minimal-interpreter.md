# 0005: Minimal interpreter

status: in-progress
agent: engine
branch: issue/0005-minimal-interpreter
depends-on: 0002, 0004
touches: crates/psh-engine/, crates/psh/ (driver), crates/psh-spec/ (in-process engine), tests/**/expected-failures.txt
guide: engine.md (not written yet; list it as a follow-up)
decisions: D1050–D1059

## Goal
The first version of the stepped engine (ARCH.md "psh-engine", D005), enough that L1, L2
and L3 start moving: simple commands, pipelines, `&&`/`||`/`;`, redirections, variables
and assignments, basic expansion, exit status. It fixes the engine's shape (step API,
control handle, waker, Host trait, virtual processes) that everything later builds on.

## Acceptance criteria
- [ ] `Shell::step(budget) -> Step` (`Done`, `Blocked`, `Yield`, `Idle`), a
      `ControlHandle` checked at safepoints (cancel works), wakeups via
      `std::task::Waker` only. No threads inside the engine, no runtime.
- [ ] A first `Host` trait (spawn with `poll_wait`, fs ops that may return `Pending`,
      files with `poll_read`/`poll_write`, timers, env) and a test host with an
      in-memory fs and fake processes, including a mode where every op is `Pending`
      once before it completes.
- [ ] Virtual processes: pipeline stages run concurrently, in-memory pipes between
      them; subshells `( )` as vprocs.
- [ ] Language: simple commands with assignments, redirections (`>`, `>>`, `<`, `2>&1`,
      `&>`, `n>&m`, here-docs, here-strings), `&&`, `||`, `;`, `!`, `{ }`, `( )`,
      `$?`, parameter expansion `$x ${x} ${x:-y}`, quoting and quote removal, word
      splitting on IFS, `$(…)`. Shell builtins: `echo`, `printf` (basic), `true`,
      `false`, `:`, `cd`, `pwd`, `export`, `unset`, `exit`, `test`/`[`.
- [ ] A simple Unix host good enough for the runner: std `Command` spawn with a waiter
      thread per child that wakes the engine, helper threads for pipes. 0006 replaces
      it; keep it small.
- [ ] `psh -c CMD` and `psh FILE` run through a standalone driver (step, park on
      `Blocked`).
- [ ] psh-spec runs psh in-process; ratchets updated; the Result records the scores.
- [ ] The engine builds for `wasm32-unknown-unknown` (gate).

## Notes
- Read `research/execution-core.md` §5 "Pitfalls" before designing; all 14 points
  apply. Never block the engine thread in the kernel; never write a here-doc before its
  consumer starts.
- brush-core's expansion code may be copied with attribution; its interpreter and sys
  layer are async and must not be (D008).
- Keep the Host trait minimal and say in the log what you expect 0006 and the utilities
  to need; the trait will change.

## Log

## Follow-ups

## Result
