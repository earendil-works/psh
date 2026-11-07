# 0031: Grow the L3 agent layer to the shapes agents actually use

status: done
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
- [x] Coverage by construction: take the top shapes by session count and the top
      utility/flag combinations from research/corpus/results.json; for each, write
      cases that exercise it with realistic synthetic files (a small fake repo tree,
      source files, logs). Record in tests/agent/README.md which shapes/patterns each
      file covers, and the share of corpus commands (by shape) the layer now covers.
- [x] Include the agent failure modes from the corpus: `cd X && …` chains, `| head`
      truncation and SIGPIPE, `2>&1`, here-docs into interpreters (use only commands
      available in the recording container: no python/node — or `cat`/`sh` stand-ins),
      `sed -i ''` vs GNU, `timeout`, unmatched globs, `set -euo pipefail` scripts,
      background + `wait`, `find … -exec`, `xargs`, `rg`/`grep` with typical flags.
- [x] Cases needing git: allowed if the container has git (add it to the recording image
      if needed, pinned) — git usage is ~24% of agent commands; keep repos synthetic and
      local.
- [x] Goldens recorded in the container; cases that depend on GNU vs BSD utility
      behavior annotated so L3 stays meaningful on macOS once psh's utilities land.
- [x] Failures ranked and routed: the Result lists the failing L3 cases grouped by the
      area/issue that owns them (engine, builtins, utilities batches, procs).

## Log
- 2026-11-07: Recording image: git 2.47, ripgrep 14.1, jq 1.7 pinned (docker/ is infra's;
  a three-package change, D1310). Existing goldens were unaffected (agent layer
  re-recorded; oils/brush cases do not call them).
- Wrote 557 cases in 9 new files (git, search, view, tools, scripts, files, text, procs,
  top-shapes): eight topic files by parallel sub-agents from a shared brief
  (work/scratch/0031/brief.md) with each topic's top shapes, then top-shapes by hand for
  the most frequent shapes still missing. 701 cases in total.
- Determinism: git cases first differed between recordings in `.git` (index mtimes):
  psh-spec now leaves `.git` and bare `*.git` dirs out of file snapshots (sandbox.rs,
  one condition; psh-spec is infra's, noted). `scripts/record-goldens --check agent`:
  0 differences.
- Coverage (D1311, `tests/agent/coverage.py`): args level 49.7% of the commands the
  committed shapes stand for (20.7% of the corpus); exact 26.8%, syntax 71.9%.
- psh: 607/701 pass on macOS. One psh bug found and fixed on the way (my 0028 code):
  `set -u`/`${x?}` in a subshell under `-c` ends it with 1, not 127 (bash: 127 only for
  the shell itself).

## Follow-ups
- A macOS platform ratchet (like `expected-failures.wasm.txt`) would separate the 33
  BSD-utility failures from real psh failures; until then they are in the main ratchet
  with reasons.
- `coverage.py --missing 30` lists the next shapes worth a case (`ls -la` needs a
  normalized form of its output first).

## Result
- 701 L3 cases (557 new) with goldens, deterministic under re-recording; README with the
  per-file coverage and the writing rules (stubs, git env, `# dialect: gnu`).
- Coverage: half of the commands the shape corpus stands for (args level, D1311).
- Failing on macOS: 94 of 701, all in the ratchet with reasons, routed:
  - utilities, search batch (rg built in): 53 (`search/*` rg cases, `top-shapes/9–23`)
  - utilities, `timeout`: 10 (`globs-status/18,19`, `procs/0–6,8`)
  - utilities, BSD vs GNU output on macOS (wc/uniq padding, ls, sed -i, od, nl, seq,
    cmp, head -n -N): 29 — pass on Linux and once psh's own utilities run them
  - parser/engine, `[[ ]]` (0027): 1 (`control/6`)
  - engine: no warning for a here-doc ended by end of file: 1 (`heredocs/17`)
