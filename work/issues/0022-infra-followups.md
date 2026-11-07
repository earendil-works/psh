# 0022: L0 leg, gate caching, hit list on the parser

status: done
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
- [x] **Gate caching by leg inputs** (0001 follow-up): each leg declares its inputs
      (paths/globs); a leg whose inputs and the leg script are unchanged since its last
      pass on a recorded tree hash is skipped with a note. Docs-only changes gate in
      seconds; the Windows leg is skipped when no Rust/script input changed. A `--full`
      flag disables the cache. Keep it simple and safe (a wrong skip is worse than a
      slow gate).
- [x] **Hit list on psh-syntax** (0002 follow-up): detect constructs from the AST
      instead of lexically, where the parser accepts the case.
- [x] Workspace-wide dependency allowlist in the `deps` leg (0001 follow-up): every
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
- 2026-11-07, part 2 (gate cache): `# inputs:` lines in every leg, cache in the primary
  checkout's `work/scratch/gate-cache/`, `--full`, `$PSH_GATE_NOCACHE` for the skipped
  Windows leg (D1221). `scripts/test-gate-cache.sh` (in `script-tests`) covers unchanged,
  docs-only, changed, dirty, untracked, `--full`, leg edits, opt-out and failing legs; a
  mutation that ignores the opt-out fails it. A second full local gate: every leg cached,
  1 s. While testing I ran the gate once with `PSH_GATE_WINDOWS=0`, which logged a line
  in `work/scratch/windows-gate/skipped.tsv` (not a merge).
- 2026-11-07, part 3 (hit list on the parser): new module `ast_features.rs` walks
  psh-syntax's tree for the corpus features and command names (D1222); the lexical scan
  remains for scripts the parser rejects. Today 5,623 of 5,696 failing cases are weighed
  from the tree, 73 from the text.
- 2026-11-07, part 3 merge blocked by the Windows leg (VM mid-suspend once; then
  `test-x64` LNK1104 on locked exes in `C:\psh-gate\infra`, probably processes left
  from an earlier timed-out x64 run). The coordinator has the windows agent fixing the
  leg; part 3 waits on the branch.
- 2026-11-07, part 4 (deps allowlist): `scripts/deps-allowlist.txt` with a decision per
  pair (10 pairs today: psh's deser-json and its three transitive crates, and the
  workspace crates on each other) and the `deps` leg comparing it with `cargo tree
  --target all` for every crate (D1223). Checked by hand: an unlisted, a stale and an
  uncited line each fail the leg. AGENTS.md's "Allowed dependencies" paragraph could
  point at the list (coordinator's file).

## Follow-ups
- Platform-dependent cases (coordinator's plan, implement when the macOS/Windows spec
  legs need it): goldens stay Linux bash 5.3; per-platform ratchets
  `tests/<layer>/expected-failures.<os>.txt` next to the shared one, listing only cases
  that pass on Linux; a `platform` tag in spec.toml for inherently host-specific cases
  (`/bin` layout, case-insensitive file systems, signal numbers). Many differences go
  away once psh's own utilities shadow the host's (modes S/W).

## Result
- **L0** (main 59b798e): `cargo run -p psh-spec -- --l0 [FILTER] [--diff]
  [--update-ratchet]`; verdicts in the case goldens and `tests/goldens/{shapes,l0}/
  bash-n.txt` (`scripts/record-goldens --l0` re-records the latter); ratchet
  `tests/l0/expected-failures.txt` (23 shape round trips); gate leg `l0`; shown by
  `scripts/status`.
- **Gate cache** (main 2f968f7): `# inputs:` per leg, `scripts/gate.sh --full`, cache
  in the primary's `work/scratch/gate-cache/`.
- **Hit list on the parser**: `ast_features.rs`; lexical fallback for rejected scripts.
- **Deps allowlist**: `scripts/deps-allowlist.txt`, checked by the `deps` leg.

Known gaps: a leg's `# inputs:` are trusted (a leg reading an undeclared path can be
skipped wrongly); the Windows VM's state is not part of any key.
