# Portable shell landscape and the language question

Research for a possible Pi-owned portable bash-compatible shell ("psh"). Data gathered
2026-11-06 using `gh api`, shallow clones in `/tmp/ls`, local builds, and a full run of
the Oils spec suite against ten shells. oh-my-pi's pi-shell and vercel's just-bash are
covered in separate studies and only appear here for context.

Unless noted otherwise, star counts and last-push dates come from the GitHub API today,
LOC comes from `tokei` (code lines only), and binary sizes come from my own release
builds (macOS arm64 or Linux arm64).

---

## 0. TL;DR

* **No existing project covers all of our requirements.** Ranked by usable bash semantics:
  **brush** (Rust, MIT) is clearly first, **mvdan/sh** (Go, BSD-3) second. Every other
  project is a POSIX-only shell (yash-rs, busybox ash, dash), a different language
  (fish, nushell), a deliberately tiny subset (Bun Shell, deno_task_shell, just-bash), or
  something that is not a shell interpreter at all (zx, shelljs, tree-sitter-bash).
* **I measured conformance with the Oils spec suite** (2,738 cases, 134 files, Oils' own
  "compat" selection). Bash 5.2 passes 84.8% of the raw assertions. Brush passes **72.1%**,
  gosh (mvdan/sh) **57.5%**, yash-rs 40.1%, deno_task_shell 12.3% and Bun Shell 10.2%.
  Counting only the 2,310 cases bash itself passes: brush 82.4%, gosh 64.9%, Bun 10.5%.
  The harness is checked in at `research/spec-harness/`, and we can use it as our own
  conformance gate from day one.
* **Coreutils:** uutils/coreutils (Rust, MIT) passes **669/720 (92.9%)** of the GNU test
  suite. Microsoft now ships it as "Coreutils for Windows" (winget). It also builds for
  `wasm32-wasip1`: I verified 80 utilities working under wasmtime, at 10.6 MB (3.7 MB
  gzip).
* **Language recommendation: Rust.** pi-env is already Rust, and the strongest reusable
  pieces are Rust: brush-parser/brush-core, uutils, ripgrep's `grep-*`/`ignore`, jaq, and
  Bun Shell, which has been ported to Rust. Rust also gives one codebase for native static
  binaries, N-API (napi-rs) and WASM. Go is the only serious alternative (mvdan/sh is
  excellent), but it cannot link into pi-env, its WASM output is heavy (6.2 MB / 1.7 MB gz),
  and its N-API story is poor. TypeScript is good for the host-side parser/classifier but
  cannot be the remote static binary. Zig and C are not recommended.

---

## 1. Shell implementations

### 1.1 Summary table

