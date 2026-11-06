# oh-my-pi's embedded shell: technical study

Source: `~/Development/oh-my-pi` at `75e7801e24` (v18.17.0, 2026-11-06). Read-only; nothing in the repo was modified.
All paths are relative to that repo unless noted. Line counts come from `wc -l` over `*.rs`.

## TL;DR

- `crates/pi-shell` is a Rust bash 5.3 engine (plus a zsh 5.9 dialect with ZLE). It is not a wrapper around GNU bash or brush, and it is not machine-transpiled C. It is a hand-structured Rust VM (IR, cooperative scheduler, host seam) whose lexer, parser, expansion, globbing, printf/test and so on are described in the source itself as *ports* of the GNU bash C functions. The crate is labeled MIT and carries no GPL notice. That is a real license question (see §1.5).
- The engine replaced the vendored `brush-core` on **2026-11-03** (`44711a78c1`, +163k/−81k lines, 1044 files), three days before the HEAD I studied. The commit message says it came from another workspace. As a pi component it is therefore very new.
- `crates/pi-builtins` has about 80 in-process utilities: uutils coreutils/findutils/sed ports, jaq, a ripgrep-based grep/rg, fd, ast-grep, and a 63k-line "clean-room" rsync. They shadow PATH binaries by default so the shell rarely forks for common tools.
- Measured: a release `obash` (the engine as a standalone shell) builds in **3m12s** (fat LTO, M-series, 18 cores). The binary is **9.7 MB** (arm64, only libSystem linked), versus about 1.3 MB for `/bin/bash`. Startup was **2.9 ms vs 4.7 ms** for bash 5.3.9, and a 300k-iteration arithmetic loop took **0.10 s vs 0.46 s**. My 15-construct differential script gave output identical to bash 5.3.9.
- There is no WASM build and no browser support. Only the parser module claims `no_std` + `wasm32-unknown-unknown`. The engine does sit entirely behind a `Host`/`Fs` trait with an in-memory test host, so it is architecturally WASM-ready but not proven.
- Approval and permission prompts do **not** use the engine's AST. They use regexes plus a hand-written TS tokenizer. The AST is used by the output minimizer (`minimizer/plan.rs`), the live execution visualization (`RunTrace`), and runtime git policy (the `git` guard judges expanded argv).

---

## 1. Architecture of `pi-shell`

### 1.1 Size and layout

`crates/pi-shell/src`, 330,426 lines in total:

| Area | Lines | Notes |
|---|---|---|
| `bash/` without zsh/zle files | ~99.3k | engine: `syntax/` 21.9k, `vm/` (non-zsh), `pattern/` 9.1k, `ir/` 4.0k, `trace/` 9.4k |
| `bash/` zsh + ZLE files | ~128.6k | `vm/zle` 55.5k, `vm/zsh` 24.3k, `vm/builtins/zsh_mod_*`, `syntax/zsh`, `pattern/zsh` |
| `minimizer/` | 29.5k | output compressor (§5) |
| `sys/` | 14.5k | OS host: spawn, pumps, fs, pathext, shebang, Job Objects |
| `testkit/` + `suites/` | 17.4k | fake host, MemFs, fault injector, goldens |
| `shell/`, `session.rs`, `host.rs`, `vfs.rs`, `tools/`, `git/`, `extras.rs`, `process/` | rest | pi integration |

Module overview: `crates/pi-shell/src/lib.rs`. Engine overview: `src/bash/mod.rs`.

### 1.2 Engine design (`src/bash`)

