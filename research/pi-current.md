# How Pi executes shell commands today (pi-mono survey)

Surveyed: `~/Development/pi-mono` at `36142c1c7` (read-only). Paths below are relative to that repo unless absolute.
Issue numbers refer to `earendil-works/pi` on GitHub.

## TL;DR

- Every built-in shell path ends the same way: **spawn an external shell process per command** (`bash -c <string>`,
  or `-s` with the command on stdin for the legacy WSL launcher). Pi has no persistent shell session, no parser, and does
  no interpretation of its own. "bash" means whatever executable shell resolution finds: `/bin/bash`, `bash` on PATH,
  `sh`, Git Bash/MSYS/Cygwin/WSL on Windows, or **`$SHELL` (often zsh) in p2**.
- Shell resolution is implemented **three times and kept in sync by hand**: in coding-agent
  `src/utils/shell.ts`, in Durable `src/env/node.ts`, and in the pi-env daemon `daemon/src/sys/{unix,windows}.rs`.
  The copies have already drifted (the macOS SIGSTOP-before-SIGKILL fix exists only in coding-agent).
- The tool contract everywhere is: **`{ command: string, timeout?: seconds }` in; combined stdout+stderr (tail, 2000
  lines / 50 KB), exit code, and a spill file out.** Codemode scripts get a structured version, with different field
  names in coding-agent and in Durable/p2.
- Command "understanding" is minimal: renderers show the raw command string (p2 TUI quiet mode shows its first line).
  Permission and plan-mode checks exist only as **example extensions doing regex matching** on the raw string. There is no
  shell parser anywhere in pi-mono: no tree-sitter-bash, shell-quote, shlex or similar.
- pi-env daemon: Rust 2024, toolchain 1.96.0, **no async runtime** (std threads), 4 runtime deps. The stripped LTO release
  build is about **1.9 MB** (darwin-arm64, measured; about 1.0 MB gzipped). It ships for 8 targets (musl Linux,
  Android, macOS and MSVC Windows, each x64 and arm64), is deployed over SSH as
  `~/.pi/mobile/tools/pi-env-<sha256[0:32]>`, and is verified before every start. Its `exec` op is the natural place to
  plug in an in-process shell interpreter.
- Windows is where most complaints come from: Git Bash discovery, the WSL `bash.exe` trap, `taskkill /T` not reaching
  MSYS pipeline children (orphans that run for hours), OEM/ANSI code pages decoded as UTF-8, CRLF, `/dev/null` → `nul`
  files, path dialects, conhost flashes, and NUL stdin busy loops. The community's workaround is "use PowerShell", and Pi
  added an optional `powershell` tool.

---

## 1. Map: every place a shell command is executed or interpreted

### 1.1 coding-agent (`packages/coding-agent`), the classic `pi` CLI

