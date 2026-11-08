# 0038: Classify live examples before execution finishes

status: done
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
- [x] Classification appears before execution finishes.
- [x] Editing invalidates older results, including during the autorun debounce.
- [x] Regression tests exercise blocked execution and overlapping edits/runs.
- [x] Guide prose describes the independent classification.

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

- 2026-11-08: Moved classification before shell execution, invalidated old run IDs
  immediately on editing, and marked classification pending until the new text is
  classified. Guarded both asynchronous module-loading boundaries against stale input.
- Added five dependency-free Node regression tests against the actual editor module:
  blocked execution, edits during debounce, out-of-order completion, execution errors,
  and edits during module loading. Wired these into the existing wasm-host gate leg.
- Updated docs/guide/README.md and docs/guide/classify.md; no live example output or
  architecture/decision changes. Five tests pass; psh-docs check and build pass.
- Browser recheck with the supplied third script: classification updated at 630 ms
  after editing (including the 600 ms debounce), versus 5,627 ms for execution to end.
  A real tab click at 1,005 ms showed the current classification; no browser long tasks.

## Follow-ups
- Superseded executions still finish or reach their existing five-second limit;
  cancelling them immediately is separate from publishing current classification.

## Result
Classification is available as soon as the edited script is classified, before its
execution finishes. Old executions and asynchronous module loads cannot publish
results for superseded input. No parser, classifier, engine, or ABI changes.