- **Parsing** (`bash/syntax/`, `#![forbid(unsafe_code)]`). The lexer and parser are ports of bash's `read_token`, `read_token_word`, alias expansion and here-doc gathering (`syntax/parser/lex.rs:1`, `syntax/parser/word.rs:1`). There is a `declare -f` printer that ports `print_cmd.c` (`syntax/print.rs:3`), and a JSON export, including an shfmt-compatible form (`obash --json=shfmt`). The parser is incremental: it parses and compiles one top-level command at a time, as bash does. Node ids are a deterministic preorder numbering, so clients can re-parse a `UnitCompiled` event and get the same ids (`syntax/mod.rs` docs).
- **IR** (`bash/ir/mod.rs`). Each top-level command, function body, eval string and so on becomes a `Unit`: a flat array of 16-byte ops (`Exec`, `Jmp`, `BrOk`, `ForIn`, `Spawn`, `CaseArm`, `Arith`, `CondTest`, …) with side tables. Words stay structured and are expanded by the expansion engine. Units are cached content-addressed in `Engine` (32 MiB budget, `bash/engine.rs`).
- **VM** (`bash/vm/mod.rs`). This is a cooperative scheduler of *virtual processes* (vprocs). `Shell::step` runs ready vprocs round-robin, 1024 ops per slice (`SLICE`). **Every op is a safepoint**: the loop checks a control word before each op, so pause, cancel, signals and suspend take effect between ops. Subshells, pipelines, command substitutions and background jobs are vprocs, not forks. `fork()` is never used for shell constructs; only externals become OS processes. `BASH_VERSION = "5.3.0(1)-release"`.
- **Host seam** (`bash/host.rs`). The `Host` trait carries every side effect: `resolve`, `start` (native tasks), `spawn` (returns a pollable `SpawnFuture`), `fs()` (the `Fs` trait in `bash/fs.rs`: open/stat/access/read_dir/realpath/mkfifo_or_temp/…, all pollable), `stdio`, `timer`, `now_ns`, `random`, `user`, `tty`, `terminal`, `sockets`, `pipe_tap`, and `event(&Event) -> Verdict {Continue, Pause, Cancel(sig)}`. The rules are documented: `resolve`/`start` are synchronous registry lookups, and nothing may block. Three hosts exist: `process::NativeHost` (obash and `omp bash|zsh|sh`), `host::PiHost` (the embedded agent shell), and `testkit::FakeHost` + `MemFs` (deterministic, with seeded latency and EIO/ENOSPC injection).
- **State store** (`bash/store.rs`, `mem_store.rs`). Variables, functions, options, traps and cwd live in a host-owned `StateStore`. `Shell::externalize`/`restore` (`bash/snapshot.rs`) serialize the execution position (frames, partial words, fd tables including in-memory pipe contents) with postcard. A running shell can therefore be snapshotted and resumed in another process; the `exec` fault mode of obash tests exactly this.
- **Events** (`bash/event.rs`). Node-by-node events with spans. They feed `bash/trace` (`RunTrace`), which the TS UI renders as a "Raw/Pseudo" live view of the running script.
- **Job control** (`bash/vm/jobctl.rs`). Interactive job control ports zsh 5.9 `jobs.c`/`exec.c`/`init.c`, with bash's `jobs.c` where the two differ. The embedded agent shell has no terminal (`PiHost` "never reports a terminal"), and every external leads its own session and process group (`Sessions::Detach`, `src/host.rs:1-10`).
- **Process spawning** (`src/sys/spawn/`). Unix uses `posix_spawn` and falls back to fork+`pre_exec` when the umask differs (`sys/spawn/unix.rs`, 1756 lines; bench `benches/sys_spawn.rs` targets ≤1.05× bash). Windows is covered in §3.
- **PTY.** The engine does not own a PTY for agent runs. `pty: true` tool calls and the zsh/fish `!` path use `portable-pty` in `crates/pi-natives/src/pty.rs` (999 lines). The engine has its own `Terminal`/`Pty` abstractions (`bash/term.rs`) for interactive `omp zsh`/`obash -i` with ZLE.
- **Cancellation** (`src/session.rs` docs, `src/cancel.rs`). Cancel first sends `TERM` through the engine (traps run, externals get TERM, status 143). After `TERM_GRACE` = 1 s it sends `KILL` (status 137). The spawner signals the process groups of abandoned externals. `nohup` runs its operand in a detached lineage that survives cancel and teardown. `kill` refuses to signal pi itself, its process group or its ancestors (`host.rs:check_signal_target`).
- **Persistent sessions** (`src/session.rs`). One driver task per session on a tokio runtime steps the shell. Each command runs on the root vproc, so variables, functions, aliases, options, traps and cwd persist. `exit`/`set -e` end the *run*, not the session. Standard streams are swapped per command (`StdStreams`).

### 1.3 Third-party crates

From the root `Cargo.toml` comment "Third-party crates of pi-shell's bash engine … at the versions of the workspace it came from": `blake3` (rayon), `bstr`, `encoding_rs`, `hashbrown`, `jiff`, `libm`, `postcard`, `strum`, `tempfile`.

`crates/pi-shell/Cargo.toml` also lists `anyhow`, `async-trait`, `base64`, `bytes`, `clap`, `flume`, `parking_lot`, `pcre2-sys` (zsh `pcre_*`), `regex`, `serde`/`serde_json`, `thiserror`, `tokio` (full), `tokio-util`, `toml`, `xxhash-rust`, `libc` (unix), `windows-sys`, `winreg` and `smallvec` (windows). It also depends on `pi-builtins`, `pi-vcs` (which pulls `gix` and `jj-lib`), and `pi-vfs`. The engine core does **not** use a parser generator or any existing shell crate.

### 1.4 Provenance

