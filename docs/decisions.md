# Decisions

An append-only log of decisions. Newest at the bottom. To revise a decision, add a new
entry that supersedes it and say so; never edit an old one.

IDs: the coordinator uses D001–D999. An issue NNNN uses the ten IDs from
D(1000 + 10·NNNN) to D(1009 + 10·NNNN), so 0004 uses D1040–D1049 and 0012 uses
D1120–D1129; its header names the range. Parallel branches never collide.

Format:

```markdown
## DNNN: Title
date: YYYY-MM-DD · by: <agent/coordinator> · issue: NNNN
Decision, with the reason in one or two sentences. Alternatives considered, if relevant.
```

## D001: Work tracking in files, worktrees in `.worktrees/`
date: 2026-11-06 · by: coordinator · issue: -
Issues live in `work/issues/`, the board in `work/BOARD.md`. Sub-agents work in git
worktrees under `.worktrees/` on `issue/NNNN-slug` branches and merge with
`scripts/merge-to-main.sh`. Scratch space is `work/scratch/`, external checkouts go in
`vendor/` of the primary checkout; both are gitignored. Everyone commits in small steps
as they go.

## D002: Scope: bash for driving agents
date: 2026-11-06 · by: owner · issue: -
psh implements non-interactive bash 5.x semantics, as `bash -c` runs them. No line
editing, completion, history, zsh mode or POSIX-sh mode. Job control only as far as `&`,
`wait`, `kill`, `$!` (and `jobs`). The interface stays "command string in; stdout,
stderr and exit status out". Real bash stays available as an escape hatch (`shellPath`).
Reason: agents are the only user; everything interactive is cost without benefit.

## D003: The oracle is GNU bash 5.3, recorded once
date: 2026-11-06 · by: owner · issue: -
psh matches what bash 5.3 does, including where Oils marks bash as `BUG`. Goldens are
recorded from bash 5.3 in a pinned Linux container and committed; the gate never runs
bash. Deviations need a decision of their own and are declared in the tests. Reason: a
deterministic, fast gate that every platform compares against, and one clear target.
`/bin/bash` 3.2 on macOS is never a reference.

## D004: Rust, stable, edition 2024
date: 2026-11-06 · by: owner · issue: -
psh is written in Rust (stable toolchain, edition 2024), a Cargo workspace with
`crates/*`. Reason: pi-env is Rust, the reusable pieces (brush, uutils, ripgrep, jaq) are
permissive Rust, and one codebase covers static binaries for all of pi-env's targets,
N-API and WASM. Alternatives: Go (mvdan/sh; can't link into pi-env, heavy WASM, lower
conformance) and TypeScript (no static remote binary).

## D005: No tokio, no async runtime
date: 2026-11-06 · by: owner · issue: -
The engine is a synchronous state machine stepped by its embedder. It uses only
`std::task::Waker` for wakeups; all OS waits sit behind a `Host` trait (waiter threads,
pidfd/select on Unix, Job Objects and overlapped pipes on Windows). Reason: pi-env is
synchronous and threaded, and oh-my-pi's tokio-based shell hit a long series of
deadlocks and signal hijacks (`research/execution-core.md`). It also keeps WASM free of
Asyncify/JSPI.

## D006: Near-zero dependencies, with an rg/fd exception
date: 2026-11-06 · by: owner · issue: -
No crate is added without a decision here, dev-dependencies included. We copy permissive
code (brush, uutils, ...) with attribution rather than depend on it. The deliberate
exception: `rg` and `fd` are built on the ripgrep/fd crates (`grep-*`, `ignore`,
`globset`, `regex`), because reimplementing them well is not worth it. Reason: size,
build time on eight targets, auditability, and embedding into a 1.9 MB daemon.

## D007: Clean room
date: 2026-11-06 · by: owner · issue: -
Only MIT/Apache/BSD-style code may be used. Nobody working on psh reads GNU bash's
source or oh-my-pi's bash-derived engine (`crates/pi-shell/src/bash`, which describes
itself as a function-by-function port of bash), or any other GPL source it would mirror.
Manuals may be read; GPL programs and test suites may be run, never vendored. Reason:
psh is shipped inside Pi under a permissive license.