| Project | Lang | License | Stars | Last push | Size (code LOC) | Bash level | Windows | WASM | Library? |
|---|---|---|---|---|---|---|---|---|---|
| **reubeno/brush** | Rust | MIT | 2,298 | 2026-11-06 | 55.9k Rust | High (bash-oracle tests, 2,500+ cases; 72% Oils) | "Preview" tier: builds and runs a small suite in CI, no release binaries yet | wasm32-unknown-unknown + WASI 0.3 builds in CI ("many gaps"); I ran both | Yes: `brush-parser`, `brush-core` (tokio async) |
| **mvdan/sh** (shfmt, gosh) | Go | BSD-3 | 9,181 | 2026-11-03 | 34.6k Go (syntax+interp+expand+pattern) | Medium-high (57.5% Oils); best printer/formatter | Yes, designed to run without a system shell | `GOOS=js` and `wasip1` work (pipes fail on wasip1); npm `sh-syntax` ships the parser as 749 KB wasm | Yes: `syntax`, `interp` with Exec/Open/ReadDir handlers |
| **Oils / OSH** | Python→C++ (mycpp) | Apache-2.0 | 3,398 | 2026-11-02 | n/a | Highest bash fidelity outside bash; its spec suite is the conformance reference | No native Windows | No | No (a standalone shell); project "paused" since early 2026 per its blog |
| **magicant/yash-rs** | Rust | GPL-3.0 (3 helper crates MIT/Apache) | 101 | 2026-11-05 | 93.8k Rust | POSIX only (40% Oils) | No (WSL only) | No | Crates exist, but GPL |
| **denoland/deno_task_shell** | Rust | MIT | 146 | 2026-09-25 | 8.7k Rust | Tiny subset: no `if/for/while/case`, functions, heredocs, `$#`, `$*` | **Yes, first class** (Job Objects, shebang emulation, `which` crate) | Not targeted | Yes (`parse` + `execute`) |
| **Bun Shell** (`Bun.$`, `bun x.sh`) | **Rust** (ported from Zig; repo is now 85 MB Rust, 0 Zig) | MIT (Bun) | 96,582 | 2026-11-06 | 20.8k Rust (shell + parser) | Small subset: `if`, pipes, redirects, globs, braces, `$(...)`; **no** loops, functions, `case`, heredocs, `$((...))` (verified on 1.4.4) | **Yes, first class**; careful BatBadBut handling | No | Embedded in Bun runtime only |
| busybox ash / **busybox-w32** | C | **GPL-2.0** | 897 (w32) | 2026-11-04 | n/a | POSIX + a few bashisms (47% Oils) | **busybox-w32 is the best native Windows POSIX shell**: 626–707 KB exe for i686/x64/arm64, ~200 applets | Not maintained | No |
| toybox (toysh) | C | 0BSD | 3,161 | 2026-10-06 | n/a | toysh is incomplete | No | No | No |
| GNU bash | C | **GPL-3.0** | – | – | – | Reference | Only via MSYS2/Cygwin (Git Bash): POSIX emulation DLL, slow `fork`, path mangling | Only WASIX builds (wasix-org/bash, 8 stars) or full-VM emulation (WebVM, v86, container2wasm) | No |
| fish | Rust | GPL-2.0 (GitHub: NOASSERTION) | 34,404 | 2026-11-03 | – | Not bash | Cygwin/MSYS/WSL only | No | No |
| nushell | Rust | MIT | 40,792 | 2026-11-05 | – | Not bash (structured data) | **Native Windows** | Partial | Crates, but a different language |
| shellgei/rusty_bash (sush) | Rust | BSD-3 | 243 | 2026-11-01 | – | Moderate (Oils runs it in `spec-compat.sh`) | No | No | No |
| ipetkov/conch-parser / conch-runtime | Rust | Apache-2.0 | 68 / 26 | **archived 2021** | – | – | – | – | Dead |
| tree-sitter-bash | C (generated) | MIT | 335 | 2025-12-02 | grammar.js 1,087 lines | Error-tolerant parse only, no semantics | n/a | `tree-sitter-bash.wasm` 1.36 MB; 3.4M npm downloads/week | Parser only |
| idank/bashlex | Python | **GPL-3.0** | 652 | 2024-04-08 | – | Port of bash's own parser | – | – | Not usable (license, language) |
| google/zx | JS | Apache-2.0 | 45,834 | 2026-11-05 | – | Not an interpreter: quotes args and calls the system bash (or PowerShell on Windows) | Needs bash or pwsh | – | – |
| shelljs | JS | BSD-3 | 14,401 | 2026-10-25 | – | Not a shell language: ~30 Unix commands as JS functions (12.2M npm downloads/week) | Yes | Node only | Yes |

Context, not covered here: just-bash (vercel-labs, 4,465 stars, 3.6M npm downloads/week)
and oh-my-pi's pi-shell (oh-my-pi itself uses brush-parser for parsing).

### 1.2 Conformance: Oils spec suite, measured

I ported Oils' `test/sh_spec.py` to Python 3 (small patch) and ran it in a Debian
bookworm arm64 container. The run used the 134-file `SUITE-compat` selection (all OSH spec
files except `strict-options`), a 10-second per-case timeout, and the scripts fed on stdin.
Versions: bash 5.2.15, dash 0.5.12, mksh 59c, zsh 5.9, busybox 1.35.0; brush @9435534,
mvdan/sh @5943e49, yash-rs @e957083, deno_task_shell 0.33.3 (via a 20-line stdin driver),
Bun 1.4.4 (via `bun file.sh`).

| Shell | Raw `pass` / 2,738 | pass + Oils-annotated OK/BUG/N-I* | Pass among the 2,310 cases bash passes |
|---|---|---|---|
| bash 5.2 | 2,321 (84.8%) | 96.8% | 100% |
| **brush** | **1,973 (72.1%)** | n/a | **82.4%** |
| mksh | 1,503 (54.9%) | 80.2% | 60.7% |
| **gosh (mvdan/sh)** | **1,575 (57.5%)** | n/a | **64.9%** |
| zsh | 1,357 (49.6%) | 65.9% | 55.3% |
| busybox ash | 1,281 (46.8%) | 54.0% | 52.1% |
| yash-rs | 1,097 (40.1%) | n/a | 44.1% |
| dash | 1,065 (38.9%) | 62.2% | 42.6% |
| deno_task_shell | 336 (12.3%) | n/a | 13.5% |
| Bun Shell | 279 (10.2%) | n/a | 10.5% |

\* Oils annotates known deviations only for shells it tracks, so the column is n/a for
newcomers. For our purposes the third column is the best "bash-compatibility" proxy.
Oils' own OSH passes 2,482/2,756 of its suite (oils.pub release 0.38.0 quality page).

Pass counts for selected files:

