# 0033: Per-platform ratchets and the `platform` tag

status: open
agent: -
branch: issue/0033-platform-ratchets
depends-on: 0002, 0031
touches: crates/psh-spec/ (groups/ratchets), tests/spec.toml, tests/**/expected-failures*.txt, scripts/status, TESTING.md (ratchet section)
guide: -
decisions: D1330–D1339

## Goal
The plan from 0002/0022's follow-ups: goldens stay Linux bash 5.3; cases that differ only
because the *host's* external programs or OS differ get platform-specific handling, so
the shared ratchet only holds real psh gaps.

## Acceptance criteria
- [ ] `tests/<layer>/expected-failures.<os>.txt` (macos, windows; wasm exists) next to
      the shared one: a case may be listed there only if it passes on Linux; the gate on
      that OS consults shared + its own file. `--update-ratchet` writes the right file
      (a case failing only on this OS goes to the OS file).
- [ ] A `platform` tag in spec.toml for inherently host-specific cases (/bin layout,
      case-insensitive FS, /proc, signals that don't exist) with a reason; such cases run
      but count separately in scripts/status.
- [ ] Move the macOS-only failures out of the shared ratchets (e.g. the 29 BSD-utility
      L3 cases from 0031 that pass on Linux, `ls` exit codes, GNU-only programs). Many
      will start passing anyway as psh's own utilities land; the per-OS file shrinks then.
- [ ] A Linux spec run is possible locally (Docker) so the shared ratchet can be checked
      against Linux: `scripts/spec-linux` (build psh for linux-musl or in the container,
      run psh-spec there). Optional gate leg if fast enough; else a documented manual step.
- [ ] scripts/status shows shared vs per-OS failures.

## Log

## Follow-ups

## Result