- `git log` shows oh-my-pi ran on **brush** (`brush-core`/`brush-parser`, MIT) from 2026-02-01 (`5657367802`) until 2026-11-03. It vendored and heavily patched brush, with fixes for heredoc deadlocks on Windows and macOS, zombie processes, cancellation, Windows PATHEXT, and `Display` round-trip bugs in the minimizer. See `packages/natives/CHANGELOG.md` around lines 1376–1588.
- `44711a78c1` "feat(shell): replaced brush with pi's own bash engine in pi-shell" (2026-11-03) merged the engine in. It "removed vendored brush-core and brush-parser and every brush mention, licenses included". The engine came from another (private) workspace, so its history is not in this repo.
- Since then there have been 60 commits to `crates/pi-shell` in 3 days, from five authors (can1357 28, Brit 17, Sami Jawhar 6, jchanghong 6, Will 3). They include "fix(shell): build the engine on Windows again" (`035e4f95c3`, 11-04) and "restore Windows PATH and PATHEXT command lookup" (`4dec281692`). Windows regressed when the switch landed.

### 1.5 Port of GNU bash? License implications

**This is the most important finding for reuse.**

- `lib.rs:28-31`: `#![allow(clippy::many_single_char_names, reason = "the bash and zsh ports keep upstream's single-letter locals so the code reads against the C sources")]`.
- Explicit "port of bash" claims in module docs:
  - `syntax/parser/lex.rs` (`read_token`, `special_case_tokens`, `alias_expand_token`)
  - `syntax/parser/word.rs` (`read_token_word`)
  - `syntax/print.rs` (`print_cmd.c`)
  - `vm/glob.rs` (`glob_filename`, `glob_vector`)
  - `pattern/mod.rs` (`lib/glob/sm_loop.c`, "quirks included")
  - `vm/builtins/test.rs` (`test.c`, "quirks and messages included")
  - `vm/builtins/printf.rs` (`builtins/printf.def`)
  - `vm/expand.rs:3496`
- 171 references of the form ``bash's `function_name` `` across 148 distinct bash C functions, for example `shell_execve`, `array_expand_index`, `waitchld`, `sv_path`, `skipsubscript`, `parse_and_execute`, `fc_replhist`.
- The zsh parts are attributed correctly. Many files say "Ported from zsh 5.9 (MIT-style licence, Copyright (c) 1992-1997 Paul Falstad …)" (`vm/zle/*`, `vm/exec.rs:4722`). zsh's license permits this with notice.
- **No equivalent GPL notice exists for the bash-derived code.** `crates/pi-shell/LICENSE` is MIT ("Can Bölük, Stencil Labs"), and `NOTICE` mentions only rtk. GNU bash is GPLv3+. A function-by-function translation that preserves structure, locals and messages is, in my reading, very likely a derivative work. I am not a lawyer.
- Further signals: `obash --version` prints bash's own banner: "GNU bash, version 5.3.0(1)-release … Copyright (C) 2025 Free Software Foundation, Inc. License GPLv3+". The rsync builtin calls itself "clean-room" (`pi-builtins/src/rsync.rs:1`) but prints `"license": "GPLv3"` and the Tridgell/Davison copyright in `--version` (`rsync/usage.rs:23,207,508`).
- Whether it was produced by an LLM or by hand is not stated. A 330k-line engine with exhaustive per-function correspondence, developed off-repo, is consistent with large-scale assisted translation.
- **Implication for Pi:** copying `pi-shell/src/bash` (non-zsh parts), or even closely reading it while writing our own, risks GPL contamination. The zsh port, the minimizer (MIT, with rtk attribution), `pi-builtins` (MIT uutils/jaq ports, notices in `crates/pi-builtins/LICENSE`) and the architecture *ideas* are safe to study. Their code is MIT.

## 2. Builtins / coreutils (`crates/pi-builtins`, 172k lines)

- **Inventory** (`src/extras.rs`, 80 table rows):
  - `ast-grep`/`sg`, `b2sum`, `base32`, `base64`, `basename`, `cat`, `chmod`, `cksum`, `cmp`, `comm`, `cp`, `combine`, `cut`, `date`, `dd`, `df`, `diff`, `dirname`, `du`, `env`, `errno`, `fd`, `find`, `fold`
  - `grep`, `rg`, `head`, `hostname`, `id`, `ifne`, `isutf8`, `jq`, `ln`, `ls`, `md5sum`, `mkdir`, `mktemp`, `mv`, `nl`, `nproc`, `od`, `paste`, `printenv`, `readlink`, `realpath`, `rm`, `rmdir`, `rsync`, `sed`, `seq`, `sha*sum`, `sort`, `sponge`, `stat`, `tac`, `tail`, `tee`, `touch`, `tr`, `truncate`, `ts`, `uname`, `uniq`, `uptime`, `wc`, `which`, `whoami`, `xargs`, `yes`
  - process builtins: `ps`, `top`, `sleep`, `pgrep`, `pkill`, `pidwait`
  - Pi tools (`src/tools/`): `timeout`, `nohup`, `git`
  - Not provided: `awk`, `tar`, `gzip`, `curl`, `make`, `perl`, `python`, `less`, `file`.
