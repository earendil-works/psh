# 0011: WASM build and JS host

status: done
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
- [x] L1 cases that need no externals pass under node through the JS host at the same
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
- Part 2 (L1 under node): `psh-spec --wasm` (D1113): a pool of node drivers
  (`js/test/spec-driver.mjs`), the case's directories mounted lazily from disk and its
  changes written back, missing commands reported as run notes; platform ratchets
  `tests/<layer>/expected-failures.wasm.txt`. MemHost grew `/dev/null`, `/dev/zero`,
  `/dev/full` (character devices, ENOSPC), a sticky `/tmp`, empty system directories, and
  opening a file no longer makes its missing parent directories (three Oils and nine brush
  cases found these). Errors from the JS side carry bash's `strerror` text.
  Result on L1: of the 1,251 Oils cases psh passes natively, 867 pass in wasm; the other
  384 all need external programs (`needs external: ...`), none fails for another reason:
  the no-externals rate is the native one. brush: 223 need externals, 1 needs host system
  files (`/bin/sh`, `/etc/os-release`); agent: 65 need externals. The whole run takes
  ~20 s (native ~22 s). Gate leg `65-wasm-host` now runs it.
  The first merge attempt failed: the engine had improved meanwhile, 175 more cases
  passed natively and failed in wasm, all needing externals. Listing such cases would
  make every engine change edit the wasm ratchet, so cases that needed externals are now
  allowed without a line; `expected-failures.wasm.txt` holds only the other differences
  (one brush case). After the rebase: Oils 526 cases need externals, 0 other.
  Sizes at this point (the engine grew since part 1): psh.wasm 846,806 bytes, 312,634
  gzipped; psh-parser.wasm 388,202 / 148,282.

## Follow-ups
- Move the JSON of parse and classify out of the psh binary into a crate both use (now a
  `#[path]` include).
- Parser module size: 148 KB gzipped; candidates are a JSON-less parse export, nightly
  `-Zlocation-detail=none`/`build-std` (djs saves ~11 KB gzip that way) and wasm-opt.
- guide: wasm.md.

## Result
- `crates/psh-wasm`: `scripts/build-wasm` builds `target/wasm/psh.wasm` (the shell) and
  `target/wasm/psh-parser.wasm` (parse + classify). Sizes at main 89528f0: psh.wasm
  926,941 bytes / 339,751 gzipped (it grows with the engine), psh-parser.wasm 388,202 /
  148,282. ABI: one import `psh.host_call`, answers through `psh_complete` (D1110); 16 MiB
  stack, nesting limit 768 (D1111).
- `js/psh.mjs`: `loadPsh(bytes)`, `psh.run(cmd, {cwd, env, stdin})`, `createShell()` /
  `shell.run()`, `psh.parse()`, `psh.classify()`; `MemHost` with `fs` (MemFs, lazy mounts)
  and `host.command(name, fn)` (D1112).
- `psh-spec --wasm`: every layer through the JS host under node; platform ratchet
  `tests/<layer>/expected-failures.wasm.txt` (D1113). L1: every case psh passes natively
  and that needs no external program passes in wasm.
- Gate leg `65-wasm-host`: wasm clippy, both builds, `node js/test/smoke.mjs`, the
  `--wasm` run (about 25 s).

Known gaps: no utilities in wasm yet (0008's in-process utilities will make most of the
"needs external" cases pass); host commands cannot be stopped by signals; no OPFS or
remote file system host yet; the JSON code is shared with the psh binary by `#[path]`.
