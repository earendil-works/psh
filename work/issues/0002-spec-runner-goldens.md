# 0002: psh-spec runner, vendored cases, recorded goldens, ratchets, status

status: in-progress
agent: infra
branch: issue/0002-spec-runner-goldens
depends-on: 0001
touches: crates/psh-spec/, scripts/record-goldens, scripts/status, scripts/fetch-*, docker/, tests/oils/, tests/brush/, tests/agent/, tests/goldens/, tests/spec.toml, tests/**/expected-failures.txt, tests/scores/, scripts/gate.sh (one leg), TESTING.md (the planned sections)
guide: testing.md (not written yet; list it as a follow-up)
decisions: D1020–D1029

## Goal
The first real milestone: **a red scoreboard**. Every case of L1 (Oils) and L2 (brush),
plus a first set of L3 agent cases, has a golden recorded from bash 5.3, the runner
compares psh against them, ratchets sit at 0%, and the status tool shows the numbers.
See TESTING.md ("Oracle", "Cases and goldens", "Groups", "Ratchets", "Status and the hit
list", "The runner") and `research/hillclimb.md`.

## Acceptance criteria
- [ ] Vendored, with their license files and a README naming the upstream commit:
      Oils `spec/*.test.sh` (+ the helper files they need) at `d114978` in
      `tests/oils/`; brush's `brush-shell/tests/cases/**/*.yaml` at `943553433ae0` in
      `tests/brush/`. A `scripts/fetch-*` script reproduces each from upstream into
      `vendor/` and checks the copy.
- [ ] `docker/` holds the recording image (TESTING.md "The container"): GNU userland,
      bash 5.3 built from the release tarball pinned by sha256, fixed locale, TZ, HOME,
      USER, hostname, umask. Pinned by digest once built.
- [ ] `scripts/record-goldens` records stdout, stderr, status and resulting files for
      every case, plus the `bash -n` verdict of each case script (for L0), into
      `tests/goldens/`. Deterministic: a second recording produces no diff (record the
      cases that are inherently nondeterministic, e.g. `$RANDOM`, `$$`, timing, and put
      them in a group).
- [ ] `tests/spec.toml`: skip / later / active rules, each with a reason (TESTING.md
      "Groups").
- [ ] `crates/psh-spec` (no dependencies): Oils `.test.sh` reader, brush YAML subset
      reader, case model, normalization table, parallel runner (std threads, `-j`),
      per-case timeout, filters, `--diff`, ratchet (`--expected FILE`,
      `--update-ratchet`; stale entries reported), Markdown summary per layer and file.
      An `Engine` trait with two engines: an external shell binary (subprocess) and
      in-process psh (a stub until 0005).
- [ ] Runner self-check: `/opt/homebrew/bin/bash` through the external engine against
      the goldens. Every mismatch is explained in the Log (platform utility
      differences, normalization gaps) and the runner fixed where it is wrong.
- [ ] A first `tests/agent/` set (L3) of at least 100 cases written to the patterns in
      `research/corpus.md` §4 and §9, recorded like the rest.
- [ ] `expected-failures.txt` per layer with every active case (0%), and a gate leg that
      runs the ratchets.
- [ ] `scripts/status` (summary per layer and area, appended to
      `tests/scores/status.tsv`) and `--hitlist` (failures weighted by
      `research/corpus/results.json` `features`).
- [ ] TESTING.md's *(planned)* sections updated to what was built (only those sections).

## Notes
- `research/spec-harness/` holds the earlier harness (Dockerfile, `SUITE-compat.txt`,
  results for 10 shells); reuse what helps. Oils' own expectations in the files (`##
  stdout:`, `## BUG bash`) are not our oracle; the recording is (D003).
- A case's files: record the directory listing and file contents after the run, bounded
  in size; skip what the case did not create.
- Running bash inside Docker is fine; reading its source is not (D007).
- Use a timeout for every recorded case (Oils uses 10 s).
- If the issue is too big for one pass, land it in this order and tell the coordinator:
  vendoring + runner + external engine, then recording, then ratchets + gate, then
  status/hit list, then L3.

## Log

## Follow-ups

## Result