- **Origins** (`crates/pi-builtins/LICENSE`): uutils coreutils (most), uutils findutils (`find`, `xargs`), uutils sed, jaq 3.1.1 (`jq`), rust-utf8 (in `wc`). `grep`/`rg` use the ripgrep `grep-*` crates and `pi-walker`. `rsync` is a 63k-line reimplementation of rsync 3.5.1 (protocol 29–33) that interoperates with real rsync peers. About 50 crates used to be vendored separately and were consolidated (CHANGELOG line ~1010).
- **Contract** (`crates/pi-builtins/README.md`, `src/host.rs` 3.9k lines). Each utility is a `clap` parser plus `run(self, host: &mut Host) -> i32`. `Host` gives the command's fds, the shell cwd (`host.resolve(path)` is required, since the process cwd is unrelated), the exported env, cancellation, and a `CommandRunner` for `RUNS_COMMANDS` utilities (`env`, `xargs`, `find -exec`, `ifne`), whose commands run as *subshells of the calling shell* so they reach builtins and URL paths too. The adapter handles `--help`/`--version`, usage errors, emulated SIGPIPE, cancellation (status 130), and panic containment (`panic = "unwind"` in the release profile specifically for this; root `Cargo.toml` comment).
- **Resolution order** (`crates/pi-shell/src/extras.rs` docs, `src/host.rs:533 PiHost::resolve`): a name containing `/` never resolves to an extra. Otherwise the order is engine intrinsics (bash builtins), then pi tools, then pi extras, then `PATH`. **Extras shadow system binaries unconditionally.** `type`/`command -v` report them as shell builtins. There are opt-outs via the session env, read once at session start: `PI_DISABLE_UUTILS_BUILTINS` (all), `PI_DISABLE_UUTILS_DESTRUCTIVE` (`rm mv cp ln dd rsync`), `PI_DISABLE_RM_BUILTIN`, `PI_DISABLE_MV_BUILTIN`. `env jq …` reaches the system binary.
- **"Never fork" rationale** (README): "a long-lived embedded shell resolves these names itself, on every platform, whether or not the host has the real binaries." Other benefits: URL paths (`skill://…`) work in builtins, cancellation is cooperative, and Windows gets a POSIX toolset without MSYS. Externals run with stdin `/dev/null` and other non-interactive env hardening.
- **GNU vs BSD flags.** The semantics are GNU, because uutils targets GNU compatibility. There are targeted BSD shims: `sed -i ''` (`sed.rs:9121`) and `stat`'s `rewrite_bsd_invocation` (`stat.rs:1480`). `grep` accepts some BSD-only options (`grep.rs:378`), and `ps -o` accepts Apple specifiers. On macOS this changes behavior compared with the system's BSD tools, and on balance helps, because LLMs mostly write GNU syntax. `jq` is jaq, with documented differences from jq 1.8 (`docs/bash-tool-runtime.md`, "Bundled jq compatibility").
- **Build hygiene.** Each utility sits behind a feature (`util.<name>`). The crate opts out of the workspace clippy policy because it is ported code (`Cargo.toml` tail).

## 3. Windows

- **No Git Bash or MSYS required.** The engine *is* the shell. `packages/coding-agent/src/exec/bash-executor.ts` (around line 645) says: "Never wrap in cmd.exe: it is only the Windows no-bash fallback for spawn paths, and the embedded shell runs the POSIX line better directly." With no bash installed, `getShellConfig()` falls back to `cmd.exe` (`packages/utils/src/procmgr.ts:148-192`). The tool still runs the command in the embedded engine, so POSIX syntax plus about 80 builtins work. Anything outside the builtin set (awk, tar, …) must be on PATH.
- **`src/windows.rs`** (464 lines):
  - `configure_windows_path` discovers Git for Windows (registry and common roots) and appends its tool dirs to PATH.
  - `translate_msys_segment` rewrites MSYS-style PATH entries (`/c/Users/x` → `C:\Users\x`, `/usr/bin` → `<git root>\usr\bin`). This translation applies **only to PATH entries**. I found no general argument or path translation of `/c/...` in the engine.
  - It also provides 8.3 ↔ long path helpers (`GetLongPathNameW`/`GetShortPathNameW`), exported to JS as `expandWindowsLongPath`/`getWindowsShortPath`.
