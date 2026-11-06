# 0010: pi-env integration behind a flag

status: open (later)
agent: -
branch: issue/0010-pi-env
depends-on: 0006, 0007, L3 high; owner decision on sessions (ARCH.md "Embedding into pi-env")
touches: pi-mono `packages/env/daemon` (outside this repository; coordinate with the owner), crates/psh-engine/ (embedding API)
guide: pi.md (not written yet)
decisions: D1100–D1109

## Goal
Run pi-env's `exec` through psh in-process, behind a feature flag, so that psh owns
every child process (ARCH.md "Embedding into pi-env", `research/pi-current.md` §6–7).

## Acceptance criteria
- [ ] An embedding API in psh: create a shell with cwd/env, run a command string with a
      waker-driven driver, combined output, cancel/timeout via the control handle.
- [ ] In pi-env: feature `psh` in `hello.features`; `exec { shell: "psh" }`; one thread
      per exec runs the driver; no signal handlers installed; output through the
      existing decoding, windowing and spill.
- [ ] pi-env's differential tests (`docs/semantics.md`) pass for psh, or each difference
      is documented.
- [ ] Binary size impact on pi-env measured for all eight targets.
- [ ] A `parse` op returning the classification (needs 0009).

## Notes
- Changes in pi-mono follow pi-mono's own process; this issue tracks the psh side and
  the plan.

## Log

## Follow-ups

## Result
