# Decisions

An append-only log of decisions. Newest at the bottom. To revise a decision, add a new
entry that supersedes it and say so; never edit an old one.

IDs: the coordinator uses D001–D999. An issue NNNN uses the ten IDs from
D(1000 + 10·NNNN) to D(1009 + 10·NNNN), so 0004 uses D1040–D1049 and 0012 uses
D1120–D1129; its header names the range. Parallel branches never collide.

When an issue runs out of its ten IDs, the coordinator hands out an extension block of
ten from D1900 upward (D1900–D1909, D1910–D1919, ...; e.g. 0006 has D1900–D1909) and
names it in the issue header next to the default range. Ask the coordinator before you
need one. The default ranges of issues 0090 and later would overlap the extension
blocks, so those issues get their range from the coordinator explicitly.

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
## D1050: The interpreter is async fns polled by the engine's own scheduler
date: 2026-11-07 · by: engine · issue: 0005
The interpreter is a tree walker written as Rust `async fn`s over the arena AST; the
engine owns a ~300-line scheduler (no crate, no threads) that polls vprocs and helper
tasks as boxed futures, each with its own `std::task::Waker`; a wake queues only that
task and wakes the embedder's waker. Reason: expansion must wait in the middle of a word
(`"${x:=a}$(cmd)"`, side effects in order) and every compound command anywhere; frame
machines written by hand would be several times the code and fragile, the compiler
generates them. It keeps D005's substance (no runtime crate, no engine threads, Waker
only, the embedder steps and parks); coordinator-approved. Conditions, all designed in:
(1) nesting within a task uses native stack (measured ~5 KiB/level release, ~15 KiB
debug, `examples/stackprobe.rs`): `ShellConfig::max_depth` (default 1000) fails with
"maximum nesting level exceeded", embedders run the engine with
`std_host::ENGINE_STACK` (20 MiB), wasm32 embedders use ~150 or a larger stack;
(2) cancel = dropping tasks, so every host resource is released by `Drop` (externals
abandoned, files and pipes closed), tested with the test host's live counters;
(3) per-task wake queue, so unrelated vprocs are not polled; measured ~1 µs per builtin
command in release (100k `true`s in 0.08 s with parsing); (4) `Shell` is `!Send`
(explicit `PhantomData<*const ()>`). Alternatives: hand-written frame machines
(oh-my-pi's IR VM), threads per vproc (ruled out by D005).

## D1051: Step API and control
date: 2026-11-07 · by: engine · issue: 0005
`Shell::step(budget) -> Step::{Done(status), Blocked, Yield, Idle}`; the budget counts
safepoints (one before each command; loops will add one per iteration). `Done` is
reported once. A `ControlHandle` (`Send + Sync`) sets cancel or pause bits; safepoints
and the scheduler check them. Cancel drops all tasks (the virtual TERM's default action
until traps exist) and ends the run with 143; `start` clears the bits, so a late cancel
never reaches the next run; the shell continues from its state before the cancelled run.
Pause makes `step` return `Blocked` until `resume` wakes the waker. When the main vproc
finishes, detached tasks get one more round, then are dropped with their externals
released rather than killed (bash leaves background jobs running).

## D1052: Host trait v1
date: 2026-11-07 · by: engine · issue: 0005
`psh_engine::host::Host`: `open`, `stat`, `access`, `read_dir`, `canonicalize`,
`resolve` return `HostFuture<T>` (a boxed std `Future`; a host answers at once with
`ready`, the test host's latency mode is `Pending` once); `spawn` is synchronous and
returns an `ExtProc` (`poll_wait`, `signal`, `release`; drop = abandon) plus the engine's
ends of `ChildFd::Pipe`s; `sleep`, `pid`. Files are `HostFile` (`poll_read`,
`poll_write`, `is_terminal`, `Any` for downcasting by their own host). Paths are bytes,
absolute and `/`-separated. Expected additions: for 0006, process groups and signals to
groups, extra child fds (>2), umask, FIFO opens off-thread, temp files, a real `access`;
for the utilities (0008), mkdir/remove/rename/symlink/metadata setters and a way to run
a utility body on a worker thread polled like an `ExtProc`; for pi-env, an exec-time
approval hook in `resolve`/`spawn`.

## D1053: Shell values are bytes
date: 2026-11-07 · by: engine · issue: 0005
Variable values, fields, argv, env and paths are `Vec<u8>` in the engine (names of shell
variables are `String`, always ASCII). Char-wise operations decode UTF-8 when the locale
variables say UTF-8 (`LC_ALL`, then `LC_CTYPE`, then `LANG`), else work on bytes; bytes
that are not valid UTF-8 count one by one. Reason: command output and `$'\xff'` are
arbitrary bytes, and bash is byte-exact. Scripts themselves are parsed as UTF-8 (lossy).

## D1054: Engine pipes, pumps and here-documents
date: 2026-11-07 · by: engine · issue: 0005
Pipes between vprocs are in-memory (64 KiB, as Linux); writing without readers ends the
vproc with status 141. Externals get host pipes for descriptors that are engine
objects; the pumps are state machines polled inside the wait for the external (not
tasks), so dropping the wait drops them: input pumps stop when the external exits,
output pumps run to EOF before the command completes (pitfall 7; bounded draining for
grandchildren holding the pipe is 0006). Descriptors sharing one engine object share
one host pipe (`ChildFd::Dup`). Here-documents and here-strings are pipes preloaded
with the expanded body and no writer: no body is written into a kernel pipe before its
consumer starts (pitfall 2), and an external gets it through an ordinary pump.

## D1055: /dev/stdin, /dev/stdout, /dev/stderr and /dev/fd/N are emulated
date: 2026-11-07 · by: engine · issue: 0005
In redirections these names always duplicate the vproc's own descriptors, on every
platform. Reason: embedded (pi-env, psh-spec in-process) the process's descriptors are
not the shell's, and Windows has no such files. bash does the same where the OS lacks
them.

## D1056: psh-spec runs psh in-process, with two exceptions
date: 2026-11-07 · by: engine · issue: 0005
Each case gets a fresh shell on its own engine thread (`ENGINE_STACK`) over the std
host, with OS pipes for stdout and stderr (externals write to them directly) and the
case's cwd and environment. `$$` is a fake pid (99999999) so `kill … $$` cannot hit the
runner. Cases whose arguments or stdin mention `PPID` or `kill` run in the `psh` binary
as a subprocess in its own process group instead: their children signal their parent
or their process group (`kill -USR1 0`), which in-process is the runner.
The command line (`-c`, `-s`, file, stdin) is parsed by `psh_engine::invocation`, shared
with the `psh` binary.

## D1057: Unimplemented features fail loudly
date: 2026-11-07 · by: engine · issue: 0005
A construct the engine does not run yet (`if`, functions, arrays, arithmetic, `set -e`,
...) prints `NAME: line N: X: not supported yet` and fails with status 2; inside an
expansion it ends the (sub)shell like other expansion errors. Never a silent
different behavior; the messages double as a feature histogram for the hit list.
## D1232: A second printer in bash's `declare -f` layout
date: 2026-11-07 · by: parser · issue: 0023
`psh_syntax::print_function_bash(ast, id)` prints a function definition exactly as bash
5.3's `declare -f` does (`type` adds `NAME is a function` before it); `print` (D1044) stays
the round-trip printer. The layout rules were derived only from bash's output, by
differential testing (`work/scratch/0023/fncompare.sh`): every one of the 1027 functions
in the Oils and brush case scripts, and 99% of ~2000 randomly generated functions,
print identically. The remaining differences are in bash's handling of here-documents
inside `$((…) )` text that is not arithmetic. To reproduce bash, the AST records whether
a list item ended at a newline (`AndOr::newline`), which bash keeps inside substitutions,
and `decode_ansi_c` decodes `$'…'` (bash prints it single-quoted). `tests/l0/declare-f.txt`
holds 206 cases with bash's output.

## D1233: Sources are bytes; the AST has a UTF-8 view of them
date: 2026-11-07 · by: parser · issue: 0023
The parser works on bytes (`parse_bytes`, `Parser::from_bytes`), so scripts that are not
UTF-8 parse byte-exactly and spans are offsets into the input as given. Words keep no
text of their own (D1040), so nothing in the AST needs to change for bytes:
`Ast::bytes(span)` gives the exact bytes. For the `&str` APIs (`Ast::text`, `print`,
`print_function_bash`, `dump`, `literal_text`) the source has a UTF-8 view (`Source`):
the input itself when it is UTF-8 (no copy, no cost); otherwise each byte that is not part
of a valid UTF-8 sequence appears as the private-use char U+F700 + byte, and
`Ast::restore_bytes` maps view text back to bytes. An undecodable byte is a char of its
own, so spans never split a char of the view and no valid char is swallowed. Rejected:
lossy U+FFFD (not reversible, shifts offsets), bytes in every API (churn for the engine
and classifier, whose words are text for UTF-8 scripts anyway). `parse(&str)`,
`Parser::new`, `Ast::source() -> &str` and `LineIndex::new(&str)` keep working;
`Ast::source_bytes`, `LineIndex::from_bytes` and `line_col_bytes` are the byte forms.
Collision: a script that is not UTF-8 and also contains real U+F780..U+F7FF chars would
restore those chars as bytes; such scripts are not expected.
## D1060: The Unix host places every external in a session of its own
date: 2026-11-07 · by: procs · issue: 0006
`psh_os::unix::UnixHost` spawns with `posix_spawn` (macOS; glibc with
`addchdir_np`, looked up with `dlsym` so glibc < 2.29 still loads) and `POSIX_SPAWN_SETSID`,
`SETSIGDEF` over every catchable signal (undoing Rust's ignored SIGPIPE), an empty mask and
`CLOEXEC_DEFAULT` on macOS; elsewhere (musl, Android, old glibc) a bare `fork` + `execve`
with async-signal-safe setup and a close-on-exec error pipe. Placement `Session` is the
default (embedding, psh-spec, L5): a cancel or `signal` reaches the whole tree and no
external touches the terminal's foreground. The `psh` binary uses `Inherit` (externals in
psh's own group, signals narrow to the pid) when its stdin is a terminal whose foreground
group is psh's, so a person's terminal programs and Ctrl-C work as under bash. Alternatives:
`setpgid` groups in psh's session (background groups stop on terminal reads), std
`Command` (no setsid + signal reset + exact errno without `pre_exec`).

## D1061: One readiness thread instead of pump helper threads
date: 2026-11-07 · by: procs · issue: 0006
Supersedes the "bounded helper thread per pump" v1 of ARCH.md for Unix (coordinator-approved).
The engine's pumps (D1054) poll host files; the Unix host makes those files never wait in
the kernel: our private pipe ends to externals are `O_NONBLOCK` (only we hold them), and
`EAGAIN` registers the task's waker with one process-wide readiness thread (`poll`;
`select` on macOS, where `poll`/kqueue miss a FIFO's last writer closing and `poll` gives
`POLLNVAL` for terminals; descriptors ≥ `FD_SETSIZE` rechecked by `poll` every 20 ms).
Shared or inherited descriptors (the shell's stdio, files from redirections, FIFOs) keep
their flags (pitfall 5): a zero-timeout readiness check comes before every read or write,
and a write to such a stream carries at most `PIPE_BUF` bytes, which a writable pipe takes
without waiting. Each pump keeps its own bounded buffer (16 KiB, in the engine) and
returns `Pending` alone, so a wedged consumer stalls only its own pump. Several tasks may
wait on one descriptor (all are woken). Reasons: no thread per pump, no read-ahead that
could swallow bytes meant for a later reader of a shared stdin, and the same mechanism
covers pitfall 1 for every stream, not only pumps. Remaining risk, accepted: a second
reader of a shared pipe in another process can take the data between the check and the
read, which then waits.

## D1062: Exit observation, reaping and abandonment
date: 2026-11-07 · by: procs · issue: 0006
Pooled waiter threads (up to 8 idle, 64 KiB stacks) block in `waitid(WEXITED | WNOWAIT)`
per external; macOS stop reports are consumed with a non-reaping `waitid`. A process is
never reaped while its `ExtProc` exists (signals cannot hit a recycled id). Reaping is
owned by exactly one party under the lifecycle lock (`psh_os::lifecycle`, platform-neutral
for 0007): the `ExtProc`'s drop after a collected exit; the waiter for a released process;
the escalation thread for an abandoned one. Abandon: TERM and CONT to the group at once,
then after the grace period (default 2 s) KILL to the group, also when the leader exited
meanwhile (its unreaped zombie pins the group id), then reap. Released externals are not
leaks. `UnixHost::leftovers()` lists the groups it created that still have a member: the
leak check of L5 tests and later of psh-spec.

## D1063: Files without `#!` run in /bin/sh for now
date: 2026-11-07 · by: procs · issue: 0006
When `execve` fails with `ENOEXEC`, the Unix host retries with `/bin/sh FILE ARGS…`, as
`execvp` (and so std's `Command`, the std host) does; the cases that passed on the std
host keep passing. bash runs such files in a copy of itself; the engine should do that
(0005 follow-up), and then this fallback goes.

## D1064: L5 process tests run in-process with a leak check
date: 2026-11-07 · by: procs · issue: 0006
`tests/process/` is an integration test of `psh-os` (`cargo test -p psh-os --test
process`): each test runs a script on an engine thread over `UnixHost` (grace 300 ms) with
OS pipes for stdout and stderr read to their end, under a 30 s deadline, then waits for
`leftovers()` to be empty and fails if a group survives. A test of the check itself
leaves a grandchild on purpose. Tests that need the `psh` binary (standalone signals) come
with it.
## D1072: Windows runs take turns in one shared checkout; XtaCache holds are waited out
date: 2026-11-07 · by: windows · issue: 0007
Supersedes D1071's refcount and D1070's checkout per worktree. Concurrent gates on the
4-vCPU VM starved each other (a run took 40 minutes, emulated x64 test processes missed
their deadlines), and one run's suspend could hit another between states. Now one run
uses the VM at a time (`run.lock` in the primary checkout's
`work/scratch/windows-gate/`), the others wait at most `PSH_WINDOWS_QUEUE_WAIT` seconds
(default 3600) with a message naming the holder; the wait does not count against the
legs' timeouts (99-windows: 7500s, merge-to-main's gate default 10800s). The lock is held
from the first look at the VM until it is asleep again, and a run puts it to sleep only
if no run is waiting and a run woke it; transitional states (suspending, resuming, ...)
are waited out, and prlctl's own failure (255) retries the run once. All runs share
`C:\psh-gate\shared` and its `target\`, so builds are incremental across worktrees.
After every leg the runner kills and waits for all other processes of its Job Object
(JobObjectBasicProcessIdList); one running an executable of the checkout fails the leg
as leaked by a test. LNK1104 on test-x64 came from Windows on arm64's XtaCache service
(xtac64se.exe), which opens x64 executables that ran under emulation to build its
translation cache and holds them, under load for minutes (found with the Restart
Manager, which the runner now uses): before a test leg the runner waits up to 300s for
the checkout's executables to be released, naming the holders, and a leg that failed
with LNK1104 or "os error 5" runs once more. Not done: a cargo runner that runs copies of
the test executables, because `psh.exe`, which the tests spawn through
`CARGO_BIN_EXE_psh`, would still be held; disabling XtaCache in the VM needs the owner.

## D1222: The hit list reads constructs from psh-syntax's tree
date: 2026-11-07 · by: infra · issue: 0022
Supersedes the lexical detection of D1028 where the parser accepts a case's script:
`psh-spec`'s `ast_features` walks the tree (lists, pipelines, compound commands,
redirections and here-docs, word parts, parameter operators, `set` and declaration
arguments) and yields the corpus feature names plus literal command names; separators,
comments and line continuations, which the tree does not keep, still come from the text.
Scripts psh-syntax rejects (about 1% of the failing cases, all also rejected by bash) keep
the lexical detection. The weighting rule (the rarest construct) is unchanged.

## D1223: A workspace-wide dependency allowlist
date: 2026-11-07 · by: infra · issue: 0022
Supersedes the per-crate lists in the `deps` leg (D1013). `scripts/deps-allowlist.txt`
lists every (crate, dependency) pair of the workspace: normal, build and dev
dependencies, direct and transitive, for every target (`cargo tree --target all`), each
with the decision that allows it. The gate fails on a pair missing from the list, a
listed pair the workspace does not have, or a line without a decision id. Adding a
dependency is: decision, then the list line(s) for it and everything it pulls in, then
the code. Workspace crates depending on each other cite ARCH.md's crate layout (D004)
or their own decision.
## D1058: Errors that abandon the line, and errors that end the shell
date: 2026-11-07 · by: engine · issue: 0005
`Unwind::Abort(status)` abandons the rest of the top-level command line (also from
inside functions and loops), sets `$?` and goes on with the next line; in a subshell it
ends the subshell. Used where bash does that (probed with 5.3): bad substitution,
arithmetic errors in `$((…))`, invalid indirection, assignment to a readonly variable,
the function nesting limit. `Unwind::Exit` ends the (sub)shell: `exit`, `${x?}` (127
under `-c`, else 1, as bash), SIGPIPE (141), usage errors of `break`/`continue`, and
features that are not supported yet (status 2). Keeping unsupported features fatal
costs about 100 passing cases (they pass when the script goes on) but is the safe
choice for agents: a script whose `set -e` was not understood must not run on.
Supersedes the expansion part of D1057.

## D1059: Functions, locals and loops; stack per level
date: 2026-11-07 · by: engine · issue: 0005
Functions are `(Rc<Code>, CmdId)` in an `Rc` map in `Vp`, found before builtins (bash
outside POSIX mode). A call saves `$@`, the loop count and pushes a `Frame` recording
the previous value of each variable made local (`local`, prefix assignments) and
restores them on return: dynamic scoping over one variable table, cheap to clone for
subshells. `break`/`continue` count loops per function body; `continue` in a loop's
condition ends the loop (bash). Stack: after splitting the dispatch into plain
functions returning boxed futures (no `async` frame for the dispatcher) and a fast path
for single-command list items, a function call level costs ~3 KiB in release and
~15 KiB in debug (was ~13/52 KiB); 1000 levels run in 3 MiB (release) and 16 MiB
(debug). `STACK_PER_LEVEL` stays 16 KiB, `ENGINE_STACK` ~20 MiB; the limit counts
function calls and subshells together.
## D1073: A Defender exclusion for the VM's build directory; test-x64 stays whole
date: 2026-11-07 · by: windows (asked by the coordinator) · issue: 0007
The cli tests' 10s deadlines failed under x64 emulation only while 3-4 gates built at
once on the 4-vCPU VM, before D1072. Measured on the VM without other runs: an x64
`psh.exe -c exit` starts in 30-60ms, the first run of a freshly linked one in 0.5-0.8s
(scan and translation); the x64 cli tests take 0.4s, and 4.7s with twelve busy processes
competing for the cores. So emulation is not inherently too slow and test-x64 keeps all
the tests, serialized like every leg (D1072). Defender's real-time scan of the build
output was the remaining cost: `C:\psh-gate` is excluded from it (`Add-MpPreference
-ExclusionPath C:\psh-gate`, as SYSTEM through `prlctl exec`), which halved an x64
relink-and-test (23s to 12s) and the first start of a fresh x64 exe (to 80-150ms). Only
gate checkouts live there; nothing downloaded or foreign runs from it.
## D1250: Aliases expand in the chunked parser, over a copy of the alias text
date: 2026-11-07 · by: parser · issue: 0025
Bash expands aliases while it reads, so `Parser::set_aliases(Option<&Aliases>)` gives
the chunked parser the shell's table before each chunk (`None` when `expand_aliases` is
off); `parse`/`parse_with` never expand, like `bash -n`. Where a command word may stand
(also after `!`, `time`, assignments, redirections, and before reserved words are
recognized), an unquoted word naming an alias that is not already being expanded is
replaced by reading the alias text; a text ending in a blank makes the next word a
candidate. A chunk that expands aliases is parsed twice: once to learn which aliases it
uses, then over a buffer of the script followed by one copy of the text per use, so
every node from an alias has its own span (`span.start >= Ast::script_len()`), spans
that run from alias text into the script are widened to the alias word, and
`Ast::original_span` maps any span back onto the script (line numbers). Chunks without
aliases are untouched. Alternatives: textual pre-expansion of the line (wrong: bash
decides command positions while parsing) and a token stream with pushed-back text (no
token stream here). Known gap: a quote opened in alias text does not continue into the
script, as it does in bash.

## D1224: Serial cases hold a machine-wide lock
date: 2026-11-07 · by: infra · issue: 0022
Cases that use fixed paths outside their own directory race when two run at the same
time. `tests/spec.toml` marks them with `[[serial]]` tables (match/tag, reason; not a
group), and psh-spec runs each such case while holding an exclusive `File::lock` on
`$TMPDIR/psh-spec-serial.lock`: serial cases never overlap, neither between workers nor
between concurrent psh-spec runs (gates of other worktrees, recordings). Other cases stay
parallel. Alternatives: a per-worker `/tmp` (needs OS isolation; not portable), or running
serial cases in a separate phase (does not cover other processes).
## D1065: An abandoned group is killed when its leader exits
date: 2026-11-07 · by: procs · issue: 0006
Supersedes D1062 in one point. After TERM, the escalation sends KILL to the group as
soon as the leader exits or when the grace period ends, whichever comes first, then
reaps. Waiting out the full grace period after the leader is gone made every cancel of
an obedient pipeline cost the whole grace (and a standalone psh waits for its
escalations before exiting); whatever is still in the group had TERM already and would
be an orphan. Measured why there is no cheaper "is only our zombie left?" check: with
only an unreaped leader, `kill(-pgid, 0)` succeeds on Linux and fails with EPERM on
macOS.

## D1066: The standalone psh cancels on INT, TERM and HUP and dies by the signal
date: 2026-11-07 · by: procs · issue: 0006
`psh_os::unix::signals::catch` installs handlers (atomic bit + self-pipe byte, nothing
else) for the signals not ignored at start; a watcher thread cancels the run through the
`ControlHandle`. The driver then waits (`psh_os::unix::settle`, bounded by the grace
period plus 1 s) until abandoned externals are killed and reaped, resets the signal and
raises it, so the parent sees psh die by it as with bash. bash differs in waiting for a
foreground child to react to INT itself; psh tears the run down, which is what an agent
harness sending TERM wants. Signals ignored when psh starts stay ignored, in psh (no
handler) and in its children (the host passes them on; SIGPIPE and SIGCHLD excepted,
since Rust ignores SIGPIPE before `main`). Embedded hosts install nothing.

## D1067: Who survives a run: only what was explicitly detached
date: 2026-11-07 · by: procs · issue: 0006
Supersedes D1060's placement of the standalone psh. Problem (0005 found it): released
background jobs plus a session per external were orphans no group kill reached; the gates
left dozens of TERM-ignoring loops behind, the corpus shows agents' servers alive for days.
- **Embedded (pi-env, psh-spec, L5): a host is one run.** Each external still leads a
  session of its own (cancel of one command never hits its siblings, no terminal), and
  the host records every session it made. Dropping the host ends the run: with
  `Leftovers::Kill` (the default) every recorded session with a live member gets
  TERM+CONT, then KILL after the grace period: released background jobs and whatever
  finished externals left in their sessions. `UnixHost::kill_all()` and the `Send`
  `RunKiller` do the same on demand ("the embedder kills the run"); `settle()` waits.
  `Leftovers::Keep` leaves them, as bash does. Only `SpawnSpec::detach` (`nohup`,
  `setsid`; the engine sets it) and `ExtProc::disown` (`disown`) leave the run;
  a process that calls setsid itself also escapes. The engine's `exit_grace` lets
  background jobs finish before the end.
- **Standalone `psh`: psh's own process group**, as a non-interactive bash: externals
  and background jobs stay in it, so a harness's or terminal's group kill reaches
  everything; jobs outlive psh as with bash (`Keep`). On a fatal signal psh abandons its
  externals and, when it leads its group, signals the whole group as it dies, so what
  they left goes too even when only psh's pid was signalled. Detached externals get a
  session of their own.
Alternatives: one process group per run in the embedder's session (one id for outside
killers, but a group cannot be joined once its leader is reaped and empty, the leader
must be pinned as a zombie, and `disown` could not escape a group kill); one session per
run (impossible without a helper process: nothing can join another session); a reaper on
the harness side only (leaves the embedded case open). Residual risk: a session whose
leader was reaped is signalled at the end only while it still has members, so its id
cannot have been reused, but the check and the signal are not atomic.

## D1290: Assignments in front of any builtin are temporary and exported
date: 2026-11-07 · by: builtins · issue: 0029
Bash 5.3 outside POSIX mode treats `x=1 cmd` the same for every builtin, special or not
(probed: `x=1 :; echo $x` prints nothing, `x=1 eval 'sh -c "echo \$x"'` prints 1,
`y=0; y=1 eval y=2; echo $y` prints 0). `run_builtin` sets them exported for the
builtin's run and restores the previous variables afterwards. `Builtin::special` stays
for POSIX mode (`set -o posix`, where the assignments persist).

## D1291: The command hash table belongs to a PATH value
date: 2026-11-07 · by: builtins · issue: 0029
`Vp::hash` (shared with subshells until one changes it) remembers what `PATH` lookups
found, with hit counts; running a remembered name uses its path as is (bash: a stale
path fails with "No such file or directory"). Instead of hooking every assignment to
`PATH`, the table records the `PATH` value it was filled under and starts empty when the
value differs. Lookups go one `PATH` entry at a time through `Host::resolve`, so a
relative entry yields a relative path (`dir/name`, as bash prints it) and, when no
entry has an executable, the first plain file is used (it then fails with 126).
## D1110: psh-wasm: a plain extern "C" ABI and hand-written JS glue
date: 2026-11-07 · by: infra · issue: 0011
`crates/psh-wasm` builds psh for `wasm32-unknown-unknown` without wasm-bindgen, Asyncify or
JSPI. The module has one import, `psh.host_call(kind, op, arg, ptr, len)`: every host
operation (open, stat, read, write, spawn, sleep, ...) is a request with an operation id;
the JS side answers with the export `psh_complete(op, errno, value, len)`, during the call
or later from its event loop, and the waiting future wakes the engine, which asks the JS
side to step the shell again (`K_WAKE`). Files, pipes and commands live on the JS side;
the module holds handles. Data crosses through an input buffer (`psh_in`) and an output
buffer (`psh_out_ptr/len`), as djs does. `js/psh.mjs` is the glue plus a default host
(`MemHost`). psh-wasm is the second crate allowed `unsafe` (`deny`, allowed per site): the
`extern` import block, its call, and the `#[unsafe(no_mangle)]` exports, each with its
reason. It depends on psh-syntax, psh-classify, psh-engine (feature `shell`) and, for the
JSON of parse and classify, deser-json (D029). Until that JSON lives in a crate of its own,
psh-wasm includes `crates/psh/src/{json,parse_json,classify_json}.rs` by `#[path]`.

## D1111: The wasm module's stack and nesting limit
date: 2026-11-07 · by: infra · issue: 0011
The shell module links with a 16 MiB stack (`-zstack-size`, psh-wasm's build.rs), the
budget of the native default, and caps the nesting limit at three quarters of it over
`ShellConfig::STACK_PER_LEVEL` (16 KiB): 768 levels (`psh_max_depth`). A deeper recursion
ends with "maximum function nesting level exceeded" instead of a trap. Linear memory starts
above the stack, so a module instance takes about 17 MiB before any allocation.

## D1112: The JS host: MemHost and host commands
date: 2026-11-07 · by: infra · issue: 0011
`MemHost` (js/psh.mjs) is the default host: an in-memory file system (directories, files,
symlinks; `/dev/null`), unbounded pipes between the shell and commands, timers, and
commands written in JS (`host.command(name, async ctx => status)`, with `ctx.argv`,
`ctx.env`, `ctx.cwd`, `ctx.stdin`/`stdout`/`stderr` streams and `ctx.fs`), the devices
`/dev/null`, `/dev/zero`, `/dev/full`, a sticky `/tmp` and the usual empty system
directories (`/bin`, `/usr/bin`, `/etc`, ...). A command is
found under `/usr/bin/NAME` by every PATH lookup; anything else is "command not found",
recorded in `host.missing`. Signals cannot stop a JS function: a signalled command counts
as killed at once and its later output goes nowhere. Hosts with other storage (OPFS, a
remote pi-env) implement the same methods.
## D1068: Engine pipes become OS pipes when an external starts on them
date: 2026-11-07 · by: procs · issue: 0006
`Host::pipe()` (default `Unsupported`) returns two host files. When `run_external` maps
an engine pipe end to a child descriptor and the pipe's buffer is empty, the pipe is
materialized once: both OS ends are kept in the pipe object, the external gets its end
as `ChildFd::File`, and from then on vproc writes go to the OS write end and vproc reads
take the engine buffer, refilled by one read of the OS read end at a time. Order holds
because nothing was buffered at the switch; read-ahead is safe because an external
started later on a pipe with buffered bytes gets a pump that reads buffer first. The
engine drops an OS end when its last reader or writer of that side goes; the kernel
counts the externals' copies, so EOF and EPIPE are the kernel's. Here-documents
(preloaded) and hosts without pipes keep pumps. Measured: 1 GB through
`head -c | cat | cat | wc -c` in 1.8 s with 0.12 s of psh CPU (release, macOS). Alternative:
deciding statically which stages are externals (impossible before expansion).
## D1260: Variables are a value enum with attributes
date: 2026-11-07 · by: engine · issue: 0026
`Var { value: Value, exported, readonly, integer, case, nameref }` with
`Value::{Unset, Str, Indexed(BTreeMap<i64, Vec<u8>>), Assoc(Assoc)}`; one table per
vproc, dynamic scope through function frames as before (D1059). Code outside `vars.rs`/
`arrays.rs` uses the accessors: `vp.var` (scalar or element 0, namerefs followed),
`vp.set_var` (raw, for the shell's own variables), `vp.assign` (what an assignment does:
readonly, `-i`, `-l`/`-u`, namerefs), `vp.assign_elem`/`assign_array`/`get_elem`/
`unset_elem`, `vp.is_set`. Arrays are never exported. Namerefs resolve up to 8 levels;
cycles and longer chains warn on read (`NAME: circular name reference`) and read as
unset. Reason: one variable model for all later language work (ARCH "Engine internals").

## D1261: Associative arrays iterate in bash's order
date: 2026-11-07 · by: engine · issue: 0026
`${!h[@]}`, `${h[@]}` and `declare -p` list keys the way bash 5.3 does: a hash table of
1024 buckets on the 32-bit FNV-1 hash of the key (a public algorithm), new keys at the
head of their chain, four times the buckets once it holds twice as many entries as
buckets (rehashing in bucket order, each entry to the head of its new chain). Derived
black-box by running bash on thousands of keys (clean room, D007); matches exactly up to
9000 keys. Reason: Oils and brush cases print assoc arrays without sorting; insertion
order would fail them.

## D1262: Declaration builtins get assignments, not words
date: 2026-11-07 · by: engine · issue: 0026
`declare`, `typeset`, `local`, `export`, `readonly` (when not shadowed by a function)
receive `DeclArg`s from the interpreter: `name=(…)` with expanded array items,
`name[=+]value` and `name[sub]=value` expanded like assignment values (no splitting),
and other words expanded normally and then classified. The same builtins called with
plain words (`builtin declare`, `command export`) parse `name=value` from the text.
Assignments in front of them are temporary. They live in `builtins/declare.rs`.

## D1069: Host umask, creation mode and clock
date: 2026-11-07 · by: procs · issue: 0006
Additive, with defaults: `Host::umask()` is the process's umask when the host was made
(the shell's own mask starts there; default `0o022`); `OpenOptions::mode` gives the exact
bits of a file the open creates (the engine applies the shell's mask); `Host::now()` is
the time since the Unix epoch (default: the epoch, for hosts without a clock; the test
host answers a fixed instant). Hosts never change the process's umask (threads, the
embedder): psh-os reads it from `/proc/self/status` on Linux and through a forked child
elsewhere, and applies `mode` by creating exclusively and then `fchmod`ing, so a mask
looser than the process's still yields exact bits; an existing file keeps its bits.
## D1113: psh-spec's wasm engine and platform ratchets
date: 2026-11-07 · by: infra · issue: 0011
`psh-spec --wasm` runs the cases in `target/wasm/psh.wasm` under node: each worker thread
takes a long-lived `node js/test/spec-driver.mjs` from a pool and sends it the case
(invocation parsed in Rust, as for the in-process engine). The driver gives each case a
fresh `MemHost` whose file system mounts the case's directory, HOME and REPO_ROOT from
disk (loaded lazily) and writes the case's changes back, so the snapshot and the goldens
work unchanged; there are no external programs, and the commands the shell looked for and
did not find come back as notes on the run. Ratchets: the shared
`tests/<layer>/expected-failures.txt` plus `expected-failures.wasm.txt`, which lists cases
that pass natively but not in wasm, with their reason. Cases that needed external programs
may fail without a line: listing them would make every native improvement of an
external-using case edit the wasm file. So the wasm ratchet holds only real differences of
the wasm build or its host, and the gate keeps the no-externals pass rate equal to the
native one. This is the per-platform ratchet the coordinator planned for macOS and
Windows (0002 follow-up), with `wasm` as the first platform. A whole run takes about 20 s,
like the native one.
## D1074: Windows spawns: a kill-on-close job per external, stdio only, crashes as signals
date: 2026-11-07 · by: windows · issue: 0007
Every external gets its own Job Object (no breakaway); the process is created
suspended, assigned, then resumed. The run's jobs carry `KILL_ON_JOB_CLOSE`, so when psh
dies (crash, `TerminateProcess`) what it started dies too; detached and disowned
externals' jobs do not, and `Leftovers::Keep` clears the flag on every job when the host
is dropped (background jobs then outlive the shell as with bash). Unix cannot offer the
crash case. Only descriptors 0-2 reach a child: native programs have no inherited
descriptors beyond the standard handles, so `exec 3>f; cmd` passes nothing for 3
(ignored, not an error). TERM/INT/HUP/QUIT are a ctrl-break to the external's console
group (`Placement::Group`); where none can be delivered (no shared console, or
`Placement::Inherit`) the job is terminated. Exit codes: our termination is KILL (137);
`STATUS_CONTROL_C_EXIT`, and `STATUS_DLL_INIT_FAILED` after our break, is the signal
we broke with (INT when the console sent it); access violation, stack overflow and
in-page errors are SEGV, illegal instructions ILL, arithmetic faults FPE, fail-fast and
assertion failures ABRT; every other code is an exit code, which the engine truncates.

## D1075: Windows defaults: /dev/null, /tmp, HOME and TMPDIR
date: 2026-11-07 · by: windows (as proposed by the coordinator) · issue: 0007
The host maps `/dev/null` to `NUL`, `/tmp` and paths below it to the user's temp
directory, and `/c/...` to `C:/...` (Git Bash style), at its boundary. The engine still
sees the path as written (`cd /tmp; pwd` prints `/tmp`). The standalone shell (and any
embedder that calls `psh_os::windows::shell_env`) names the OS's `Path` `PATH` (shell
variables are case-sensitive, Windows names are not), and sets `HOME` from
`USERPROFILE` and `TMPDIR` to the temp directory (long names), `/`-separated, unless
they are set.

## D1076: Windows command lookup
date: 2026-11-07 · by: windows · issue: 0007
`resolve` splits the shell's `PATH` on `;`, and also on `:` unless the colon follows a
single drive letter at the start of an entry (`C:/x`): both native values and bash-style
`PATH="$HOME/bin:$PATH"` work. The owner may still choose the separator policy; the
splitting is one function (`path::split_path_var`). For each entry: the name as it is
when it already ends in a `PATHEXT` extension, else the name plus each extension in
`PATHEXT` order, then the bare name only if it is a `#!` script (so `tsc.cmd` wins over
the `tsc` sh script npm puts beside it, as cmd would choose). Empty and relative entries
are relative to the cwd; there is no implicit cwd search. App Execution Aliases (0-byte
reparse points such as `winget.exe`) count as files. `PATHEXT` comes from the process
environment. `test -x` is true for directories, `PATHEXT` extensions and `#!` files.

## D1077: Windows files: pump threads for children's pipes, peeking reads for shared ones
date: 2026-11-07 · by: windows · issue: 0007
The engine's end of a `ChildFd::Pipe` is a private named pipe (anonymous pipes cannot do
overlapped I/O) whose overlapped server end a helper thread owns, moving bytes to or from
a 64 KiB buffer; dropping it sets a cancel event and `CancelIoEx` stops the thread at
once (a reader) or after the buffer is handed over (a writer, so the child sees end of
file). Shared pipes (the shell's own stdio, `Host::pipe` ends) are never read
speculatively, since a thread blocked in `ReadFile` would take bytes meant for a child:
reads take what `PeekNamedPipe` shows, and otherwise one poller thread checks the
watched pipes (1 ms, backing off to 16 ms) and wakes them. Writes to shared pipes are
synchronous: their reader is another process (vprocs exchange data through engine
pipes), so a full pipe only applies backpressure; anonymous pipes give no reliable free
space (a pending read reduces the write quota), which rules out a non-blocking check.
Regular files, `NUL` and consoles are synchronous; console input is read synchronously.

## D1078: Code-page decoding of native output is a host option
date: 2026-11-07 · by: windows (as proposed by the coordinator) · issue: 0007
`WindowsConfig::decode = Decode::Ansi` makes the host's pump pipes convert child output
read by the engine (`$(cmd)`, pipes into builtins) that is not valid UTF-8 from the ANSI
code page (`GetACP`) to UTF-8, chunk by chunk, keeping an incomplete UTF-8 sequence for
the next chunk. Valid UTF-8 passes unchanged, and nothing happens when the code page is
UTF-8. Off by default. It uses Windows' own conversion (`MultiByteToWideChar`), no new
crate. Output that goes straight from a child to a file, another child or psh's own
stdout is not touched. A double-byte character split across reads is decoded in halves
(a known gap).

## D1079: Scripts psh must run itself on Windows: an engine hook, pending
date: 2026-11-07 · by: windows · issue: 0007
`#!/bin/sh`, `#!/bin/bash` (also through `/usr/bin/env`) and files with neither an
interpreter line nor an executable extension cannot be started on Windows, where there
is no `/bin/sh`; the Unix host's ENOEXEC fallback does not exist. Until the engine runs
such files in-process, `spawn` fails with `ErrorKind::Unsupported` and a message naming
the file. Proposed engine hook: a marker error from `spawn` (or a variant of the spawn
result) that tells the engine to run the program file as a psh script with the same
argv, environment and descriptors, the way bash runs a file that `execve` rejects. The
Unix host would use the same marker for ENOEXEC instead of `/bin/sh`.
## D1280: Shell options live per vproc; POSIX mode is refused
date: 2026-11-07 · by: shapes · issue: 0028
`set` and `shopt` options are one `Copy` value per vproc (`Vp::opts`, `options.rs`):
subshells inherit them and keep their changes, as in bash. `SHELLOPTS` and `BASHOPTS`
are readonly variables kept in step by `set`/`shopt`; `$-` is built from the options
(bash's letter order `abefhkmnptuvxBCEHPT`, then `c`/`s`). The command line takes the
same options (`psh -e -o pipefail -O extglob -c …`) through `Script::options`.
`set -o posix` (and `--posix`) fails loudly as an unsupported feature: POSIX mode is out
of scope (D002), and running on with bash semantics after a script asked for POSIX ones
would be a silent difference (D1057). Options implemented: errexit, nounset, pipefail,
noglob, noclobber, allexport, noexec, verbose (xtrace and the trap options come with
part 2); `shopt` stores all 59 of bash 5.3's and applies extglob (per chunk),
expand_aliases, nullglob, failglob, inherit_errexit and lastpipe.

## D1281: errexit as bash 5.3 applies it
date: 2026-11-07 · by: shapes · issue: 0028
errexit (and, with part 2, the ERR trap) looks at the status of a simple command (a
function call included), a subshell, `((…))`, `[[ … ]]`, and a pipeline of several
commands (its status after pipefail); never at a pipeline stage on its own, and never at
`{ }`, `if`, `while`, `for` or `case`, which fail only through a command inside them that
errexit has seen. `Vp::no_errexit` suspends it, inherited by everything run meanwhile
(functions, compound commands, subshells): while an `if`/`while`/`until` condition runs,
for every `&&`/`||` operand but the last, and for a `!` pipeline. Command substitutions
run with errexit off unless `inherit_errexit` is set. A failing command exits with its
own status. All of this was probed against bash 5.3 (`set -e; f() { false && true; }; f`
exits, `{ false && true; }` does not, `x=$(false)` exits, `true | { false && true; }`
exits).

## D1282: set -u and fatal expansion errors
date: 2026-11-07 · by: shapes · issue: 0028
An unset parameter without an operator that handles it (`-`, `=`, `?`, `+`) is
`NAME: unbound variable` (`$1` and `$!` unbraced keep their `$`, `x[3]` its subscript,
`${!x}` is `!x`); `$@`, `$*` and `${a[@]}` are exempt. Like `${x?}`, it ends the shell
with 127 under `-c` and 1 otherwise, and with 1 under `-c` when errexit is on: that is
what bash 5.3 does. Unset variables inside arithmetic are not checked yet (follow-up).

## D1283: Traps
date: 2026-11-07 · by: shapes · issue: 0028
`Vp::traps` (an `Rc` shared until changed) maps EXIT, signals (Linux numbering, as bash
on Linux lists them), DEBUG, ERR and RETURN to their action text; an empty action
ignores the signal. Actions run in the vproc through the `eval` path with `$?` set and
restored; a bare `exit` inside one keeps the status the trap was entered with. EXIT runs
once when the shell or a subshell ends (also after errexit and fatal signals; `exit N`
in it sets the status). ERR runs where errexit would look (D1281), whether or not `-e`
is on, with `$LINENO` and `$BASH_COMMAND` of the failing command. DEBUG runs before
simple commands, `((…))`, `[[ ]]`, `case` and each `for` iteration. A function call
hides DEBUG and RETURN unless functrace, and ERR unless errtrace; they come back on
return unless the function set its own, and RETURN runs as the function (or a sourced
file) returns. Subshells keep their parent's traps for `trap` to list but do not run
them, except ignored signals and, with errtrace/functrace, ERR/DEBUG/RETURN; the first
`trap` in the subshell drops the parent's. No DEBUG or ERR trap fires inside a trap.
Not done: extdebug's skip/return semantics, DEBUG in pipeline children, FUNCNAME.

## D1284: Signals to the shell: the engine half
date: 2026-11-07 · by: shapes · issue: 0028 (agreed with procs, 0006, via the coordinator)
`ControlHandle::signal(sig)` (Linux numbering) sets a bit in an atomic pending mask in
`sched::Shared` and wakes the embedder. The shell's own vproc (`Vp::top`, not subshells)
takes pending signals at its next safepoint, before a command: a trap runs its action
(`$?` kept); an ignored signal does nothing; otherwise the default action: signals that
terminate end the shell with `128+n` after the EXIT trap (CHLD, CONT, STOP, TSTP, TTIN,
TTOU, URG, WINCH do nothing). A foreground command finishes first, as in bash. `cancel`
stays a forced TERM without traps. `start` clears pending signals. `Vp::
signal_trap_ignored(sig)` tells spawn which signals children start ignored. Procs' half:
`kill` on the shell (`$$`, 0, its group) sets the mask instead of signalling the host;
the standalone `psh` forwards its OS signals (mapped to Linux numbers) to
`ControlHandle::signal`; `wait` and foreground waits wake on pending signals.

## D1285: xtrace
date: 2026-11-07 · by: shapes · issue: 0028
`set -x` writes `PS4` (expanded like a double-quoted string, its first char repeated once
more per level of command substitution and `eval`) and the traced line to
`BASH_XTRACEFD` (if it names an open fd) or stderr: simple commands after expansion, with
their prefix assignments on lines of their own; assignment-only commands one line per
assignment (`x=` for an empty value); declaration builtins and then their assignments;
`(( … ))`, each clause of `for ((…))`, `case WORD in` and the `for` header on each
iteration. Words are quoted as bash does: as is when safe, in single quotes when they
contain blanks or shell metacharacters (`#`/`~` only at the start, `~` also after `=`
or `:`), `$'…'` for control characters and bytes that are not UTF-8.
## D1900: Jobs: `$!` is the program for a one-command job, a job pid otherwise
date: 2026-11-07 · by: procs · issue: 0006
`cmd &` is a job: a detached vproc task in the starting vproc's job table (a subshell
starts with an empty table, as it cannot wait for its parent's jobs; a pipeline stage's
`jobs` still lists them, as bash's does). A job that is one simple command is its program,
as bash runs it as the job's own process: `$!` is the external's OS pid (so `kill $!`,
`wait $!` and pid files mean the program); reading `$!` (and `jobs`, `kill %n`) first
gives such a job a bounded number of scheduler rounds to start it. Any other job is a
bash subshell: its `$!` is a pid of its own above every real one (2^22 + n), and a signal
that ends processes ends its task at once with status 128+n, its externals released as a
killed subshell's children would run on. Background jobs ignore SIGINT and SIGQUIT (their
externals start with them ignored). `wait` returns the job's status, remembers reported
jobs for `wait PID`, reports signal deaths other than INT, PIPE and TERM as bash does;
`jobs` shows Running/Done/Exit N/glibc signal texts in bash's columns and drops reported
finished jobs; `disown` hides a job and its externals leave the run (D1067). Signal names
and numbers are the OS's (BSD numbering on macOS), descriptions glibc's. Job control
proper (`fg`, `bg`, `set -m`, stopped jobs) stays out of scope (D002).

## D1901: The standalone psh takes its signals on the engine thread
date: 2026-11-07 · by: procs · issue: 0006
psh-os starts its helper threads (waiters, escalations, readiness, timers, FIFO opens,
the signal watcher) with every asynchronous signal blocked from their first
instruction, so a process signal never lands on them: embedded, it reaches the
embedder's threads; standalone, `psh` blocks INT, TERM and HUP in its main thread and
unblocks them only on the engine thread, and the handler also sets the engine's cancel
bit itself (`ControlHandle::cancel_word`, an atomic or). A `kill $$` (or `kill -INT $$`
inside `$(…)`) is then delivered before `kill` returns and the run stops before the next
command, as bash dies at once; before, the watcher thread's cancel raced the following
commands. A `( … )` subshell, being a vproc, stops with the shell (bash's subshell
process would go on).

## D1902: A pipeline stage that is one external command lets go of its descriptors
date: 2026-11-07 · by: procs · issue: 0006
bash execs the last command of a stage's subshell, so a stage `… | head -1` holds no
pipe end of its own. The engine marks a stage that is one simple command (`exec_tail`);
when it runs an external, the vproc drops its descriptor table once the external has its
descriptors (and, with redirections, the saved table), so the pipe closes when the
external closes it and a writer gets EPIPE at once, not when the stage's task ends.
Functions, builtins and nested commands never see the mark.

## D1903: Signals to the shell: the process half of D1284
date: 2026-11-07 · by: procs · issue: 0006
Supersedes D1066 and the cancel part of D1901. `kill` aimed at the shell itself (`$$`,
and `0`, which also asks the host to signal the rest of the group) sets the engine's
pending mask instead of signalling the host, so traps run in-process as in the binary.
The standalone `psh` catches the signals a script may trap whose default action ends a
process (HUP INT ALRM TERM USR1 USR2 XCPU XFSZ VTALRM PROF IO PWR; not QUIT, which bash
ignores, PIPE or the faults), only on the engine thread; the handler sets the signal's
bit (OS numbers mapped to Linux's) in `ControlHandle::signal_word` and the watcher thread
calls `ControlHandle::signal` to wake the shell. Waits of the shell's own vproc give way
to pending signals (`signals::or_signal`, woken by `Shell::step`): a foreground external,
a pipeline's or a subshell's wait ends for a signal whose default action ends the shell
(the external is abandoned, the subshell's task runs on as bash's subshell process
would); `wait` also ends for a trapped one, with `128+n`, and the trap runs next. A signal
that came with the last command still acts. When a default action ended the run,
`Shell::killed_by` says which signal and `psh` dies by it (after its abandoned externals
were killed); a trapped or ignored signal leaves psh running. Blocking reads (`read`, a
FIFO open) do not give way yet.
## D1910: Host::path_entries: the host splits PATH-like values
date: 2026-11-07 · by: windows (with the coordinator's OK) · issue: 0007
The engine's own lookups (`type`, `command -v`, `hash`, `source`) walk `PATH` one entry
at a time; splitting on `:` there broke every lookup on Windows, where `C:\WINDOWS` has a
colon. `Host::path_entries(path)` returns the entries; the default splits on `:` (Unix,
the test host, wasm), `WindowsHost` splits as D1076 says. The separator policy is then
one host function. The test host can split on `;` to cover the engine side; the Windows
L5 suite runs system programs through the real PATH, so a regression fails the gate.

## D1300: Scripts the host cannot start run in psh (RunInShell)
date: 2026-11-07 · by: procs · issue: 0030
Implements D1079, superseding D1063. `host::RunInShell` is a marker error `spawn` returns
(inside `io::Error`, `RunInShell::is`) for a program the engine must run itself: Unix
for `ENOEXEC` (no `#!`), Windows for `#!/bin/sh`, `#!/bin/bash` and files with neither an
interpreter line nor an executable extension (the Windows host switches to it). The
engine reads the file through the host and runs it as bash runs a file `execve`
rejects: in a subshell that starts afresh (exported variables only; no functions,
aliases, traps or options), `$0` the name as written, `$@` the arguments, the command's
descriptors and cwd; a file with a NUL in its first line (within 80 bytes) is refused
with "cannot execute binary file: Exec format error" (126). Messages name `$0` per vproc
now (`Vp::msg_name` uses `arg0`, syntax errors and `-c` wording use the vproc's
options), so the script's errors say `./s: line N:`. Also: `trap '' SIG` passes on to
externals (`SpawnSpec::sig_ignore`, D1284), which 0006 had left unwired.
## D1080: Utilities are async and run on the engine scheduler; blocking ones are the exception
date: 2026-11-07 · by: infra · issue: 0008
Supersedes ARCH's "the engine runs utilities on worker threads". A utility is an async
function over a context (`UtilCtx`, D1081), called on the vproc that runs the command,
where shell builtins run; pipeline stages are concurrent vprocs already. Its I/O is the
command's own descriptors (engine pipes, host files), awaited; its file system is the
Host; cancel (and `timeout`) is dropping its future; `ctx.check().await` in long loops
yields when the step budget is spent. The same code runs natively and in wasm, needs no
thread, keeps output order deterministic and keeps `cat f | head` in-process.
Ported code (uutils' `Read`/`Write` loops) is rewritten onto the context; algorithms are
reused. The exception, for code built on synchronous crates (0013's ripgrep crates and
walker, jaq, a large `sort`): blocking utilities, `fn(&mut SyncCtx) -> i32` with std
`Read`/`Write` stdio. Natively with the OS host they run on a worker thread whose stdio
are channels the engine pumps; cancel sets a flag and closes the channels; the command's
status comes only after the thread ended or can no longer write. In wasm or with a
sandbox host they run inline: stdin is read to its end first, within a bound (64 MiB or
the run's time budget, whichever comes first; beyond it the utility fails with
`NAME: input too large for the in-process NAME in this environment` and status 2, never
a hang: `yes | grep x | head -1` ends there with that error), and their fs view makes
synchronous host calls the host must answer at once. Rejected: synchronous utilities on
threads natively plus another strategy in wasm (inline synchronous code deadlocks on a
pipe fed by another vproc; threads make cancel and output order approximate).

## D1081: UtilCtx, version 1
date: 2026-11-07 · by: infra · issue: 0008
`psh_engine::util` is the utility API every batch builds on; `UTIL_API_VERSION` = 1 and
any change is a new decision. A utility is `for<'a> fn(&'a mut UtilCtx<'_>, &'a
[Vec<u8>]) -> LocalBoxFuture<'a, i32>` (argv[0] is the name it was called by). The
context offers: the shell's cwd and the environment the command sees (prefix assignments
included); stdin reads (`read`, `read_to_end`); buffered stdout writes (`write`,
`flush`); stderr writes (`error`, `error_raw`); the Host's file operations resolved
against the cwd (`open`, `stat`, `read_dir`, `access`, `canonicalize`); `sleep`;
`is_terminal(fd)`; `utf8()` (the locale); `check()` (safepoint); and `spawn(argv, env)`,
which starts a command as a child vproc for the wrappers and returns a handle with
`wait()` and `signal(sig)`. Output buffering rule: stdout is buffered (64 KiB) and
flushed when full, at exit, before every stdin read (so `cat | while read` and prompts
never wait on a buffer), before every stderr write (so stdout and stderr interleave as a
line-buffered program's would) and before `spawn`; output to a terminal is flushed at
every line end.

## D1082: Utility options are declared and checked against the classifier
date: 2026-11-07 · by: infra · issue: 0008
Each utility declares its options as data (`opts::Spec`: short and long names, whether
they take a value, plus a per-utility argv rewrite for forms like `head -5`), parsed by
one shared parser with GNU conventions (permutation, `--`, `-n5`, `-n 5`, `--name=v`,
unambiguous long prefixes). An option outside the spec goes through one helper: the real
program from PATH if there is one (unless the utility is the only one, as in wasm), else
`NAME: unsupported option 'X' (psh built-in NAME)` and status 2. A test compares every
utility's spec with its `psh_classify::utils` entry (D1090): the same value-taking short
and long options, and every effect option the classifier lists is accepted.

## D1083: The utility registry and where it is looked up
date: 2026-11-07 · by: infra · issue: 0008
psh-utils' `REGISTRY` is a static table of `UtilEntry { names, mode, destructive, run }`;
`psh_utils::policy(sandbox)` turns it into the `UtilPolicy` an embedder puts in
`ShellConfig::utilities` (the psh binary and psh-spec natively, psh-wasm with
`sandbox: true`; empty by default, so a bare engine has none). The engine looks a command
name up after functions and builtins and before PATH; a name with a slash, a name bound to
a path in the hash table (`hash -p`, or a program already run) and `command -p NAME`
run the program. Off switches (all, the destructive group, names) are fields of the
policy. `type` says `NAME is a psh utility`, `type -t` says `builtin`, `command -v` prints
the name. Wrappers (`UtilCtx::spawn`) run standalone builtins (`printf echo test [ true
false pwd kill`), utilities and programs, never functions. In wasm the utilities shadow
JS host commands of the same name.

## D1084: L4: GNU and BSD recordings, one expectation per case
date: 2026-11-07 · by: infra · issue: 0008
L4 cases live in `tests/utils/NAME/*.test.sh` (the Oils format, run as `bash -c`). Each
is recorded twice: GNU in the container (`tests/goldens/utils/`) and BSD on macOS
(`tests/goldens/utils-bsd/`, `scripts/record-goldens --bsd`: Homebrew's bash 5.3, PATH of
the system's directories only, so no GNU tool answers). A case declares `## expect: gnu`,
`bsd` or `both` (default); with `both` the two recordings must agree on stdout, status and
files, else the case fails as "goldens disagree" until it is annotated with the dialect and
the D016 rule or the flag's origin. The BSD goldens are recorded by hand on a Mac, not in
CI; their determinism is checked by recording twice.

## D1085: A utility job has no OS process id
date: 2026-11-07 · by: infra · issue: 0008
A background job that is an in-process utility (`sleep 5 &`) has no OS process: its `$!`
is the shell's own job id. `kill`, `wait` and
`jobs` in the shell handle it; another program cannot signal it (`sh -c "kill $!"`). One
brush case does exactly that and fails for this reason (its ratchet line says so).
## D1911: Windows umask, creation modes and clock
date: 2026-11-07 · by: windows · issue: 0007
Windows has no umask and one permission bit, the read-only attribute. `WindowsHost`
reports `umask()` as `0o022` (bash's usual default, so `umask` prints what scripts
expect) and ignores a spawn's `umask`. `OpenOptions::mode` on a file the open creates:
when it has no write bit, the file is created exclusively and gets the read-only
attribute through the open handle, which can still write (as `open(2)` with 0444);
every other bit has no Windows meaning; an existing file keeps its attributes. `now()` is
`SystemTime::now()` since the epoch.

## D1912: Test deadlines stretch under emulation
date: 2026-11-07 · by: windows · issue: 0007
The psh cli tests' 10 s limit per run was missed now and then in the Windows VM's
`test-x64` leg (x64 under emulation), also after D1072/D1073. Measured with the VM to
itself, a freshly linked x64 `psh.exe` starts in 20-35 ms, six at once in 55-115 ms,
and the cli tests take under a second; the misses come from moments when the host is
saturated (other gates' macOS legs and builds), which slow the VM's emulated processes
by an order of magnitude. Rather than lengthen every deadline everywhere, tests with
deadlines read `PSH_TEST_TIME_SCALE` (a factor ≥ 1, default 1); `windows-gate.ps1` sets
it to 3 for `test-x64` only. Tests still catch hangs; native runs keep their limits.
## D1263: Dynamic variables and `$_`
date: 2026-11-07 · by: engine · issue: 0026
`RANDOM`, `SRANDOM`, `SECONDS`, `EPOCHSECONDS`, `EPOCHREALTIME`, `BASHPID` and
`BASH_SUBSHELL` are variables marked dynamic (`Var::dynamic`): their value is computed
when read (time from `Host::now`, so a host without a clock reads the epoch) until they
are unset; `RANDOM=n` seeds and `SECONDS=n` restarts the count. `RANDOM` is a
xorshift sequence, not bash's (no case depends on bash's numbers). `BASHPID` in a
subshell is a made-up id, as vprocs have no pid. `$_` is the last word of the last simple
command, set when the command completes (a function body sees the value from before the
call; an assignment-only command clears it); externals get `_` as their path, except
through `exec`. psh does not set `BASH_VERSION`/`BASH_VERSINFO`: scripts that detect
bash then switch on options psh lacks (`set -o posix`); `PSH_VERSION` is set instead.

## D1264: psh sets BASH_VERSION and BASH_VERSINFO
date: 2026-11-07 · by: coordinator (engine records it) · issue: 0026
Supersedes the last sentences of D1263. psh sets `BASH_VERSION=5.3.9(1)-release` and
`BASH_VERSINFO=(5 3 9 1 release ARCH)` (readonly), its compatibility target, and also
`PSH_VERSION` so scripts can detect psh. Reason: agent and installer scripts gate on
`BASH_VERSION`, and some re-exec `bash` when it is missing (on Windows there may be no
bash); a script that then uses something psh lacks fails loudly on that feature, the
honest outcome. Cases that detect bash and switch on `set -o posix` (unsupported, D002)
are in the ratchet.

## D1114: wasm ABI 2: invocation options, the clock, creation modes
date: 2026-11-07 · by: infra · issue: 0011
`psh_shell_start`'s payload ends with the command-line options (a count, then each word,
as `Script::options` and `invocation::parse_args` have them), so an embedder starts a
script with `-e`, `-o pipefail`, `-O extglob` or `-s` exactly as the `psh` binary does;
the JS API takes them as `run(source, { options: [...] })` and psh-spec's wasm runner
passes the case's. `Host::now()` (D1069) is the host_call `K_NOW`: the JS side answers
synchronously by writing an f64 of milliseconds into the input buffer, since a call's i32
cannot hold them (MemHost's clock is `Date.now()`, replaceable as `new MemHost({ now })`).
`OpenOptions::mode` rides in `K_OPEN`'s flags (`F_MODE`, bits in 16..28); MemHost gives a
created file those bits. `printf '%(…)T'` now reads the Host's clock instead of
`SystemTime`, so it has the time in wasm too. `ABI_VERSION` is 2.

## D1301: Host::info: user, group, parent and machine facts
date: 2026-11-07 · by: procs · issue: 0030
`Host::info() -> HostInfo { uid, euid, groups, ppid, hostname, ostype, machtype,
hosttype }`, additive with a default (uid/euid 1000, groups [1000], no parent,
`localhost`, `unknown`, `ARCH-unknown-unknown`, the compile-time arch) for hosts without an
OS (test host, WASM). The Unix host answers from the running system, in bash's formats:
`OSTYPE` `linux-gnu` (`linux-musl`, `linux-android`) or `darwin` + the kernel release
(`darwin25.6.0`: bash bakes in its build machine's; the running one is more useful),
`MACHTYPE` `x86_64-pc-linux-gnu` / `aarch64-unknown-linux-gnu` / `aarch64-apple-darwin…`,
`HOSTTYPE` the arch; `GROUPS` from `getgroups` with the real group first if missing;
`PPID` the parent, or, embedded (a fake `$$`), the embedding process itself. Proposed for
Windows: `OSTYPE=msys`, `MACHTYPE=x86_64-pc-msys` (`aarch64-…` on arm64), as Git Bash
reports, so agents' scripts take their Windows branch (psh accepts `/c/...` paths); uid
and euid 197609-style values are not needed: report 1000 (or the RID if cheap), groups
[1000]. The engine wires the variables (readonly `UID`, `EUID`, `PPID`; `GROUPS` array).
## D1225: Flaky cases are quarantined, never retried
date: 2026-11-07 · by: infra · issue: 0022
`tests/spec.toml` `[[flaky]]` tables (the `match`/`tag` keys of the groups, `reason`
required) mark cases whose result depends on timing. They run normally and count in
their group's numbers, but never fail psh-spec or the gate, never appear in a ratchet
(shared or platform; `--update-ratchet`/`--add-failures` skip them, and a stray entry
is stale), and psh-spec and `scripts/status` print them separately: count, how many
passed this run, each id with its reason. No retries: a retry turns a real intermittent
bug into a green run. The reason names the race and the owner who fixes it; known real
bugs (as opposed to races in the test itself) do not go there.

## D1302: The signal watcher only wakes the shell
date: 2026-11-07 · by: procs · issue: 0030
Amends D1903. The standalone handler sets the signal's bit in the shell's pending mask;
the watcher thread now only wakes the shell (`ControlHandle::wake`, new) instead of
calling `ControlHandle::signal`, which set the bit a second time: when the shell had
already taken the first (a trap ran), the second delivery ran the trap again
(`traps_run_for_signals_from_outside` failed about half the time).
## D1320: The call-stack variables are dynamic, computed from the vproc's frames
date: 2026-11-08 · by: engine · issue: 0032
`FUNCNAME`, `BASH_SOURCE`, `BASH_LINENO`, `BASH_ARGV` and `BASH_ARGC` are dynamic
variables (D1263) read from the vproc's call frames (`Call`: name, source, call line,
arguments), not arrays kept up to date on every call. A script run from a file has a
bottom frame (`main`, the script, line 0); `-c` and stdin have none, as in bash.
`BASH_ARGV`/`BASH_ARGC` carry function frames only under `extdebug`, plus `source`
frames (the file and its arguments) and the script's frame (when `extdebug` is on or
another frame exists). `declare -p` refreshes them first. Alternative: maintained
arrays (a write per call for variables almost no script reads).

## D1321: One pattern matcher with options; extglob groups backtrack
date: 2026-11-08 · by: engine · issue: 0032
`pattern::Pattern::with(pat, MatchOpts { utf8, extglob, nocase })` serves globbing,
`case`, `${x#…}`/`${x%…}`/`${x/…}` and later `[[ == ]]`; `Vp::match_opts(nocase)`
fills it from the shell (`extglob`; the caller passes `nocaseglob` or `nocasematch`).
Patterns without groups keep the linear star-backtracking matcher; groups
(`?( ) *( ) +( ) @( ) !( )`, nested) use a recursive backtracking matcher over the
token list, fine at the sizes shells see. Case folding lowers pattern and text
(one-char mappings). An unquoted `Part::ExtGlob` word part expands each alternative
as a pattern and joins them back as `op(a|b)`, never split. Glob options: `dotglob`
lets `*` match leading dots (never `.`/`..`, which `read_dir` omits), `globstar` makes
a `**` component every directory below (symlinked directories not followed) and,
last, every file; `a/**` lists `a/` itself, `**/` only directories.

## D1322: `~user` asks the host: `Host::home_dir(user)`
date: 2026-11-08 · by: engine · issue: 0032
An additive Host method with a default of `None` (the tilde word stays as written).
psh-os on Unix answers with `getpwnam_r`; Windows and the JS host keep the default
until someone needs it. A small op rather than a general user-info op: nothing else
needs user data yet.

## D1323: Engine paths keep a Windows drive in front: `C:/x`
date: 2026-11-08 · by: engine · issue: 0032
`normalize` (cd, `$PWD`) leaves `C:/x` and `C:\x` as `C:/x` (backslashes become `/`;
`..` stops at the drive) instead of turning them into `/C:/x`. The Windows host's
`/C:/x` workaround can go (windows agent).

## D1324: `$(…)` drops a `\r` before each removed newline on Windows only
date: 2026-11-08 · by: engine · issue: 0032
Native Windows programs end lines with CRLF; `x=$(git rev-parse HEAD)` must not keep a
`\r`. On Windows, each trailing newline removed by command substitution takes a `\r`
right before it along. Elsewhere bash behavior is unchanged (a `\r` is data there and
the goldens keep it). `read` should follow the same rule on Windows (builtins agent).
Alternative: all platforms (would break byte-exact output on Unix).
## D1270: psh-regex: GNU's POSIX regex semantics in a linear-time engine
date: 2026-11-07 · by: parser · issue: 0027
A new crate `psh-regex` (no_std + alloc, no dependencies) implements POSIX ERE and BRE
with GNU's extensions (`\w \W \s \S \b \B \< \> \` \'`, back-references in ERE, `\+ \?
\|` in BRE, `{,n}`), GNU's error codes and messages, case folding, newline mode and
UTF-8 or byte text. The oracle is GNU libc's `regcomp`/`regexec` (bash's `=~` and the
GNU tools use it), driven by a small C harness in the recording container: overall
leftmost-longest match; subexpressions as a backtracking matcher taking alternatives in
order and repetitions greedily finds them within those bounds (GNU, not strict POSIX:
`(a|ab)(c|bcd)` gives `a` and `bcd`); a group in a repetition keeps an earlier
iteration's value; only the first iteration of `*`/`+` may be empty; GNU's syntax rules
(no repeated anchors, BRE stacking rules, a back-reference only to a group in the same
alternative). Matching without back-references is linear: an NFA simulation finds the
bounds, a memoized backtracking search (each instruction/position pair once) finds the
groups; with back-references a budgeted backtracking search. 4,053 recorded GNU results
are a test (`crates/psh-regex/tests/gnu.txt`); 4 known differences remain, all in which
alternative or iteration gets a group when assertions or bounded repetitions of groups
are involved. macOS's regex differs widely (no GNU escapes) and is not a target.

## D1271: Non-ASCII ranges work by code point (unlike glibc's C.UTF-8)
date: 2026-11-07 · by: parser · issue: 0027
glibc in C.UTF-8 (the recording locale) rejects a bracket range or collating element
with a non-ASCII char (`[à-é]` is "Invalid collation character"). psh-regex accepts it
and compares code points, as other UTF-8 locales effectively do. Reason: agent-first
over bash's behavior (D033); an agent's `[[ $x =~ [à-ÿ] ]]` should work, and no golden
depends on the error.
## D1150: Host file system changes and UtilCtx v2
date: 2026-11-07 · by: files · issue: 0015
The `Host` trait grows what the file utilities need, every method with a default so that
hosts without it still build: the changes answer `ErrorKind::Unsupported` (a utility
reports `Operation not supported` and fails; nothing is emulated), the queries `None`/UTC.
Paths are absolute like `open`'s; a symlink target is stored as given.
- `stat_extra(path, follow) -> HostFuture<io::Result<(Metadata, ExtraMetadata)>>`;
  `ExtraMetadata { atime_ns, ctime_ns, btime_ns: Option<i64>, nlink, blocks (512-byte),
  blksize, rdev }`. A separate struct, not new `Metadata` fields, so the hosts that
  build `Metadata` literals (Windows, JS) need no change; the default derives it from
  `stat` (`ExtraMetadata::derived`: times = mtime, one link, size in blocks).
- `mkdir(path, mode)`: exactly these bits (the caller applies the shell's umask; the
  host undoes the process's). `remove_file(path)` (unlink; a symlink or junction itself,
  never followed), `remove_dir(path)` (empty), `rename(from, to)` (POSIX replace rules;
  `CrossesDevices` across file systems), `symlink(target, link)` (Windows: a junction for
  a directory when symlinks are not allowed, else an error, never a copy),
  `hard_link(existing, link)` (the symlink itself, not its target), `read_link(path)`
  (`InvalidInput` for a non-link), `set_mode(path, mode)` (follows; Windows maps the
  write bits to the read-only attribute), `set_times(path, atime_ns: Option<i64>,
  mtime_ns: Option<i64>, follow)` (`None` keeps that time).
- `user_name(uid)`, `group_name(gid) -> Option<Vec<u8>>`; `utc_offset(epoch_secs) -> i32`
  (seconds east of UTC in the process's zone).
`UtilCtx` (UTIL_API_VERSION 2, additive) has the same operations with paths made absolute
against the cwd (`stat_extra mkdir remove_file remove_dir rename symlink hard_link
read_link set_mode set_times user_name group_name`), plus `now()`, `umask()` (the shell's
`umask`, else the host's) and `utc_offset(secs)`, which is 0 when the command's `TZ`
names UTC (`UTC`, `UTC0`, `GMT`, `Etc/UTC`, empty) and the host's otherwise. Implemented
in psh-os unix, std_host and the test host (in-memory symlinks, inode numbers, link
counts; `MemFs::set_unsupported` plays a host without them); the Windows and JS hosts
follow in their own issues. Reason: ARCH listed these as "to come with their features";
copy (`cp`) stays a read/write loop over `open`, so no copy primitive is added.

## D1303: The report of a foreground program a signal ended
date: 2026-11-07 · by: procs · issue: 0030
As bash prints it non-interactively: after a simple command whose external died of a
signal other than INT and PIPE (status 128+n), TERM as `DESCRIPTION COMMAND`
(`Terminated                 sh -c 'kill -TERM $$'`, the description padded to 27), every
other signal as `NAME: line N: PID DESCRIPTION COMMAND` (pid right-aligned in 5). Not
inside `$(…)`, not for pipeline stages or a job's own command (`wait` reports those, also
with the pid in 5 columns now). Gaps: a pipeline whose last stage died (bash reports the
whole pipeline) and a subshell that died (bash prints `( … )`) are not reported.

## D1304: Signal dispositions the tests and the runner start from
date: 2026-11-07 · by: procs · issue: 0030
`UnixConfig::inherit_ignored` (default off; the standalone `psh` sets it): only then do
signals the process ignored when the host was made stay ignored in externals (bash's
rule for its own start). Embedded, the embedder's dispositions (a daemon under `nohup`, a
test binary started as a background job, which ignores SIGINT) are not the shell's.
psh-spec and the standalone tests reset the asynchronous signals they ignore to their
defaults before running anything (`signals::reset_ignored(&HARNESS_RESET)`: HUP INT QUIT
USR1 USR2 ALRM TERM XCPU XFSZ VTALRM PROF; never PIPE), so the psh binary they start and
the cases' goldens see default dispositions however the gate was started. Checked: all
psh-os, L5, standalone and psh-spec runs pass under a parent ignoring INT TERM HUP QUIT
USR1 USR2 ALRM.
## D1130: Blocking utilities: `UtilCtx::run_blocking` and `SyncCtx`
date: 2026-11-07 · by: infra · issue: 0013
Builds D1080's exception, additively (UtilCtx v1 is unchanged: `UtilFn` and `UtilEntry`
stay; the new items are additive and need no version bump). A utility on synchronous
crates is an ordinary async utility whose front end parses arguments on the engine
thread (so `run_program` fall-through stays async), then awaits
`ctx.run_blocking(BlockingOptions { stdin }, arg, body)` with `body: fn(&mut SyncCtx,
T) -> i32`, `T: Send`. `SyncCtx` offers name, cwd, env, TTY state of fds 0–2, `path`,
`native_path`, `cancelled()`, std `Read`/`Write` stdin/stdout/stderr with D1081's buffering
rules, `error`, and a synchronous fs view (`open` for reading, `stat`, `read_dir`).
- **Backend choice is the Host's**: new additive `Host::native_paths() ->
  Option<fn(&[u8]) -> io::Result<PathBuf>>` (default `None`; psh-os and StdHost answer
  their own path mapping). With it, the body runs on a worker thread and its fs view
  is `std::fs` on the OS paths, which is also what lets the `ignore` walker run
  natively. Stdio are queues the utility's vproc pumps: output in flight is bounded (1
  MiB), a write after a failed one (EPIPE) fails, stdin reads go through the engine
  one at a time. Dropping the future (cancel, `timeout`, a fatal signal) sets the
  cancel flag and fails every later I/O of the thread, which therefore can no longer
  write; the thread is not joined.
- **Inline otherwise** (wasm, sandbox and test hosts): stdin is read to its end first
  only when `stdin` is set (64 MiB bound, D1080's message and status 2 past it);
  output is collected in order (256 MiB bound, writes past it fail) and written after
  the body returns; fs calls are Host futures polled once, and one that is not ready
  is a `WouldBlock` error, never a hang.
- A panic in the body is caught (`internal error: MSG`, status 2) where unwinding
  exists.
Incremental searching in wasm (`yes | grep x | head -1`) needs the search crates driven
over chunks by an async utility; it is 0013 follow-up work, not this mechanism.

## D1115: wasm ABI 3: the D1150 file system in the JS host
date: 2026-11-07 · by: infra · issue: 0011
D1150's Host operations reach JS through three host_call kinds. `K_FS` (async, like
`K_OPEN`) carries every change and `stat_extra`, with the operation in `arg` (`FS_MKDIR`
… `FS_SET_TIMES`) and length-prefixed paths and u32s in the payload; times travel as
decimal text so nanoseconds stay exact, and `stat` now answers integer nanoseconds too.
`K_NAME` (user/group name, answered in the input buffer like `K_NOW`) and
`K_UTC_OFFSET` (`-getTimezoneOffset()*60` at the instant) are synchronous. MemHost
implements them over MemFs with POSIX semantics (rename's replace rules and EINVAL into
itself, EISDIR/ENOTDIR/ENOTEMPTY, hard links share the node and count `nlink`, ctime
and the parent's mtime move with changes; times set exactly keep their nanoseconds);
`userName`/`groupName`/`utcOffset` are methods an embedder may override (default: uid
and gid 0 are root, 1000 is user; the local zone). EBUSY and EXDEV join the errno
table. psh-spec's wasm driver now writes back directories made in memory (only lazy
mounts nobody opened are skipped), which the D1150 cases need.

## D1131: POSIX patterns for the search utilities: psh-regex parses, regex matches
date: 2026-11-07 · by: infra · issue: 0013
The BRE/ERE translator the search utilities share (grep, later sed and `find -regex`)
is `psh_regex::to_rust_syntax`, built on psh-regex's parser (D1270), not a second
parser: syntax, GNU extensions, error messages and bracket rules are the ones psh-regex
was verified with against glibc. It emits a pattern for `regex-syntax` whose meaning
does not depend on the caller's parser flags except case folding: anchors, `.` and
negated sets are spelled out from `Options::newline` (`(?m:^)`, `[^\n]`, or `(?s:.)`),
a C-locale pattern is `(?-u)` bytes, POSIX classes in UTF-8 follow psh-regex's `Class`
(ASCII members plus Unicode properties), undecodable bytes are `(?-u:\xHH)`. Two
flags tell the caller when to use `psh_regex::Regex` instead: `pattern: None`
(back-references) and `ambiguous` (an alternation, or more than one variable-length
part: only then can a leftmost-first match end elsewhere than the leftmost-longest one;
line selection is the same, spans can differ). A unit test runs psh-regex's GNU corpus
through the translation: every unambiguous case gives glibc's bounds, every ambiguous
one its match/no-match.

## D1132: The walker seam: `ignore::Walk` natively, the same walk over WalkHost otherwise
date: 2026-11-07 · by: infra · issue: 0013
`search::walk::Walker` runs one of two walks with identical results, checked by a
parity test over fixture trees (nested `.gitignore`/`.ignore`/custom ignore files,
`.git/info/exclude`, negations, parent-directory rules, `require_git`, hidden files,
symlinks and loops, globs, types, depth and size limits, unreadable directories, ignore
file errors, skip and quit).
- **Order**: depth first, a directory's entries sorted by name (`--sort path`) in both,
  so output is deterministic. A multithreaded search keeps this order by searching in
  parallel behind a sequential walk (part of rg), not by `WalkParallel`.
- **Native** when the file system is the process's (`native_path` exists): `ignore::Walk`
  over absolute OS roots without `.` components, `current_dir` = the shell's cwd;
  entry and error paths (including walkdir's "IO error for operation on PATH") are
  mapped back to the user's spelling. `min_depth` is applied by the seam, not by
  `ignore` (whose `Walk` would skip the root's own ignore files). Not native when the
  process environment names another global gitignore than the shell's (HOME,
  USERPROFILE, XDG_CONFIG_HOME, GIT_CONFIG_GLOBAL, GIT_CONFIG_SYSTEM differ) or when
  explicit ignore files are given (`ignore` matches those against the walk's own,
  absolute, spelling of paths).
- **Virtual** otherwise: `ignore`'s traversal and ignore chain re-implemented over
  `WalkHost` (`SyncCtx` implements it), with `ignore`'s `gitignore`/`overrides`/`types`
  matchers; the chain is adapted from `ignore/src/dir.rs` (attribution in NOTICE). Paths
  are matched as `ignore` matches a walk rooted at the user's spelling; parents are
  canonicalized lexically (no realpath in the fs view); the global excludes file is
  found from the shell's environment (`search::gitconfig`). Hidden means a leading `.`
  only (no Windows attribute in the fs view); loops are found by (dev, ino) where the
  host has them.

## D1115: wasm ABI 3: the D1150 file system in the JS host
date: 2026-11-07 · by: infra · issue: 0011
D1150's Host operations reach JS through three host_call kinds. `K_FS` (async, like
`K_OPEN`) carries every change and `stat_extra`, with the operation in `arg` (`FS_MKDIR`
… `FS_SET_TIMES`) and length-prefixed paths and u32s in the payload; times travel as
decimal text so nanoseconds stay exact, and `stat` now answers integer nanoseconds too.
`K_NAME` (user/group name, answered in the input buffer like `K_NOW`) and
`K_UTC_OFFSET` (`-getTimezoneOffset()*60` at the instant) are synchronous. MemHost
implements them over MemFs with POSIX semantics (rename's replace rules and EINVAL into
itself, EISDIR/ENOTDIR/ENOTEMPTY, hard links share the node and count `nlink`, ctime
and the parent's mtime move with changes; times set exactly keep their nanoseconds);
`userName`/`groupName`/`utcOffset` are methods an embedder may override (default: uid
and gid 0 are root, 1000 is user; the local zone). EBUSY and EXDEV join the errno
table. psh-spec's wasm driver now writes back directories made in memory (only lazy
mounts nobody opened are skipped), which the D1150 cases need.
## D1310: The recording image has git, ripgrep and jq; .git is not compared
date: 2026-11-07 · by: shapes · issue: 0031
The goldens image (`docker/Dockerfile`) installs git 2.47, ripgrep 14.1 and jq 1.7 at
pinned Debian versions: agents run them in about a quarter of their commands, and L3
must cover those commands. Cases using them build everything they need locally (git with
a fixed identity and dates). psh-spec leaves `.git` and bare `*.git` directories out of
the file snapshot it compares (index mtimes and git-version defaults are git's business,
not the shell's). Tools the image lacks (node, python, npm, make, cargo, gh, …) appear as
stub scripts in the case's `bin/`, so the agent's command keeps its exact shape. If
Debian drops a pinned version, rebuilding needs snapshot.debian.org or a new pin and a
re-recording.

## D1311: L3 coverage is measured against the shape corpus
date: 2026-11-07 · by: shapes · issue: 0031
`tests/agent/coverage.py` turns every run of consecutive command lines of every agent
case into a skeleton (scripts/shape-corpus's rules) and matches it with the committed
shapes at three levels: exact (placeholders unified), args (argument lists collapsed,
tool subcommands kept) and syntax (every word a word). "args" is the reported number:
the share of the commands the shapes stand for that have a case with the same command
structure (49.7% with 701 cases). The script reads only committed files.
## D1305: psh-spec checks every case for leaked processes
date: 2026-11-07 · by: procs · issue: 0030
In-process cases use a host with `UnixConfig::detached_survive` off (new; default on):
`nohup`/`setsid` jobs are recorded and killed with the run's leftovers too. When the
shell and the host are gone (TERM, KILL after the grace period), the case's recorded
groups must empty within the grace plus 1 s; what is still alive is killed and the case
fails with `leak` (`RunOutput::leaked`, shown by `--diff`). Cases run in the `psh`
binary: the runner already kills the binary's process group after it exits; a group
that does not empty within 2 s fails the case likewise. Out of reach: a process that
calls setsid itself (a session of its own is invisible to group checks), and, in the
binary, `nohup`/`setsid` jobs (psh detaches them as designed). Windows: no check yet.
## D1272: `[[ ]]` evaluates to statuses; quoted right sides are escaped part by part
date: 2026-11-07 · by: parser · issue: 0027
`crates/psh-engine/src/cond.rs` runs `[[ ]]`. Each primary yields a status, as in bash:
0 true, 1 false, 2 an error (invalid regex, `-t x`); an arithmetic error in `-eq` and
friends is 1. `!` maps any non-zero status to 0, `&&`/`||` test zero only, so
`[[ a =~ ( || x ]]` is true after its error message. The right side of `==`/`!=`/`=~`
is expanded part by part: literal text and expansions are live pattern/regex text,
quoted parts (`'…'`, `"…"`, `\c`, `$'…'`) are escaped (pattern: `*?[]\@!+()|`; regex:
`$()*+.?[\^{|`, which leaves GNU's `\'`/`\<` escapes untouched), and a leading tilde
prefix is literal only when it expands. `==` matches as if extglob were on. `=~` is
psh-regex in extended mode (icase under nocasematch, UTF-8 by locale); `BASH_REMATCH`
becomes an indexed array of the match and its groups (unset groups empty), empty on no
match, untouched with an error on an invalid regex or when readonly/associative. `set
-x` prints each primary as evaluated (`+ [[ -n a ]]`, `! ` under negation), with the
right side escaped as bash shows it. The unary and file/string binary tests are shared
with `test` (`builtins::test::{unary, binary}`).
## D1161: cut -c counts UTF-8 characters in every locale
date: 2026-11-07 · by: text · issue: 0016
`cut -c` selects characters: valid UTF-8 sequences are one position, other bytes one
each, whatever the locale. `-b` (with or without `-n`) stays bytes. GNU's `-c` counts
bytes (it splits `é`), BSD's counts characters in a UTF-8 locale; psh takes the BSD
meaning everywhere, a deliberate difference from GNU. Reason: agents mean characters
(`cut -c1-80` to shorten lines), and a split character breaks the next tool's UTF-8.

## D1162: tr works on bytes unless a set has a non-ASCII character
date: 2026-11-07 · by: text · issue: 0016
When both sets are ASCII (escapes included, `'\200-\377'` too), `tr` works on bytes,
exactly as GNU's (and BSD's in the C locale): `tr -c 'a-z' _` turns `é` into two `_`.
When a set contains a non-ASCII character, input and sets are UTF-8 characters (bytes
that are not valid UTF-8 stay single units), as BSD's `tr` in a UTF-8 locale: `tr 'é' e`
means the character, where GNU would map its two bytes separately and corrupt other
characters. Classes (`[:alpha:]`) are the C locale's in both modes. Reason: GNU's
behavior wherever it is sane; the character meaning where GNU's output is garbage.

## D1160: sort collates by bytes in every locale
date: 2026-11-07 · by: text · issue: 0016
Settles D034's open point. psh's `sort` (and `comm`, `uniq`, which compare as it
orders) compare bytes, as `LC_ALL=C sort` does, whatever `LANG`/`LC_ALL` say; for UTF-8
text that is code point order. `-f`, `-d`, `-i` and `-M` use the C locale's (ASCII)
classes. Evidence: for `b A a B _x -y ä z` in en_US.UTF-8, GNU (glibc) gives `a A ä b B _x
-y z`-like dictionary order, macOS's sort gives `_x -y a A ä b B z`, and both give `-y A B
_x a b z ä` in the C locale; the recording container and most CI run in C or C.UTF-8
(byte order) already. Locale collation is the only setting that is neither portable nor
deterministic, and the common agent uses (`sort | uniq -c | sort -rn`, `sort -u` of
paths, `sort -n -k`, `comm` of two sorted lists) depend on consistency between tools, not
on dictionary order. Rejected: falling through in non-C locales (the output would differ
by OS, which is what the utilities exist to avoid). Known cost: `sort` in en_US.UTF-8
orders `B` before `a`, unlike GNU and BSD there; a real `join` or `comm` (not psh's) in
such a locale may call psh's output unsorted.

## D1330: The gate type-checks for Linux
date: 2026-11-07 · by: shapes · issue: 0033
The gate runs on macOS and Windows, so `cfg(target_os = "linux")` code was never
compiled: main stopped building for Linux (`[bool; 65]` has no `Default`) and only a
Docker build noticed. Leg `linux` (`scripts/gate.d/72-linux.sh`) runs `cargo check
--workspace --all-targets` for `aarch64-unknown-linux-musl` and `x86_64-unknown-linux-musl`:
seconds, no linker or C toolchain, both targets installed with rustup. Running the spec
cases on Linux needs Docker and stays a manual step (`scripts/spec-linux`, 0033).
## D1306: Blocking reads give way to signals
date: 2026-11-07 · by: procs · issue: 0030
Extends D1903 to the waits of the shell's own vproc that read: an open that waits (a
FIFO) and a command substitution's output give way to a signal whose default action
ends the shell (the open fails, the substitution yields `128+n`; the shell ends at its
next safepoint); `read` runs the trap of a trapped signal while it goes on reading, as
bash does outside POSIX mode, and a fatal one ends the shell.
## D1913: Windows HostInfo in Git Bash's forms
date: 2026-11-07 · by: windows · issue: 0007
As D1301 proposed: `OSTYPE` `msys`, `MACHTYPE` `x86_64-pc-msys` / `aarch64-pc-msys`,
`HOSTTYPE` the arch, so agents' scripts take their Windows branch; uid, euid and groups
1000 (Windows users have no numeric ids worth reporting); the hostname from
`GetComputerNameExW` (DNS host name); `PPID` from a process snapshot, or, embedded (a fake
`$$`), the embedding process.

## D1914: psh-spec on Windows checks a windows platform ratchet; L4 runs in the VM
date: 2026-11-07 · by: windows · issue: 0007
Run on Windows (in-process psh), psh-spec checks `tests/<layer>/expected-failures.windows.txt`
on top of the shared ratchet, which it never changes there, the way `--wasm` does
(D1113). The in-process engine wraps the OS host to note the commands a case looked up
and never found, so a case that needs a program Windows lacks (`seq`, `yes`, `sh`,
`grep` until psh has them) may fail without a line; every other difference must be
listed with its reason. The Windows gate runs L4 (`--layer utils`) this way as its
`spec` leg: today 56 of 73 cases pass and the other 17 need external programs, with no
other difference, so there is no windows ratchet file yet. The other layers follow when
their Windows differences are triaged.

## D1915: Windows file system changes
date: 2026-11-07 · by: windows · issue: 0007
D1150's operations on `WindowsHost`, making up for what scripts would notice: a read-only
file is removed like any other (on Unix the directory decides, not the file); a
directory replaces an empty directory in `rename`; `remove_file` removes a symlink or
junction to a directory as the link, never following it; `symlink` stores the target
with `\`, makes a directory symlink when the target is a directory, and, when Windows
refuses for lack of the privilege (neither Developer Mode nor administrator rights),
makes a junction to a directory and gives a clear error for a file, never a copy;
`read_link` returns the stored target with `/` (a junction's without `\??\`). `mkdir`
ignores the mode (the read-only attribute does not protect a directory's contents);
`set_mode` maps the write bits to the read-only attribute (D1911). `stat_extra` gives
access and creation times, `ctime` = mtime, links from the file information, 512-byte
blocks. Files are owned by uid/gid 1000, the user `HostInfo` reports; `user_name(1000)`
and `home_dir` know the current user. `utc_offset` follows the system's time zone rules.
psh-spec writes the case's cwd and recognizes its paths in the `C:/x` spelling psh uses.
## D1163: Utility messages quote values as GNU does for the locale
date: 2026-11-07 · by: text · issue: 0019 (for 0016, coordinator's rule)
Values and operands in utility messages (`invalid field value ‘x’`, `extra operand ‘c’`,
`invalid argument ‘x’ for ‘--sort’`) are quoted by one helper, `common::quote(ctx, s)`
(`quote_in(utf8, s)` where no context is at hand): `‘x’` when the locale's character set
is UTF-8, `'x'` otherwise. The character set is GNU's test (LC_ALL, then LC_CTYPE, then
LANG, as `UtilCtx::utf8`); LC_MESSAGES only selects translations, which psh has none of.
What GNU prints literally stays ASCII: getopt's `invalid option -- 'x'`, `Try 'NAME
--help'`, fixed texts (`'upper' and 'lower'`, `after '.'`), and messages GNU quotes with
plain apostrophes (`tr`'s `range-endpoints of 'z-a'`, `sort`'s `extra operand 'b' not
allowed with -c`). File names in `NAME: FILE: reason` stay unquoted, as GNU's are unless
they need shell quoting. psh's own texts (`unsupported option '--x' (psh built-in
NAME)`) stay ASCII. `fsutil::q` is this helper. Applied to the text filters and to
head, tail, sleep, timeout and wc; other utilities follow it when they are next touched.
Reason: the stderr score against GNU's goldens (recorded in C.UTF-8), and one rule
instead of one per batch.
## D1325: `patsub_replacement`: the replacement is expanded with `\` and `&` marked
date: 2026-11-08 · by: engine · issue: 0032
With `patsub_replacement` (on by default) the replacement of `${x/p/r}` is expanded as
one word whose quoted `\` and `&` are backslash-escaped; then an unescaped `&` is the
matched text and `\&`, `\\` are the char, while other backslashes (also those an
unquoted expansion yields, `r='\$1'`) stay, as bash does. `nocasematch` applies to
`${x/…}` but not to `${x#…}`/`${x%…}` (bash).

## D1326: `{var}>` redirections persist; restores apply them again
date: 2026-11-08 · by: engine · issue: 0032
`{var}>file` takes the lowest free descriptor from 10, assigns its number to `var` after
the redirection succeeded, and outlasts the command (bash without `varredir_close`).
`{var}>&-` closes the descriptor `var` holds. Each such change is recorded in
`Vp::var_fds`; restoring a command's saved descriptor table (`restore_fds`) applies them
again, so the rest of the command's redirections are undone as before.

## D1327: `${ list; }` runs in the shell with a reader task
date: 2026-11-08 · by: engine · issue: 0032
Bash 5.3's function substitution runs its list on the current vproc with stdout set to
an engine pipe; a scheduler task reads the pipe meanwhile, so output beyond the pipe's
capacity cannot deadlock. `${| list; }` captures nothing and yields `$REPLY`, which is
local to it (unset at the start, the old value restored after).

## D1916: The Windows VM stays up between gates for a while; every run keeps its output
date: 2026-11-07 · by: windows · issue: 0007
Instead of suspending the VM as soon as the last queued run ends (D1072), `windows-gate.sh`
records when a run last used it and starts one detached watcher (in a process group of
its own, so a gate's timeout does not take it along) that, once no run used the VM for
`PSH_WINDOWS_IDLE` seconds (default 600; 0 restores the immediate suspend), takes
`run.lock`, checks that nobody is queued and that a run woke the VM, and suspends it.
Gates come in bursts, so this saves the resume, the readiness wait and the cold caches
after a resume for most runs, and leaves fewer VM transitions to go wrong; the
`PrlJob_GetRetCode` failures themselves also happen with the VM running for hours, so
they keep their own remedy (the result file and one retry, from a settled VM). Every
run's whole output goes to `work/scratch/windows-gate/logs/NAME-TIME-PID.log` in the
primary checkout (the newest 40 kept), and the runner copies a failing leg's log from the
VM next to it (`...-LEG.log`), so a failure can be read afterwards even when prlctl lost
the stream; a failing run prints the path.
## D1904: One signal numbering everywhere: Linux's
date: 2026-11-07 · by: procs · issue: 0030
Coordinator decision, recorded here. psh numbers signals as Linux does on every platform,
macOS and Windows included: `kill -l`, `trap -l`, `kill -10`, `$?` = 128+n for a death by
a signal all use Linux numbers, as the engine and the goldens do (D1283); psh-os maps them
to the OS's own at its boundary (sending, `sig_ignore`, decoding wait statuses: on macOS
SIGUSR1 is 30 natively and 10 in psh). Reason: portable and consistent rank above exact
host-bash parity (D033), and agents' scripts behave the same everywhere. A signal the
system lacks (SIGSTKFLT, SIGPWR, the real-time ones on macOS) cannot be sent: `kill`
says `kill: SIGSTKFLT: no such signal on this system` (status 1); to a job without an
OS process (a virtual pid) it is still delivered. `kill $$`, `kill 0` and `kill -- -$$`
reach the shell's pending mask (D1284), the latter two also the rest of its group where
the host may signal it.

## D1191: UtilCtx v3: spawn_shell runs a command string in a fresh psh shell
date: 2026-11-08 · by: text · issue: 0019 (coordinator-approved)
`UtilCtx::spawn_shell(script, ShellStdio { stdin, stdout }) -> ShellChild`
(`UTIL_API_VERSION` 3, additive) is what a utility uses where the real one runs
`/bin/sh -c SCRIPT`: `awk`'s `system()`, `"cmd" | getline` and `print | "cmd"`, `sed`'s
`e`, `find`/`xargs` helpers that take a shell string. The shell is psh, on the same
scheduler, never `/bin/sh`, so the command behaves the same on every platform and in WASM.
Semantics are `sh -c`'s, through the code path of D1300 (`exec::start_afresh`, shared with
running a file the host cannot start): only exported variables (the utility's environment,
its command's prefix assignments included), no functions, aliases, traps or options; `$0`
is `sh` and errors use `-c` wording; cwd, stderr and every other descriptor are the
utility's. `ShellIo::Inherit` gives it the utility's own stdin or stdout, `ShellIo::Pipe`
an engine pipe whose other end is `ShellChild::stdin` / `stdout` (`UtilFile`s: async
`write_all`, `read`; dropping one closes it; externals in the shell get OS ends as for any
pipe, D1068). The utility's stdout is flushed first. The shell runs as its own task from
the call on (not when awaited, unlike `spawn`'s `UtilChild`), so a utility can feed its
stdin and read its stdout meanwhile. `ShellChild::wait()` closes stdin and returns the
status; dropping an unfinished child drops its task (cancel: its externals get TERM, then
KILL). Reason: a second fresh-shell mechanism would drift from D1300's; `spawn` runs an
argv, not a command string, and only when awaited.
## D1151: type reports the utility that runs, not a shadowed PATH file
date: 2026-11-07 · by: files · issue: 0015 (coordinator's call)
When an S-mode utility shadows a program on PATH (`mv` with `/tmp/mv` on PATH), `type`,
`type -p`, `type -f` and `command -v` report the utility, because that is what runs;
printing the PATH file would tell an agent the wrong program runs. `type -P` is bash's
forced PATH search and prints the PATH file, as bash does for real builtins. Two Oils
cases that expect `type -p`/`-f` to print the file (builtin-type-bash 8 and 16, written
for bash where `mv` is no builtin) are ratcheted with this reason.

## D1331: Shared ratchets hold Linux's failures; platform files and host-specific cases
date: 2026-11-07 · by: shapes · issue: 0033
The goldens come from Linux, but the gate runs on macOS, where BSD utilities and the
host's own layout made a sixth of the shared ratchet entries macOS-only (and hid two
Linux failures). Now `expected-failures.txt` is what fails on Linux; each platform
(`macos`, `windows`, `wasm`) adds `expected-failures.<platform>.txt` for cases that fail
only there because its *programs* differ, with reasons. psh-spec checks the shared file
plus the file of the OS it runs on. Cases that depend on the *host itself* (`/dev/full`,
`/proc`, the `/bin` layout, `/private/tmp`, root's home, signal numbers, the time zone
database, APFS file names) are tagged `[[platform]]` in tests/spec.toml: gated on Linux
only, reported apart elsewhere, in no platform file. The Linux truth comes from
`scripts/spec-linux` (Docker: psh built in the pinned Rust image, run with `--init` in the
recording image, repository read-only), whose `--results` file `--linux-results` feeds
to `--update-ratchet` (moves between the files) and `--add-failures` (picks the file).
It is a manual step; the gate type-checks Linux (D1330) but runs no Linux cases, since
it would need Docker and two minutes.

## D1332: `command -p` searches the host's standard path
date: 2026-11-07 · by: shapes · issue: 0033
bash's `command -p` uses the system's standard `PATH` (`confstr(_CS_PATH)`):
`/bin:/usr/bin` on Linux, `/usr/bin:/bin:/usr/sbin:/sbin` on macOS. psh used its own
default `PATH`, which on merged-/usr Linux finds `/usr/bin/cat` where bash finds
`/bin/cat` (hidden on macOS, which has only `/bin/cat`). `Host::standard_path()` gives
it: a default of `/usr/bin:/bin:/usr/sbin:/sbin`, `confstr(_CS_PATH)` in psh-os on Unix;
Windows keeps the default until its host overrides it. `command -p NAME` runs, and
`command -pv`/`-pV` report, the program found there (not a psh utility, and a hashed
program still wins, as in bash).
## D1328: UID, EUID, PPID, GROUPS and the host names come from `Host::info()`
date: 2026-11-08 · by: engine · issue: 0032
Read once when the shell starts (`Host::info()`, D1301), as bash sets them:
`UID`/`EUID` readonly integers unless the environment has them (then the inherited
exported value wins, as in bash), `PPID` a readonly integer always (left alone when the
host reports no parent), `HOSTNAME`, `OSTYPE`, `MACHTYPE`, `HOSTTYPE` plain variables
unless inherited. `GROUPS` is a dynamic indexed array refreshed from the host on each
read, so assignments to it have no effect; unset, it becomes an ordinary variable.

## D1917: Windows standard PATH
date: 2026-11-07 · by: windows · issue: 0007
`WindowsHost::standard_path()` (D1332, `command -p`) is Windows' own default search path
of a fresh account: `%SystemRoot%\system32`, `%SystemRoot%`, `System32\Wbem` and
`System32\WindowsPowerShell\v1.0`, `/`-separated and joined with `;` (which
`path_entries` splits). The user's directories (npm, cargo, ...) stay out on purpose:
`command -p` means the system's programs whatever `PATH` says.

## D1918: psh-spec's leak check on Windows
date: 2026-11-07 · by: windows · issue: 0007
As on Unix (D1305): psh-spec's in-process Windows host has `detached_survive: false` (a
new `WindowsConfig` field, true by default, as `UnixConfig`'s), so `nohup`/`setsid`
externals' jobs belong to the case's run too, and after the host is dropped (a
ctrl-break to the run's jobs, termination after the grace period) any job with a live
process left after the grace period plus a second is the case's leak: it is killed and
reported, and the case fails. The jobs see grandchildren that left their parent, which
the Unix group check sees only while they stay in the group.

## D1919: Windows execute bits and read-only directories are not emulated
date: 2026-11-07 · by: windows · issue: 0007
On Windows a file's execute bits come from the file itself: set for directories, files
with a `PATHEXT` extension and `#!` scripts (D1076), as the lookup decides what runs.
`chmod +x`/`-x` changes nothing there and psh keeps no side record of them (an alternate
data stream or a table would be invisible to every other program, lost by copies,
archives and git, and could disagree with what actually runs). `chmod` on a directory
sets its read-only attribute, which Windows does not enforce for the directory's
entries, and psh does not emulate Unix's protection either (only psh would honor it).
Utilities that report modes should report what `stat` shows afterwards, so a request
Windows cannot keep is not described as done. Reviewed against
`tests/utils/expected-failures.windows.txt` (27 cases): file symlinks without Developer
Mode fail with "Permission denied" and nothing is created; directory links are junctions
with absolute targets that work as links; the two behavior differences are `chmod -v/-c`
reporting x-bit changes that do not stick (utility follow-up) and `rm` succeeding in a
read-only directory (this decision).
## D1170: xargs runs no command for empty input; GNU's statuses and stdin
date: 2026-11-07 · by: procs · issue: 0017
`xargs` follows BSD on empty input (the D016 question left open): when the input holds no
items, the command does not run at all, and `-r` (`--no-run-if-empty`) is accepted and
changes nothing. GNU runs the command once without arguments, which turns `find … |
xargs rm` with no matches into a usage error from `rm`, a trap for agents. The rest is
GNU's: blanks, newlines, quotes and backslashes split items (`-0`/`-d` a single byte,
literally); `-I` takes whole lines with leading blanks dropped; `-L` lines where a
trailing blank continues the line; `-s` 128 KiB by default; statuses 123 (a command
exited 1–125), 124 (255, which also stops xargs), 125 (a signal death), 126/127; the
commands' stdin is `/dev/null` (`/dev/tty` with `-o`, xargs's own with `-a`). For this
`SpawnEnv` gained `stdin` (a file opened as the child's stdin) and `cwd` (the child's
working directory, for `find -execdir`). Prompting (`-p`), `--show-limits`,
`--process-slot-var` and BSD's `-J -R -S` fall through to the real program.

## D1171: find walks on its own over the async file system, sorted by name
date: 2026-11-07 · by: procs · issue: 0017
`find` does not use the walker seam (D1132). Its actions interleave with the walk:
`-exec` and `-execdir` run commands through `UtilCtx::spawn` between two steps of the
walk, `-prune` and `-quit` steer it, `-delete` needs the contents before the directory.
The seam is synchronous and runs on the blocking backend, which on the inline backend
(wasm, sandbox hosts) is the engine thread: it cannot wait there for an async command.
With all ignore filtering off, a walk is `read_dir` plus `lstat`/`stat`, so find has its
own (~150 lines) over `UtilCtx`, the same on every host. Order: depth first, each
directory's entries sorted by name (GNU walks in directory order, which is not stable
across file systems, so goldens sort). `-L` detects loops by device and inode of the
directories above (GNU's message, status 1); `-H` follows the start paths only.

## D1172: find and timeout are mode S everywhere (supersedes D015 for them)
date: 2026-11-07 · by: procs (coordinator decision) · issue: 0017
`find` and `timeout` were W (the builtin on Windows, the real program first elsewhere).
They are now S: the builtin runs on every platform, and an option it does not implement
still falls through to the real program (D010). Agents on macOS write GNU `find`
(`-printf`, `-regextype`, `-type f,l`) which BSD find rejects, and psh's `timeout` stops
the whole process tree and behaves the same everywhere; a find and a timeout that act
the same on every platform is the agent-first choice. `sort` was already S. A
consequence: `type -t find` says `builtin` (D1083), two Oils cases expect `file`.
## D1329: Well-known variables bash sets at startup; `$(…)` CRLF follows the host
date: 2026-11-08 · by: engine · issue: 0032
Set when the environment has none: `PATH` (bash's default list; not on Windows, whose
PATH conventions differ), `PS4="+ "`, `TERM=dumb`, `COMP_WORDBREAKS`, `OPTERR=1`,
`OPTIND` (an integer). Inherited `PS1`/`PS2` are dropped, as a non-interactive bash
does. Dynamic: `BASH_ARGV0` (`$0`; assigning sets it), `BASH_ALIASES` (the aliases;
assigning an element defines one), `BASH_MONOSECONDS`, `LINENO`. `BASH_VERSINFO[5]` is
the host's `MACHTYPE`. `unset` refuses `BASH_SOURCE`, `BASH_LINENO`, `BASH_ARGV`,
`BASH_ARGC`. A pipeline stage that is a `( … )` is that subshell itself (one
`BASH_SUBSHELL` level, as in bash). Not set: `BASH`, `SHELL` (the host reports no
executable path or login shell), `HISTCMD`, `DIRSTACK`, `BASH_CMDS`, `BASH_COMMAND`.
The CRLF rule of command substitution moves from `cfg!(windows)` to
`Host::strips_crlf()` (D1292) when that method lands, superseding D1324's "Windows
builds only".
## D1133: rg is ripgrep's crates/core with the seams patched
date: 2026-11-07 · by: infra · issue: 0013
`crates/psh-utils/src/rg/` is ripgrep 91f5d22 (15.2.0): `flags/` whole, `main.rs`,
`search.rs`, `haystack.rs`, `messages.rs`, and lexopt 0.3.2, every change marked `psh:`.
Small stand-ins replace the crates ripgrep uses there: `anyhow.rs` (a chain of messages
that keeps an I/O error's kind), `textwrap.rs` (textwrap 0.16.2's first fit, for
`--help`), `cli.rs` (grep-cli's size and pattern-file helpers; stdout as a termcolor
stream over the utility's `SyncWriter`). The module is formatted in ripgrep's style
(`rg/rustfmt.toml`, `#[rustfmt::skip]` on the module; `rustfmt --config-path` by hand), so
it stays a reviewable diff from upstream. The seams: the front end parses argv (with
`RIPGREP_CONFIG_PATH` read through the utility's file system), prints help and version,
and runs the search as a blocking utility (D1130); message and error state is a
thread-local installed for one run (search threads never print messages), written to the
utility's stderr; no `process::exit`, no logger (`--debug`/`--trace` run the real rg).
cwd, environment, TTY (headings, line numbers, color `auto` decided by TERM/NO_COLOR of
the utility's environment, ANSI or none) and the stdin heuristic come from `SyncCtx`. The
walk is the walker seam (D1132) with `walk_builder`'s settings; it is always sorted by
name. More than one thread only natively: the walk stays on the rg thread, haystacks go
to scoped search threads with a buffer each, output is written in walk order (at most
4×threads ahead), so it equals the single-threaded output. Errors are worded as ripgrep's
parallel walker words them when real rg would use it (more than one thread). Memory maps
are off (`MmapChoice::auto` is unsafe). Real rg runs (D1082) for `--pre`, `-z`, `-P`,
`--engine auto`, `--json` (until psh's JSON sink) and `--debug/--trace`. Windows: `/` is
the default path separator and CRLF a line end unless `--crlf`/`--no-crlf` is given
(D030). The version is `ripgrep 15.2.0 (psh)`; `--help` is upstream's otherwise. The
classifier's rg table is checked against ripgrep's flag table (missing value flags such as
`--pre`, `--engine`, `--type-clear` added). On wasm32-unknown-unknown rg reports that it is
unavailable: grep-printer reads `std::time::Instant`, which panics there (a patched
grep-printer follows).

## D1134: The recording image has the upstream rg 15.2.0 and fd 10.5.1 binaries
date: 2026-11-07 · by: infra · issue: 0013 (supersedes the ripgrep part of D1310)
The oracle for rg and fd is the version psh implements: `docker/Dockerfile` downloads the
upstream static musl release binaries of ripgrep 15.2.0 and fd 10.5.1 for the image's
architecture (arm64, amd64), each pinned by sha256, into `/usr/local/bin`; Debian's
ripgrep 14.1 is gone. Re-recording the agent layer with it changed no golden.

## D1135: grep-printer is patched in-tree for a clock on wasm
date: 2026-11-07 · by: infra · issue: 0013 (supersedes D1133's "rg is unavailable on wasm")
grep-printer 0.3.1 calls `std::time::Instant::now()` whenever a sink is created or a
search begins (for `Stats`' elapsed time), and that panics on wasm32-unknown-unknown, so
no printer works there. `third_party/grep-printer/` is the crates.io package of 0.3.1
(first commit verbatim) with one change: `Instant` comes from a new `src/instant.rs`,
std's natively and a clock that reads zero elapsed time on wasm32-unknown-unknown
(`PATCHES.md` lists every changed line). It replaces the registry crate through
`[patch.crates-io]`; `third_party` is excluded from the workspace, so fmt and clippy keep
the upstream code as it is (rustc still shows its two dead-code warnings on wasm, which
upstream has too). The crate keeps its name, so the dependency allowlist is unchanged.
rg's own `--stats` start time is likewise zero there. The copy goes when upstream stops
reading the clock unconditionally (an upstream issue text is in 0013's log).
## D1200: `which` prints the program's path first; utility-only names say so
date: 2026-11-07 · by: shapes · issue: 0020
`which NAME` prints the first executable on `PATH` even when psh runs NAME as a utility,
and `NAME: shell builtin` only when no program exists (wasm, Windows without the tool);
`-a` lists the utility line first, then every `PATH` hit. Reason: `$(which env)` and
`command $(which ls)` are common idioms (Oils and brush cases use them) that need a path
that runs; ARCH's "report utilities truthfully" stays true for `type` and `command -V`,
which name the utility. GNU (debianutils) and BSD agree otherwise: `-s`, status 1 when a
name is not found or none is given.

## D1201: Checksums in-house, each name with its native format
date: 2026-11-07 · by: shapes · issue: 0020
MD5 (RFC 1321), SHA-1 and SHA-2 (FIPS 180-4, including SHA-512/224 and /256 for
`shasum -a`) are ours (`sum_hash.rs`, no dependency), tested against the specifications'
vectors. `md5sum sha1sum sha224sum sha256sum sha384sum sha512sum` follow GNU (escaped
names, `--tag`, `-z`, `-c` with GNU's messages, quoted names, `--quiet --status --strict
-w --ignore-missing`); `md5` follows macOS (`MD5 (f) = h`, `-q -r -p`, `-c HASH`
mismatch is status 2; `-s` prints the bare hash, as macOS 26 does); `shasum` follows the
perl script (`-a`, and `-c` takes each line's algorithm from the hash's length). `-c`
reads both line formats. b2sum and cksum are left for later.

## D1202: base64 is GNU's encoder and decoder with BSD's file flags
date: 2026-11-07 · by: shapes · issue: 0020
Encoding wraps at 76 (GNU; BSD does not wrap) and `-w 0`/`-b 0` writes one line without
a newline (GNU). Decoding skips newlines, accepts missing padding and padding mid-stream,
and stops at any other byte with `invalid input` (status 1) after what decoded so far,
as GNU does (BSD also skips spaces). The D016 `-i` rule is an argv rewrite: `-i` (or a
cluster ending in it) before an option or at the end is `--ignore-garbage`, else BSD's
input file; `-o FILE`, `-D`, `-b N` are BSD's. One file operand (GNU).

## D1203: cmp and printenv follow GNU; env -C and -S
date: 2026-11-07 · by: shapes · issue: 0020
`cmp` prints GNU's `differ: byte N, line L` (BSD: `char`), `EOF on F after byte N[, line
L]` / `which is empty`, `-l` offsets right-aligned to the digits of the largest offset
that can be printed (the smaller remaining regular-file size or `-n`, else 19), `-b`
with `cat -v` spellings; usage errors are status 2. `printenv` takes several names (BSD
one). `env -C DIR` checks the directory (`cannot change directory to 'DIR'`, 125) and
runs the command there: `SpawnEnv::cwd`, set on the command's subshell. `env -S` splits
by GNU's documented rules and its words may be options again. `which` uses two new
`UtilCtx` calls: `is_utility(name)` and `search_path(name, PATH)`.
## D1192: PSH_UTILS_FORCE: the L4 cases test psh's F/W utilities
date: 2026-11-08 · by: text · issue: 0019 (coordinator-approved)
`UtilPolicy::force_names` lists names whose utility runs in-process even in
`PathFallback` and `WindowsShadow` mode when `PATH` has the program (the inverse of
`off_names`). The psh binary and psh-spec's in-process engine set it from the
environment variable `PSH_UTILS_FORCE` (`psh_utils::FORCE_VAR`): a comma-separated list
of names, or `all` for every registered F/W utility (`psh_utils::force_names`); they
take the variable out of the shell's environment, so commands never see it (the wasm
engine drops it too: a sandbox runs every utility anyway). psh-spec sets
`PSH_UTILS_FORCE=all` for the utils layer only, and only when the shell under test is
psh (a recording bash would export it). So every utility's L4 cases test psh's
implementation, not the host's program, whatever its mode. Other layers are unchanged.
Reason: in F mode the spec runner's PATH (/usr/bin/awk) would answer the awk cases.

## D1193: awk: gawk's behavior where gawk and BSD awk differ; for-in in insertion order
date: 2026-11-08 · by: text · issue: 0019
psh's `awk` (a port of GoAWK, F mode for now) is POSIX awk with the gawk extensions
agents use. Where gawk 5.2 and BSD awk (macOS's onetrueawk 20200816) disagree, it does
what gawk does (D016, GNU on conflict); the vendored suites list 83 such cases
(tests/vendor-awk/README.md): syntax errors exit 1 (fatal errors 2) in gawk's words,
a multi-character RS is a regex and sets RT, CONVFMT/OFMT take integer conversions,
integral numbers print with all their digits (`1e30`), `close()` returns a command's
exit status, `printf "%c", ""` is a NUL, `-Ft` is the letter t, length and substr count
characters in a UTF-8 locale. Numbers in input are decimal only (no hex), `inf`/`nan`
only with a sign. `for (k in a)` visits keys in insertion order: awk leaves the order
unspecified, gawk's and BSD's hash orders cannot be reproduced, and insertion order is
stable and what people expect. `system()`, `cmd | getline` and `print | cmd` run the
command in psh's shell (`spawn_shell`, D1191), never /bin/sh. The switch to S mode is
the coordinator's, once the suites pass.

## D035: The psh binary's own command line uses the `argument` crate
date: 2026-11-07 · by: coordinator (owner request) · issue: 0034
The psh binary parses its own modes and flags (`--parse`, `--classify`, `--help`,
`--version`, future modes) with `argument` (crates.io, Apache-2.0, depends only on
`argument-parser`): a static spec, a plain parse loop and generated help, usage and error
text. The shell language and bash's invocation rules are not affected; where `argument`
cannot express bash's invocation (`+o`, combined `-euc`), the existing invocation parser
stays for that part. Allowed in the `psh` binary crate (and the repository's tool
binaries if useful), not in library crates.
## D1136: rg --json writes grep-printer's messages through deser-json
date: 2026-11-07 · by: infra · issue: 0013
`crates/psh-utils/src/rg/json.rs` is grep-printer 0.3.1's JSON printer (its sink, message
types, base64 data and the replacement and context helpers it uses) with serde replaced:
each message is a small ordered JSON tree written by deser-json (D029), which escapes as
serde_json does, so the output is the real rg's byte for byte (begin/match/context/end in
grep-printer's field order; rg's summary with its keys sorted, as serde_json's `json!`
map sorts them). psh-utils depends on deser-json and deser-core for it. Real rg no longer
runs for `--json`.

## D1340: The psh binary's command line: `argument` for psh's own options, invocation.rs for bash's
date: 2026-11-07 · by: cli · issue: 0034
`argument` cannot express bash's invocation: `+e`/`+o NAME` are options, `-o` and `-O`
take the *next* argument even inside a cluster (`-eo pipefail`, not `-opipefail`), `-c`
is a flag whose command is the first operand (`-c -- CMD`, `-euc CMD`), and `-h` is
`set -h`. So the split is by position, as bash's own rule allows (GNU long options come
before the single-letter ones): `crates/psh/src/cli.rs` runs `argument` over the leading
`--name` arguments only. bash's long options (`invocation::LONG_OPTIONS`: `--norc`,
`--noprofile`, `--noediting`, `--posix`) are passed on untouched; any other `--name`
is psh's own (`--parse`, `--classify`, `--help`, `--version`, later modes such as
`--utils`) or an `argument` error with a suggestion (`--pars` → `--parse`). The first
argument that is not a `--name` (an option, `--`, `-`, an operand) and everything after
it go to `psh_engine::invocation::parse_args` unchanged, which the conformance runner
shares; its errors keep bash's wording (`psh: -Z: invalid option`) followed by the
generated usage. `--parse` and `--classify` are `argument` sub-specs (`psh --parse
--help`, `-h` works there) with `-c COMMAND`, `--compact`, `-O extglob` and `FILE`. The
top-level help lists bash's options for display only; hidden `-h` and `-V` entries keep
`argument` from showing them as help/version. Values stay `OsString` (bytes on Unix).
`--help` exits 0 on stdout now (as bash), not the old usage on stderr with 2. Size: the
release binary grows by about 110 KB (8.87 MB → 8.99 MB on macOS arm64).
## D1920: The whole psh-spec runs on Windows in the gate
## D1930: The whole psh-spec runs on Windows in the gate
## D1920: The whole psh-spec runs on Windows in the gate
date: 2026-11-07 · by: windows · issue: 0007
The Windows leg's `spec` runs every layer (L1 Oils, L2 brush, L3 agent, L4 utils) in the
VM against the shared goldens and `tests/<layer>/expected-failures.windows.txt` (D1331,
D1914): about 85 s with the build, so it stays in the gate. A case that needs a program
Windows lacks may fail without a line (the in-process engine notes lookups that find
nothing and `#!` interpreters that are missing; a shell binary's stderr "command not
found"). Every other Windows-only failure is listed with its reason, by kind: Windows
file names (no `* ? | < > " \ :`, no trailing dot), no file symlinks without Developer
Mode and junctions for directories, no execute or permission bits beyond read-only
(D1919), no sticky/setuid/setgid, descriptors above 2 not passed (D1074), no Unix file
system layout (`/bin/sh`, `/etc`, `/usr`), drive-rooted paths (`HOME` is `C:/...`), no
SIGUSR1 to deliver, Oils' python2 helper; psh bugs are listed with their owner until
fixed. For a fair comparison psh-spec on Windows gives cases their paths in psh's
spelling (`C:/x`, no `\\?\`), treats exit `128+n` of a shell binary as death by signal `n`
(nothing dies by a signal on Windows), and the host emulates what scripts rely on:
`/dev/zero`, `/dev/urandom` (system random bytes), character-device `stat` for the
`/dev` names, "Is a directory" for reading a directory, a directory size of 4096, a
missing program as 127 before any interpreter guess, `python3` falling back to `python`
(what Windows installs). prlctl's exec failures that never started the runner (seen in
runs of up to half of all execs) are retried up to five times.

## D1341: `psh --utils` asks a shell what runs; an unreadable script is 126
date: 2026-11-07 · by: cli · issue: 0034
`psh --utils [--json]` lists every name of `psh_utils::REGISTRY` with its entry's first
name, mode (S/F/W), destructive flag, what a command of that name runs here (`psh`,
`program`, `builtin`) and the program `PATH` has for it. It does not re-implement the
lookup: it starts a shell configured exactly as the standalone shell (one
`shell_config()` for both: environment, `PSH_UTILS_FORCE`, Windows env and cwd) with
stdout captured in memory, runs `type -a -- NAME` for each name and reads the lines
(`is a psh utility`, `is a shell builtin`, `is PATH`; the first is what runs). So the
answer is the engine's own lookup and cannot drift from it. The JSON is
`{"utilities": [{name, utility, mode, destructive, runs, path|null}]}`.
A script operand that cannot be read is 126 unless it does not exist (127), as bash
(`psh /tmp` was 127).

## D1342: merge-to-main holds the merge lock from the start, on every attempt
date: 2026-11-07 · by: cli (coordinator request) · issue: 0034
`scripts/merge-to-main.sh` takes the merge lock before its rebase and holds it through
the gate to the fast-forward, on the first attempt (it used to hold it only for the
fast-forward, and throughout only after losing two races, D1010). Merges are serialized:
a waiting merge prints `merge lock: waiting for the merge lock (held by WHO since TIME)`
(from the lock's owner file; again every 5 minutes) and rebases only once the holder has
landed, so a gate is never wasted on a `main` that moved meanwhile. A commit on `main`
made without the lock still makes it rebase and gate again, keeping the lock. The
dirty-primary wait (D1011) stays: the merge releases the lock so the coordinator can
commit through `scripts/with-merge-lock.sh`, then takes it again. The cost: a
coordinator commit waits for a running gate, and merges queue; merge-to-main's lock wait
(`PSH_MERGE_LOCK_WAIT`) defaults to 4 hours, with-merge-lock's stays 1 hour. The
lost-race counter is gone. (D1341 is `psh --utils`, so this is D1342.)
## D1204: date: one parser, the command's zone, GNU ∪ BSD flags
date: 2026-11-07 · by: shapes · issue: 0020
`date` uses timefmt's date-string parser (shared with `touch -d` and `find -newermt`),
extended with days of the week (`friday`, `next friday`, `last friday`, `2 friday`; GNU
ignores them next to a calendar date, so do we), `YYYYMMDD`, US/European zone
abbreviations (`PST`, `CEST`) and a sign apart from its number (`+ 2 weeks`); no
`parse_datetime` dependency (D028's alternative). Its own GNU `strftime` (`%N`, `%:z`
`%::z` `%:::z`, `%q`, `%P`, flags and widths) is in timefmt. The zone is the command's:
`UtilCtx::load_zone` reads its `TZ` (zone file, path or POSIX rule) or `/etc/localtime`
through the engine's TZif reader (the one `printf %(…)T` uses), so `TZ=Asia/Tokyo date`
and `%Z` work; without zone files (wasm, Windows) the host's offset stays and `%Z` is
`UTC` or the numeric offset. `-u` is `UtilCtx::use_utc`. BSD: `-j`, `-f INFMT` with
`-j` (strptime; unnamed fields from now, as BSD does), `-v` (months clamp the day, as
macOS does: Jan 31 + 1 month is Feb 29), `-j [[[mm]dd]HH]MM[[cc]yy][.SS]`. `-r`:
D016's rule. `-f FILE` without `-j` is GNU's dates file. Setting the clock (`-s`, a date
operand without `-j`) runs the real program, or fails in a sandbox. GNU's quirk of
reading `10:00 -3 hours` as a zone is not reproduced.

## D1205: diff is ours: Myers in linear space, tuned against GNU's choices
date: 2026-11-07 · by: shapes · issue: 0020
`diff` is written for psh (no diffutils port: its algorithm code is GPL; uutils'
diffutils lacks `-y`, `-I`, `-B` and directory output in GNU's shape). The algorithm is
Myers' O(ND) middle-snake recursion from the paper, with two choices tuned by running
GNU diff as an oracle on 400 generated and 108 real file pairs (this repository's
history): the forward search tries diagonals from the deletion side first, and a
compaction pass slides each changed run up (merging), then down to the last position
where it meets a change of the other file, else to the bottom. psh then prints GNU's
hunks for 102 of the 108 real pairs and 343 of the 400 generated ones; the rest are
equally minimal diffs with other alignments. Past an edit distance of 2048 the split is
the furthest forward point (no longer minimal), bounding the time on very different
files. Formats follow GNU's manual and output: normal, `-u`/`-U`, `-c`/`-C` (times as
`%Y-%m-%d %H:%M:%S.%N %z`), `-q`, `-s`, `-y` (column geometry measured: right column at
`(W+11)/2` rounded down to a tab stop, half width `min(off-3, W-off)`, the gutter mark
at `(hw+off-1)/2`), `--color` (GNU's default palette), `-p`/`-F`, and `diff -r` lines
that echo the options shell-quoted. `-I` and `-F` take BREs through the search
utilities' translator; a pattern it cannot carry runs the real diff.
## D1140: sed is written for psh on psh-regex, GNU's language and messages
date: 2026-11-07 · by: parser · issue: 0014
`sed` (`crates/psh-utils/src/sed*`, mode S) implements GNU sed 4.9's language: every
command (`e` falls through to a real sed), GNU's addresses (`0,/re/`, `addr,+N`,
`addr,~N`, `first~step`, `I`/`M`), one-line `a`/`i`/`c`, `s///` with `Ng`, `p`, `w`,
`I`, `M` and `\U \L \u \l \E`, `-n -e -f -E/-r -s -z -l -u --posix --sandbox`, GNU's
error texts with their positions and statuses (1 syntax, 2 unreadable input, 4 I/O, a
missing label and "no input files" are 4). Regexes are psh-regex (D1270) with sed's
own escapes (`\n \t \dNNN \oNNN \xHH \cX`, also in brackets) resolved first; `\delim`
becomes the bare char. It is a fresh byte-oriented implementation rather than a port
of uutils sed: uutils works on `String`/chars with the regex crate and a synchronous
reader, so a port would have been a rewrite of every layer; nothing was copied, so no
NOTICE entry. Behavior was taken from GNU sed run in the recording container (probes
in `work/scratch/0014/`), including the output rule for a last line without a newline
(kept so; a newline is supplied before later output; `G`/`x`/`g` carry the hold
space's state), empty matches next to a match not counting in `s///g`, `D` keeping the
append queue, and `-s`/`-i` starting each file with fresh line numbers, ranges and
hold space. Speed is on par with the system sed on a million lines.

## D1141: `sed -i`'s two forms, as D016 states
date: 2026-11-07 · by: parser · issue: 0014
A `-i` with text attached (`-i.bak`, `--in-place=.bak`) has that suffix. A bare `-i`
looks at the next word: `''` is BSD's empty suffix; an option is not a suffix (GNU,
no backup). When the script is an operand: a word that compiles as a sed script with
more operands after it is GNU's script; a `.`-prefixed word without command
characters (`;/{}\!$=,~#`, blanks) followed by a word that compiles is BSD's suffix;
anything else is GNU's. When `-e`/`-f` give the script, a dotted word followed by an
option is BSD's suffix (`sed -i .bak -e …`), else a file. In-place editing writes a
temporary file in the file's directory with the file's mode and renames it over the
file (after renaming the original to its backup name, `*` in the suffix standing for
the file's name); non-regular files are refused with status 4, missing ones skipped
with status 2.
## D1180: jq on jaq-core with psh's own values, builtins and JSON; libm
date: 2026-11-07 · by: procs · issue: 0018
`jq` depends on jaq-core 3.1.2 only (the interpreter: parser, compiler, path updates;
with dyn-clone, once_cell, typed-arena) and libm 0.2.16 (jq's math builtins with C libm's
results, pure Rust). Not jaq-json and not jaq-std: their values, messages and number
printing are jaq's, and they pull in num-bigint, indexmap, hifijson, jiff (time zones),
regex-bites, base64 and urlencoding. psh-utils' `jq/` has jq 1.7's value model instead
(doubles that keep their literal, `1.000` and `100000000000000000000` print as read and
compare exactly; string keys in insertion order), jq's error texts (`Cannot index number
with string "b"`, values abbreviated to 11 bytes), jq's builtins (`defs.jq` adapted from
jq's own `builtin.jq`, natives for what jq implements in C), regexes on the regex crate
psh already has (Oniguruma's Ruby defaults; no look-around), dates on timefmt, and a
hand-written JSON reader and printer: jq's leniencies (`nan`, `Infinity`, leading zeros,
a byte order mark), its parse error texts and positions, its output (indent, `--tab`,
`-S`, `-a`, colors, `1e+17`, `1e-05`) are what the goldens compare, which deser-json
(D029) does not model; jq's JSON is the exception to D029. Size: +1.2 MiB stripped
natively (macOS arm64 release, 6.9 → 8.1 MB), +687 KB wasm (+210 KB gzipped). The target
is jq 1.7.1 (Debian stable's and macOS's /usr/bin/jq; `jq --version` says `jq-1.7.1`).

## D1181: jq syntax jaq lacks is rewritten before parsing
date: 2026-11-07 · by: procs · issue: 0018
jaq's parser lacks jq's destructuring alternative `?//`, `$__loc__`, surrogate-pair
escapes in string literals, and accepts only the formats it defines. Instead of forking
jaq-core, `jq/prep.rs` rewrites the program text first: `E as P1 ?// P2 | BODY` becomes
`E as $alt | . as $dot | null as $v... | try ($dot | $alt as P1 | BODY) catch ($dot |
$alt as P2 | BODY)` (every pattern variable bound, null where its pattern does not bind
it; an error in a body tries the next alternative, as jq does), `$__loc__` the object jq
gives, `@name` of an unknown format `format("name")` (an error when it runs, as in jq).
The body of an alternative extends to the end of the enclosing group (bracket, `;`,
`then`/`else`/`end`), found by a small tokenizer. Still different from jq, listed in the
upstream ratchet: modules (`import`, `include`, `-L`), `$name: pattern`, `.5` literals,
`|= empty` on several paths at once (jaq deletes path by path), which access a path error
names, unary minus precedence (`-1/0`), and bison's syntax error texts.

## D1182: jq's exit status and positions are 1.7.1's
date: 2026-11-07 · by: procs · issue: 0018
The status is the last input's (an error in an earlier input does not count, as in jq
1.7.1); 5 for an error or a parse error of the input, 4 for no output with `-e`, 1 for a
false or null last output with `-e`, 3 for a compile error, 2 for a usage error or a file
that cannot be opened (the other files are still read). A runtime error names the input
position as jq does, the lines read so far: jq reads its input by lines, so a value on a
line counts that line (`(at <stdin>:1)`), `<unknown>` before any input. Input files are
read one after the other, each its own JSON text (jq parses the concatenation as one
text; a value split across two files is an error here).
## D1194: awk and gawk are S mode
date: 2026-11-08 · by: text · issue: 0019 (coordinator's decision)
Supersedes D015 for `awk` and `gawk`: psh's awk (D1193) always runs in-process, also when
the host has an awk on PATH. One gawk-compatible awk on every platform is the agent-first
choice: macOS agents otherwise get BSD awk, whose behavior differs (83 of the 772
vendored cases). Options psh does not implement still fall through to the real program
(D1082). Basis: the vendored suites pass but for documented, ratcheted cases (767/772,
onetrueawk's T.* only 3 checks beyond gawk's), and the 69 L4 cases pass on GNU's goldens.
## D1137: grep, egrep, fgrep: GNU grep 3.11 on the shared search layer
date: 2026-11-08 · by: infra · issue: 0013
`grep`, `egrep` and `fgrep` are always-on utilities (mode S) written for psh after GNU
grep's manual and its observed behavior (3.11, the recording container); BSD-only flags
and conflicts follow D016 (GNU). A blocking utility (D1130): the front end parses options
and patterns (`-f`, `--exclude-from` read through the utility's file system, `-f -` from
stdin), the body searches with `grep-searcher` and psh's own GNU sink.
- **Matching.** Patterns are one per line of `-e`/`-f`/the operand. Line selection is
  `grep-regex` over every pattern's `psh_regex::to_rust_syntax` translation (D1131), with
  `-i`/`-w`/`-x` as its options (`-w` is GNU's: no word character on either side).
  A pattern with back-references has no translation: then lines are matched one at a
  time, those patterns by `psh_regex`. Spans (`-o`, `--color`) are GNU's leftmost-longest:
  an `ambiguous` pattern goes to `psh_regex`; several patterns are matched separately and
  the leftmost, then longest match wins; `-F` literals are one alternation sorted longest
  first. `-P` uses the regex crate as is (Perl's leftmost-first); a pattern it rejects,
  or several `-P` patterns, run the real grep.
- **GNU's dfa extensions** are a pre-pass over BRE/ERE patterns, since psh-regex is
  glibc's `regcomp`: ERE `*`/`+`/`?` at the start of an expression warn and are dropped,
  `{...}` there warns, a `{` that starts no interval is literal, `^*` is no anchor; a
  class outside brackets (`[:space:]`) is GNU's error.
- **Binary files.** grep reads each file's first 96 KiB itself (GNU's buffer; the
  searcher's decoder hands its first bytes over in pieces, so its own detection would
  come after the first line). A NUL there, or later in a buffer before its lines are
  matched (`BinaryDetection::convert`), stops all output of the file; a later selected
  line ends the search and `grep: FILE: binary file matches` goes to stderr, as in GNU
  3.5+. In a UTF-8 locale an output line (or `-o` part) that is not valid UTF-8 is left
  out with the same message. `-a` searches as text, `-I` makes a binary file not match;
  `-z` has no binary detection.
- **Recursion** is the walker seam with every ignore source off and hidden files in;
  `-R` follows links, `-r` only those named on the command line; devices, FIFOs and
  sockets met while recursing are skipped; a loop is a warning, not an error. Order is
  by name (GNU's is the directory's). `--include`/`--exclude` match the base name while
  recursing and any name suffix after a `/` for operands, last matching glob wins, no
  match means included unless the first is an `--include`; `--exclude-dir` likewise for
  directories (not the implicit `.`). Globs are fnmatch without `FNM_PATHNAME`: `*`
  matches `/`, braces are literal.
- **Output** goes through grep's own 64 KiB buffer, flushed before each stdin read and
  stderr write (D1081), at every line with `--line-buffered` or on a terminal. GREP_COLORS
  (`ms mc sl cx fn ln bn se rv ne`) and the deprecated GREP_COLOR with GNU's warning.
  Messages say `grep:` for all three names (Debian's egrep/fgrep run grep). A closed
  pipe ends grep with 141; the exit status is otherwise 0 (a line selected), 1 or 2 (an
  error, unless `-q` selected a line). With no pattern at all and no `-v`, GNU reads no
  file (no errors, no counts, `-L` lists the operands): so does psh.
- **Fall-through** (D1082): options psh does not implement (`-T`, `--help`, `--version`,
  BSD's `-O -S -p -J -X -M`), unknown `--color`/`-d`/`-D` values.
- The classifier's grep/egrep/fgrep tables now list all GNU value options (`--include`,
  `--exclude`, `--exclude-dir`, `--exclude-from`, `--group-separator` were missing, so
  `grep --include '*.rs' x` misread the glob as the pattern), and `--exclude-from` as a
  file read.
Incremental search in wasm (`yes | grep x | head -1`) is still D1130's follow-up: the
inline backend reads stdin first.
## D1292: CRLF line ends are dropped only where the host says lines end so
date: 2026-11-07 · by: builtins (rule by the coordinator) · issue: 0029
`Host::strips_crlf()` (default false; the Windows host true; the test host has a switch)
says whether the programs and files of the host end lines in `\r\n` as a rule; the
platform is never guessed with `cfg!(windows)`. Then `read`, when it reads a whole line
(delimiter `\n`: no `-d`, `-n` or `-N`), drops one unescaped `\r` right before the
newline, before IFS splitting, with or without `-r`; a `\r` before end of file without a
newline stays. Command substitution drops a `\r` right before each trailing newline it
removes; `\r\n` inside the output stays. Elsewhere bytes are bash's: `\r` is data.

## D1293: ulimit limits what the shell starts, never the shell itself
date: 2026-11-07 · by: builtins · issue: 0029
Additive Host items: `Host::rlimit(Rlimit) -> Option<(Limit, Limit)>` (the process's
soft and hard limit; `None` where the platform lacks the resource, and `ulimit` then
rejects its option with status 2, as bash does there; default `None`, so Windows and
the JS host have none) and `SpawnSpec::rlimits`. psh-os on Unix reads them with
`getrlimit` and sets a child's with `setrlimit` in the fork path (posix_spawn cannot,
as with the umask); a limit the kernel refuses leaves the inherited one. What `ulimit`
sets lives in `Vp::rlimits`, so a subshell's settings stay in it. Raising a hard limit
is refused ("Operation not permitted": psh assumes no privileges), a soft limit above
the hard one is "Invalid argument"; `-p` is fixed at 8 (PIPE_BUF in 512-byte blocks).
Known gap: the shell process is never limited (an embedded engine must not lower its
embedder's limits), so in-process work — redirections, builtins, utilities — ignores
`ulimit -f`/`-n` that a bash process would hit.

## D1940: Command substitution's CRLF rule follows `Host::strips_crlf()`
date: 2026-11-08 · by: engine · issue: 0032
Supersedes D1324's "Windows builds only" and the CRLF note in D1329. The `\r` before
each trailing newline that `$(…)`, backticks, `$(< file)` and `${ …; }` remove goes too
when `Host::strips_crlf()` (D1292) is true: the Windows host, or the test host switched
on. A `\r\n` inside the output stays. A property of the host, not of the build: an
embedder with its own host decides, and tests cover both rules on every platform;
`read` follows the same method (builtins).

## D1905: No descriptor of another thread leaks into a spawned process
date: 2026-11-07 · by: procs · issue: 0030
std on macOS makes pipes with `pipe` + `fcntl(FD_CLOEXEC)` (there is no `pipe2`), so a
process spawned by another thread in between inherits the pipe for good. Under gate load
that made trivial brush `kill` cases time out (kill/5, 6, 19–23): a psh binary run by
psh-spec hung for its full timeout (a known failure) while holding another case's
output pipe, so that case never saw its end. Fixes: psh-os's fork path (umask, signals
to ignore) closes every descriptor the child does not keep (`proc_pidinfo` on macOS,
`close_range` on Linux), as its posix_spawn path already does with
`POSIX_SPAWN_CLOEXEC_DEFAULT`; psh-os makes its pipes under `psh_os::unix::FD_LOCK`
(shared), which an embedder holds exclusively around a std `Command` spawn (psh-spec
does, and makes its in-process cases' pipes through `psh_os::unix::pipe`). A stress run
(two full suites and four loops of brush kill/signals at once, 200 rounds) went from
about 9 spurious timeouts to none.

## D1142: sed's `e` runs in a fresh psh shell
date: 2026-11-08 · by: parser · issue: 0014
`e command`, bare `e` and `s///e` run their command through `UtilCtx::spawn_shell`
(D1191: `sh -c` semantics, exported variables only) with an empty stdin and the
output captured: `e command` prints it at once; bare `e` and the `e` flag replace the
pattern space with it, less one trailing newline; the command's status is ignored,
as in GNU. They no longer fall through to a real sed (D1140); `--sandbox` rejects them.
## D1138: fd: fd 10.5.1 on the walker seam, commands from the front end
date: 2026-11-08 · by: infra · issue: 0013
`fd` is an always-on utility (mode S), fd 10.5.1 (the recording container's release
binary is the oracle). A blocking utility (D1130) with an async front end:
- **Command line** on psh's option parser (D1082), not clap: every documented flag and
  the hidden ones (`--no-hidden`, `--ignore`, `--relative-path`, ...), clap's aliases
  (`--newer`, `--maxdepth`, `--mount`, ...), overrides (the last of `-H`/`--no-hidden`,
  `-s`/`-i`, `-g`/`--regex` wins; `-u` is the default for both), conflicts and value
  errors in clap's words with status 2. `-x`/`-X` take every word to `;`, so they are
  taken out before the parser runs. `-h`/`--help` print fd's help text (recorded from
  the release binary; Windows' lacks `--owner`), `-V` `fd 10.5.1`. Unknown options run
  the real fd.
- **Copied** from fd at 193d0df with attribution (NOTICE): `filter/size.rs`,
  `filter/owner.rs` (numeric ids only), `filter/time.rs` (only `@timestamp`), `fmt/`
  (on byte paths; a plain search replaces aho-corasick), `exec/` (templates and
  batching), `output.rs`, `patterns.rs`/`regex_helper.rs`, `sanitize.rs`, and the
  walker setup, filters and receiver rules of `walk.rs`. What fd uses of lscolors
  0.21.0 (`LS_COLORS` parsing, indicators, the suffix map's precedence) and of
  nu-ansi-term 0.50.3 (rendering: attributes, background, foreground; bright colors as
  256-color codes) is adapted into `colors.rs`; fd's default colors are its own string,
  newlines included. None of clap, jiff, lscolors, nu-ansi-term, ctrlc, nix, etcetera,
  normpath, faccess, argmax, crossbeam-channel, aho-corasick or memchr is a dependency.
- **Walk** on the walker seam (D1132) with fd's `WalkBuilder` settings (`.fdignore`,
  parents only with an ignore source, the global `fd/ignore` from `XDG_CONFIG_HOME`/
  `HOME` or `APPDATA` and `--ignore-file` as explicit ignore files), `-E` as override
  globs relative to the first root, the parallel walker's error wording. The base
  directory is the walk's cwd and the commands' cwd; the process's never changes.
  Broken links met with `-L` are results, as in fd. Executability is the permission
  bits (fd asks access(2)); no `mh` (link counts are not in the Host's metadata).
- **Order and output.** The walk is sequential and sorted, so results come in path
  order; fd's receiver buffers up to 1000 results or 100 ms and sorts them (by `Path`
  components), so its small outputs are in this order too and psh keeps the rule (a
  buffer of 1000 sorted, for several roots). `--max-results`/`-1` take the first in
  that order (fd's are whichever thread was first). Raw bytes when piped, sanitized on
  a terminal, `./` stripped only for the implicit search path and not before a `-`.
  A failed write ends fd with status 1 (fd's flush failure), silently for a closed
  pipe.
- **Commands.** A blocking body cannot spawn, so with `-x`, `-X` or `-l` the body
  collects the (stripped) results and the front end runs the commands through `spawn`
  (utilities, standalone builtins, programs), one after another in result order; `-x`
  commands get `/dev/null` as stdin unless `--threads=1` (fd captures their output
  with several threads). A missing command is fd's `Command not found: "NAME"` without
  spawning. `-X` batches are cut at `--batch-size` or 128 KiB of arguments (no
  argmax). `-l` runs `ls -l -h -d --color=…` (BSD `ls -l -h -d [-G]` on macOS without
  `gls`), as fd does.
- **Fall-through** (D1082): `--changed-within`/`--changed-before` with a duration or a
  date (until 0020's date parser), `--owner` with a user or group name (the utility
  context cannot look names up), `--owner` on Windows (fd has none there),
  `--hyperlink` (needs the host name).
- **Windows**: `/` separators by default (`--path-separator` overrides; fd does this
  only under MSYS); the leading-dot pattern error applies on every platform (fd: Unix
  only), so a command behaves the same everywhere (dot files are hidden on Windows too).
- The classifier's fd table lists every value option now (`-C`, `--and`, `--size`,
  `--format`, `--search-path`, ... were missing) and `--ignore-file` as a file read.
## D1152: ls -G and -T take their BSD meanings
date: 2026-11-07 · by: files · issue: 0015
Settles D034 for `ls`. Corpus (295,401 commands, 6,849 `ls` with flags): `-T` appears 23
times, every one BSD's full-time form (`ls -lT`, `ls -la -T PATH`, `ls -laT`), never
GNU's `-T COLS`; `-G` once, as `ls -laG` (BSD color). So `-T` shows full times in the
long format (`Jan  2 03:04:05 2024`) and takes no value; `-G` is BSD's color flag, and
psh's `ls` prints no color (as macOS `ls` does off a terminal, which is where agents
run). GNU's `-G` (no group) has `-o` (long without the group; GNU's meaning, where BSD's
`-o` shows file flags); GNU's `-T COLS` is not accepted. The rest of
`ls` is GNU's: `total` in 1 KiB blocks, aligned columns with single spaces, `-C` with
tab stops of 8, exit 2 for a missing operand, byte-order sorting (the C locale's).

## D1153: stat: GNU and BSD formats side by side
date: 2026-11-07 · by: files · issue: 0015
`stat -c/--format/--printf` are GNU's directives (with flags, width and precision,
`%.3Y`); `-f FORMAT` with a `%` is BSD's `%[flags][width][.prec][DOUXFS][HML]datum`,
psh's own formatter written from the macOS manual. `-t` is ambiguous: with a BSD format
present, or an argument containing `%`, it is BSD's time format for `%S` times;
otherwise GNU's terse flag and the word after it a file. `-f` before plain file names is
GNU's file-system status, which needs statfs: it falls through to the real `stat` or
fails clearly. The default output is GNU's; BSD's `-l`, `-s`, `-x`, `-r` fall through.

## D1154: mktemp takes BSD's prefix and several templates
date: 2026-11-07 · by: files · issue: 0015
`mktemp -t ARG`: `ARG` with `XXX` is GNU's template in `$TMPDIR` (else `/tmp`), any
other is BSD's prefix (`ARG.XXXXXXXXXX` there), `-t` as the last word is GNU's flag
(D016). Several templates are created one by one (BSD); fewer than three X's fail
(GNU; BSD would create the name as written). Names are 62 letters and digits from a
splitmix64 stream seeded by `/dev/urandom` where it exists and the clock; creation is
exclusive (`O_EXCL`, mode 0600; directories 0700) with retries, so predictability only
costs retries. psh uses `$TMPDIR` where macOS's `mktemp` prefers its per-user directory.

## D036: No permission policy layer; classification is for reporting
date: 2026-11-08 · by: coordinator (owner) · issue: 0010
psh is not a sandbox and Pi will not build a permission policy on it. The classifier
exists to help reporting: UI summaries of what a call does (read, mutate, network, …),
the paths it touches, and a read-only flag Durable can use for replay. The planned
exec-time approval hook is dropped; ARCH's open question about where it sits is closed.
## D1143: sed's numeric escapes are bytes
date: 2026-11-08 · by: parser · issue: 0014
`\xHH`, `\dNNN`, `\oNNN` and `\cX` make one byte, in regexes (also in bracket
expressions and addresses), replacements and `y`. GNU matches such a byte even inside a
multibyte char (`s/\xce//` on `μ` in C.UTF-8), so a regex whose escapes leave bytes that
are not whole UTF-8 chars is matched on bytes; `\xce\xbc` stays the char `μ`. (A
side effect: in such a regex `.` matches one byte, where GNU's `\xce.` matches nothing.)

## D1360: The guide is a static site built by psh-docs, in Elanor's design
date: 2026-11-08 · by: coordinator (owner) · issue: 0036
`psh-docs build` renders docs/guide into a static site (target/docs-site), the way djs's
`djs-docs build` does: navigation from SUMMARY.md, a page outline, previous/next,
search over every section in the browser (search.json, no server), light and dark,
highlighting, and repository links to GitHub at the commit the site is built from.
index.md is its front page (checked by the writing rules too, not a chapter). The look is
Elanor, Earendil's design system: its published tokens, fonts, icons and logos are
vendored into crates/psh-docs/assets/elanor by scripts/elanor-update.sh (pinned to one
elanor commit, as djs pins it), so building needs no login or network. Markdown is
rendered by pulldown-cmark (default features off, `html` only; with its own
pulldown-cmark-escape, bitflags and unicase), in psh-docs only; the writing rules keep
their line scanner, which reports line numbers. Alternatives: mdBook (a tool to install,
its own theme), a hand-written Markdown renderer (tables, footnotes and HTML blocks are
not worth writing). Owner's request: docs like djs's.

## D1361: Live examples run in psh at build time and in psh.wasm in the page
date: 2026-11-08 · by: coordinator · issue: 0036
A ```` ```sh live ```` block is an editable example on the site: live.js runs it in
psh.wasm over js/psh.mjs's MemHost and shows its output and its classification. The
build runs it natively, in psh-engine's in-memory test host with psh-utils in sandbox
mode, in an empty /work with the same environment (HOME=/work, PATH=/usr/bin:/bin), so
psh-docs depends on psh-engine, psh-utils, psh-syntax and psh-classify. Its output
(stdout and stderr as they interleave, then `(exit status N)` when not 0) is kept in a
`gen:run` region after the block, for readers of the Markdown: `regen` writes a missing
one, a changed output fails `check` until `regen --examples` accepts it, as in djs. The
wasm leg checks that psh.wasm prints the same (crates/psh-docs/tools/live-check.mjs), and
a psh-docs test checks that assets/highlight.js, the editor's highlighter, matches
highlight.rs on every shell block of the guide. The examples have no external programs,
so they show the language and the utilities, not `git` or `cargo`.

## D1362: REPORT.md is folded into the guide
date: 2026-11-08 · by: coordinator (owner) · issue: 0036
The project report (REPORT.md, a snapshot of 2026-11-08) duplicated the guide's status
page and README's table, and went stale with every merge. Its numbers and costs are on
status.md, its list of what is not built and its known gaps are status.md's last
sections, and where the paused work stands is in work/BOARD.md and the issue logs. The
README points at the guide and the site. ARCH.md, TESTING.md, this log and research/
stay: they are the reference for people changing psh, which the guide is not.