- **Engine platform mode** (`bash/ids.rs:237 Platform::{Linux,MacOs,Windows}`). On Windows the PATH separator is `;`, `PATHEXT` suffixes are searched (`vm/probe.rs:469-540`, `sys/pathext.rs`), `\` counts as a slash in command names (`vm/command.rs:1848`), and drive-absolute paths are recognized (`vm/probe.rs:574`).
- **Spawning** (`sys/spawn/windows.rs`, 830 lines):
  - `CreateProcessW` creates the child suspended in a new console process group, assigns it to a **Job Object per job**, then resumes it.
  - Only the 3 stdio handles are inherited (`PROC_THREAD_ATTRIBUTE_HANDLE_LIST`).
  - Command lines use MSVCRT quoting, and `.bat`/`.cmd` run via `cmd.exe` with std's CVE-2024-24576 hardening (`sys/cmdline.rs`, ported from Rust std).
  - Signals: KILL = `TerminateJobObject`; TERM/INT = `CTRL_BREAK_EVENT`, then KILL after the grace period. Pause/resume = `NtSuspendProcess`/`NtResumeProcess` per member (`sys/job.rs`).
  - Fds above 2 are unsupported for externals.
- **Shebangs** (`sys/shebang.rs`). The host parses `#!` itself, because Windows has no kernel support. `sh`/`bash`/`zsh` interpreters (directly or via `/usr/bin/env`) run *in-engine* (`Resolved::SelfShell`) on every platform.
- **Process substitution.** It uses named pipes `\\.\pipe\obash-<pid>-<seq>` (`sys/fs.rs` around line 894).
- **Output decoding** (`src/output_decode.rs`, `sys/relay.rs`). Each external program's output is decoded separately: UTF-8 until the first invalid sequence, then the ANSI code page (`GetACP`, e.g. CP936). Builtin output is judged line by line. This fixes CJK mojibake (乱码).
- **Gaps and risk:**
  - Rust tests run only on Linux CI (`.github/workflows/ci.yml` `rust_validate`, bazel, post-merge only). Windows gets only release-binary smoke tests.
  - I found no `/dev/null` → `NUL` mapping in `sys/fs.rs::host_path`; `pi-builtins/src/host.rs:3214` special-cases it only in tests. Behavior of `>/dev/null` on Windows is unverified by me.
  - `PATH`/`PATHEXT` lookup regressed on the switch and was fixed on 11-04.

## 4. VFS and isolation

- **`crates/pi-vfs`** (7.6k lines) is an *injectable async filesystem* shared by the engine and the builtins. Host paths go straight to std/libc. `scheme://authority/...` URLs go to a provider (`FileSystem` trait, `src/provider.rs`). In pi the provider is a **JS callback** (`ShellFilesystem`, `crates/pi-natives/src/shell/vfs.rs`, 1.6k lines; TS `InternalUrlFilesystem`) that serves `skill://`, `artifact://`, `local://`, `proc://` and so on. With `nativeLocalPaths: false`, *all* paths, host ones included, go to the handler (`docs/natives-shell-pty-process.md` "Host-injected filesystem").
- **This is not a sandbox.** Externals see the OS filesystem; URL arguments are translated to their backing host files before spawn (`host.rs:581-593`). "read-only is otherwise enforced only for builtins" (`docs/bash-tool-runtime.md`). There is no seatbelt, landlock or bwrap in the agent. Approval is not containment, and the doc says so.
- **`crates/pi-iso`** (4.9k lines) is not shell sandboxing. It creates copy-on-write *worktrees* for task isolation: APFS `clonefile`, btrfs/ZFS snapshots, Linux reflink, overlayfs/fuse-overlayfs, Windows block clone or ProjFS, with a `git worktree`/copy fallback, plus diffing (`src/lib.rs`). The shell's `cp` uses `pi_iso::cow`, and smart `git worktree add` (`src/git/mod.rs`) uses `pi-vcs` clones.
- **WASM and running without an OS:**
  - `bash/syntax/mod.rs:22` claims "The crate is `no_std` + `alloc`, builds for `wasm32-unknown-unknown`, and contains no `unsafe`". The claim is inherited from the origin workspace, where syntax was its own crate. Inside `pi-shell` it is just a module, and `pi-shell` depends on tokio `full`, libc and others.
  - `pi-builtins/Cargo.toml:150` has a `cfg(target_family = "wasm")` tokio dependency, and there are about 21 `cfg(not(any(unix, windows)))` fallbacks.
  - There is **no wasm target in CI, no wasm build script, and no browser integration**. The `pi-ast` wasm use is for tree-sitter *grammars*, which is unrelated.
  - Architecturally the engine is close: `bash/` is `#![forbid(unsafe_code)]`, all I/O goes through the `Host`/`Fs` traits, there is an in-memory `MemFs`, and a deterministic `FakeHost` drives scripts with no OS (`testkit/`). The vm only references `libc::E*` constants in `vm/fsprobe.rs` and a few zsh modules. A wasm port would need to split `bash/` (and zsh) into its own crate without tokio and with cfg'd libc, and supply a JS host.