| What | File | How |
|---|---|---|
| Model `bash` tool | `src/core/tools/bash.ts` (`createShellToolDefinition`, `createLocalShellOperations`) | `spawn(shell, [...args, command], { detached: !win32, stdio: [ignore\|pipe, pipe, pipe], windowsHide: true })` |
| Model `powershell` tool (Windows-only, opt-in) | `src/core/tools/powershell.ts` | Same ops with `getPowerShellConfig()`; prepends `[Console]::OutputEncoding=UTF8` |
| Shell resolution, env, kill | `src/utils/shell.ts` (`getShellConfig`, `getPowerShellConfig`, `getShellEnv`, `killProcessTree`, tracked detached PIDs) | see §2 |
| Waiting for exit without hanging on inherited pipes | `src/utils/child-process.ts` (`waitForChildProcess`, 100 ms idle grace after `exit`); `spawnProcess` = cross-spawn on Windows | |
| User `!cmd` / `!!cmd`, RPC `bash` | `src/core/agent-session.ts:3978` `executeBash` → `src/core/bash-executor.ts` `executeBashWithOperations` | Same `BashOperations`; strips ANSI, sanitizes binary, removes `\r` |
| `shellCommandPrefix`, `shellPath` settings | `src/core/settings-schema.ts:260-279`; applied at `agent-session.ts:3742-3753, 3987` | prefix + `"\n"` + command |
| models.json / auth `!command` config values | `src/core/resolve-config-value.ts:153-200` | `spawnSync` via `getShellConfig()` on Windows (10 s timeout); `execSync` (that is, `/bin/sh -c`) elsewhere |
| Extension `pi.exec(cmd, args)` | `src/core/exec.ts` | `spawn(..., { shell: false })`, argv. `.cmd` shims fail on Windows (#10616, open) |
| Package manager / git / self-update | `src/utils/child-process.ts` `spawnProcess` (cross-spawn on win32) | argv |
| Managed `fd`/`rg` download | `src/utils/tools-manager.ts` | bootstraps binaries from GitHub releases (precedent for "ship our own tools") |
| Extension hooks | `src/core/extensions/types.ts`: `tool_call` (can block or modify input), `user_bash` (can supply `operations` or `result`; fails closed), `BashOperations` (pluggable exec, for example SSH), `spawnHook` (rewrite command/cwd/env), `commandPrefix` | |
| Example extensions | `examples/extensions/ssh.ts` (`ssh host "cd <json-quoted> && cmd"`), `gondolin/` (micro-VM: `vm.exec([shellPath,"-lc",cmd])`), `bash-spawn-hook.ts`, `inline-bash.ts`, `permission-gate.ts`, `confirm-destructive.ts`, `plan-mode/utils.ts` | |
| Rendering | `src/core/tools/renderers/bash.ts`: `formatShellCall` prints `$ <command>` (or `PS> `) verbatim; streaming throttled | no parsing |

### 1.2 Durable (`packages/durable`), the harness used by p2

| What | File |
|---|---|
| `ExecutionEnv` / `Shell` interface | `src/env/index.ts:291-377`: `exec(command: string \| readonly string[], { cwd, env, inheritEnv, timeout, onOutput, spill, window }, context) → Result<{ exitCode, spillPath? }, ExecutionError("timeout"\|"shell_unavailable"\|"spawn_error"\|"aborted"\|...)>`, plus `cleanup()` |
| `NodeExecutionEnv` (local, Node `child_process`) | `src/env/node.ts:612+`; shell resolution at `226-268` ("Keep in sync with coding-agent"); `killProcessTree` at `283` (no macOS SIGSTOP); output via per-stream `StreamDecoder` (UTF-8, leading BOM dropped, `src/env/decode.ts`) |
| `bash` tool | `src/tools/bash.ts`: schema `{command, timeout?}`; `prepare` hook and `commandPrefix`; `env.exec(string)`; structured output `{ output, truncated, fullOutputPath?, exitCode }`; tool is `replay: "unsafe"` by default (`harness/tool.ts:196`) |
| `powershell` tool | `src/tools/bash.ts:170+`: argv `[pwsh\|powershell, -NoProfile, -NonInteractive, -ExecutionPolicy, Bypass, -Command, UTF8_PREFIX+script]`; tries programs in order on `spawn_error` |
| Codemode | `src/codemode/*`: the QuickJS sandbox; scripts call `tools.bash({command, timeout})` and get the structured output |

### 1.3 pi-env (`packages/env`): remote and local daemon

| What | File |
|---|---|
| `RemoteExecutionEnv` (client of the daemon) | `src/remote-env.ts:354+`; `exec` sends `{ command \| argv, cwd, env, inheritEnv, shellPath?, timeoutMs, spill, window }` (`:853-896`) |
| `LocalExecutionEnv` | `src/list-files.ts:84`: **is `NodeExecutionEnv`**. Only `listFiles` goes through a local daemon; exec is still Node spawn |
| SSH bootstrap / deploy | `src/ssh.ts`: `detectPlatform` (POSIX `uname` probe, or PowerShell `-EncodedCommand` probe on Windows; detects whether the remote default shell is cmd or PowerShell via `echo %OS%`), `deployDaemon` (SHA-256 named, verified with `sha256sum`/`shasum`/`openssl`, or `Get-FileHash` on Windows), `packagedDaemon` → `bin/pi-env-<platform>-<arch>/pi-env[.exe]`, `localDaemon()` |
| Daemon `exec` | `daemon/src/exec.rs` (`run`, `spawn`, readers, waiter, spill, window) |
| Daemon shell resolution | `daemon/src/sys/unix.rs:266` (configured → `/bin/bash` → `which bash` → `sh`); `daemon/src/sys/windows.rs:502` (configured → `%ProgramFiles%\Git\bin\bash.exe`/x86 → `where bash.exe`; legacy WSL launcher gets stdin) |
| Daemon `pty` (interactive terminals) | `daemon/src/pty.rs`: runs `$SHELL` as a login shell; **Unix only** (`ENOTSUP` on Windows) |
| Daemon `process` (MCP stdio etc.) | `daemon/src/process.rs`: argv, own process group / own Windows job object, killed when the program exits |
| Semantics contract | `docs/semantics.md`: daemon must match `NodeExecutionEnv` (checked by a differential test suite) |

### 1.4 p2 (`apps/p2`), Pi Mobile (this app)

| What | File |
|---|---|
| Local env construction | `src/runtime/worker.ts:438-452`: `new NodeExecutionEnv/LocalExecutionEnv({ cwd, shellPath: process.env.SHELL })`. **The "bash" tool runs `$SHELL -c`**, so zsh on macOS. This session's own errors (`zsh:1: no matches found`) show it |
| Remote envs | `plugins/envs/remote-env.ts:48`: `RemoteExecutionEnv` with optional per-env `shellPath`/`shellEnv` (`plugins/envs/schema.ts`, editable in `EnvsPanel.tsx`) |
| Coding tools | `plugins/coding-tools/worker.ts`: Durable `CodingTools` (read/write/edit/bash) |
| PowerShell | `plugins/powershell/worker.ts`: offers Durable's `powershell` (deferred) per conversation when its location's machine reports `os == windows` |
| User `!`/`!!` | `plugins/chat/shell.ts` (job per command) + `src/runtime/shell.ts` (`p2.shell` entries, `shellTail`, `shellContextText`) |
| Terminals | `plugins/terminals/*`: pi-env `pty` + ghostty-vt **WASM** renderer (`vendor/ghostty/ghostty-vt.wasm`); the agent `terminal` tool sends tmux-like argv commands (`commands.ts` `parseCommand`) |
| Config `!command` | `plugins/models/config-value.ts:125`: `execSync(cmd, { shell: process.env.SHELL })` |
| Codemode | `plugins/codemode/*`: QuickJS; nested call summaries in `src/web/Chat.tsx:232` `callSummary` (`args.command` raw) |
| UI rendering | TUI `plugins/quiet-tools/tui.ts:100-133` (`bash`/`powershell` → `firstLine(command)` + `…`), `src/tui/tools/shell.ts`, web `src/web/Chat.tsx:454` (`<pre class="tool-command">`), `src/web/Code.tsx` (aliases `zsh`→`bash` for highlighting only) |

### 1.5 Browser

- `packages/web-ui` source was removed (`b141e1fa2 chore: remove web-ui workspace`; only a stale `dist/` remains). It had
  sandboxed iframes for JS artifacts and a REPL, with no shell.
- No WASI shell, virtual FS (memfs/OPFS/ZenFS), WebContainer or just-bash anywhere.
- Existing WASM in production: `quickjs-wasi` (codemode, Node and Cloudflare) and ghostty-vt (terminal renderer).
  Codemode already runs in a Cloudflare Durable Object (`packages/codemode/src/cloudflare.ts`, `portable.ts`), which
  shows that "tools without a host OS" is a supported deployment shape.

### 1.6 Shell parsing / classification: none

- No dependency on tree-sitter-bash, shell-quote, bash-parser or shlex. The only hit is a test fixture
  (`test/clipboard-paste-file-paths.test.ts`, quoting pasted paths).
- Regex "classifiers", all in example extensions:
  - `examples/extensions/plan-mode/utils.ts`: `DESTRUCTIVE_PATTERNS` (`\brm\b`, `>`, `git push`…) and `SAFE_PATTERNS` (`^\s*cat\b`…)
  - `permission-gate.ts`: `rm -rf`, `sudo`, `chmod 777`
  - `confirm-destructive.ts`
- These are trivially bypassed: `$(…)`, `eval`, `bash -c`, aliases, `;` after a safe prefix, and so on.
- Durable codemode exposes an LLM `models.classify()`, but nothing uses it for commands.
- Durable's `replay: "unsafe"` on `bash` exists **because the harness cannot tell read-only commands from mutating
  ones**. #10624 proposes keyed, idempotent exec to work around this.

---

## 2. Shell resolution per platform

| | coding-agent `utils/shell.ts` | Durable `env/node.ts` | daemon `sys/*.rs` | p2 |
|---|---|---|---|---|
| explicit | `shellPath` setting (must exist, `access(F_OK)`) | `shellPath` option | `shellPath` field | `$SHELL` locally; env config remotely |
| Unix | `/bin/bash` → `which bash` → `sh` | same | same | `$SHELL` wins (zsh/fish users get zsh/fish `-c`) |
| Windows | `%ProgramFiles%\Git\bin\bash.exe` → `%ProgramFiles(x86)%\…` → `where bash.exe` → error with install hints | same | same | same (remote Windows env) |
| WSL launcher (`%SystemRoot%\System32\bash.exe`) | `-s` + command on stdin (#5893) | same | same | |
| args | `-c <command>` | same | same | |
| PowerShell | `where pwsh.exe` → `%ProgramFiles%\PowerShell\7\pwsh.exe` → `where powershell.exe` → `%SystemRoot%\…\v1.0\powershell.exe`; `-NoProfile -NonInteractive -ExecutionPolicy Bypass -Command` | `pwsh`, then `powershell` (argv) | n/a (argv) | Durable's |
| cmd.exe | never used for tool commands (only by the SSH bootstrap to start the daemon) | | | |

Notes:
- On Windows, the `where bash.exe` fallback frequently finds `C:\Windows\System32\bash.exe` (WSL) before or instead of
  Git Bash. Commands then run inside a Linux distro with different paths and tools (#9229, #9361 open, #9963, #8763).
- Store-alias executables needed `access()` instead of `existsSync` (#9504).
- `shellCommandPrefix` is the only "rc file" mechanism. Commands are non-interactive, so there are no aliases, and
  `.bashrc` is not loaded (`docs/shell-aliases.md`).
- Env: `getShellEnv()` prepends Pi's bin dir (managed fd/rg) to PATH and handles case-insensitive `Path` on Windows. The
  tool adds or strips `PI_SESSION_ID`, `PI_SESSION_FILE`, `PI_PROVIDER`, `PI_MODEL`, `PI_REASONING_LEVEL`. Durable:
  `process.env` + `shellEnv` + `env` (or only `env` with `inheritEnv: false`). The daemon on Windows mimics libuv's env
  sorting and required-variable copying (`windows.rs` `child_environment`).
- cwd: checked to exist first ("Working directory does not exist… Cannot execute bash commands."). It is passed as the
  spawn `cwd`. The SSH example does `cd "<json>" && cmd`, which is wrong quoting for POSIX shells. A Windows prompt cwd
  with backslashes broke bash and is normalized to forward slashes (#2080).

## 3. Execution lifecycle details

- **Timeout**: optional, in seconds, with no default (#11674 made unbounded commands a known hazard; a
  `BASH_DEFAULT_TIMEOUT_MS = 120000` exists only for a beta tool). Validated as finite > 0 and ≤ 2^31−1 ms. On timeout
  the process tree is killed. Since #11613 the error also reports when the shell itself had exited but background
  children kept the pipes open.
- **Kill**:
  - Unix: the child is spawned `detached`, so it gets its own process group; `kill(-pid, SIGKILL)`. On macOS the group
    is SIGSTOPped first because of #11252 (`a; b` started `b` during cancellation). Durable and the daemon lack this
    fix.
  - Windows: `taskkill /F /T /PID` from System32, fire-and-forget and unverified. The daemon adds one daemon-wide job
    object with `KILL_ON_JOB_CLOSE | SILENT_BREAKAWAY_OK` (`windows.rs:637`), so grandchildren break away. Only the
    `process` op uses a per-program, non-breakaway job.
  - Detached PIDs are tracked and killed on SIGHUP/SIGTERM.
- **Pipes after exit**: `waitForChildProcess` (Node) and `EXIT_STDIO_GRACE = 100ms` (Rust) keep reading while
  descendants write, then give up once the pipes have been idle for 100 ms (#2389, #5303, #5753).
- **stdin**: `ignore`, which is the NUL device on Windows. Some console programs busy-loop on it (#8955).
- **Output**:
  - `OutputAccumulator` decodes UTF-8 incrementally, keeps a rolling tail (2000 lines / 50 KB for the model), and
    spills the full raw output to a temp file once it exceeds the limits.
  - Codemode's structured output is up to 1 MiB (first and last 512 KiB).
  - User `!` strips ANSI (handling escapes split across chunks, #10504), removes control characters and `\r`.
  - Durable streams `onOutput` with an optional `window` so remote daemons can skip output the caller would drop
    anyway.
- **Exit code**: a signal termination is reported as `128 + signo` (#9577). Non-zero is an `isError` result, but
  codemode still resolves with the structured value.

## 4. Known pain points (from CHANGELOG, docs, issues)

**Windows**
1. *Discovery*: Git Bash at non-standard paths, WSL `bash.exe` shadowing (#9229, #9361 open, #9963, #8763), and Store
   aliases (#9504). With no bash installed, the tool is simply missing, and models flail trying cmd or PowerShell syntax
   (#8108).
2. *Process trees*: MSYS pipeline stages have dead Windows parent links, so `taskkill /T` misses them. `find | xargs rg`
   ran for 70 min after a timeout (#9129 open, #11774). The kill is not verified. The daemon's breakaway job does not
   help.
3. *Encoding*: everything is decoded as UTF-8. Native console programs emit the OEM/ANSI code page (CP936/CP437…), which
   becomes U+FFFD garbage (#9264 shows `cmd /c ver`). The PowerShell tool forces `[Console]::OutputEncoding`. A UTF-8
   BOM from PS 5.1 also breaks files Pi reads.
4. *CRLF*: `\r` leaks into model-visible output (#9264).
5. *Paths*: `/c/foo` vs `C:\foo` vs `/mnt/c/foo`; file tools needed MSYS/Cygwin/WSL drive mapping (#7064, #7547). The
   maintainer's position (#7547): central path translation "is a losing game", so tell the model about `cygpath`.
6. *`/dev/null` → `nul` files* created in the cwd that only Git Bash can delete (#7547 thread).
7. *Console windows*: `windowsHide: true` (`CREATE_NO_WINDOW`) leaves bash without a console, so every native console
   grandchild flashes a conhost window (#8846). Without the flag, background SDK processes popped windows (#4699).
8. *Quoting*: the args go through libuv's `quote_cmd_arg` and then MSYS's own command-line re-parsing, and WSL needed
   stdin (#5893). The community PowerShell plugins use `-EncodedCommand` to dodge nested quoting.
9. *`.cmd` shims*: argv exec of `npm`/`npx` fails (#10616 open; the daemon refuses `.bat`/`.cmd` in argv by design).
10. *Policy*: issues #11049 and #11051 asked to prefer or enable PowerShell by default on Windows. The user consensus in
    #7547 is "Git Bash is terrible, WSL mangles paths, PowerShell is decent".
11. *Terminals*: pty is not supported on Windows (no ConPTY in the daemon yet).

**Cross-platform**
- No default timeout (#11674); background processes holding pipes (#11613, #2389); abort races (#11252); no-output
  hangs on TTY-wanting programs.
- The tool says "bash" but runs `sh` (fallback) or `$SHELL` (p2), so bash-isms and `shopt` fail (`docs/shell-aliases.md`
  troubleshooting).
- Three copies of resolution and kill logic drift apart.
- No safe or read-only classification, so: `bash` cannot be replay-safe (#10624); permission gates are regex toys; the
  UI can only show the raw first line (#10259 asks for a tool-presentation API).
- Remote: one daemon per SSH connection. A dropped link kills the commands (#10624 proposes on-disk jobs and a
  `pi-env shim`).

## 5. pi-env daemon: language, toolchain, size, targets, deployment

- **Language/toolchain**: Rust edition 2024, pinned `rust-toolchain.toml` channel `1.96.0`. `Cargo.toml` deps (exact
  pins): `encoding_rs`, `ignore`, `notify`, `serde_json`, plus `libc` on Unix and `windows-sys` on Windows. **No
  tokio/async.** Each `exec`/`pty`/`process`/`watch` gets its own threads, and file ops use a fixed worker pool. About
  4.5 kLoC (`src/*.rs`). Release profile: `lto = true`, `codegen-units = 1`, `strip = true`. Windows is built with
  `+crt-static`.
- **Size**: no release binaries are present locally (`bin/` is populated by CI; `daemon/target/debug/pi-env` is 10.4 MB
  unstripped debug). A local release build (copied to /tmp) gives **1,964,848 bytes for aarch64-apple-darwin (≈1.9 MB),
  992 KB gzip -9**. Expect a similar 1.5–2.5 MB across targets.
- **Targets** (`.github/workflows/env-daemons.yml`): `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`,
  `aarch64-linux-android`, `x86_64-linux-android` (NDK 27, API 24), `aarch64-apple-darwin`, `x86_64-apple-darwin`,
  `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`. All are shipped inside the npm package
  (`bin/pi-env-<name>/pi-env[.exe]`, checked in `build-binaries.yml:345-360`).
- **Deployment**: `ssh` with BatchMode, a pinned known_hosts alias and no forwarding. The platform is probed via `uname`
  or PowerShell, then the hash is checked remotely. If the binary is missing, it is uploaded, the temp file is verified,
  and then renamed to `~/.pi/mobile/tools/pi-env-<sha256[:32]>[.exe]`; stale ones are removed. The daemon is started
  as `pi-env serve --token <hex>`: through the login shell on POSIX, or on Windows through cmd.exe (quoted path) or
  PowerShell (`& 'path'`). The protocol syncs on a `PI-ENV <token>` line, which skips rc-file noise.
- **Protocol** (`docs/protocol.md`, v1): length-prefixed frames (`u32 len, u8 type, u32 id, u32 jsonLen, json,
  payload`; 16 MiB max). Frame types: request, result, error, event, cancel, ping. Liveness pings every 5 s; the daemon
  kills all its groups after 30 s of silence or on stdin EOF. `hello` returns `features` for capability negotiation
  (currently `["listFiles","process"]`). This is the extension point for a new op or flag.

## 6. How a shell engine could plug into pi-env

The daemon already owns everything a shell host needs: per-request cancellation (`Control`), process-group or job
tracking (`Groups`), output decoding, windowing, spilling, timeouts, and env/cwd construction compatible with Node. A
shell crate fits at `exec.rs::run` where the `(Some(command), _)` branch currently calls `sys::shell_config` and
spawns `/bin/bash -c`.

Options, from least to most invasive:

1. **Sidecar binary** (`psh`) deployed next to `pi-env` with the same SHA-256 scheme. `shellPath` points to it (or it
   is preferred when present), so no protocol change is needed. Node-side `NodeExecutionEnv` and coding-agent can use
   it too, simply as the resolved shell. It is also usable without the daemon (CLI, local). The cost: double the upload,
   and kill semantics still depend on the OS process tree.
2. **In-process interpreter in the daemon**: `exec { command, shell: "psh" }`, or a new feature flag `"psh"` in
   `hello.features`. The daemon interprets the script itself: builtins (cd, export, echo, test, printf…) run
   in-process, and external commands are spawned by the interpreter, which owns every child. Benefits:
   - Exact process-tree control. Every spawned child goes into a per-command, non-breakaway Windows job object or Unix
     process group, which fixes #9129 and #11774 by construction.
   - Pipelines without MSYS intermediate subshells.
   - Native Windows path, encoding and `/dev/null` handling. For example, `/dev/null` maps to `NUL`, and output from
     non-UTF-8 console programs can be transcoded via `encoding_rs` (already a dependency) using the child's code page.
   - The parse result can be returned in the response (e.g. `{ ast | summary | effects }`) for UI and permissions.
   - Constraints: the daemon is sync and thread-based, so a shell crate that requires tokio adds a runtime and size. A
     thread-per-pipeline-stage design fits the existing architecture. Keep the dependency set small: current binary
     ~1.9 MB, and an interpreter plus coreutils-lite builtins probably adds 0.5–2 MB.
   - Statelessness: each `exec` is a fresh shell today. A persistent session (cwd/env carried across calls) would be a
     new semantic, and Durable's replay model (`replay: "unsafe"`, crash and restart) assumes stateless commands.
     Session state would have to be journaled or explicitly not kept.
3. **Parse-only op** (`parse { command }` → AST and classification), usable even when execution still uses system bash.
   The quickest win for UI and permissions, and a candidate for marking provably read-only commands replay-safe.
4. **Library crate shared by the daemon, a standalone CLI, and a WASM build** (`wasm32-unknown-unknown`/WASI) for the
   browser or Cloudflare. This needs a virtual-FS/process abstraction trait. The daemon already isolates OS access
   behind `sys::{unix,windows}`, which is a template. In the browser, externals would map to injected tools (like
   codemode) instead of `spawn`.

Local use: today `LocalExecutionEnv` runs exec through Node, not the daemon (#10624 proposes routing local tools through
the daemon). Shipping a shell inside the daemon therefore means p2 local also needs `LocalExecutionEnv.exec` → daemon,
or option 1 (a binary on disk). coding-agent (Bun-compiled single binaries for linux/darwin/android/windows, plus npm)
has no daemon at all and would need either the sidecar binary or a JS/WASM build.

## 7. Constraints for any replacement

- **Interface stays "string in, text out".** Model tool schemas are `{ command: string, timeout?: number }` in
  coding-agent and Durable, with strict constrained sampling. Models are trained on bash, so the input must stay a bash
  string. The output is combined stdout and stderr in arrival order (not separated), tail-truncated to 2000 lines / 50 KB
  with a spill-file path, plus an exit code. Non-zero is an error result, and timeout/abort are thrown errors with
  partial output.
- **Structured outputs must be preserved**, and they differ:
  - coding-agent: `{ output, truncated, full_output_path?, exit_code, wall_time_seconds }`
  - Durable/p2: `{ output, truncated, fullOutputPath?, exitCode }`
- **ExecutionEnv contract**: `exec(string | argv, { cwd, env, inheritEnv, timeout, onOutput(per-stream decoded text),
  spill, window })`, with the Result error codes `timeout | shell_unavailable | spawn_error | aborted | unknown`. Signal
  deaths report `128+n`. `cleanup()` gives exit 137. Output keeps being collected after exit until the pipes go idle for
  100 ms. `docs/semantics.md` and the differential tests require **remote == NodeExecutionEnv on that host**, so a new
  shell needs either a Node-side twin or a new, explicitly different mode.
- **Hooks must keep working**: `tool_call` block/modify, `user_bash`, `spawnHook`, `BashOperations` replacement
  (SSH/Gondolin/containers), `commandPrefix`/`shellCommandPrefix` (users rely on `shopt -s expand_aliases; source
  ~/.bash_aliases`), Durable `prepare`.
- **Escape hatch**: `shellPath` must still be able to select real bash, zsh or Cygwin; some users want their `$SHELL`
  (p2 default). Programs that need a real tty belong to the `pty`/terminal path, not `exec`.
- **Env vars** `PI_*` are injected or stripped per call. The PATH prefix holds managed fd/rg. On Windows the env must
  mimic libuv (case-insensitive names, sorted, required variables).
- **Deployment**: anything shipped remotely must build for all 8 targets on stable Rust 1.96, statically (musl,
  crt-static), and fit the SHA-256 upload scheme. It must run where the remote has only `sh` + `sha256sum` (or only
  PowerShell on Windows).
- **Durable replay**: commands may be re-run after a crash only if replay-safe. A shell that can prove a script
  read-only (parse + effects analysis) could unlock `replay: "safe"` per call, a concrete payoff beyond the UI.