## D008: brush as a source to copy, not a dependency
date: 2026-11-06 · by: owner · issue: -
brush (MIT, reference checkout at `943553433ae0`, 2026-11-05) may be copied with
attribution. Its parser is the starting point for `psh-syntax`, with `peg` and `winnow`
replaced by a hand-written lexer and recursive-descent parser. brush-core's interpreter
and sys layer are not copied: they are async (tokio, async-recursion), which D005 rules
out; pieces of its expansion logic may be. Reason: brush leads the open shells on the
Oils suite, but its runtime model and dependencies don't fit.

## D009: Test backbone: Oils spec and brush cases with our goldens
date: 2026-11-06 · by: owner · issue: -
The language is climbed on the Oils spec tests (Apache-2.0, pinned to `d114978`,
vendored) and brush's YAML cases (MIT, vendored), both judged against bash 5.3 goldens
(D003), plus our own agent-shaped cases. The private corpus of agent commands is used
only through a sanitized, parse-only shape corpus; it is never executed or committed.

## D010: Builtin utilities: generous, honest, switchable
date: 2026-11-06 · by: owner · issue: -
psh errs on the side of too many in-process utilities (`research/builtins.md`). Never
builtins: interpreters, toolchains, package managers, VCS, network and database clients
(e.g. `python`, `cargo`, `npm`, `git`, `curl`, `sqlite3`). Every utility falls through to
the host program of that name, or fails clearly, on flags it does not support; it never
silently diverges. Each can be turned off. An explicit path always runs the real program.

## D011: Windows from day one
date: 2026-11-06 · by: owner · issue: -
Every layer builds and is tested on Windows in CI. No dependency on Git Bash, MSYS,
Cygwin or WSL. Reason: Windows is where psh helps most, and oh-my-pi's Linux-only CI let
Windows regress.

## D012: Ratchets, not targets
date: 2026-11-06 · by: coordinator · issue: -
Each test layer has an expected-failures file. The gate fails on any case that newly
fails and reports cases that newly pass; `--update-ratchet` removes them. Progress is
measured as ratchet lines deleted. Nobody weakens the gate to get green.

## D013: One parser for execution and classification
date: 2026-11-06 · by: coordinator · issue: -
Classification, summaries and permission checks run on the AST of the same parser the
engine executes, built for native and WASM. Reason: a classifier that parses differently
from the executor can be bypassed (`research/landscape.md` §4).

## D014: Never-builtins: the owner rule and the proposed extensions
date: 2026-11-06 · by: coordinator · issue: -
Supersedes the examples in D010's never-builtin sentence. **Owner rule** (never
builtins): database clients and stateful engines like `sqlite3`, interpreters and language
runtimes, toolchains and build systems, package and environment managers.
**Proposed by `research/builtins.md` §4.5, pending owner confirmation:** `git`, `curl`/
`wget`, `ssh`/`scp`/`rsync` and other network clients, interactive/TTY programs,
privileged and OS-specific system tools, heavy domain tools, compressors in compress mode
when a real one exists, `shred`/`mkfs`. Until confirmed, nobody implements any of them
as a utility. Reason for the proposals: huge flag surfaces, TLS/auth/state, and keeping
network and VCS effects visible to the classifier as real programs.