| File | bash | brush | gosh | yash-rs | ash | Bun |
|---|---|---|---|---|---|---|
| pipeline | 25/26 | 24 | 21 | 17 | 17 | 3 |
| redirect | 38/41 | 32 | 18 | 18 | 29 | 7 |
| here-doc | 31/36 | 31 | 29 | 28 | 30 | 0 |
| word-split | 48/55 | 50 | 37 | 47 | 47 | 7 |
| var-op-patsub | 27/28 | 21 | 20 | 0 | 14 | 0 |
| arith | 63/74 | 63 | 55 | 33 | 42 | 4 |
| array | 63/78 | 56 | 55 | 4 | 7 | 3 |
| array-assoc | 34/42 | 29 | 27 | 0 | 0 | 0 |
| dbracket `[[ ]]` | 40/49 | 46 | 33 | 12 | 19 | 9 |
| brace-expansion | 46/55 | 45 | 46 | 7 | 8 | 26 |
| errexit | 35/36 | 30 | 32 | 29 | 30 | 5 |
| background (jobs) | 24/27 | 10 | 16 | 19 | 20 | 0 |
| builtin-printf | 53/63 | 40 | 16 | 36 | 34 | 20 |
| builtin-read | 58/64 | 52 | 53 | 26 | 45 | 0 |
| builtin-trap | 36/38 | 26 | 14 | 25 | 29 | 2 |
| alias | 47/48 | 23 | 28 | 45 | 43 | 1 |
| extglob-match | 29/29 | 28 | 21 | 0 | 0 | 0 |
| nameref | 25/32 | 4 | 13 | 2 | 2 | 2 |

Takeaways:

* brush is strong on the parts an agent actually uses: expansions, `[[ ]]`, arrays,
  heredocs, word splitting, arithmetic. Its weak spots are aliases, namerefs, traps and
  background jobs, all of which matter little for non-interactive agent use.
* mvdan/sh is solid on core syntax but weak on `printf`, traps and redirects (18/41).
* The "bash-like" JS-runtime shells (Bun, deno) cover about 10% of real bash. That is
  fine for `package.json` scripts and nowhere near enough for LLM-written commands, which
  routinely use `for`, `while read`, heredocs, `$((...))` and functions.
* **The Oils suite is directly usable as our conformance gate.** It is Apache-2.0, has
  2,700+ cases, and supports per-shell annotations. The harness is reproducible from
  `research/spec-harness/`. Brush additionally has 2,500+ YAML cases that diff against
  real bash (`brush-shell/tests/cases`), and mvdan/sh has about 2,000 table cases in
  `interp_test.go`. Both are also mineable.

### 1.3 Notes per project