## 5. Minimizer, taps, output_decode, git

- **Minimizer** (`src/minimizer*`, 29.5k lines; on by default via `shellMinimizer.enabled: true` in `packages/coding-agent/src/exec/settings.ts:217`):
  - It rewrites verbose command output before it reaches the model. There are 24 Rust filter modules (`git`, `cargo`, `bun`, `go`, `python` (pytest), `jvm`, `docker`, `gh`, `glab`, `dotnet`, `lint`, `node_tests`, …) and 67 declarative TOML filters (`minimizer/defs/*.toml`: terraform, helm, make, apt, brew, xcodebuild, …), concatenated by `build.rs`.
  - `minimizer/plan.rs` parses the command with the **engine's own parser**. Pipes are opaque (never rewritten), `&&`/`;` chains are segmented, and anything else passes through. `minimizer/detect.rs` identifies the program and subcommand.
  - The original output is returned (`MinimizerResult.original_text`) and saved as an artifact, and the agent sees `[raw output: artifact://<id>]`. Settings files have an optional xxhash trust gate (`settingsHash`).
  - Only the pytest state machine is adapted from rtk-ai/rtk (MIT; `crates/pi-shell/NOTICE`, `minimizer/filters/python.rs`).
- **Taps** (`src/taps.rs`). These mirror every `|` pipe of a run (up to 16 streams, 1 MiB retained each) so the agent and UI can read `proc://<id>/streams` while e.g. `cargo build 2>&1 | tail -5` is still running and `tail` is holding its output back.
- **output_decode**: see §3 (per-producer UTF-8/ANSI code page decoding).
- **git** (`src/git/`, `src/tools/git.rs`). `git` is a host callout that is opt-in via env:
  - `PI_SMART_GIT` serves `git worktree add` in-process through `pi-vcs` with copy-on-write clones, so ignored build caches come along.
  - `PI_GIT_GUARD` (from `bash.gitGuard`; `git/guard.rs`, 1.7k lines) refuses `stash`, `reset --hard`, `switch`/`checkout` of branches, and similar commands in a checkout other agents may share. In linked worktrees it records discarded changes on `refs/omp/discarded`. It judges the **expanded argv in the directory git actually runs in**. This is the one place where the engine's runtime semantics power a safety policy.
  - Everything else runs the git binary by absolute path.

## 6. Command parsing and classification

- **Approval** (`packages/coding-agent/src/tools/bash.ts:185-245, 541-630`) uses `CRITICAL_BASH_PATTERNS` regexes (`rm -rf /`, `curl | sh`, `mkfs`, fork bomb, …) plus user `bash.patterns` (allow/deny/prompt regexes). With `bash.allowCompoundCommands`, flat `&&` chains are split by the **TS tokenizer** `tools/shell-tokenize.ts` (662 lines; "deliberately not a full POSIX parser"). The interceptor (`tools/bash-interceptor.ts`, which suggests the dedicated read/grep/find tools) also uses regexes and the TS tokenizer. **None of this uses the engine AST**, even though the AST is in-process.
- **Engine AST consumers:** the minimizer plan (§5), `RunTrace`/`ShellVisualization` (live Raw/Pseudo rendering of the executing script, source-mapped with per-command outcomes, replay and last-setter provenance; `bash/trace/`, `docs/bash-tool-runtime.md` "Native shell visualization"), and `obash --json`/`--print`. 18.17.0 removed the JS `parseShell()` export (natives CHANGELOG "Breaking"). There is currently **no** JS API to parse a command into an AST.
- **Runtime hooks that could support policy:** `Host::resolve` sees every command name after expansion, `Host::spawn` sees the full argv/cwd/env, and `Host::event` can return `Verdict::Pause|Cancel` at any safepoint. Pi uses this only for the git guard and for `kill` target checks. An approval-at-exec-time design would fit naturally here.
- **`pi-ast`** is a tree-sitter/ast-grep code search and edit engine. It links `tree-sitter-bash` as a code grammar only. **`pi-predict`** is prompt-composer word completion (ngram, SmolLM2, NSSpellChecker), unrelated to shells.

## 7. Testing, benchmarks, size

