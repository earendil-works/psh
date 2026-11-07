# 0031: Grow the L3 agent layer to the shapes agents actually use

status: in-progress
agent: shapes
branch: issue/0031-agent-cases
depends-on: 0002, 0003
touches: tests/agent/, tests/goldens/agent/, tests/agent/expected-failures*.txt, tests/spec.toml (agent groups only)
guide: -
decisions: D1310–D1319

## Goal
L3 is the layer that matters most (TESTING.md: "what agents actually do"), but it has
only 144 hand-written cases. Grow it to ~600 cases that mirror the corpus: the most
frequent syntax shapes (tests/shapes/shapes.txt) and command patterns
(research/corpus.md), instantiated with synthetic, self-contained content in the case's
temp directory. No private data; nothing from the raw corpus.

## Acceptance criteria
- [ ] Coverage by construction: take the top shapes by session count and the top
      utility/flag combinations from research/corpus/results.json; for each, write
      cases that exercise it with realistic synthetic files (a small fake repo tree,
      source files, logs). Record in tests/agent/README.md which shapes/patterns each
      file covers, and the share of corpus commands (by shape) the layer now covers.
- [ ] Include the agent failure modes from the corpus: `cd X && …` chains, `| head`
      truncation and SIGPIPE, `2>&1`, here-docs into interpreters (use only commands
      available in the recording container: no python/node — or `cat`/`sh` stand-ins),
      `sed -i ''` vs GNU, `timeout`, unmatched globs, `set -euo pipefail` scripts,
      background + `wait`, `find … -exec`, `xargs`, `rg`/`grep` with typical flags.
- [ ] Cases needing git: allowed if the container has git (add it to the recording image
      if needed, pinned) — git usage is ~24% of agent commands; keep repos synthetic and
      local.
- [ ] Goldens recorded in the container; cases that depend on GNU vs BSD utility
      behavior annotated so L3 stays meaningful on macOS once psh's utilities land.
- [ ] Failures ranked and routed: the Result lists the failing L3 cases grouped by the
      area/issue that owns them (engine, builtins, utilities batches, procs).

## Log

## Follow-ups

## Result
