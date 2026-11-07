# 0033: Per-platform ratchets and the `platform` tag

status: in-progress
agent: shapes
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
- 2026-11-07: Linux build was broken on main (`[bool; 65]: Default`, NSIG on Linux):
  fixed and landed separately with gate leg `linux` (cargo check for aarch64/x86_64
  linux-musl, D1330), at the coordinator's request (f89dc7e).
- First Linux run (scripts/spec-linux): 185 shared entries pass on Linux (macOS-only),
  2 Linux failures listed nowhere (hidden by macOS): oils/bugs/13 (`ulimit` not
  supported; /dev/full only on Linux) and brush hash/18 (`command -p` used psh's default
  PATH). The 5 `leak` failures of that run were the container's: psh-spec as PID 1 does
  not reap orphans; the script now uses `docker run --init`.
- Outside touches (announced): `command -p` fix in psh-engine (builtins/command.rs, a new
  `Host::standard_path` in host.rs; psh-os unix `confstr(_CS_PATH)`), D1332, ARCH.md's
  Host list. brush/compat/builtins/command/32 passes now, hash/18 on Linux.
- psh-spec (infra's): platform.rs now does all ratchet checking (shared + the OS's file,
  moves with `--linux-results`, `--results`), `[[platform]]` in groups.rs, the status
  section; main.rs calls it. TESTING.md "Ratchets", D1331.
- Ratchets: `scripts/spec-linux --update` moved 161 entries to
  `expected-failures.macos.txt` (oils 58, brush 37, agent 66); 35 of them are
  host-specific and became `[[platform]]` rules (/private/tmp, /var/root, signal
  numbers, /usr/bin/cd scripts, the /bin layout, tz database, APFS names, /proc,
  /dev/full). Every macOS entry has a reason by cause (od padding, no tac, BSD ls/sed/
  touch/ln, rg missing, ...). agent/files/7 fails on Linux too: a real glob bug
  (`pkg-*/` matches the file pkg-file); its reason says so.
- wasm: oils/builtin-bracket/31 moved out of the shared file, so wasm needs its own
  line (psh's `touch -d` rejects 2017/12/31); oils/bugs/13's wasm line went (host-specific).
- brush/compat/background_jobs/18 quarantined ([[flaky]], timing under load; procs).

## Follow-ups

## Result
