# psh: should Pi ship its own portable bash?

Initial investigation, 2026-11-06. This file is the synthesis. The detailed reports are next to it:

| Report | Scope |
|---|---|
| [corpus.md](corpus.md) | 295k real shell commands from Pi agent sessions and p2 sessions: what agents actually run |
| [pi-current.md](pi-current.md) | How pi-mono executes shell commands today, and how pi-env could host a shell |
| [oh-my-pi.md](oh-my-pi.md) | oh-my-pi's `pi-shell` engine and `pi-builtins` |
| [just-bash.md](just-bash.md) | vercel-labs/just-bash, a TypeScript bash interpreter over a virtual filesystem |
| [landscape.md](landscape.md) | brush, mvdan/sh, Oils, Bun Shell, deno_task_shell, busybox-w32, uutils, WASM options, the language question, a Windows checklist |
| [spec-harness/](spec-harness/) | Oils spec suite harness, run against 10 shells; reusable as a conformance gate |
| [corpus/](corpus/) | Corpus extraction and analysis scripts, aggregate results only. The raw extracted commands stay private in `~/.cache/psh-corpus/` |

## 1. What agents actually run (corpus)

The corpus is 295,401 commands from 6,112 sessions.

- **The syntax used is narrow.**
  - `&&` appears in 55% of commands, pipes in 47%, `cd X && …` in 46%, `;` in 37%, `2>&1` in 15%.
  - Quoted here-docs appear in 13%, mostly feeding python or node.
  - `$(…)` appears in 5% and `for` in 3.5%.
  - Each of `if`, `[[ ]]`, `case`, functions, arrays, here-strings and process substitution is at or below 0.6%.
- **Utility names are concentrated.** The top 20 names fully cover 76% of commands, and the top 50 cover 92%.
  - A builtin set of about 117 names could run 37.6% of commands entirely in-process.
  - With one external program allowed, that rises to 85%. The external program is mostly `git` (24%), then `python3`, `node`, `npm`, `cargo` and `gh`.
  - **A sandbox-only shell is not enough. Real processes are mandatory.**
- **Most commands only read.** 52% are pure reads, and 47% cut their output (`| head` alone is 32%). This makes classification and output control valuable.
- **Platform-caused failures are rare on macOS:** about 0.34% of commands.
  - Another ~1.3k commands hit a platform error but exited 0. `;`, pipes without pipefail and `|| true` hide those errors.
  - The GNU habits that break on macOS: `timeout` (389 "command not found"), `grep -P`, `find -printf`, `stat -c`, `cat -A`.
  - Agents mostly write BSD-correct `sed -i ''` (5,307 against 144 GNU-style). A builtin `sed` must accept both forms.
- **The two products run different shells.** The agent runs `/bin/bash` 3.2. p2 runs `$SHELL`, which is **zsh**, under the name "bash". As a result, 815 p2 commands hit `zsh: no matches found`.
- **There is no native Windows usage in the data at all.** That is a selection effect: Windows users fall back to PowerShell (see pi-current). The only Windows traffic was `prlctl exec … powershell`, which failed 15% of the time, mostly on bash→PowerShell quoting.
- **Long-running work:** `sleep` appears in 3.2% of commands, polling loops in 1% and `&` in 0.7%. Some orphaned `http.server` processes were still alive days later.

## 2. Where Pi is today (pi-current)

- Every command spawns a fresh `bash -c`. There are **three hand-synced copies** of the shell lookup logic:
  - coding-agent: `utils/shell.ts`
  - durable: `env/node.ts`
  - pi-env daemon: `sys/{unix,windows}.rs`

  They have already drifted apart.
- Nothing parses shell syntax.
  - The UI shows the raw command.
  - Permission and plan-mode checks are regex extensions that are easy to bypass.
  - Durable cannot mark `bash` as safe to replay, because it can't tell reads from writes.
