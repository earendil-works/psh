# AGENTS.md

Instructions for every agent working in this repository: the coordinator and the
sub-agents that implement issues.

## What this is

psh is a portable, bash-compatible shell in Rust for running the commands coding agents
write: native on macOS, Linux and Windows, with in-process utilities, embeddable into the
pi-env daemon, with a parser for classifying commands, and buildable for WASM. Read these
before you start any work:

1. `research/README.md`, §8 "Agreed constraints". They are not up for debate in an issue.
2. `ARCH.md`: the design reference. Do not silently deviate from it. If the design needs
   to change, say so in your issue log and propose an edit.
3. `TESTING.md`: the oracle, the test layers, ratchets and the gate.
4. `docs/decisions.md`: decisions made so far. Append to it, never rewrite it.
5. `work/README.md`: how issues, worktrees and merging work.
6. Your issue file in `work/issues/`.

## Hard rules

- **Clean room.** Never read, copy or paraphrase GNU bash's source code, or
  `~/Development/oh-my-pi/crates/pi-shell/src/bash` (a port of it). The same holds for
  any GPL/LGPL source (GNU coreutils, sed, grep, gawk, busybox, yash, vim's xxd, GNU
  tree, GNU patch). Manuals and man pages may be read. GPL programs may be *run* as
  oracles; GPL test suites may be run, never vendored. When in doubt, ask.
- **Allowed sources.** brush (MIT; `/tmp/ls/brush`, or clone
  `github.com/reubeno/brush`) may be copied: keep its copyright notice in a header
  comment of each file that contains copied code, and list the file in `NOTICE`. The
  rest of oh-my-pi (builtins, Host design, Windows spawn recipe, minimizer, taps) is MIT
  and may be read; copy with attribution. uutils, ripgrep, fd, jaq and the other
  MIT/Apache/BSD projects named in `research/` likewise.
- **No tokio, no async runtime.** The engine is synchronous and stepped by its embedder,
  using only `std::task::Waker`. OS waits live behind the `Host` trait (ARCH.md).
