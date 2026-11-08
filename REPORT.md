# psh: state of the project (2026-11-08)

psh is a portable, bash-compatible shell written in Rust for running the commands coding
agents write. It was built in about two days by a coordinator and up to eleven agents
working in parallel, from the investigation in `research/` (why: one shell with the same
behavior on macOS, Linux, Windows and in the browser, that understands the commands it
runs). This report says what psh can do today, how well, what it costs, and what is left.

## What it can do

- **Run bash as agents write it.** Non-interactive GNU bash 5.3 semantics (`bash -c`,
  script files, stdin): pipelines, `&&`/`||`, redirections and here-docs, functions,
  control flow, arrays (indexed and associative, in bash's exact key order),
  every parameter expansion, arithmetic, `[[ ]]` with POSIX regex and extglob, `set -e`
  with bash's exemptions, `set -u`, `pipefail`, traps and signals, `set -x`, aliases,
  `declare`/`local`/namerefs, `read`, `printf`, `eval`/`source`, job control
  (`&`, `wait`, `kill`, `jobs`, `disown`), process substitution.
- **Run natively everywhere.** macOS and Linux (posix_spawn, sessions, waiter threads,
  one readiness thread) and Windows without Git Bash or MSYS (Job Objects, MSVCRT argv
  quoting, safe `.cmd`/`.bat` handling, PATHEXT, `#!/bin/sh` scripts run in psh, Ctrl-C
  and Ctrl-Break, `C:/` and `/c/` paths). Cancelling a command kills its whole process
  tree; at the end of an embedded run, leftover background processes are killed unless
  they were detached (`nohup`, `setsid`, `disown`), which fixes the agent problem of
  servers left running for days.
- **Run 59 utilities in-process**, the same on every platform, falling through to the
  real program (or failing clearly) for flags they don't support:
  `awk gawk jq find xargs timeout gtimeout head tail cat wc env sleep basename dirname
  realpath readlink mkdir rmdir touch cut paste seq tee tr uniq comm nl sort rm mv cp ln
  chmod grep egrep fgrep fd rg base64 cmp md5sum sha1sum sha224sum sha256sum sha384sum
  sha512sum md5 shasum diff date which printenv sed ls stat du mktemp`.
  Highlights: `rg` and `fd` are ripgrep's and fd's own code (rg matches rg 15.2.0
  byte for byte incl. `--json`); `grep` is GNU grep 3.11 on the ripgrep crates; `sed`
  follows GNU sed 4.9 and accepts both `sed -i ''` (BSD) and `sed -i` (GNU); `awk` is a
  GoAWK port with gawk semantics (767/772 upstream suite cases); `jq` follows jq 1.7.1
  (704/738 of jq's own suite, all 224 manual cases) on jaq-core; `find`/`xargs` match GNU
  findutils on ~175 probe commands. `psh --utils` shows what runs for each name.
- **Understand commands without running them.** `psh --parse` prints the syntax tree as
  JSON with byte spans; `psh --classify` reports every command a script would run
  (nested `$(…)`, `bash -c` strings, `xargs`/`find -exec`/`timeout` targets), its
  category (read, mutate, vcs, build, network, install, long-running, unknown), the paths
  it reads and writes, whether output is truncated, and a conservative read-only verdict.
  On the 295k-command agent corpus the classifier calls 52.1% of commands read-only. Per
  D036 this is for reporting (UI summaries, replay safety), not for permissions.
- **Run in the browser.** `psh.wasm` is the whole shell plus utilities over a JS host
  (in-memory file system, JS-defined commands, no Asyncify); `psh-parser.wasm` is the
  parser and classifier alone (148 KB gzipped).

## How well: conformance

Every number compares psh's stdout, exit status and file changes with GNU bash 5.3
recorded in a pinned Debian container.

| Layer | What it is | Linux | macOS | Windows |
|---|---|---:|---:|---:|
| L0 | parser agrees with `bash -n` and round-trips (34k inputs incl. 26k shapes of real agent commands) | 100% | 100% | – |
| L1 | Oils spec suite, 2,511 active cases | **93.8%** (2355) | 90% | 85% (2132) |
| L2 | brush's test cases, 3,171 active | **90.0%** (2855) | 87% | 81% (2579) |
| L3 | 701 cases built from what agents actually run | **100%** | 98.7% | 40% (283)* |
| L4 | utility cases, recorded against GNU and BSD | **100%** (994) | 100% | 82%* |

\* On Windows most failures are programs Windows doesn't have (`sh`, `ps`, python…); they
may fail without a ratchet line. macOS-only failures are platform differences (BSD tools,
`/var/root`, SIGUSR1 = 30, no `/dev/full`), each listed with its reason.

What still fails on Linux is the long tail: rare expansion and printf corners, here-doc
reader quirks, error-message wording, and features out of scope by design (interactive
shell, `set -o posix`, `coproc`, `fg`/`bg`).

## What it costs

| Artifact | Size |
|---|---:|
| `psh` binary, current `release` profile (no LTO, not stripped) | 11.9 MB |
| same code, fat LTO + 1 codegen unit + strip + opt-level=s | **5.0 MB** (2.5 MB gzipped) |
| `psh.wasm` | 4.3 MB (1.5 MB gzipped) |
| `psh-parser.wasm` | 0.39 MB (0.15 MB gzipped) |

Of the code (`research/checkpoint-2026-11-08.md`): the shell core (engine, parser,
classifier, OS layer) is ~24%, std ~18%, and the utilities with their dependencies are the
rest; the regex stack (13%), the ripgrep family (16%) and jq (10%) are the largest parts.
Recommended next steps: a shipping profile, and cargo features for the heavy groups
(`search`, `jq`, `awk`) so pi-env and the browser choose (core-only estimate ~2.5–3 MB).

Code: ~111k lines of Rust in 10 crates (`psh-syntax` 10k, `psh-classify` 5k,
`psh-engine` 25k, `psh-os` 8k, `psh-regex`, `psh-utils` 64k, `psh-wasm`, `psh-spec`,
`psh-docs`, `psh`); dependencies limited to an allowlist (ripgrep crates, jaq-core,
libc/windows-sys, deser-json, argument), one patched crate (`third_party/grep-printer`).
No async runtime: the engine is a stepped interpreter driven by its embedder.

## How it is built and tested

- One oracle: bash 5.3 recorded once into committed goldens; the gate never runs bash.
- Ratchets: per-layer expected-failure lists that only shrink, a separate list per
  platform (macOS, Windows, wasm), quarantine for understood races, serial cases.
- The gate (`scripts/gate.sh`, cached by inputs): fmt, clippy, tests, MSRV 1.96, Linux
  build check, wasm build and the whole suite under node, the parser and shape checks,
  the guide check, and the full suite on Windows (arm64 and x64) in a Parallels VM.
  Merges are serialized through `scripts/merge-to-main.sh`.
- `scripts/spec-linux` runs the suite on Linux in Docker (the gate only type-checks
  Linux); `scripts/status --hitlist` ranks failures by how often agents use the construct.
- 225 recorded decisions in `docs/decisions.md`; the design reference is `ARCH.md`;
  the guide is `docs/guide/`.

## What is not done

1. **Pi integration** (issue 0010): an embedding API for a daemon (combined streaming
   output, cancel/timeout), pi-env's `exec { shell: "psh" }`, Node-side execution through
   the daemon, and the classifier in Pi's UI. Nothing in pi-mono has changed yet.
2. **Shipping**: release builds for pi-env's eight targets, the size budget and feature
   split, and CI off the owner's machine (no git remote or GitHub Actions yet; the Windows
   leg depends on a local Parallels VM whose `prlctl exec` currently fails often).
3. **Robustness for embedding**: resource limits (output size, memory), the planned
   stress layer (cancel at random points), engine fuzzing, benchmarks against bash.
4. **Agent-facing output control** (a minimizer, long-running command handling) — not
   designed.
5. **Smaller items**: Tier 2 utilities (`ps`/`pgrep`/`pkill` started, `tar`, `gzip`,
   `tree`, `column`, `od`…), `nice`/`nohup`, Windows CRLF switch in the Windows host, the
   Windows `PATH` separator policy (busybox-style built, owner to confirm), the upstream
   rg/fd test suites, a licensing pass before release.

## Where work stopped

All agents are stopped. Unmerged branches with resume notes in their issue logs:
`issue/0013-utils-search` (upstream ripgrep/fd suites), `issue/0020-utils-misc`
(`nice`/`nohup` plan), `issue/0032-engine-climb` (bash hit list), `issue/0035-utils-tier2-process`
(started), `issue/0007-windows` (pause note). `work/BOARD.md` lists paused issues.
