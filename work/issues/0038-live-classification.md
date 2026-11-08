# 0038: Classify live examples before execution finishes

status: in-progress
agent: coordinator
branch: issue/0038-live-classification
depends-on: 0036
touches: crates/psh-docs/assets/live.js, crates/psh-docs/tools/live-test.mjs, scripts/gate.d/65-wasm-host.sh, docs/guide/README.md, docs/guide/classify.md
guide: docs/guide/README.md, docs/guide/classify.md
decisions: D1380–D1389

## Goal
Keep the docs' Classification pane current without waiting for a live example to
finish executing, including scripts that sleep until the five-second limit.

## Acceptance criteria
- [ ] Classification appears before execution finishes.
- [ ] Editing invalidates older results, including during the autorun debounce.
- [ ] Regression tests exercise blocked execution and overlapping edits/runs.
- [ ] Guide prose describes the independent classification.

## Notes
Investigation of the three files supplied in /tmp/silly-bash-invocations found no
slow parsing or browser long tasks. The third script requests repeated three-second
sleeps and reaches the five-second limit; live.js computes classification only after
awaiting shell.run. Tabs respond, but show stale classification meanwhile.
Use synthetic scripts in committed tests, not the user's exported session commands.

## Log
- 2026-11-08: Started the docs-only sequencing fix. No engine or WASM ABI changes
  needed; no new dependencies or design decisions. The requested push is prohibited
  by repository instructions.

## Follow-ups

## Result