- **Near-zero dependencies.** No crate without a recorded decision in
  `docs/decisions.md`, dev-dependencies included (the allowed list is under "Code
  conventions"). We copy from brush instead of depending on it.
- **Windows from the start.** Every layer builds and is tested on Windows. Never assume
  Git Bash, MSYS or WSL.
- **Never weaken the gate** to get green: no deleted tests, no ratchet entries added to
  hide a regression you caused, no skipped legs. Fix the failure or stop and report.
- **Timeouts on everything that runs a shell**: psh, bash, the spec runner, cargo tests
  that spawn processes. macOS has no `timeout`; use `scripts/timeout.sh SECONDS cmd ...`
  (from issue 0001). A hung test blocks you silently for hours.

## Oracle

GNU bash 5.3 is the reference (TESTING.md "Oracle"):

- The gate never runs bash. It compares against committed goldens, recorded once in a
  pinned Linux container by `scripts/record-goldens` (Docker is available locally).
- For quick probes while you work: `/opt/homebrew/bin/bash` (5.3, macOS). Always with a
  timeout. macOS differs from the recording container in its utilities (BSD `sed`,
  `ls`, ...), so a probe that involves external programs can disagree with the goldens;
  the goldens win.
- Do not use `/bin/bash` (3.2) or `sh` as a reference.

## Code conventions

- Rust edition 2024, a Cargo workspace with `crates/*`. Develop on current stable, but
  the MSRV is **1.96** (pi-env's pin, D025): use no language or library feature newer
  than 1.96. `cargo fmt` and clippy with `-D warnings` must be clean.
- `psh-syntax`, `psh-regex` and `psh-classify` are `#![no_std]` + `alloc` (D023);
  never force std into them from the engine. `psh-engine` uses std and must
  build for `wasm32-unknown-unknown` (gate).
- Allowed dependencies so far: the ripgrep crate set (D020/D021) in `psh-utils`
  (`grep-printer` patched in `third_party/`, D1135), `libc`/`windows-sys` in `psh-os`
  (D024), jaq for `jq` (D028), `deser-json` for all JSON (D029), `argument` in the `psh`
  binary only (D035). `scripts/deps-allowlist.txt` lists every pair with its decision
  and the gate checks it (D1223); anything else needs a decision first.
- No `unsafe` outside the OS host crates (`psh-os`) and the wasm export layer
  (`psh-wasm`: only its JS import and its `no_mangle` exports, D1110) unless a decision
  records why. Every `unsafe` block says which invariant makes it sound.
- Prefer plain, direct code. Comments explain *why*. No speculative abstraction.
- Every feature comes with tests: unit tests next to the code, behavior cases in
  `tests/` (TESTING.md).
- psh is unreleased: change formats and APIs in place; no compatibility shims.

## Workflow (sub-agents)

You work on exactly one issue at a time, in your own git worktree under `.worktrees/`,
on the branch `issue/NNNN-slug`. The coordinator creates the worktree for you.

1. Read the issue. Set `status: in-progress` in its header and commit that.
2. Work in small steps and **commit often** on your branch. Message format:
   `NNNN: short imperative summary`, plus a body when the reason is not obvious.
3. Keep the issue's `## Log` current: what you did, what you found, open questions. This
   file is how context survives between agents and sessions.
4. A design decision others must know about goes into `docs/decisions.md` on your
   branch, using the ID range in your issue header (format in that file).
5. Follow-up work goes under `## Follow-ups` in your issue. Do not create issue files;
   the coordinator does.
6. When the acceptance criteria are met:
   - set `status: done` and fill in `## Result` (what landed, how to use it, known gaps),
   - if cases newly pass, update the ratchets (`--update-ratchet`, TESTING.md) on your
     branch: a stale ratchet hides later regressions,
   - commit, then run `scripts/merge-to-main.sh` from your worktree. It rebases onto
     `main`, runs `scripts/gate.sh` and fast-forwards `main` under a lock. If the rebase
     conflicts, resolve it and run it again. If the gate fails, fix the failure.
7. Report back to the coordinator: what landed, the commit range, follow-ups, surprises.

Rules:

- Never commit directly on `main`, never push. `main` always passes the gate.
- Never use `git stash`: the stash is shared by all worktrees. Use a WIP commit on your
  branch or a patch in `work/scratch/<issue-id>/`.
- Do not edit `work/BOARD.md` (the coordinator owns it) or other agents' issue files.
- Scratch files, logs and large outputs go in `work/scratch/<issue-id>/` (gitignored).
  External checkouts (Oils, brush, upstream test suites) go in `vendor/` **of the
  primary checkout**, so all worktrees share one copy (gitignored). What the tests need
  is vendored into `tests/` and committed.
- Never read or copy the private corpus (`~/.cache/psh-corpus/`) into the repository.
  Only its sanitized shape corpus and aggregate numbers are committed (TESTING.md).
- If you are blocked, or the issue is much bigger than described, stop and tell the
  coordinator instead of expanding the scope.
- Stay inside your issue's `touches:`. If you must change something outside them, say so
  in the log and keep it minimal: others may be working there.

## Testing while iterating

The gate runs once, inside `scripts/merge-to-main.sh`; don't run it yourself before
merging. While iterating, run only what your change can break:

- Unit tests of one crate or module: `scripts/timeout.sh 300 cargo test -p psh-syntax
  lexer::`.
- Spec cases, filtered: `scripts/timeout.sh 300 cargo run -q -p psh-spec -- <filter>`
  (file or case name), `--diff` to see the mismatch.
- Never re-run the oracle; read the goldens.
- Leave cores for others: cap parallel runs (`-j 6`).

## Keeping the docs current

`docs/guide/` is the human-facing explanation of psh (`docs/guide/README.md` says how
to write it). If your change alters what a guide page, ARCH.md or TESTING.md says (your
issue's `guide:` field names the pages), update that prose on your branch and say in the
log which pages you updated, or that none were affected. If a whole chapter becomes wrong
or a new one is needed, list it under `## Follow-ups`. Guide rules in short:
self-contained (no links to ARCH.md, TESTING.md, docs/*.md, work/ or issue numbers), no
decision IDs, no history, pi named only on `pi.md`, goals by name with a link to
`goals.md`.

## Coordinator

The coordinator (the main agent) owns `work/BOARD.md`, writes issues, creates worktrees,
assigns agents, reviews merged work and keeps ARCH/TESTING/decisions and the guide
coherent (with a docs agent). It works directly on `main` only for coordination files
(issues, board, docs) and commits right away, so `main` is never left dirty; once
`scripts/with-merge-lock.sh` exists it commits through it. `work/COORDINATOR.md` is its
checklist.
