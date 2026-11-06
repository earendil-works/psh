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