- **Windows pain points:**
  - WSL's `bash.exe` gets picked over Git Bash.
  - Git Bash pipeline children survive a kill (#9129, still open).
  - Output is always decoded as UTF-8, so other code pages come out garbled.
  - `/dev/null` becomes stray `nul` files.
  - `.cmd` shims fail.
  - There are no terminals on Windows.
- **pi-env daemon:** Rust, threads with no async runtime, 4 dependencies, about 1.9 MB (about 1 MB gzipped). It ships for 8 targets: musl, android, darwin and windows-msvc, each on x64 and arm64. It is deployed over SSH, named by its SHA-256, and advertises a feature list. That makes it the natural vehicle for a shell.
- **Constraints on any replacement:**
  - The interface stays "bash string in, combined output out".
  - Remote results must match `NodeExecutionEnv` running on that host.
  - The existing hooks and a `shellPath` escape hatch to real bash must keep working.

## 3. What others did

**oh-my-pi** went all the way.
- `pi-shell` is about 330k lines of Rust: a bash 5.3 engine plus a zsh 5.9 dialect, built as a VM with a single Host/Fs trait, cooperative cancel and pause, and snapshot/resume.
- `pi-builtins` is about 170k lines and holds about 80 in-process utilities: uutils, ripgrep crates, jaq, fd, ast-grep, and even a 63k-line rsync.
- Windows works without Git Bash: Job Objects, decoding output per program, host-side shebang handling.
- It replaced brush only 3 days before our snapshot. Windows regressed and has Linux-only CI.
- Approvals still use regexes. There is no WASM build.
- **License risk:** the bash parts describe themselves as function-by-function ports of GNU bash C code (GPLv3+). The crate is labelled MIT, and `obash --version` prints the GPLv3 banner. **Do not vendor or closely read `pi-shell/src/bash`.** These parts are MIT and safe to learn from:
  - the builtins
  - the minimizer
  - the output taps
  - the Host design
  - the Windows spawn recipe
- Its 9.7 MB `obash` binary matched bash on our probes and starts in 2.9 ms. It is built at `/tmp/obash-omp` if we want to compare against it.

**just-bash** (TypeScript, Apache-2.0, very popular) is a good *sandbox* shell and a usable parser.
- It has a virtual/overlay filesystem, about 80 reimplemented commands and 50 resource limits.
- The browser bundle is 355 KB gzipped; the parser alone is 24 KB gzipped.
- Its limits:
  - It cannot run real processes.
  - Nothing streams.
  - There is no `trap` and no job control.
  - Arithmetic is 32-bit, and regex is RE2.
  - It patches JS globals while running.
  - Its `rg` is about 150× slower than real rg.
- It passes about 83% of its imported Oils cases.
- Its AST carries only line numbers, with no character positions.

**Landscape** (full Oils spec run: 2,738 cases, counting only the cases bash passes):

| Shell | Pass rate |
|---|---|
| brush (Rust, MIT) | 82.4% |
| mvdan/sh (Go, BSD) | 64.9% |
| yash-rs (GPL) | 44.1% |
| deno_task_shell | 13.5% |
| Bun Shell | 10.5% |

- Bun Shell and deno_task_shell are too small: they have no loops, functions or here-docs. They are still the best references for Windows spawning: process trees, shebang emulation, and the BatBadBut quoting rules for `.cmd` files.
- uutils/coreutils passes 92.9% of the GNU tests, is shipped by Microsoft as "Coreutils for Windows", and builds for WASI.
- GPL rules out shipping bash, busybox-w32, yash-rs and fish inside Pi.

## 4. Is it worth it?

Not mainly for macOS reliability; platform failures there are small. The case rests on five other things:

1. **Windows as a first-class platform.** Today Windows works only with Git Bash, with kill, encoding and path bugs, and users avoid it. An engine that owns process spawning (Job Objects, PATHEXT, argv/batch quoting, per-program decoding, `/dev/null`) fixes this class of bugs structurally. This is the strongest argument.
2. **One shell everywhere.** Today we run bash 3.2 on macOS, zsh in p2, Git Bash or WSL on Windows, and whatever bash a remote host has. A shell we ship gives identical semantics locally, over pi-env and in sandboxes.
3. **Understanding commands.** The same parser can drive several things:
   - UI summaries ("read 3 files", "ran tests")
   - read-only detection: 52% of commands, which also unlocks Durable replay and auto-approval
   - permission prompts based on the real AST instead of regexes
   - output-aware tooling (`| head` in 32% of commands)
4. **Control over execution.** We get cancellation, timeouts and process-tree kill, plus a builtin `timeout` and GNU/BSD-tolerant `sed`/`grep`/`find`/`stat`. Output can be minimized and taps added. Orphaned background servers become visible.
5. **The browser and sandboxes.** The same engine compiled to WASM over a virtual FS, or just-bash as a stopgap.

Costs:
- The last ~15% of bash compatibility is a long tail.
- brush has essentially one maintainer.
- We'd own a meaningful amount of Windows process code.

## 5. Language

**Rust.**
- pi-env is already Rust, and the shell should live in or beside it.
- The reusable pieces are all permissively licensed Rust:
  - brush-parser and brush-core (MIT)
  - uutils (MIT)
  - ripgrep's `grep-*`/`ignore` crates
  - jaq
- Rust covers static binaries for all 8 pi-env targets, N-API for the Node host and WASM for the browser:
  - brush parser alone: 146 KB gzipped
  - brush in full: about 1.46 MB gzipped
  - uutils on WASI: 3.7 MB gzipped
- **Go** (mvdan/sh) is the only serious alternative.
  - It can't link into pi-env.
  - Its WASM output is heavy: 6.2 MB, 1.7 MB gzipped.
  - It scores about 17 points lower on the Oils suite.
- **TypeScript** only makes sense on the host side. Even there, the classifier should be the Rust parser compiled to WASM, so the permission check parses exactly like the executor.

The open technical decision: brush-core is built on tokio, and pi-env is synchronous and threaded. Either pi-env adopts tokio (reasonable, since it already multiplexes processes and file watches), or we fork brush's runtime layer.

## 6. Proposed plan (phased, each phase useful alone)

**Phase 0: fixes we should make regardless** (small, in pi-mono):
- p2: stop running `$SHELL` (zsh) as "bash"; resolve real bash like coding-agent does.
- Collapse the three shell lookups into one, ideally owned by pi-env.
- Port the coding-agent's macOS kill fix into the daemon.
- Windows: run commands through the daemon, using Job Objects for process-tree kill.

**Phase 1: parser and classifier** (`psh-parse`, Rust → native + WASM + N-API):
- Build on brush-parser, with byte spans added if missing.
- Classify each command into the corpus categories: read, mutate, VCS, build, network, install, long-running.
- Report redirects, substitutions and nested commands, e.g. `git commit -m "$(rm -rf /)"`.
- **Use the 295k-command corpus as a regression and accuracy set.** tree-sitter-bash already parses 99.9% of it, so it is a baseline to compare against.
- Deliverables: UI summaries in p2, a read-only flag for Durable, and a replacement for the regex permission checks.
- Low risk. Only needs a shell later if we go further.

**Phase 2: the `psh` binary** (brush-core + curated builtins), shipped with or inside pi-env:
- Opt in through `shellPath` first; real bash stays the escape hatch.
- **Start with Windows**, where the gain is largest and the bar (Git Bash) is lowest.
- Builtins, in order of value from the corpus:
  - `timeout`
  - `head`/`tail`/`wc`/`cat`/`ls`
  - `grep`/`rg`/`find`
  - `sed` (GNU and BSD `-i`)
  - `jq`
  - `xargs`
  - Each builtin also needs a "use the host binary" switch.
- Shell syntax beyond the narrow set in §1 comes from brush.
- **Conformance gates:**
  - the Oils spec harness: track brush's 82.4% and improve it upstream
  - differential replay of read-only corpus commands against bash in a container
- Then switch the daemon's `exec` to run psh in-process, so it owns every child process.

**Phase 3: browser and sandbox.**
- psh compiled to WASM over a virtual-FS Host, with uutils-on-WASI or our own builtins.
- just-bash is a working stopgap in the meantime.
- The browser can't run git, python or node, so this mode is for exploration and editing only, unless it is paired with WASM runtimes or a remote pi-env.

## 7. Open questions for the owner

1. Do we build on brush (upstream contributions, single maintainer, tokio) or fork it? The alternative is writing our own engine, oh-my-pi style. That is not recommended: it's huge, and the only fast way there is the GPL trap.
2. For builtins, GNU semantics, BSD semantics, or tolerant of both? The corpus suggests "tolerant": agents write BSD `sed -i ''` but GNU `timeout`/`grep -P`.
3. Should p2 keep honoring the user's `$SHELL` (zsh) anywhere, e.g. for the interactive terminal only?
4. How much Windows do we want? A psh that is "bash on Windows without Git Bash" is the clearest product win, but it needs Windows CI, which oh-my-pi lacks and which bit them.
5. In the browser, is a read-only exploration shell enough, or do we expect to run builds there? The answer decides whether Phase 3 matters at all.

## 8. Agreed constraints (owner discussion, 2026-11-06)

1. **Scope: driving an agent only.** Non-interactive bash 5.x semantics (`bash -c` style).
   No line editing, completion, history, zsh or POSIX-sh mode. Job control only as far as
   `&`, `wait`, `kill`, `$!`. The interface stays "command string in; stdout, stderr and
   exit code out". Real bash stays available as an escape hatch (`shellPath`).
2. **Tests: Oils spec + brush cases** as the backbone, with goldens recorded from bash 5.3
   (see [hillclimb.md](hillclimb.md)). The private corpus is used only through a sanitized
   *shape corpus* (parse-only); it is never executed.
3. **Builtins: err on the side of too many.** The corpus is one signal; just-bash's demand
   history and the other shells' lists are others (see builtins.md). Never builtins:
   database clients like sqlite3, interpreters, toolchains, package managers. VCS and
   network clients (`git`, `curl`/`wget`, ssh/rsync-like tools) are out of scope for now
   rather than "never": not builtins and no work on them today, but psh may later hook
   them (host-provided commands, interception). (Revised 2026-11-06, D022.)
   Every builtin falls through to the host binary (or fails clearly) on flags it does not
   support, never silently diverges, and can be turned off.
4. **No tokio, no async runtime.** A synchronous engine stepped by its embedder, std
   `Waker`s only, OS waits behind a `Host` trait (waiter threads, pidfd/select on Unix, Job
   Objects and overlapped pipes on Windows). See [execution-core.md](execution-core.md).
5. **Near-zero dependencies.** We copy from brush (MIT, with attribution) rather than depend
   on it, and replace its peg/winnow parsing with hand-written code. The deliberate exception:
   `rg` and `fd` are built on the ripgrep/fd crates (`grep-*`, `ignore`, `globset`, `regex`).
   Everything else needs a recorded decision.
6. **Licensing: clean room.** MIT/Apache dependencies only. Nobody reads GNU bash's source or
   oh-my-pi's bash-derived engine. GPL test suites may be run, never vendored.
7. **Windows from the start:** Windows CI for every layer, no Git Bash/MSYS dependency.
8. **Commit along the way.**
