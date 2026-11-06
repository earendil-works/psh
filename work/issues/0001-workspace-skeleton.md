# 0001: Workspace skeleton and the empty gate

status: done
agent: infra
branch: issue/0001-workspace-skeleton
depends-on: -
touches: Cargo.toml, crates/, scripts/, .gitattributes, .cargo/, NOTICE, LICENSE*
guide: -
decisions: D1010–D1019

## Goal
The Cargo workspace, the gate (empty but enforced) and the merge tooling, so that every
later issue lands through the same process (AGENTS.md, work/README.md, TESTING.md "The
gate").

## Acceptance criteria
- [x] Workspace (`Cargo.toml`, edition 2024, stable, `crates/*`) with stub crates:
      `psh-syntax` (lib, zero deps), `psh-engine` (lib, depends only on psh-syntax),
      `psh` (binary: `psh -c CMD` prints "psh: not implemented" to stderr and exits 2),
      `psh-spec` (binary stub). The other crates of ARCH.md are created by their issues.
- [x] `scripts/gate.sh`: `cargo fmt --all --check`, clippy `--workspace --all-targets -D
      warnings`, `cargo test --workspace`, and `cargo build -p psh-syntax -p psh-engine
      --target wasm32-unknown-unknown`. A marked place where later legs are added.
- [x] `scripts/merge-to-main.sh`: run from an issue worktree on an `issue/*` branch;
      refuses a dirty worktree or a dirty primary checkout; takes
      `work/scratch/merge.lock` (mkdir lock, with owner and wait message); rebases onto
      `main`; runs the gate; fast-forwards `main` in the primary checkout.
- [x] `scripts/with-merge-lock.sh CMD...`: runs a command under the same lock (for the
      coordinator's commits on `main`).
- [x] `scripts/timeout.sh SECONDS CMD...`: macOS has no `timeout`. Runs CMD under
      `nice -n 10` in its own process group, kills the whole group (TERM, then KILL
      after a few seconds) when the time is up, exits 124 like GNU timeout, passes the
      exit status through otherwise. Works on macOS and Linux; tested.
- [x] `.gitattributes`: `union` merge for `docs/decisions.md` and
      `tests/**/expected-failures.txt`.
- [x] `.cargo/config.toml` caps build jobs and test threads (6) to leave cores for
      parallel agents.
- [x] `NOTICE` with a section for code copied from brush (MIT, copyright notice), empty
      file list for now.
- [x] The gate passes on the result; merge-to-main is exercised once end to end with a
      throwaway branch (then deleted).

## Notes
- Check that the `wasm32-unknown-unknown` target is installed (`rustup target list
  --installed`); say in the log if you had to add it.
- psh's license is an open owner question (README.md): do not add LICENSE files until
  the coordinator says which.
- Keep the scripts plain bash 3.2-compatible: they run under macOS's `/bin/bash`.

## Log
- 2026-11-06: Owner decisions override the notes: license MIT OR Apache-2.0 (D032;
  LICENSE files copied from djs, copyright "Earendil Inc." as in djs), psh-syntax
  `#![no_std]` + alloc (D023), rust-version 1.96 (D025). `wasm32-unknown-unknown` and
  the 1.96.0 toolchain were already installed.
- Workspace: psh-syntax (no_std, zero deps), psh-engine (std, psh-syntax only), psh
  (`-c` stub, exit 2), psh-spec (stub, exit 2). `unsafe_code = "forbid"` workspace lint.
  Binary invocation tests spawn psh under a 10 s deadline.
- timeout.sh: bash + perl (fork/setpgrp/alarm). Found that Debian's perl-base has no
  Time::HiRes; it is optional now (fractions round up without it). Tested on macOS
  (/bin/bash 3.2, bash 5.3) and in a debian:stable container.
- merge-to-main: gates outside the lock, lock only for the fast-forward, re-gates when
  main moved, holds the lock throughout after two lost races; waits for a dirty primary
  checkout instead of failing (D1010, D1011). It runs from a temp copy because the
  rebase rewrites the script. scripts/test-merge-to-main.sh covers it end to end in a
  throwaway repository with a stub gate (plain merge, refusals, failing gate, main moving
  during the gate, two lost races, dirty primary wait and give-up, rebase conflict,
  stale and live locks, with-merge-lock); a mutation (disabling the moved/dirty checks)
  makes 9 of its checks fail.
- Gate: added `deps`, `msrv` (closes the D025 follow-up) and `script-tests` legs beyond
  the issue (D1013). Full gate: ~14 s warm, mostly the script tests.
- End to end: a throwaway clone of the repository with `issue/9999-throwaway` and a
  commit on its main made after branching: rebased, real gate, fast-forwarded; clone
  deleted.
- Docs touched outside `touches:` (small): TESTING.md "The gate" legs, work/README.md
  "Merging" (now describes the optimistic lock), decisions D1010–D1014.

## Follow-ups
- Gate caching by leg inputs (TESTING.md "The gate": docs-only changes in seconds).
- Windows: the gate and scripts are bash; 0007 decides how the Windows legs run
  (`prlctl exec`, D027). timeout.sh is Unix-only by design.
- A workspace-wide dependency allowlist (all crates, not just syntax/engine) once
  psh-utils and psh-os add their crates.
- Whether gates should share one warm gate worktree (as djs does) instead of each
  agent's worktree, if concurrent gates start to hurt.

## Result
Landed: the Cargo workspace (edition 2024, rust-version 1.96, MIT OR Apache-2.0) with
stub crates psh-syntax, psh-engine, psh, psh-spec; `.cargo/config.toml` (6 jobs, 6 test
threads); `.gitattributes` (LF everywhere, union merge for decisions and ratchets);
LICENSE-MIT, LICENSE-APACHE, NOTICE (brush section, no files yet); and the scripts:

- `scripts/timeout.sh DURATION CMD...`: own process group, nice 10, TERM then KILL to
  the group, 124 on timeout. `PSH_TIMEOUT_GRACE` sets the grace (default 5 s).
- `scripts/gate.sh [LEG...|--list]`: legs scripts, fmt, clippy, test, deps, wasm, msrv,
  script-tests; later legs go into `LEGS` and `run_leg` (marked).
- `scripts/merge-to-main.sh`: from an issue worktree; rebase, gate, lock, fast-forward.
  Knobs: `PSH_GATE_TIMEOUT` (3600), `PSH_MERGE_LOCK_WAIT` (3600),
  `PSH_MERGE_DIRTY_WAIT` (1800), `PSH_MERGE_POLL` (2). Log:
  `work/scratch/merge-log.tsv`.
- `scripts/with-merge-lock.sh CMD...`: the coordinator's commits on main.

Known gaps: no leg caching; the scripts need bash and perl (no native Windows run);
the deps leg covers psh-syntax and psh-engine only.