- **Test counts:** 3,031 `#[test]` in `pi-shell` and 1,950 in `pi-builtins`.
  - 62 engine suites (`src/testkit/suites/*.rs`, many annotated "Goldens from bash 5.3.20"; 107 such references).
  - 53 "skeleton" acceptance scripts (`src/testkit/fixtures/skeleton/*.{sh,out,err,status}`), checked against bash 5.3 *and* re-run under the fault injector (pause/resume, and externalize → drop → restore at every k-th safepoint, k ∈ {1, 7, 97}) with identical output (`testkit/suites/skeleton.rs`).
  - Parser goldens are generated from real bash 5.3 and shfmt (`bash/syntax/suites/data/regen.py`, `declare_f.txt` 712 lines, `errors.txt`, `aliases.txt`, `shfmt/case*.{sh,json}`), plus a 4,349-line zsh corpus.
  - 40 integration tests in `tests/`: obash end-to-end, PTY differential tests against real zsh for ZLE/completion (`tests/pty_support`, `differential()`), process shell, vfs, and minimizer fixtures.
  - The GNU bash test suite (`tests/run-all`) is **not** vendored. The bash-differential corpus in the repo is modest compared with the code size. The origin workspace may hold more.
- **CI:** Rust tests via bazel on Linux, post-merge only, not on PRs (`ci.yml:226-262`). macOS and Windows are only release smoke-tested.
- **Benchmarks** (`crates/pi-shell/benches/`), each with a stated target:
  - `create` (shell create+run ≤ 100 µs)
  - `frames` (externalize ≤ 30 µs, blob ≤ 1 KiB)
  - `memory` (shell ≤ 64 KiB, ≤ 2 KiB per idle vproc)
  - `overhead` (events DEFAULT ≤ 5 %, ALL ≤ 25 %, safepoint check ≤ 3 %, measured with a `no-safepoint-check` feature build)
  - `sys_spawn` (≤ 1.05× bash)
- **My measurements** (M-series, macOS, `cargo build --locked --release -p pi-shell --bin obash`, separate target dir):

  | | obash | GNU bash 5.3.9 |
  |---|---|---|
  | build | 3m12s wall / 9m30s user (fat LTO, cu=1) | – |
  | binary | **9.7 MB** arm64, only `libSystem` linked | ~1.3 MB (`/bin/bash` 3.2 universal) |
  | `-c :` startup (200 runs) | 2.9 ms | 4.7 ms |
  | 300k-iteration `$((…))` while loop | 0.10 s | 0.46 s |
  | 300 × `/usr/bin/true` spawn | 0.29 s | 0.50 s |
  | 20k string appends with `${i//1/x}` | 0.01 s | 0.04 s |

  - obash's 9.7 MB excludes `pi-builtins` and gix: `type cat` → `/bin/cat`, and no uutils/gix strings are in the binary (dead-stripped). It does include the zsh dialect, ZLE and the minimizer. Running it as `zsh` (symlink) switches dialect.
  - A 15-construct differential script (assoc arrays, extglob, `=~`, `${x@Q}`, `coproc`, `mapfile < <(…)`, `pipefail`, printf, EXIT trap) gave output identical to bash 5.3.9. obash also reproduced bash's parse-time syntax error for `shopt -s extglob` used on the same line.
  - Full addon size for reference: the published `pi_natives.darwin-arm64.node` 18.2.8 (pre-engine, brush era) is **165 MB**, with everything included (`~/.bun/install/cache/@oh-my-pi/pi-natives-darwin-arm64@18.2.8@@@1/`).

## 8. Integration with the TS agent

- **N-API** (`crates/pi-natives/src/shell.rs`, 1,165 lines; `packages/natives/native/index.d.ts:610`):
  - `class Shell { constructor({sessionEnv, snapshotPath, minimizer, filesystem}); run({command, cwd, env, timeoutMs, signal, filesystem, streams, trace}, onChunk) → {exitCode?, cancelled, timedOut, minimized?, workingDir?}; abort(); liveBackgroundJobCount(); pids() }`
  - `executeShell(...)` for one-shot runs; `RunTrace`; `ShellVisualization`; `PipeStreams`; `PtySession`; `runShellMain(argv)` (for `omp bash|zsh|sh`)
  - Output is streamed via a ThreadsafeFunction with a bounded 64-chunk queue (backpressure). A callback stalled for 30 s disconnects.
- **Tool schema** (`packages/coding-agent/src/tools/bash.ts:337-379`): `{command, timeout?, cwd?, pty?, async?, name?, ready?{log,port,host,timeout}}`. There is no `env` parameter. The prompt (`src/prompts/tools/bash.md`) says: "Persistent shell … `cwd`, not `cd` … No head/tail/redirection; output trunc by default, full result at artifact://<id>".
- **Executor** (`src/exec/bash-executor.ts`, 992 lines):
  - `Shell` instances are cached per (shell, prefix, snapshot, sessionEnv, agent session key, minimizer config), so variables and functions persist across tool calls.
  - Concurrent calls on a busy key get a one-shot `Shell`. Timeout, cancel or failure quarantines the session.
  - The user's bash rc is captured as a snapshot (`utils/shell-snapshot.ts`) and sourced at session creation.
  - direnv/devenv is preflighted, and `buildNonInteractiveEnv` sets pagers off, `GIT_EDITOR=true`, `TERM=dumb`, `CI=true`, and so on.
  - Timeout defaults to 300 s (clamped 1–3600, `0` = none). Auto-backgrounding after 60 s (`bash.autoBackground`) turns the call into a job `bg_N`.
  - `OutputSink` keeps 50 KiB inline (20 KiB head plus rolling tail) and caps lines at 768 columns. The full output spills to an artifact (≤ 16 MB).
  - Kitty and Sixel images in output are extracted as image content.