## D015: Shadowing modes for utilities
date: 2026-11-06 · by: coordinator · issue: -
Every utility has one mode: S (always the builtin), F (builtin only when no program of
that name is on PATH, or in WASM/sandbox mode), W (S on Windows, F elsewhere: names that
collide with System32 programs such as `find`, `sort`, `timeout`, `tree`). An explicit
path always runs the real program; per-session off switches cover all utilities, the
destructive group (`rm mv cp ln dd`) and single names; `command -v`/`type`/`which`
report utilities truthfully; `env`, `xargs`, `find -exec` and `timeout` dispatch back
into the shell. Reason: a builtin that shadows a better real program is the main cost of
having builtins (just-bash's issue tracker); the modes confine it.

## D016: Utility flag dialect: GNU ∪ BSD with deterministic tie-breaks
date: 2026-11-06 · by: coordinator · issue: -
Utilities accept the union of GNU and BSD flags. On conflict, GNU semantics, except where
agents overwhelmingly use the macOS form (`sed -i ''`, `stat -f FORMAT`). Conflicts are
resolved by rules on the arguments, never by the host OS: the table in ARCH.md
"psh-utils" (from `research/builtins.md` §4.6) for `sed -i`, `stat -f`, `date -r`,
`base64 -i`, `mktemp -t`, `cp -R src/` (GNU), `wc`/`uniq -c` widths (GNU), `find` without
a path (`.`), and output formats keyed on the name (`md5` vs `md5sum`). Still open: `ls
-G`/`-T`, `xargs` on empty input, `sort` collation. Host-reporting commands (`uname`,
`hostname`, `id`) report the real host. Reason: the corpus shows agents mixing dialects
on the same machine.

## D017: Utility tiers and order of work
date: 2026-11-06 · by: coordinator · issue: -
Tier 1 (61 names, listed in ARCH.md) is the first utility milestone, starting with the
framework and `timeout`; Tier 2 (~45 names) follows; Tier 3 is on demand and mostly F
mode. Batches are cut by area so that agents work in parallel (one per batch). Reason:
the corpus ranking plus the consensus of six other shells' sets.

## D018: Utility sources: port mature permissive code onto injected I/O
date: 2026-11-06 · by: coordinator · issue: -
Utilities are ported from mature MIT/Apache/BSD code rather than written fresh: uutils
coreutils/findutils/diffutils/sed, the ripgrep crates, jaq. Ported code is refactored
onto psh's utility context (stdio, fs, cancel) instead of `std::io::stdout()`/`std::fs`;
re-executing the binary per utility (brush's approach) is ruled out because it fails in
WASM and breaks cancellation. `awk` is a port of goawk (MIT), checked against the
onetrueawk `T.*` and goawk tests, and stays in F mode until it passes them. Wrappers
(`timeout nice nohup env xargs`), `which`, `xxd`, `tree`, `column` and `strings` are our
own. Each new crate still needs its own decision (D006).

## D019: fd is a Tier 1 utility
date: 2026-11-06 · by: owner · issue: -
`fd` moves from Tier 3 (fallback) to Tier 1, mode S (always embedded), next to `rg`.
Tier 1 is now 62 names; this supersedes the count in D017. Reason: agents reach for `fd`
wherever it exists, and it is rarely installed (30 corpus uses on macOS).

## D020: Search utilities on the ripgrep crates (proposed, pending owner)
date: 2026-11-06 · by: coordinator · issue: -
**Status: proposed, pending owner confirmation.** Nobody adds these crates before it is
confirmed. Source: `research/builtins.md` §5 (measurements at ripgrep `91f5d22`, fd
`193d0df`).
- **Crate set** (the whole D006 exception): direct dependencies `grep-matcher` 0.1.9,
  `grep-regex` 0.1.14, `grep-searcher` 0.1.17, `grep-printer` 0.3.1 with
  `default-features = false` (no serde), `ignore` 0.4.34, `globset` 0.4.21, `regex`
  1.13.1, and `regex-automata` 0.4.18 / `regex-syntax` 0.8.11 where used directly; plus
  their transitive crates as of that lock (about 27 on macOS, 30 on Windows, 28 on
  linux-musl, 25 on wasm32: memchr, bstr, aho-corasick, walkdir, same-file, termcolor,
  winapi-util, crossbeam-deque/epoch/utils, encoding_rs, encoding_rs_io, simdutf8,
  memmap2, log, libc, cfg-if, scopeguard, windows-sys/windows-link and the like).
  **Excluded:** `grep-cli` (process-centric; psh owns TTY, stdin, decompression) and
  `grep-pcre2` (C code via pcre2-sys; breaks the C-free WASM build). Adding a crate to
  the set, or a new feature that pulls one in, needs its own decision; version bumps
  within the set are fine if the transitive set does not grow.
- **rg:** copy ripgrep's flag layer (`crates/core/flags/*`) and glue with attribution,
  patching the in-process seams (statics, process env/cwd/stdio, `exit`, spawning
  through psh's runner); copy `lexopt` rather than depend on it. `rg --json` falls
  through until JSON output is decided.
- **fd:** re-implement the CLI on psh's argument parser; copy fd's filter, exec and
  format code with attribution; none of fd's other dependencies (clap, jiff, ctrlc, nix,
  ...).
- **Walkers:** native = `ignore`'s walker (absolute roots, shell cwd/env, cancel checks);
  virtual fs / WASM = psh's own single-threaded walker reusing `ignore`'s matchers with
  the ignore chain copied from `ignore/src/dir.rs`. A parity test runs both natively over
  the same fixtures and requires identical results. `grep` and `find` reuse the walker
  and matchers.
- **Output order:** path-sorted by default when searching with more than one thread
  (ordered merge, not ripgrep's single-threaded `--sort path`).
- **Regex gaps fall through** (D010): backreferences until psh's own backtracking
  matcher exists; POSIX leftmost-longest spans (`grep -o`, `--color`, `-b`, `sed s///`)
  until a `regex-automata` longest-match mode is proven; PCRE-only syntax always. Where
  no host program exists (Windows, WASM), they fail clearly.
Reason: the library route costs about 0.5–0.75 MB over the regex stack psh needs anyway,
against 3.7–4.7 MB for vendoring both CLIs; it works in WASM except for the walk, which
the second walker covers; and copying ripgrep's flags keeps rg's full surface and help.

## D021: D020 confirmed
date: 2026-11-06 · by: owner · issue: -
The owner confirms D020: the search utilities use exactly that ripgrep crate set and
approach. Its "proposed, pending owner" status no longer applies.

## D022: VCS and network clients are out of scope, not never-builtins
date: 2026-11-06 · by: owner · issue: -
Supersedes the "proposed" part of D014 for these names. `git`, `curl`/`wget`, and
ssh/rsync-like network and VCS clients are **out of scope for now**: they are not
utilities and nobody works on them. They are not on the never list, because psh may
later hook them (host-provided commands, interception). Database clients and stateful
engines (`sqlite3`-like), interpreters, toolchains and package managers remain
never-builtins (D014's owner rule). The remaining D014 proposals (interactive/TTY
programs, privileged system tools, heavy domain tools, compressors when a real one
exists, `shred`/`mkfs`) stay proposals: not utilities, no work on them.

## D023: psh-syntax is no_std; psh-engine uses std
date: 2026-11-06 · by: owner · issue: -
`psh-syntax` is `#![no_std]` + `alloc`, for small, clean wasm builds of the parser
alone. `psh-engine` uses std; the `wasm32-unknown-unknown` build in the gate guards it
against OS access. The engine must never force std into the parser: no std-only types in
`psh-syntax`'s API, and no feature of `psh-syntax` that the engine enables to get std.
`psh-classify` follows the parser (no_std + alloc).

## D024: psh-os may use libc and windows-sys
date: 2026-11-06 · by: owner · issue: -
The OS host crate may depend on `libc` (Unix) and `windows-sys` (Windows), as pi-env
does. No other platform crates without a decision.

## D025: MSRV 1.96
date: 2026-11-06 · by: owner · issue: -
psh builds with Rust 1.96, pi-env's pinned toolchain, because pi-env links it. Local
development uses the current stable (1.99 at the time). A gate leg that checks 1.96 is a
follow-up; until it exists, don't use language or library features newer than 1.96.

## D026: pi-env runs a fresh shell per exec; Node goes through the daemon
date: 2026-11-06 · by: owner · issue: -
Each pi-env `exec` gets a fresh psh shell; no state (cwd, variables, functions) persists
across calls for now, which keeps Durable's replay model intact. Node-side execution
(`NodeExecutionEnv` and friends) uses psh through the pi-env daemon, not a separate psh
binary or addon.

## D027: Windows CI: Parallels VM now, GitHub Actions once there is a remote
date: 2026-11-06 · by: owner · issue: -
Until the repository has a remote, the gate's Windows leg runs on the owner's Parallels
VM via `prlctl exec`. Once a remote exists, it moves to GitHub Actions.

## D028: Utility crates: jaq allowed; others decided per utility issue
date: 2026-11-06 · by: owner · issue: -
`jq` may depend on jaq; whether to copy it instead is decided in its issue only if size
demands it. Crates for the date parser, tar/zip and process listing (`sysinfo`) are
decided in the utility issue that needs them (each still recorded as a decision, D006).

## D029: JSON through deser-json
date: 2026-11-06 · by: owner · issue: -
All JSON output and input in psh (`rg --json`, `psh --parse`, `psh --classify`, and any
later JSON) uses the `deser-json` crate (deser 0.10.x). Not serde_json, not a
hand-written printer. Note for `psh-syntax` (no_std, D023): the AST's JSON form is
produced outside the parser crate (in `psh` or a separate module) unless deser-json
works under no_std; the parser itself does not depend on it.

## D030: grep and rg treat CRLF as line ends on Windows
date: 2026-11-06 · by: owner · issue: -
On Windows, `grep` and `rg` default to CRLF-aware line terminators (ripgrep's `--crlf`
behavior), so `x$` matches lines ending in `\r\n`. Explicit flags override it. Elsewhere
the default stays LF.

## D031: Utilities are a pi-env cargo feature, on by default
date: 2026-11-06 · by: owner · issue: -
pi-env builds psh's utilities (and with them the regex stack) behind a cargo feature
that is on by default, so a size-constrained build can drop them.

## D032: License: MIT OR Apache-2.0
date: 2026-11-06 · by: owner · issue: -
psh is licensed MIT OR Apache-2.0, with `LICENSE-MIT` and `LICENSE-APACHE` at the root,
like djs. Copied code keeps its own notices (`NOTICE`).

## D033: Goal ranking
date: 2026-11-06 · by: owner · issue: -
When goals conflict: agent-first, then portable, then bash's behavior, then the rest
(consistent, understandable commands, control of execution, embeddable, sandboxable),
which are unranked among themselves; the design that makes such a trade-off says why.
Licensing holds under all of them.

## D034: Remaining utility dialect points fall through until their issue decides
date: 2026-11-06 · by: owner · issue: -
`ls -G`/`-T`, `xargs` on empty input and `sort`'s default collation are decided in the
utility issues that implement them (0015, 0017, 0016), each as a recorded decision. Until
then, those flags and cases fall through to the host program (or fail clearly).

## D1010: merge-to-main gates outside the lock, optimistically
date: 2026-11-06 · by: infra · issue: 0001
`scripts/merge-to-main.sh` rebases and runs the gate in the issue worktree without the
merge lock, then takes the lock only to check that `main` has not moved and to
fast-forward it (seconds). If `main` moved, it releases the lock, rebases and gates
again; after two lost races the next attempt holds the lock from rebase to
fast-forward, so no branch starves. Reason: the coordinator's and docs agent's commits
on `main` (through `scripts/with-merge-lock.sh`) never wait for a whole gate, and gates
of different branches overlap. Gates are not serialized by a lock; the cargo caps
(D1013) bound what several concurrent gates cost.

## D1011: A dirty primary checkout makes merge-to-main wait, not fail
date: 2026-11-06 · by: infra · issue: 0001
When the primary checkout has uncommitted changes to tracked files at fast-forward
time, merge-to-main releases the lock and polls until they are committed (up to
`PSH_MERGE_DIRTY_WAIT` seconds, default 1800), then rebases and gates again if the
commit moved `main`; it never touches those changes. Untracked files in the primary
checkout do not block (the fast-forward itself refuses to overwrite one). The issue
worktree, in contrast, must be fully clean, untracked files included, so the gate tests
exactly the commit that lands.

## D1012: scripts/timeout.sh runs on perl
date: 2026-11-06 · by: infra · issue: 0001
The portable timeout is a bash wrapper around a short perl program (fork, `setpgrp`,
`alarm`, kill the group), because bash 3.2 cannot put a command into its own process
group without job control, and perl is on every macOS and on Debian (perl-base).
Time::HiRes, when present, gives fractional durations; without it they round up to
whole seconds. On timeout the group gets TERM, then KILL after `PSH_TIMEOUT_GRACE`
seconds (default 5) unless it has emptied; the status is 124. Commands run under
`nice -n 10` except the gate and the merge scripts (`PSH_GATE=1`), so agents' builds
yield to the gate.

## D1013: Gate legs and their bounds
date: 2026-11-06 · by: infra · issue: 0001
`scripts/gate.sh` runs its legs in order, each under `scripts/timeout.sh`: `scripts`
(every script parses), `fmt`, `clippy`, `test`, `deps` (the full dependency set of
`psh-syntax` is empty and of `psh-engine` is exactly `psh-syntax`; a new dependency
there must change the leg along with its decision), `wasm`, `msrv` (`cargo check` of
the workspace with the `rust-version` toolchain, D025; fails if that toolchain is not
installed) and `script-tests` (the tests of timeout and merge-to-main). The gate keeps
`.cargo/config.toml`'s caps (6 build jobs, 6 test threads) because several gates may
run at once (D1010).

## D1014: Workspace conventions
date: 2026-11-06 · by: infra · issue: 0001
The workspace sets `unsafe_code = "forbid"` in `[workspace.lints.rust]` and every crate
opts in with `[lints] workspace = true`; `psh-os` will declare its own lint table instead.
`.gitattributes` stores and checks out text files with LF everywhere (`* text=auto
eol=lf`), so scripts and goldens are byte-identical on Windows. The copyright line in
`LICENSE-MIT` and `NOTICE` is "Earendil Inc.", as in djs.
