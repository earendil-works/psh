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

## D1030: The shape corpus is made with tree-sitter-bash, checked with stdlib Python
date: 2026-11-06 · by: shapes · issue: 0003
`scripts/shape-corpus` parses each command with tree-sitter-bash (local venv only, as
`research/corpus/` did) and rewrites the original text in place, word by word, so
skeletons keep the original's layout and the parser under test is never its own oracle.
Text the tree does not explain is sanitized word by word too, so a misparse costs
fidelity, never privacy. The gate leg `shapes` runs `scripts/shape-corpus-check`
(python3 standard library only): every token of `tests/shapes/shapes.txt` must be in
`tests/shapes/vocabulary.txt`, a placeholder or a single digit. Neither file needs the
tool or the corpus.

## D1031: Vocabulary: fixed lists plus frequent words from public positions
date: 2026-11-06 · by: shapes · issue: 0003
Fixed: shell keywords and builtins, the utility names of research/builtins.md tiers 1–3,
the never-builtins, `test` operators, `set -o`/`shopt` option names, well-known variables, a few path pieces (`dev`,
`null`, `usr`, `bin`, `tmp`, …). From the corpus, only in positions that name public
things: command names, the first argument of tools with their own subcommands (not
`make`, `just`, `pnpm`, `yarn`, `bun`, `npx`: those name project targets and
packages), option tokens matching `^--?[A-Za-z0-9][A-Za-z0-9-]*$` (never the `=value`),
here-doc delimiters, upper-case variables (only without `_` or with a public tool's
prefix). Each needs ≥ 20 distinct commands, ≥ 5 sessions and ≥ 3 projects (cwd folded
to four path components, worktrees into their checkout), so glued values (`-pSECRET`)
and project-private flags stay out. Every change to the file is reviewed by hand.

## D1032: Skeleton rules
date: 2026-11-06 · by: shapes · issue: 0003
Unlisted words become `w1, w2, …`, variables `v1, …`, numbered per command in order of
appearance, equal words sharing a placeholder. A placeholder replaces a run of word
characters joined by `. / @ % + : -`; other punctuation stays (`*.w1`, `{w1,w2}`).
Quoted text is always a placeholder, even a vocabulary word (it is data); only `$`,
`` ` ``, `"` and `\` keep their place inside double quotes, `\` and `'` inside `$'…'`.
An unlisted option keeps its dashes (`-w3`). Here-doc bodies become the line `w0`,
comments `# w0`. Single digits stay (fds, `$1`, `-n 1`); longer numbers do not.

## D1033: Which skeletons are committed
date: 2026-11-06 · by: shapes · issue: 0003
All skeletons seen in ≥ 2 sessions; of the single-session ones only those that add a
syntax 4-gram (punctuation, newlines, keywords, `W` for other words) not yet covered,
shortest first, up to 2 MB, none over 4 KB. Single-session skeletons pass the same
token check as the rest, so they carry no private words; limiting them to new syntax
keeps the file small (2.3 MB of 18 MB) while covering every 4-gram of the corpus, which
is what L0 needs.

