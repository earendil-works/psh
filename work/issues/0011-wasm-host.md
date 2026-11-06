# 0011: WASM build and JS host

status: open (later)
agent: -
branch: issue/0011-wasm
depends-on: 0005 (0008 for utilities in WASM); coordinator: is it needed before 0010? (ARCH.md open question 3)
touches: crates/psh-wasm/, js/ (host package)
guide: wasm.md (not written yet; list it as a follow-up)
decisions: D1110–D1119

## Goal
psh in the browser and in sandboxes (ARCH.md "WASM and the browser"): the whole shell
for `wasm32-unknown-unknown`, driven from JS over a virtual file system, with host
commands in place of processes.

## Acceptance criteria
- [ ] `psh-wasm` exports: create a shell, run a command, step, deliver host results;
      no Asyncify, no JSPI.
- [ ] A small JS host: in-memory fs, host-provided commands, the step loop on the event
      loop.
- [ ] The parser-only module (psh-syntax + psh-classify) as its own small build; sizes
      recorded (raw and gzipped).
- [ ] L1 cases that need no externals pass under node through the JS host at the same
      rate as native.

## Log

## Follow-ups

## Result
