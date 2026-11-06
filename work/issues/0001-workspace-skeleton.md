# 0001: Workspace skeleton and the empty gate

status: in-progress
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
- [ ] Workspace (`Cargo.toml`, edition 2024, stable, `crates/*`) with stub crates:
      `psh-syntax` (lib, zero deps), `psh-engine` (lib, depends only on psh-syntax),
      `psh` (binary: `psh -c CMD` prints "psh: not implemented" to stderr and exits 2),
      `psh-spec` (binary stub). The other crates of ARCH.md are created by their issues.
- [ ] `scripts/gate.sh`: `cargo fmt --all --check`, clippy `--workspace --all-targets -D
      warnings`, `cargo test --workspace`, and `cargo build -p psh-syntax -p psh-engine
      --target wasm32-unknown-unknown`. A marked place where later legs are added.
- [ ] `scripts/merge-to-main.sh`: run from an issue worktree on an `issue/*` branch;
      refuses a dirty worktree or a dirty primary checkout; takes
      `work/scratch/merge.lock` (mkdir lock, with owner and wait message); rebases onto
      `main`; runs the gate; fast-forwards `main` in the primary checkout.
- [ ] `scripts/with-merge-lock.sh CMD...`: runs a command under the same lock (for the
      coordinator's commits on `main`).
- [ ] `scripts/timeout.sh SECONDS CMD...`: macOS has no `timeout`. Runs CMD under
      `nice -n 10` in its own process group, kills the whole group (TERM, then KILL
      after a few seconds) when the time is up, exits 124 like GNU timeout, passes the
      exit status through otherwise. Works on macOS and Linux; tested.
- [ ] `.gitattributes`: `union` merge for `docs/decisions.md` and
      `tests/**/expected-failures.txt`.
- [ ] `.cargo/config.toml` caps build jobs and test threads (6) to leave cores for
      parallel agents.
- [ ] `NOTICE` with a section for code copied from brush (MIT, copyright notice), empty
      file list for now.
- [ ] The gate passes on the result; merge-to-main is exercised once end to end with a
      throwaway branch (then deleted).

## Notes
- Check that the `wasm32-unknown-unknown` target is installed (`rustup target list
  --installed`); say in the log if you had to add it.
- psh's license is an open owner question (README.md): do not add LICENSE files until
  the coordinator says which.
- Keep the scripts plain bash 3.2-compatible: they run under macOS's `/bin/bash`.

## Log

## Follow-ups

## Result