- **Fallbacks to system shells:**
  - Model `bash` calls **always** run the embedded engine. There is no fallback to `/bin/bash`, apart from the `PI_DISABLE_UUTILS_*` switches, which only affect utilities.
  - System shells are used for: named services (`name`/`ready`, via the configured shell on a broker PTY); `pty: true` interactive runs (`portable-pty`); the user's `!` bang commands under zsh/fish (headless PTY); and the `eval` tool, which can spawn anything and is not covered by `bash.patterns`.

## 9. Assessment

**Maturity.**
- **Engine:** It looks very high quality and ambitious. It has a safepointed VM, externalizable snapshots, a fault-injection oracle, golden tests against bash 5.3, and performance at or above bash. It is also brand new in pi (3 days at the time of study) and still regressing (Windows build and PATH lookup). Its 330k lines are largely written by one or two people, most of it zsh/ZLE that an agent tool never needs.
- **Builtins:** More battle-tested. They have been in use since about 2026-08 (brush era) and are mostly ports of maintained upstreams.
- **Minimizer:** In production since 2026-06.

**Maintenance burden.**
- About 230k lines of engine plus 170k of utilities, in a style that tracks C sources line by line.
- Rust CI is Linux-only and post-merge.
- Pinned nightly toolchain (`nightly-2026-10-06`; `rust-version = "1.101"`).
- A vendored napi patch.
- Keeping uutils ports in sync is manual.
- For Pi this would be an enormous dependency for "run a bash line reliably".

**Worth copying (license permitting):**
1. **The Host seam design.** All side effects go through one trait, filesystem operations are pollable, and spawning is a future. This is what makes VFS, testing, snapshotting, and possibly a wasm or remote host feasible. Copy the *design*, not the code.
2. **Safepoint cancel/pause plus TERM→KILL with process groups and Job Objects.** The Windows spawn recipe (suspended create → Job Object → resume; handle-list inheritance; CTRL_BREAK then TerminateJobObject; std's batch-file quoting) is a good checklist. `sys/cmdline.rs` is ported from Rust std (MIT/Apache).
3. **In-process utilities that shadow PATH**, with env kill switches and `type` reporting them as builtins. Give `xargs`/`find -exec`/`env` a command runner that dispatches back into the shell. `pi-builtins` is MIT (uutils/jaq notices) and could be reused or forked by module. Each utility is feature-gated, which makes that practical.
4. **Per-producer output decoding on Windows** (`output_decode.rs`), **pipe taps**, and the **minimizer** structure: engine-parsed command shape, pipes opaque, original kept as an artifact. All MIT.
5. **Shebang handling that runs `sh`/`bash`/`zsh` scripts in-engine** (portable, and keeps nested scripts on the VFS).
6. **Runtime policy at `resolve`/`spawn`**: judge the expanded argv and the real cwd, as the git guard does. This is better than static regex approval, and OMP doesn't yet apply it to approvals.

**Pitfalls and caveats:**
- **GPL exposure** of the bash-derived engine code (§1.5). Do not vendor `pi-shell/src/bash` into an MIT project without legal review. The zsh parts are fine with notices.
- Shadowing system tools changes behavior: GNU semantics on macOS, jaq ≠ jq, no `awk`. LLM-written scripts usually expect GNU, but users who rely on BSD or real jq quirks will be surprised. Opt-outs are env-only.
- The VFS is not a security boundary: externals bypass it, and read-only is enforced only for builtins.
- Windows is the weakest-tested platform despite being a key motivation. Missing `/dev/null` mapping is unconfirmed but plausible.
- Binary weight: about 10 MB for the engine alone, and much more with the builtins, gix/jj and so on. A pi-env bootstrap onto remote hosts would need to ship this per architecture.
- No WASM today. Getting there requires crate surgery (split syntax and vm away from tokio/libc) and writing a JS host. The `FakeHost`/`MemFs` testkit shows the shape such a host would take.

**Artifacts.** The release `obash` I built is kept at `/tmp/obash-omp` for further differential experiments. The 814 MB target dir was deleted.