## D1120: psh-docs: the guide's rules as a zero-dependency check
date: 2026-11-06 · by: shapes · issue: 0012
`crates/psh-docs` (binary, no dependencies) checks every page of `docs/guide/SUMMARY.md`
against the writing rules of `docs/guide/README.md`; the gate leg `guide` runs it. A
line scanner handles what the rules need (fenced blocks, code spans, inline links and
reference definitions, headings with mdBook's ids) instead of a Markdown crate. Modeled
on djs's `djs-docs check`, without its site building and generated regions. "used to"
counts as history except after is/are/be/been/being/get/gets/got, so "a pipe is used to
connect" passes; 4-digit numbers starting with 0 count as issue numbers even in inline
code (write a file mode as `755`).
## D1070: The Windows legs: Rust legs on the Parallels VM, from a snapshot commit
date: 2026-11-06 · by: windows · issue: 0007
`scripts/windows-gate.sh` (macOS host, bash 3.2) runs the gate's Rust legs on the owner's
Windows 11 arm64 VM (D027): `fmt`, `clippy -D warnings`, `test` (aarch64-pc-windows-msvc,
native), `test-x64` (x86_64-pc-windows-msvc, run under Windows' x64 emulation) and
`msrv` (`cargo check` with the `rust-version` toolchain). Not on Windows: `scripts` and
`script-tests` (bash and perl host tooling, not psh), `deps` and `wasm` (the same on
every host). The VM uses exactly the host's `rustc` version, installed there on first
use with the MSRV, so clippy and rustfmt agree with the macOS legs. Sync: the host
commits the worktree as it is (HEAD plus uncommitted and untracked files, through a
scratch index; no stash) to `refs/windows-gate/NAME`, and the VM `git fetch`es it over
the Parallels share (`\\Mac\Home`) into `C:\psh-gate\NAME`, its own checkout with a warm
`target\`; building on the share itself would be slow and mix both hosts' `target/`.
The VM-side runner, `scripts/windows-gate.ps1` (Windows PowerShell 5.1), enforces each
leg's timeout itself and puts itself into a kill-on-close Job Object, because killing
`prlctl exec` on the host leaves the guest processes running; the host's cleanup kills
the runner, and the job takes cargo, rustc and test processes along.

## D1071: The windows leg is on by default, last; the VM is shared by refcount
date: 2026-11-06 · by: windows (agreed with the coordinator) · issue: 0007
`scripts/gate.sh` runs `windows` (the D1070 legs) as its last leg, on every merge, since
every merge happens on the owner's Mac and optional Windows CI is how oh-my-pi let Windows
regress (D011). `PSH_GATE_WINDOWS=0` skips it when the VM is broken, for the coordinator
and the owner only, never to get green; the gate prints a loud warning and appends the
skip to `work/scratch/windows-gate/skipped.tsv` in the primary checkout. An unreachable
VM fails the leg within bounded time (every `prlctl` call has a timeout; at most 300s of
waiting for the guest). Concurrent gates share the VM: each run holds
`run-NAME.lock` (a refcount with PIDs; dead runs are dropped); a run that wakes a
suspended or stopped VM records that in `resumed`, and the last run out puts it back only
then. A VM that was already running (the owner may be using it) stays running.
## D1040: The AST lives in arenas inside an `Ast` that owns the source
date: 2026-11-06 · by: parser · issue: 0004
`psh_syntax::Ast` holds the source (`Arc<str>`) and three arenas: lists (`ListId`),
commands (`CmdId`) and here-docs (`HereDocId`). Words, redirections, assignments and
`[[ ]]` expressions are owned inline by their command; literal word parts are byte spans
whose meaning is fixed by the part kind (`Lit` verbatim, `Escaped`, `Single`, `AnsiC`), so
parsing copies no text. Reason: the stepped engine keeps `(Arc<Ast>, id)` in its frames and
function table without self-references or clones, the classifier walks from `root()`, and
the tree is `Send + Sync`. Alternatives: an owned `Box` tree (the engine cannot hold
references into it across steps) or `Rc` children everywhere (`!Send`, per-node refcounts).
The incremental `Parser::next_chunk` (one top-level line at a time, as bash runs scripts)
returns one `Ast` per chunk sharing the same `Arc<str>`, with absolute spans.

## D1041: The parser checks what `bash -n` checks, and defers the rest
date: 2026-11-06 · by: parser · issue: 0004
`$(…)`, `<(…)`, `>(…)` and bash 5.3's `${ …; }`/`${| …; }` are parsed recursively at
parse time, as bash does. Deferred to expansion, as in bash: backquote bodies
(`Part::Backtick` keeps the text; `backtick_body` unescapes it for a later parse),
arithmetic (`$((…))`, `((…))`, `for ((…))`, substring offsets and subscripts are words
with expansions, because bash evaluates arithmetic on the expanded text), invalid `${…}`
(`Part::BadSubst`, "bad substitution" at run time) and errors inside unquoted here-doc
bodies (`HereDoc::error`). Reason: L0 agreement with `bash -n`; an arithmetic expression
parser over expanded text belongs with the engine's expansion.

## D1042: Brace expansion is structure; tilde and globs stay text
date: 2026-11-06 · by: parser · issue: 0004
`{a,b}` and `{x..y[..n]}` become `Part::Brace` in the words bash brace-expands (command
words including declaration-builtin arguments, `for`/`select` lists, array elements,
redirection targets other than here-strings), never in assignment values, case words and
patterns, `[[ ]]` operands or here-strings. A leading `~` and glob characters stay in
`Part::Lit` text: the engine knows the context (assignment, `:` separators) and Lit text is
by definition unquoted. Extglob patterns are `Part::ExtGlob` where they change word
boundaries.

## D1043: extglob is a parse option; chunks follow shopt
date: 2026-11-06 · by: parser · issue: 0004
`Options::extglob` (default off, like `bash -n`) makes `?(…)`, `*(…)`, `+(…)`, `@(…)` and
`!(…)` patterns in words and case patterns; the right side of `==`/`!=` in `[[ ]]` always
accepts them, as in bash. The engine parses with `Parser::next_chunk` and passes the
current `shopt -s extglob` state before each chunk, which is how bash behaves line by line.

## D1044: The printer prints words as written
date: 2026-11-06 · by: parser · issue: 0004
`psh_syntax::print` lays out commands one per line with indentation and prints words,
assignments and arithmetic texts verbatim from the source, so quoting, comments inside
`$(…)` and nested here-docs survive. `parse(print(ast))` is checked through the span-free
`dump`, which compares structure and text. It is not bash's `declare -f`/`type` format;
that canonical printer is a follow-up for the engine.

## D1045: A nesting limit instead of a stack overflow
date: 2026-11-06 · by: parser · issue: 0004
Nesting (compound lists plus word nesting such as `"$(…${x:-…}…)"`) deeper than 100 is the
parse error "nesting too deep". 100 fits the 1 MiB default stack of wasm32 in release builds
with room to spare (tested); real scripts stay far below it. Bash has no such limit.

## D1046: What was taken from brush-parser
date: 2026-11-06 · by: parser · issue: 0004
`crates/psh-syntax/src/ast.rs` adapts brush-parser's AST vocabulary (the `[[ ]]` predicate
names, the parameter-expansion operations), attributed in its header and in NOTICE. The
lexer and parser are new: brush's tokenizer produces string tokens and re-parses words
later with winnow, which gives neither spans for word parts nor recursive parses of `$(…)`
at parse time; a single-cursor recursive descent does both and is what bash's own
context-dependent lexing needs.

## D1020: How the conformance cases run
date: 2026-11-06 · by: infra · issue: 0002
Each layer runs its cases the way its upstream runner does, so the scripts mean what
their authors meant: Oils cases get their code on the shell's stdin (`bash` with no
arguments, as `sh_spec.py` does), brush cases get their `args` and `stdin` (without
brush's own `--norc --noprofile` defaults, which change nothing for a non-interactive
bash), our agent cases run as `bash -c CODE`, the way agents' commands run. The
environment is built from scratch per case (nothing inherited): `PATH` = helper
directory + system directories, a fresh working directory and `HOME` per case,
`USER=LOGNAME=spec`, `TZ=UTC`, plus each upstream's variables (Oils: `LC_ALL=C.UTF-8`,
`TMP`, `SH`, `REPO_ROOT` pointing at a per-run copy of the vendored tree; brush:
`LC_ALL=C`, `PS1`). Oils files with `legacy_tmp_dir` get the `_tmp` directory Oils
creates. A case passes on stdout, status and the files it created or changed in its
directory; stderr is its own score (TESTING.md).

## D1021: Golden format, and bash -n verdicts for L0
date: 2026-11-06 · by: infra · issue: 0002
One golden file per source file (`tests/goldens/<layer>/<file>.golden`), a line format:
case header, `status:`, `bash-n:`, then `|`-prefixed blocks for stdout, stderr, created
files and `bash -n`'s stderr; lines with control characters or invalid UTF-8 are
`\`-escaped, a missing final newline is marked. Files over 64 KiB are recorded by size
and FNV-1a hash, at most 200 entries per case; the case's own unchanged files are left
out. Every case also records `bash -n` on its script (L0's oracle), and the shape corpus
gets `tests/goldens/shapes/bash-n.txt`: the skeletons bash rejects, by line, with
status and message. Reason: readable diffs on re-recording, byte-exact comparison.

## D1022: The recording container
date: 2026-11-06 · by: infra · issue: 0002
`docker/Dockerfile`: `debian:stable` pinned by digest (13.x), GNU userland plus
`gawk`, `procps` and `locales` (`en_US.UTF-8`), bash 5.3 from the GNU tarball with the
official patches through 5.3.9, each pinned by sha256, run as user `spec` (uid 1000),
umask 022, hostname `psh-oracle`, `--network none`. 5.3.9 matches the Homebrew bash
used for local probes, so a probe differs from the goldens by platform only; moving to
a later patch level is a change of the Dockerfile plus a full re-recording in one
commit. psh-spec is built for Linux in `rust:1.96-trixie` pinned by digest. The image
is tagged by the Dockerfile's hash; it is rebuilt from pinned inputs rather than pinned
itself.

## D1023: Oils' Python helpers are built into psh-spec
date: 2026-11-06 · by: infra · issue: 0002
`argv.py`, `printenv.py`, `read_from_fd.py`, `show_fd_table.py` and `stdout_stderr.py`
(Python 2 programs in Oils' `spec/bin`) are reimplemented in psh-spec with Python 2's
output (`repr` of byte strings, stderr written before buffered stdout); the run's helper
directory and its copy of the Oils tree link those names to the psh-spec binary, which
dispatches on `argv[0]`. Reason: no Python in the container, on Windows or in the gate,
and identical helper behavior when recording and testing. `read_from_fd.py` reaches the
fd through `/dev/fd/N`, because std offers no safe way to adopt a raw fd.

## D1024: The normalization table
date: 2026-11-06 · by: infra · issue: 0002
Applied to recorded and tested output alike: per-run paths become `<case>`, `<home>`,
`<repo>`, `<bin>`, `<run>`, `<sh>` (in every spelling, canonical included), and the
host name (full and short, if at least 4 characters) becomes `<host>`. At comparison
only, the shell's own name at the start of a message line (`bash: ` / `psh: `) or alone
on a line becomes `<sh>`. Nothing else is normalized; a case that cannot be made
deterministic goes into a group instead.

## D1025: Vendored subsets
date: 2026-11-06 · by: infra · issue: 0002
Oils at `d114978`: the 134 files of Oils' own SUITE-compat set (OSH spec files minus
`strict-options`) plus `spec/bin`, `spec/testdata` and `LICENSE.txt`; the YSH and other
non-bash files are left out. brush at `9435534`: all of
`brush-shell/tests/cases/` plus `brush-shell/tests/utils/` (a case sources from it) and
`LICENSE`. Both unmodified, reproduced and verified by `scripts/fetch-oils` /
`scripts/fetch-brush [--check]` from clones in the primary checkout's `vendor/`.

## D1026: Gate legs are files in scripts/gate.d
date: 2026-11-06 · by: infra · issue: 0002
Supersedes the leg list inside `scripts/gate.sh` (D1013). Each leg is
`scripts/gate.d/NN-name.sh`, run in `NN` order (bash, `set -euo pipefail`, repository
root) under `scripts/timeout.sh` with the `# timeout: SECONDS` of its first line and an
optional `# grace: SECONDS`. Adding a leg is adding a file, so issues that add legs in
parallel never conflict in `gate.sh`. `99-windows` stays last. The legs of 0003, 0012
and 0007 moved into files unchanged.

## D1027: Ratchets apply to psh only
date: 2026-11-06 · by: infra · issue: 0002
`tests/<layer>/expected-failures.txt` lists active case ids (`layer/file/index`) that
may fail; psh-spec checks it only when the engine is psh (in-process, or a binary named
`psh`). Runs of another shell, such as the bash self-check, are compared and
summarized only. `--add-failures` exists for newly added cases; regressions are never
added. A case "passes" on stdout, status and files, so a few cases already pass with
the stub engine (bash prints nothing and exits 2 on a syntax error); they stay off the
ratchet and must keep passing.

## D1028: Hit list weight: the rarest construct of a case
date: 2026-11-06 · by: infra · issue: 0002
`scripts/status --hitlist` weighs each failing case by the corpus frequency of the
rarest construct in its script (corpus `features` and `top_names`); constructs the
corpus did not measure, builtins outside its top names and shell invocation arguments
count as rare (weight 1), a case with none of them weighs the corpus total. Areas rank by
the summed weight of their failing cases. Reason: a spec case usually tests its most
exotic part, so the minimum, not the sum, says how much agents depend on it. The
constructs are detected lexically until psh-syntax can classify scripts; then the
detector should move onto the parser (follow-up).
## D1047: `psh --parse` JSON shape
date: 2026-11-06 · by: parser · issue: 0004
`psh --parse [--compact] [-O extglob] [-c CMD | FILE]` prints `{"ok": true, "script":
LIST}` or `{"ok": false, "error": {message, span, line, column, incomplete}}`; status 0 or
2. Every node is an object with `type` and `span` (`[start, end]` byte offsets into the
UTF-8 source); words carry `text` (as written), `value` (after quote removal when there is
no expansion, else `null`), `quoted` and typed `parts`; substitutions carry their parsed
`body`; backquote bodies are parsed for the JSON (spans relative to their unescaped
`text`). The full shape is documented at the top of `crates/psh/src/parse_json.rs`. The
JSON is built in the `psh` crate on deser-json (D029; `default-features = false`, `std`),
so psh-syntax stays dependency-free. Reason: the classifier (0009) and pi's UI need
kinds, spans, quoting and nested commands without re-deriving them from source text.
## D1090: Per-utility effect tables live in psh-classify
date: 2026-11-06 · by: shapes · issue: 0009
`psh_classify::utils::UTILITIES` (no_std data) says per utility: its category, the short
and long options that take a value, what its operands are (read, written, script then
files, ...) and which options change the effect (`sed -i` writes its operands, `sort -o`
writes its value, `tail -f` runs until stopped, `rg --pre` runs code). The utilities
(`psh-utils`) depend on `psh-classify` for it, which costs nothing (no_std, psh-syntax
only), and each in-process utility's tests check its option parser against its entry,
so the classifier and the implementation cannot disagree on which flags write.
Utilities with subcommands or nested commands (git, package managers, `find`, `xargs`,
shells, interpreters, wrappers) have rules in code (`rules.rs`). Resolves ARCH open
question 2. Alternative: a third crate for the tables; not worth it while psh-classify
is this small.

## D1091: Categories, priority and read-only
date: 2026-11-06 · by: shapes · issue: 0009
A command (and a script) has a set of categories: read, network, build, long-running,
mutate, vcs, install, unknown. The primary one for a summary is the highest in that
order (unknown highest): a build that writes a file is shown as mutate, anything with an
inline interpreter as unknown. `read_only()` holds when the set is just {read}, nothing
is written (writes to `/dev/null` and the standard streams do not count) and the script
parsed. Unlike research/corpus/analyze.py, process inspection (`ps`, `lsof`, `sleep`)
is read, `kill` is mutate, and inline interpreters and local scripts are unknown rather
than categories of their own.

## D1092: What makes a command unknown
date: 2026-11-06 · by: shapes · issue: 0009
Besides dynamic command names, `eval`/`bash -c` of non-literals, `source`, inline
interpreters, local programs and unknown programs: calls of functions or aliases defined
anywhere in the script; assignments (prefix, `export`, `env`, `${X:=}`) to variables that
change what runs (`PATH`, `LD_*`, `DYLD_*`, `BASH_ENV`, `GIT_SSH_COMMAND`, `PAGER`, ...);
a dynamic word that could be an option (it starts with an expansion) of a utility with
an option that writes or runs code (`sed "$x"`, `rg "$x"`, `sort "$x"`, any dynamic word
of `find`), while `grep "$x"`, `cat "$f"` and `src/$f` stay read; `git -c` and unknown git
subcommands (aliases); shells with startup files (`-i`, `-l`); shell code that `xargs` or
`find -exec` splices file names into; sed scripts with `e` or that do not parse; awk
programs with `system`, pipes or print redirections; `sudo`. Reason: the classifier is
used to auto-approve; argument injection (`rg "$(cat f)"` becoming `--pre=…`) and
definitions it does not follow must not pass as read.

## D1093: Literal shell code in strings is classified as nested scripts
date: 2026-11-06 · by: shapes · issue: 0009
Literal strings that a command runs as shell code are parsed with psh-syntax and walked
like the script: `bash -c`/`sh -c`, `eval` (its arguments joined), `trap` handlers, alias
bodies, backquote bodies, and a literal here-document or here-string fed to a shell
(`bash <<'EOF'`). Their commands carry `source: Some(i)`, an index into
`Classification::sources`, and spans relative to that text. Nesting deeper than 8 is
unknown. Wrapped commands (`env`, `timeout`, `nice`, `nohup`, `command`, `exec`, `sudo`,
`xargs`, `find -exec`, `uv run`, ...) are commands of their own with the wrapper as
parent.

## D1220: L0 lives in psh-spec, on psh-syntax as a library
date: 2026-11-07 · by: infra · issue: 0022
`psh-spec --l0` parses every L1–L3 case script, every shape skeleton and every snippet of
`tests/l0/syntax.txt` with psh-syntax and compares against the container's `bash -n`
verdicts (case goldens; `tests/goldens/{shapes,l0}/bash-n.txt`, whose headers carry the
source's hash so stale verdicts are refused); a parsed input must also survive print →
reparse with the same `dump`. A parser panic is a failure. Its ratchet is
`tests/l0/expected-failures.txt` and its gate leg `92-l0`. psh-spec now depends on
psh-syntax (a workspace crate, no new external dependency); the `deps` leg allows it.
`tests/l0/syntax.txt`'s inline `## status:` lines stay the parser's quick check; the
container verdicts are the oracle (they agree on all 196 snippets today).
## D1230: Arithmetic: parse expanded text, evaluate over an environment trait
date: 2026-11-07 · by: parser · issue: 0023
`psh_syntax::parse_arith(&[u8])` parses the text of `$(( ))`, `(( ))`, `for (( ))`,
subscripts and `let` after expansion into an `ArithExpr` tree with byte spans (numbers
decoded, variables left as `LValue { name, index }` with the raw subscript text).
`eval_arith`/`eval_arith_expr` evaluate it over `trait ArithEnv { type Error; is_assoc,
get, set }`: the engine keeps variable semantics (indexed vs associative subscripts,
negative indexes, `set -u`, readonly), the evaluator does bash's math (64-bit wrapping,
shifts mod 64, short-circuit `&&` `||` `?:`, `x op= v` reading x first) and evaluates
variable values as arithmetic in turn, up to bash's 1024 levels. Errors render bash's
exact text (`EXPR: message (error token is "…")`) through `ArithError::bash_text`; the
caller adds the context (`bash: line N: `, `let: `, `((: `). Agreed with the engine
agent (0005). Checked against bash 5.3 on 18k random expressions (syntax errors and
values, side effects and runtime errors).

## D1231: Where arithmetic differs from bash's parse-while-evaluating
date: 2026-11-07 · by: parser · issue: 0023
Bash evaluates while it parses; psh parses the whole text first. So a runtime error
before a syntax error in the same text (`1/0 +`, `x++ + )`) is reported as the syntax
error, and side effects before it do not happen. Bash's look-ahead is reproduced where it
changes messages: after a name the next token is read (so `x y 08` is a number error),
error tokens follow bash (the divisor's start for `/` `%`, the token after the exponent for
`**`). The evaluator's own nesting is capped at 2048 (`x=x` takes one per level), so
values nested deep inside nested values fail with "expression recursion level exceeded"
earlier than bash would, instead of overflowing a 1 MiB stack.

## D1221: Gate cache keyed by declared leg inputs
date: 2026-11-07 · by: infra · issue: 0022
Each `scripts/gate.d` leg may declare `# inputs: PATHSPEC...`. Before running it, the gate
hashes the leg file's name, the toolchains (`rustc -Vv`, `cargo -V`, `rustup toolchain
list`) and `git ls-files -s` of the inputs, the leg file, `gate.sh` and `timeout.sh`; a
pass is recorded as a file under the primary checkout's `work/scratch/gate-cache/`
(shared by all worktrees), and a later run with the same key skips the leg. No key, so no
skip and no record, when an input has uncommitted or untracked changes or the leg has no
`# inputs:`. A leg that did not really check anything touches `$PSH_GATE_NOCACHE` (the
Windows leg when skipped with `PSH_GATE_WINDOWS=0`). `--full` ignores the cache. Rust legs
declare `crates/ Cargo.toml Cargo.lock .cargo/` and `tests/` where tests or runs read it;
docs, work and research files are nobody's input, so they gate in seconds. Alternatives:
djs's per-leg input classes in a Python gate (more machinery than we need now).
## D1240: Literal variable tracking in the classifier
date: 2026-11-06 · by: shapes · issue: 0024
A variable's value is substituted where it is used when (1) every write to it anywhere in
the script (assignments, declaration builtins, `read`, `mapfile`, `getopts`, `printf -v`,
`for`, `unset`, arithmetic, `${v:=…}`, `+=`, arrays) is the same literal without `~`,
and (2) at the use, such an assignment has definitely run in the current shell: it is an
earlier item of an enclosing list (as the item's first pipeline, single-stage, not in the
background) or an earlier step of the same `&&` chain. Subshells, `{ …; }` groups, branches
and loops end the scope of their assignments. `eval`, `source`, `trap`, `alias`,
`enable`, namerefs, dynamic command names or variable names, or a backquote body that does
not parse turn tracking off for the script. Shell variables (PWD, RANDOM, IFS, ...) and the
variables of D1092 are never tracked. Unquoted values with blanks or glob characters are
not substituted (splitting). `bash -c` strings, trap handlers and alias bodies start with
nothing known; backquote bodies see the enclosing values. Flow-insensitive writes plus a
definite-assignment check are simple and conservative; a full data-flow analysis would buy
little (about 230 more read-only commands in the corpus came from this).

## D1241: sqlite3 is classified from its literal SQL
date: 2026-11-06 · by: shapes · issue: 0024
`sqlite3 [OPTIONS] FILE SQL...` is read when every SQL argument (and `-cmd` value) is
SELECT/VALUES/WITH/EXPLAIN without writing keywords, a known read pragma without a value,
or a dot-command that only shows things (`.schema`, `.tables`, `.mode`, ...); mutate when
it writes the database; unknown with no SQL arguments (SQL on stdin), `-init`, the archive
mode, ATTACH/DETACH, `load_extension`, `writefile`, `edit`, `VACUUM INTO` or any other
dot-command (`.shell`, `.system`, `.output`, `.import`, ...). `-readonly` alone does not
make it read, since dot-commands can still run programs.

## D1242: `$HOME` and `$PWD` are paths, not options
date: 2026-11-06 · by: shapes · issue: 0024
A word that starts with `$HOME`, `${HOME}`, `$PWD` or `${PWD}` (quoted or not) cannot be
an option for the dynamic-option rule of D1092: both are absolute paths in practice (PWD
always, HOME by every platform's convention), so `rg x "$HOME/.config"` is read. Their
values are still not substituted.
