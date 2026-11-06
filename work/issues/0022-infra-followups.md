# 0022: L0 leg, gate caching, hit list on the parser

status: in-progress
agent: infra
branch: issue/0022-infra-followups
depends-on: 0002, 0004 part 1
touches: crates/psh-spec/, scripts/gate.d/, scripts/gate.sh, scripts/status, tests/l0/, tests/shapes/ (ratchet only), TESTING.md
guide: -
decisions: D1220–D1229

## Goal
Infrastructure follow-ups from 0001, 0002 and 0003 that make the hill-climb faster and
measure the parser in the gate.

## Acceptance criteria
- [x] **L0 leg** in psh-spec, using psh-syntax as a library: parse agreement with the
      recorded container `bash -n` verdicts for every Oils/brush case script, the shape
      corpus (`tests/goldens/shapes/bash-n.txt`) and `tests/l0/syntax.txt`, plus
      print → reparse round-trip. Its own ratchet; shown by `scripts/status`. A gate leg
      file in `scripts/gate.d/`.
- [ ] **Gate caching by leg inputs** (0001 follow-up): each leg declares its inputs
      (paths/globs); a leg whose inputs and the leg script are unchanged since its last
      pass on a recorded tree hash is skipped with a note. Docs-only changes gate in
      seconds; the Windows leg is skipped when no Rust/script input changed. A `--full`
      flag disables the cache. Keep it simple and safe (a wrong skip is worse than a
      slow gate).
- [ ] **Hit list on psh-syntax** (0002 follow-up): detect constructs from the AST
      instead of lexically, where the parser accepts the case.
- [ ] Workspace-wide dependency allowlist in the `deps` leg (0001 follow-up): every
      crate's dependencies must be listed in a committed allowlist that cites the
      decision that allows them.

## Notes
- Coordinate with the parser agent (0004 part 2 is in flight in crates/psh) only through
  psh-syntax's public API.
- The engine agent (0005) is wiring the in-process engine into psh-spec; keep your
  psh-spec changes in separate modules to avoid conflicts, and rebase often.

## Log
- 2026-11-07, part 1 (L0): `psh-spec --l0` (new module `l0.rs`; shapes recording moved
  into it), psh-syntax as psh-spec's dependency (D1220), `scripts/record-goldens --l0`
  records `bash -n` for the shapes and `tests/l0/syntax.txt` in the container (69 of
  196 snippets rejected, all agreeing with the file's inline statuses), gate leg
  `92-l0` (under a second), L0 in `scripts/status` and status.tsv. Result: verdicts
  agree on all 33,006 inputs (6,576 case scripts, 26,234 skeletons, 196 snippets); 23
  skeletons fail the round trip (print → reparse changes the tree; mostly here-docs
  whose delimiter line is missing or followed by more commands, plus `F=/w1` and a `2>`
  case). They start the ratchet; tell the parser agent.

## Follow-ups

## Result