**brush** (https://github.com/reubeno/brush)
* Activity: 51 contributors, 353 commits in the last 6 months, mostly one author
  (reubeno 973 commits). Latest tagged release is brush-shell v0.4.0 (2026-05-03); main is
  active. Lints are strict (`clippy::pedantic` deny, no `panic`/`expect`).
* Crates: `brush-parser` (tokenizer + parser, optional serde AST, fuzzed), `brush-core`
  (runtime, async on tokio), `brush-builtins`, `brush-coreutils-builtins` (optional
  in-process **uutils** builtins behind `coreutils.*` features, so uutils integration
  already exists), `brush-interactive` (reedline).
* brush-parser is already used standalone by Zed, Vite+ and oh-my-pi.
* Platforms per its README: Linux/macOS supported; **Windows "Preview"** (CI builds plus a
  small test suite, "functional for basic usage, particularly when paired with
  Microsoft's coreutils"); **WASI 0.3 "Experimental"**. brush-core has `sys/{unix,windows,
  wasm,stubs}` backends, and the Windows backend is only about 760 LOC. PATHEXT
  resolution exists.
* My builds: native 5.7 MB (macOS arm64) and 6.6 MB (Linux arm64). `wasm32-unknown-unknown`
  is 5.1 MB (1.46 MB gzip). `wasm32-wasip2` is 5.3 MB (1.6 MB gzip) and ran a script with
  functions, arithmetic and pipelines into builtins under wasmtime. Without an in-process
  `cat`, `echo | cat` fails with "command not found", as it should on WASI.
  **Parser only** (`brush-parser` cdylib, `opt-level=z`, LTO) is 572 KB (**146 KB gzip**).
* Fit: the obvious base, or upstream, for a Rust psh. Risks: bus factor, the tokio
  dependency (pi-env deliberately has no async runtime), and Windows semantics being the
  least mature part.

**mvdan/sh** (https://github.com/mvdan/sh)
* 112 contributors, 381 commits in 6 months, latest release v3.14.1 (2026-09-06). Used by
  go-task/Task (16k stars) and many formatters; shfmt is the de-facto bash formatter.
* `interp` is designed for "no system shell, including on Windows". Its `ExecHandler`,
  `OpenHandler` and `ReadDirHandler2` hooks allow virtual filesystems and command
  allow-lists. The package docs state explicitly that handlers are **not** a sandbox.
  `x/coreutils` provides portable cat, chmod, cp, find, ls, mkdir, mv, rm, touch, xargs,
  gzip, tar, mktemp and shasum.
* Windows path model is still in flux: a `TODO(v4)` in `interp/api.go` plans to adopt
  busybox-w32's model (always `C:/foo`, backslash is only an escape, convert at the OS
  boundary).
* WASM: gosh is 6.26 MB for `GOOS=js` and 6.2 MB for `wasip1`, about 1.7 MB gzip each.
  On wasip1, pipelines fail ("pipe: Not implemented on wasip1"). The npm `sh-syntax`
  package (467k downloads/week) ships parser and formatter as a 749 KB wasm.
* Native sizes: gosh 5.1 MB, shfmt 3.1 MB stripped.

**Oils/OSH** (https://oils.pub)
* The most rigorous bash re-implementation and the source of the spec suite.
  `test/spec-compat.sh` already builds and compares brush and sush. Written in typed
  Python and translated to C++. Not embeddable, no Windows. The author announced a project
  "pause" in March 2026. Use the tests, not the code.

**yash-rs**: careful POSIX implementation (6 contributors, very active). Shell crates are
GPL-3.0, POSIX-only, no Windows. Not usable for us beyond reading.

**deno_task_shell** (https://github.com/denoland/deno_task_shell)
* Worth studying for its Windows choices, not its language coverage:
  * a `ChildProcessTracker` that puts every child into a Job Object with
    `KILL_ON_JOB_CLOSE`, so killing `deno task` kills the tree;
  * **shebang emulation**: when a command path contains a slash, it reads
    `#!/usr/bin/env [-S] prog args` and re-dispatches, so `./script.ts` works on Windows;
  * `which` crate for PATH/PATHEXT;
  * pluggable custom commands (`ShellCommand` trait).
* Builtins: cat, cd, cp, echo, exit, export, head, mkdir, mv, pwd, rm, set, shopt, sleep,
  test, unset, xargs. The parser (monch combinator) rejects reserved words, multiple
  redirects, `$#` and `$*`.

**Bun Shell** (https://bun.com/docs/runtime/shell)
* Now Rust: `src/runtime/shell` (interpreter as a state machine, `states/*.rs`) plus
  `src/shell_parser`, 20.8k LOC. All interpolations are escaped by default. Commands in a
  pipeline run concurrently. It also exposes `$.braces` and `$.escape`, and runs `.sh`
  files directly ("cross-platform, so they work on Windows").
* Builtins: cd, ls, rm, echo, pwd, bun, cat, touch, mkdir, which, mv, exit, true, false,
  yes, seq, dirname, basename. Issue #9716 tracks the rest.
* Windows details worth copying are in §4.

**busybox-w32** (https://frippery.org/busybox)
* The existence proof that a native Win32 POSIX shell plus utilities can be about 700 KB
  without MSYS. GPL-2.0, so we can ship it as a separate binary but cannot link it.
  Its path model ("use forward slashes", `c:/path`, `//host/share`, `/path` relative to
  the current drive root, per-drive cwd, no `/c/` translation) and the `BB_OVERRIDE_APPLETS`
  / prefer-builtin policy are good design references.

**GNU bash on Windows / WASM**
* Windows means MSYS2/Cygwin (Git for Windows): a POSIX emulation layer with emulated
  `fork` (slow process creation), automatic POSIX→Windows argument mangling
  (`MSYS2_ARG_CONV_EXCL` to opt out), and a large install footprint. pi-env today requires
  exactly this: `sys/windows.rs` searches for Git Bash and otherwise errors with "Install
  Git for Windows".
* WASM: bash needs fork/exec, so it only runs on WASIX (Wasmer: `wasix-org/bash`, which
  powers `webassembly.sh`, 352 stars, last push 2025-12) or inside full-system emulators
  (WebVM, v86, container2wasm). Neither fits a lightweight in-browser VFS shell.
* GPL-3.0 rules out linking into Pi. Shipping it as a separate binary is legal but brings
  the MSYS baggage.

**Parsers only**
* tree-sitter-bash: incremental and error-tolerant, which makes it great for highlighting
  partial input. It is not a semantic AST: no word-expansion structure that matches bash
  execution, and corner cases (heredocs, `case` patterns, nested quoting) are
  approximations. Good as a UI fallback, wrong as the security classifier.
* shfmt/`syntax`: an excellent lossless AST and printer (Go).
* brush-parser: full AST with spans, serde, fuzzed, 146 KB gzip as WASM.
* bashlex: GPL. conch-parser: dead.

### 1.4 Coreutils and friends (in-process builtin candidates)

| Project | Lang | License | Stars | Notes |
|---|---|---|---|---|
| uutils/coreutils | Rust | MIT | 24,336 | 1,098 contributors, 2,590 commits in 6 months, v0.12.0 (2026-09-17). **GNU tests 669/720 pass** (2026-11-06 tracking JSON). Windows is a supported feature set (`feat_os_windows`). `wasm32-wasip1` + `feat_wasm`: I built and ran it, 80 utilities, 10.6 MB unstripped / 3.7 MB gzip. Native multicall 9.8 MB. Microsoft ships it as **microsoft/coreutils** (5,352 stars, winget `Microsoft.Coreutils`, includes uutils findutils + grep) |
| uutils/findutils | Rust | MIT | 655 | find/xargs |
| uutils/diffutils | Rust | Apache-2.0 | 283 | diff/cmp |
| uutils/grep, sed, awk, tar, procps | Rust | MIT (awk Apache) | 138 / 111 / 30 / 54 / 117 | Young. sed is usable, awk is WIP |
| BurntSushi/ripgrep (`grep-*`, `ignore`, `globset`) | Rust | Unlicense/MIT | 69,749 | Library crates are the best grep engine; pi-env already depends on `ignore` |
| 01mf02/jaq | Rust | MIT | 3,825 | jq clone with a browser playground (WASM) |
| sharkdp/fd | Rust | Apache-2.0/MIT | 44,996 | |
| itchyny/gojq, benhoyt/goawk, u-root | Go | MIT/MIT/BSD-3 | 3,816 / 2,068 / 3,086 | Go equivalents |
| shelljs | JS | BSD-3 | 14,401 | TS-side reference implementations |

Caveat: uutils utilities call `std::fs`/`std::io` directly and assume the real process
stdio. Running them in-process as builtins needs either (a) brush's approach of
redirecting stdio around a uumain call, (b) a refactor towards injected I/O, or (c) WASI,
where `std::fs` is the virtual FS. For the browser, (c) is the cheapest route.

---

## 2. Requirement fit matrix

| Requirement | Rust | Go | Zig | TypeScript | C |
|---|---|---|---|---|---|
| (a) Static binary mac/linux/win × x64/arm64 | Yes: musl on Linux, static CRT on Windows; pi-env's toolchain already does this | Yes, the best cross-compile UX (`GOOS/GOARCH`, no cgo) | Yes, excellent cross-compile | Only via `bun build --compile` / Node SEA (~50–100 MB, runtime included) | Yes, but per-platform toolchains (MinGW for Windows) |
| Link into pi-env (Rust) | **Native**: same binary, shared Job Object/PTY/watch code | Two runtimes via cgo staticlib, or a separate process | C ABI possible, awkward | No | C ABI |
| (b) Browser WASM + VFS | unknown-unknown (async VFS trait, wasm-bindgen) or wasip1/p2 + JS WASI shim; brush 1.46 MB gz, parser 146 KB gz | `GOOS=js` 1.7 MB gz minimum; goroutines can await JS promises; wasip1 lacks pipes; TinyGo smaller but restricted | Small, good, but no ecosystem to reuse | **Native**, zero toolchain; VFS is just JS objects | Emscripten/wasi-sdk; fine, but everything is hand-written |
| (c) Embed in TS/Node host | napi-rs (8k stars) for parser/classifier; or the same WASM in Node; or subprocess | No real N-API story (c-shared + Go runtime in Node is fragile): WASM or subprocess | N-API via C ABI | Native | N-API via C |
| (d) Spawn, job control, PTY, signals, Job Objects, ConPTY | `std::process` + `windows-sys`/`nix`/`libc`; pi-env already has Job Objects, overlapped pipes, PTY; portable-pty (wezterm), process-wrap | Good `os/exec`; job control/ptys via x/sys, creack/pty; ConPTY via third-party | Manual via std + Win32 | Needs native addons (node-pty) or a helper process for PTY/job objects; Bun/Node child_process is OK for simple spawn | Everything manual |
| (e) Reuse | **brush-parser/core (MIT), uutils (MIT), ripgrep crates, jaq, ignore, globset, Bun Shell's Windows logic (MIT, now Rust), deno_task_shell (MIT)** | mvdan/sh (BSD), gojq, goawk, u-root | Essentially none (Bun Shell, the main Zig shell, has left Zig) | just-bash, pi-shell, shelljs | busybox (GPL), toybox (0BSD) |
| Memory safety in a security-adjacent component | Yes | Yes (GC) | No | Yes | No |

### Language assessment

**Rust: recommended.**
* One codebase covers all four deliverables: the pi-env-embedded executor, the
  standalone static `psh` binary, a `wasm32` build for the browser, and N-API or WASM for
  the TS host's parser/classifier.
* The best-in-class reusable parts are all MIT/Apache Rust: brush (highest measured
  compatibility after bash), uutils (92.9% GNU tests, Windows-first via Microsoft),
  ripgrep crates, jaq. Bun's and Deno's Windows-hardened spawn/which logic is MIT Rust we
  can read or borrow.
* pi-env already contains the hard Windows primitives (`windows-sys` Job Objects,
  overlapped pipe I/O, PTY handling, `quote_argument`).
* WASM-size data points: full brush is 1.46 MB gzip, parser alone 146 KB gzip. Go's
  minimum is about 1.7 MB gzip for the shell alone.
* Risks:
  * The async-runtime decision. brush-core requires tokio, while pi-env is synchronous
    and threaded. Embedding brush means accepting tokio inside pi-env or forking the
    runtime layer.
  * `wasm32-unknown-unknown` has no `std::fs`/`std::process`, so every builtin must go
    through our own VFS/IO traits. uutils will not, which forces WASI for those builtins.
  * Compile times and binary size: brush + full uutils is about 15 MB before trimming.
    We need feature-gating and LTO.
  * brush's bus factor. Mitigation: we can maintain a fork.

**Go: strong runner-up, wrong fit for our topology.**
* mvdan/sh is the most widely used and best-tested shell parser/printer anywhere. The
  `interp` handler design is exactly the VFS/allow-list hook model we want. Static
  cross-compiled binaries are trivial.
* Problems:
  * It cannot share a process with pi-env, so we would ship two daemons or shell out.
  * The browser bundle is at least 6 MB raw / 1.7 MB gzip.
  * No credible in-process Node embedding.
  * Its bash compatibility is measurably below brush (57.5% vs 72.1% Oils).
  * Its Windows path semantics are still slated for a v4 redesign.
* Choose Go only if we decided against embedding in pi-env and against a small browser
  build.

**Zig: not recommended.**
* Excellent cross-compilation and small binaries, but pre-1.0 language churn and no
  shell, coreutils or grep ecosystem.
* The most significant Zig shell (Bun Shell) now lives in Rust after Bun's port. The Zig
  repo moved off GitHub (last GitHub push 2025-11).

**TypeScript: for the host-side pieces only.**
* Ideal for browser execution against a JS VFS (just-bash proves the demand: 3.6M
  downloads/week) and for UI summaries.
* It cannot produce a small static binary for remote hosts. PTY, job-object and signal
  control require native addons anyway. A second implementation of the parser in TS
  invites classifier/executor disagreement, which is a security bug class: the permission
  check must parse the same way the executor does.
* Better: compile the Rust parser + classifier to WASM (about 150 KB gzip) and use it from
  Node, browser and pi-env alike.

**C: not recommended.**
* busybox-w32 shows C can be 700 KB and native on Windows, but the code we would want to
  reuse is GPL, it is not memory-safe, and it is not embeddable.

### Recommended architecture (sketch)

```
psh-syntax     Rust: lexer/parser/AST (start from brush-parser; keep spans, serde)
psh-classify   Rust: AST -> {read-only | mutating | network | unknown}, command summaries,
               redirect targets, path args; conservative on eval/source/$(...)/aliases
psh-core       Rust: interpreter over traits  Fs, Proc (spawn/wait/kill/pty), Clock, Env
               (start from brush-core, or fork its expansion/execution logic)
psh-builtins   Rust: shell builtins + coreutils subset (uutils where its I/O can be
               injected), grep via grep-searcher/ignore, jq via jaq
targets:
  native   -> `psh` static binary + linked into pi-env (Proc = real processes,
              Job Objects / process groups, ConPTY / openpty)
  wasm     -> browser build (Fs = JS VFS; Proc = builtins only, or a JS callback for
              hosted commands); ship psh-syntax+classify alone (~150 KB gz) to Node/web
  node     -> napi-rs addon or the same wasm for parse/classify; execution via pi-env
gate: Oils spec suite (research/spec-harness) + brush's bash-oracle YAML cases in CI,
      plus a Windows-specific suite (paths, PATHEXT, .cmd quoting, CRLF, Ctrl-C trees)
```

Build-vs-adopt decision: adopting **brush wholesale** gives about 72% Oils conformance on
day one. The real work is then (1) the Windows backend, (2) a VFS/Proc abstraction that
also serves WASM, and (3) the async-runtime fit with pi-env. Writing our own interpreter
in Rust while reusing **brush-parser** gives more control, but we would need about a year
to reach brush's conformance. My recommendation: build on brush, contribute upstream where
the maintainer agrees, keep a fork for Pi-specific traits, and gate everything on the Oils
suite.

WASM target choice:
* `wasm32-wasip1` + an in-memory WASI shim (`@bjorn3/browser_wasi_shim`, 433 stars;
  Wasmer SDK for WASIX; or `jco` for p2 components) runs uutils unmodified and keeps code
  paths identical to native.
* `wasm32-unknown-unknown` + wasm-bindgen fits an **async** VFS better: OPFS, IndexedDB,
  or remote calls into a pi-env session. brush-core is already async, which helps here.
* Blocking reads in WASI need JSPI or Asyncify, or a worker with `Atomics.wait`, which in
  turn requires COOP/COEP cross-origin isolation for SharedArrayBuffer.
* Expect to support both: unknown-unknown for the shell, wasip1 for heavy utilities.

---

## 3. What Windows specifically needs

Points marked [BUN] or [DENO] are implemented in Bun Shell or deno_task_shell (verified
in source). [BB] marks busybox-w32 behavior from its docs.

1. **Path model.** Internally use `C:/foo` with forward slashes, UNC as `//host/share`,
   and treat backslash as the shell escape character [BB; also mvdan/sh's planned v4
   model].
   * **Accept** `C:\foo` in quoted words and `/c/foo` (Git Bash style, which LLMs emit
     constantly) on input to builtins and `cd`.
   * Convert to native form only at the OS boundary (CreateProcess, file APIs).
   * Do **not** do MSYS-style heuristic rewriting of arbitrary argv to external programs.
     It is the #1 source of Git Bash surprises (`/opt` flags becoming paths). At most,
     translate exact `/c/...`-shaped args behind an option.
   * Keep per-drive cwd semantics in mind (`C:foo`).
   * Use long paths (`\\?\` prefix) internally to escape MAX_PATH = 260.
   * Bun converts posix↔platform separators internally (`posix_to_platform_in_place`) but
     does no `/c/` translation.
2. **Executable resolution.**
   * Search `PATH` (`;`-separated, and the variable may be spelled `Path`) with **PATHEXT**.
     [BUN] hard-codes `.exe/.cmd/.bat` and then `.com` in a final pass instead of reading
     PATHEXT. [DENO] uses the `which` crate, which honors PATHEXT. brush-core has PATHEXT
     support.
   * CreateProcess search order checks the **cwd before PATH** for bare names; respect
     `NoDefaultCurrentDirectoryInExePath` (binary-planting opt-out) [BUN].
   * There is no exec bit, so `test -x` and `command -v` must decide by extension.
   * Shebang emulation for `./script` lets `#!/usr/bin/env node`/`-S` scripts run [DENO].
     Without it, `./foo.sh` cannot run. Consider running `#!/bin/sh` and `#!/bin/bash`
     scripts in-process.
3. **argv quoting.**
   * Windows passes a single command line, and each program re-parses it.
   * Most programs use the MSVCRT/UCRT rules (`CommandLineToArgvW`-compatible):
     * whitespace separates arguments; `"` toggles quoting;
     * backslashes are literal unless they precede a `"`;
     * 2n backslashes + `"` give n backslashes and a quote delimiter, while 2n+1
       backslashes + `"` give n backslashes and a literal `"`;
     * empty arguments must be passed as `""`.
   * Some programs parse their own command line (cmd.exe, msiexec, some installers), so
     we need a raw-arg escape hatch (Rust has `CommandExt::raw_arg`).
   * **`.bat`/`.cmd` go through cmd.exe** and get re-tokenized with `% ^ & | < > "` rules
     ("BatBadBut", CVE-2024-24576/-27980; CVE-2024-43402 covers trailing dots and
     spaces). [BUN] **refuses** to pass arguments containing `"%&|<>^\r\n` to batch
     files, after trimming trailing ` `/`.` from the extension check.
     * Rust std (≥1.77.2) escapes batch arguments and errors when it cannot do so safely.
     * Node refuses `.bat`/`.cmd` without `shell:true`.
     * This matters because npm-installed CLIs are mostly `.cmd` shims, so the
       restriction must be precise rather than blanket.
   * pi-env already has a `quote_argument` in `sys/windows.rs`.
4. **CRLF.**
   * Tolerate `\r\n` in script sources (bash fails with `$'\r': command not found`).
     tree-sitter-bash even has a `crlf` corpus.
   * Strip a trailing `\r` along with newlines in `$(...)`, and in `read` when the input
     came from a native program.
   * Builtins should write `\n`. Text utilities (`grep`, `wc`, `sed`) should treat `\r\n`
     sensibly (a flag or heuristic), because Windows tools and git checkouts with
     `autocrlf` produce CRLF.
5. **Case-insensitivity.**
   * Environment variable names are case-insensitive on Windows (`Path` vs `PATH`) while
     shell variables are case-sensitive. Bun's EnvMap has explicit length-checked handling
     so `PATH` does not match `PATHEXT`.
   * Globbing and path comparison should be case-insensitive on NTFS by default (with
     per-directory case sensitivity as an edge case).
   * Executable lookup is case-insensitive.
6. **Symlinks.**
   * Creating them needs Developer Mode or `SeCreateSymbolicLinkPrivilege`, and there are
     separate file and directory link types plus junctions.
   * `ln -s` should try a symlink, fall back to a junction for directories or fail
     clearly, and never silently copy (MSYS's default `winsymlinks` behavior).
   * `rm -r` must not follow junctions.
7. **Processes, signals and job control.**
   * There is no fork, so subshells, `&` and pipelines into builtins must run in-process
     (tasks or threads). brush and mvdan/sh already work this way.
   * Kill trees with **Job Objects** (`KILL_ON_JOB_CLOSE`) [DENO, pi-env].
   * Ctrl-C means `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT)` to a child started with
     `CREATE_NEW_PROCESS_GROUP`, then terminating the job. There are no SIGSTOP or SIGTSTP
     equivalents, so `fg`/`bg`/Ctrl-Z job control is effectively unix-only.
     microsoft/coreutils does not ship `kill` for this reason.
   * PTY means **ConPTY** (Windows 10 1809+). portable-pty (wezterm) and node-pty wrap it.
   * Pipe I/O needs overlapped I/O or threads (pi-env already does overlapped reads).
8. **Device and reserved names.**
   * Map `/dev/null` to `NUL`, and emulate `/dev/stdin`, `/dev/stdout`, `/dev/stderr` and
     `/dev/tty` in redirections [BB does this for applets and redirections only].
   * Handle reserved names (`CON`, `NUL`, `AUX`…) and trailing-dot/space names.
9. **Console encoding.** Use UTF-8 code page (`SetConsoleOutputCP(65001)`) and VT
   processing. For agent use, mostly pipes, with decoding of non-UTF-8 native tool output
   (pi-env already depends on `encoding_rs`).
10. **Coreutils name clashes.** `find`, `sort`, `timeout`, `where` and `more` exist as
    different Windows programs. In-process builtins must win over PATH for these
    [BB's prefer-applet policy with `BB_OVERRIDE_APPLETS`; microsoft/coreutils documents
    the same clashes].

---

## 4. Risks and open questions

* **Conformance ceiling.** The last 15% of bash (traps, job control, namerefs, aliases,
  `printf` corners) is long-tail work. For an agent shell, define the target as "bash
  non-interactive scripts LLMs write" and measure it with the Oils suite plus a corpus of
  real agent commands from Pi transcripts.
* **Classifier soundness.** Read-only classification must be conservative:
  * treat these as non-read-only: `eval`, `source`, `$(...)` in command position,
    `xargs`/`find -exec`/`git -c`/`env`, aliases and functions defined earlier in the
    session, redirections (`>`, `>>`, `>|`, fd dup to files), `sed -i`, `tee`,
    `$BASH_ENV`;
  * the classifier must use the **same parser** as the executor (this argues for one
    Rust parser compiled to WASM/N-API rather than a TS reimplementation).
* **Licensing.** Avoid GPL code in-process: bash, busybox, yash-rs shell crates, fish,
  bashlex. All recommended reuse is MIT, Apache-2.0, BSD or Unlicense.
* **Async model.** brush-core is tokio-based, pi-env is not. Decide early whether pi-env
  adopts tokio (a reasonable choice given it already multiplexes processes, watches and
  PTYs) or psh-core gets a sync executor.
* **WASM blocking.** Interactive `read`, `sleep` and pipes between builtins need an async
  interpreter (unknown-unknown) or JSPI/Asyncify (WASI). Go handles this more naturally,
  and in Rust it is only solved by keeping the interpreter async.

## 5. Sources

* GitHub API metadata (stars, push dates, licenses, contributors) queried 2026-11-06.
* Local clones in `/tmp/ls`: brush @9435534, mvdan/sh @5943e49, oils @d114978,
  yash-rs @e957083, deno_task_shell @b784de7, bun @b1305dd, uutils/coreutils (2026-11-06),
  busybox-w32 (2026-11-04), tree-sitter-bash.
* Oils: https://oils.pub/release/0.38.0/quality.html (OSH 2,482/2,756), `test/spec-compat.sh`,
  blog "A Pause in the Project" (2026-03).
* uutils GNU results: https://github.com/uutils/coreutils-tracking (`gnu-result.json`, latest
  entry 2026-11-06: 669 pass / 720 total, 35 fail, 13 skip).
* busybox-w32 sizes and path model: https://frippery.org/busybox/ and /paths.html.
* microsoft/coreutils README (winget `Microsoft.Coreutils`, shell-conflict table).
* Bun: `docs/runtime/shell.mdx`, `src/runtime/shell/states/Cmd.rs` (BatBadBut refusal),
  `src/which/lib.rs` (search order, extensions, CVE notes).
* deno_task_shell: `src/shell/child_process_tracker.rs`, `src/shell/command.rs` (shebang),
  `src/parser.rs`.
* npm weekly downloads (2026-10-29..11-04): shelljs 12.2M, just-bash 3.6M,
  tree-sitter-bash 3.4M, zx 2.8M, sh-syntax 467k.
* Spec harness and raw results: `research/spec-harness/`.
