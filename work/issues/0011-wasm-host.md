# 0011: WASM build and JS host

status: in-progress
agent: infra
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
- [x] `psh-wasm` exports: create a shell, run a command, step, deliver host results;
      no Asyncify, no JSPI.
- [x] A small JS host: in-memory fs, host-provided commands, the step loop on the event
      loop.
- [x] The parser-only module (psh-syntax + psh-classify) as its own small build; sizes
      recorded (raw and gzipped).
- [ ] L1 cases that need no externals pass under node through the JS host at the same
      rate as native.

## Log
- 2026-11-07: Coordinator: do it now, in parallel with pi-env; no wasm-bindgen; plain
  extern "C" ABI and hand-written JS glue like djs (read for patterns only).
- Part 1: `crates/psh-wasm` (D1110, D1111): one import `psh.host_call`, answers through
  `psh_complete`, shell exports (`psh_shell_new/start/step/cancel/free`), parse and
  classify exports (JSON as `psh --parse/--classify`, sharing the psh binary's JSON
  modules by `#[path]`). 16 MiB stack, nesting limit 768. `js/psh.mjs` (D1112): the glue,
  `MemHost` (memfs, pipes, timers, JS commands), `Psh.run()` and `Shell` with the step
  loop on the event loop. `scripts/build-wasm` builds both modules into `target/wasm/`.
  Sizes (profile `wasm`: opt-level s, LTO, one codegen unit, stripped):
  - `psh.wasm` (shell): 747,014 bytes, 277,525 gzipped;
  - `psh-parser.wasm` (parser + classifier): 388,138 bytes, 148,249 gzipped. Of its
    325 KB of code, the classifier is 111 KB, the parser 84 KB, deser-json 72 KB.
  `js/test/smoke.mjs` runs both under node (gate leg `65-wasm-host`, with clippy for the
  wasm target). Found on the way: the engine has no `read` builtin yet (the engine
  agent's work), so the smoke tests read through a host `cat`.

## Follow-ups
- Move the JSON of parse and classify out of the psh binary into a crate both use (now a
  `#[path]` include).
- Parser module size: 148 KB gzipped; candidates are a JSON-less parse export, nightly
  `-Zlocation-detail=none`/`build-std` (djs saves ~11 KB gzip that way) and wasm-opt.
- guide: wasm.md.

## Result
