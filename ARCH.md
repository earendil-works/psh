# psh: Architecture

Status: the design before any code (2026-11-06). It follows the agreed constraints in
`research/README.md` §8 and the execution-core study in `research/execution-core.md`.
Where the implementation goes another way, this file is updated and the decision cited
(`Dnnn` in `docs/decisions.md`). Open questions are marked **Open** and collected at the
end.

psh is a bash-compatible shell for running the commands coding agents write. Its
interface is "command string in; stdout, stderr and exit status out", with the semantics
of GNU bash 5.3 run as `bash -c` (D002, D003). It runs on macOS, Linux and Windows
without Git Bash (D011), embeds into a host process with no async runtime (D005), and
compiles to WASM.

## Goals

The guide's `docs/guide/goals.md` is the human-facing statement of these. In short:

- **Agent-first.** Optimize for the narrow syntax agents use (lists, pipes, `cd X &&`,
  `2>&1`, quoted here-docs, `$(…)`, `for`) and their utility habits (GNU and BSD flags
  mixed), not for interactive use.
- **Portable.** The same binary semantics on macOS, Linux and Windows; Windows is not a
  port done at the end.
- **Consistent.** One shell and one utility dialect everywhere: locally, over pi-env, in
  the browser.
- **Bash's behavior.** What bash 5.3 prints and returns is the target, including error
  messages. Deviations are recorded decisions.
- **Understandable commands.** The parser is a product: hosts classify, summarize and
  approve commands with the exact parser that runs them.
- **Controlled execution.** Cancel, timeouts and process-tree kill with no orphans; the
  embedder owns the thread.
- **Embeddable and small.** No async runtime, near-zero dependencies, a size budget.
- **Sandboxable.** Every side effect goes through a `Host`, so the same engine runs over
  a virtual file system in WASM.

## Crates

```
crates/
  psh-syntax/    lexer, parser, AST with byte spans, printer      zero deps, wasm32
  psh-classify/  AST -> effects and summaries                     psh-syntax only, wasm32
  psh-engine/    the stepped interpreter; Host trait; vprocs      psh-syntax only, wasm32
  psh-os/        OS hosts: unix and windows                       platform bindings (Open)
  psh-utils/     in-process utilities (grep, sed, head, jq, ...)  ripgrep crates for rg/fd
  psh/           the `psh` binary: -c, scripts, --parse, --classify
  psh-spec/      the conformance runner (TESTING.md)
  psh-wasm/      later: the WASM build and its JS host
```

Dependency direction: `psh-syntax` ← `psh-classify`, `psh-engine` ← `psh-os`,
`psh-utils` ← `psh`, `psh-spec`, `psh-wasm`. pi-env depends on `psh-engine`, `psh-os`
and `psh-utils` (and `psh-classify` for a parse op); it lives in pi-mono, not here.

**Open:** whether `psh-syntax` and `psh-engine` are `#![no_std]` + `alloc`. For: it
guarantees no hidden OS access and keeps the wasm build honest. Against: `std::task`
and `std::sync` are convenient, `wasm32-unknown-unknown` has std anyway, and the
`Host` trait already isolates OS access. Current lean: std, with the wasm32 build in the
gate as the guard against OS access.

## psh-syntax: the parser

- Copied from brush-parser (MIT, attributed) as a starting point, with peg and winnow
  replaced by a hand-written lexer and recursive-descent parser (D008). brush's other
  dependencies (bon, cached, miette, thiserror, tracing, utf8-chars) go too.
- **Byte spans on every node** (`u32` start and end into the source). Line and column
  are computed on demand from a line table. The classifier, the UI and error messages
  all point into the original string.
- Parses bash 5.3 syntax only: no zsh, no POSIX-sh mode. Aliases are not expanded at parse
  time (non-interactive bash leaves `expand_aliases` off); if `shopt -s expand_aliases`
  is used, the engine re-parses after alias definitions, as bash does line by line.
- Here-docs: bodies are attached to their redirect node with the span of the body and
  the quoting of the delimiter.
- **A printer** turns an AST back into source. `parse(print(ast)) == ast` is tested on
  every case (TESTING.md L0).
- Errors: `bash -n` agreement on whether a script parses, and bash's message text where
  a golden shows it.
- Parses incrementally, as bash does: the engine parses and runs one complete top-level
  line at a time, so `echo a` on line 1 runs before a syntax error on line 2 is
  reported (status 2). `--parse` and the classifier parse the whole input.

## psh-engine: the interpreter

A synchronous, sans-IO state machine (D005). The shape follows oh-my-pi's engine at the
level of architecture only (clean room, D007); the code is ours or brush's.

- `Shell::step(budget) -> Step` runs ready work and returns one of: `Done(status)`,
  `Blocked` (waiting on the host; it will wake the waker), `Yield` (budget spent), or
  `Idle`. Steps are bounded, so the embedder keeps control of its thread.
- **Virtual processes.** Subshells, pipeline stages (including compound commands and
  functions), `$(…)`, `<(…)` and `&` jobs are vprocs inside the engine, scheduled
  cooperatively. Nothing forks; this is also what makes Windows and WASM possible.
  Pipeline stages run concurrently, never one after another (pipe buffers would
  deadlock).
- **Control.** A `ControlHandle` (an atomic control word: cancel, pause, signal) is
  checked at every safepoint (each command and loop iteration at least). Cancel delivers
  a virtual TERM to the vprocs (traps run), then KILL after a grace period. A late
  cancel never leaks into the next run.
- **Wakeups.** `std::task::Waker` only. Every pending thing the engine polls (an
  external process, a host fs op, a pipe, a timer) is polled with a `Context` built from
  the engine's waker. No runtime, no threads inside the engine.
- **Pipes between vprocs** are in-memory engine objects. Pipes between two externals are
  OS pipes handed to both, with no pumping. Only engine↔external pipes need pumps.
- **Here-docs and here-strings** are engine fd objects fed through the same pollable
  pipe machinery; large or seekable bodies go to a host temp file. A body is never
  written synchronously before its consumer starts.
- **Shell builtins** (`cd`, `export`, `set`, `trap`, `read`, `printf`, `test`, `kill`,
  `wait`, ...) are part of the engine. Job control is limited to `&`, `wait`, `kill`,
  `$!`, `jobs` (D002).
- **Expansion** (parameters, arithmetic as 64-bit, globbing, brace expansion, quote
  removal, word splitting) is engine code over the AST; globbing reads directories
  through the host.

### The Host trait

Every side effect goes through one trait, with pollable operations so that any of them
may be `Pending`:

- `spawn(spec) -> ExtProc` where spec carries argv, cwd, env, the fd table (engine pipe
  ends, OS fds, files, null) and placement (own process group / job). `ExtProc` has
  `poll_wait(cx) -> Poll<ExitStatus>`, `signal(sig)`, and drop = abandon (TERM, grace,
  KILL, background reap).
- File system: open, stat, read_dir, mkdir, remove, rename, symlink, ... each may return
  `Pending`. Open files expose `poll_read` / `poll_write`.
- Timers (`sleep`, `timeout`, `read -t`), clock, temp files, environment of the host
  process, user and host names.
- `resolve(name)` for command lookup (PATH, PATHEXT on Windows) and a hook where an
  embedder can veto or rewrite a command at execution time (approval at exec time).

Hosts:

| Host | Where | Notes |
| --- | --- | --- |
| `psh-os::unix` | macOS, Linux, Android | Below |
| `psh-os::windows` | Windows | Below |
| test host | `psh-engine` tests | In-memory fs and fake processes; a latency mode makes every op `Pending` to prove no engine path expects synchronous answers |
| JS host | `psh-wasm` | Virtual fs, host-provided commands, driven from the JS event loop |

## psh-os: the OS hosts

From `research/execution-core.md` §5; the pitfalls listed there are requirements.

**Unix.**
- Spawn: `posix_spawn` with its own session or process group per external, `SETSIGDEF`
  for all non-ignored signals (SIGPIPE back to default), an empty signal mask,
  `CLOEXEC_DEFAULT` on macOS, `addchdir_np` where available; `fork`+`exec` fallback for
  umask, ignored signals and child-side FIFO opens.
- Wait: a pooled waiter thread per running external in `waitid(..., WNOWAIT)`, which
  wakes the engine. No SIGCHLD handler. Exited group leaders stay unreaped while their
  group may still be signalled (no pid reuse). pidfd parking on Linux is a later
  optimization.
- Pumps (engine ↔ external): v1 is a bounded helper thread per pump doing blocking
  `read`/`write` into an engine pipe object. Later a single readiness thread (`poll`;
  `select` on macOS, where `poll` misses FIFO writer close). Never set `O_NONBLOCK` on
  inherited or shared fds.
- Kill: TERM to the group, KILL after a grace period, background reap.
- Signals: embedded, install no handlers at all. Standalone `psh`, handlers only set
  atomic bits and wake the driver; SIGPIPE is reset to default for children.

**Windows.**
- Spawn: `CreateProcessW` suspended, assigned to a Job Object per job (no breakaway),
  then resumed; only the three stdio handles inherited (`PROC_THREAD_ATTRIBUTE_HANDLE_LIST`);
  `CREATE_NEW_PROCESS_GROUP`; no console window flashes.
- argv: MSVCRT quoting rules; `.bat`/`.cmd` through `cmd.exe` with the hardened quoting
  Rust std uses (BatBadBut), refusing what cannot be quoted safely.
- Wait: a waiter thread per external on `WaitForMultipleObjects(process, wake)`.
- Kill: TERM/INT = `CTRL_BREAK_EVENT` to the group, then `TerminateJobObject`; KILL =
  `TerminateJobObject`. The whole tree dies, including grandchildren.
- Pumps: v1 helper threads; later overlapped named pipes (anonymous pipes cannot do
  overlapped I/O).

**Windows rules** (from `research/landscape.md` §3):
- Paths: forward slashes internally; accept `C:\x`, `C:/x` and `/c/x` in builtins and
  `cd`; convert to native form only at the OS boundary. No MSYS-style rewriting of argv
  to external programs. `\\?\` for long paths.
- `/dev/null` maps to `NUL`; `/dev/stdin|stdout|stderr` are emulated in redirections.
- Command lookup: PATH (`Path`, `;`-separated) with PATHEXT, case-insensitive; cwd is
  not searched for bare names. `test -x` and `command -v` decide by extension.
- Shebangs: `./script` with `#!` is run by the named interpreter (`/usr/bin/env X`
  resolves X on PATH); `#!/bin/sh` and `#!/bin/bash` run in psh itself.
- Environment names are case-insensitive for the OS environment, case-sensitive for
  shell variables.
- Output of native programs in the ANSI/OEM code page is decoded per program. **Open:**
  where decoding lives (engine output layer or host), and how much `encoding_rs` costs us
  in a near-zero-dependency tree.
- CRLF: scripts with `\r\n` parse; `$(…)` and `read` strip a trailing `\r` from native
  output.
- Utilities whose names collide with System32 programs (`find`, `sort`, `timeout`,
  `tree`, `more`, `hostname`, `whoami`) are always the builtin on Windows.
- `ln -s`: symlink, else a junction for directories, else a clear error; never a silent
  copy.

**Open:** which platform binding crates `psh-os` may use (`libc`, `windows-sys`, as
pi-env does), or whether it declares its own FFI. Needs a decision before 0006/0007.

## psh-utils: in-process utilities

"Builtin utilities" are in-process implementations of external programs (`grep`, `sed`,
`head`, `jq`, `timeout`, ...), distinct from the shell builtins in the engine. The list
and its tiers come from `research/builtins.md` (D010).

- **Interface.** A utility is a synchronous `run(&mut UtilCtx) -> i32` over a small
  context: stdin/stdout/stderr streams, cwd, env, a cancel flag (which also makes a
  blocked stdin read return EOF), the host fs, and a command runner for `xargs`,
  `find -exec`, `env`, `timeout`, `nice`, `nohup`. The engine runs utilities on worker
  threads (or inline in WASM) and polls them like externals; a utility never detaches a
  thread that still writes to its fds.
- **Fall-through policy** (D010). A utility that meets a flag or form it does not
  implement does not guess: it runs the real program of that name from PATH if there is
  one, or fails with a clear message naming the unsupported flag. It never silently
  produces different output.
- **Shadowing modes**, per utility: *shadow* (always the builtin), *fallback* (the
  builtin only when no program of that name is on PATH, or in WASM), *Windows-shadow*
  (shadow on Windows, fallback elsewhere). An explicit path (`/usr/bin/grep`) always
  means the real program.
- **Off switches.** All utilities, the destructive group (`rm mv cp ln dd`) or single
  names can be turned off per session. `command -v`, `type` and `which` report
  utilities truthfully.
- **Dialect.** The union of GNU and BSD flags; GNU semantics where they conflict, except
  where agents overwhelmingly write the BSD form (`sed -i ''`, `stat -f FMT`), with the
  deterministic tie-break rules of `research/builtins.md` §4.6. `uname` and friends
  report the real host.
- **Never builtins:** interpreters, toolchains, package managers, VCS (`git`), network
  and database clients (`curl`, `ssh`, `sqlite3`), interactive programs. An embedder may
  supply such commands through the host (WASM), but they are not psh utilities.
- Sources: ported from uutils and others (MIT/Apache, copied with attribution, D006);
  `rg` and `fd` on the ripgrep/fd crates. **Open:** whether `grep` and `find` may reuse
  those crates once they are in the tree, and how `jq` gets jaq (dependency or copy).

## The `psh` binary

`psh -c CMD [ARGS]`, `psh SCRIPT [ARGS]`, script on stdin. Diagnostics:
`psh --parse` (AST as JSON with spans), `psh --classify`, `psh -n`. Standalone, it drives
the engine with a thread parker: step, and on `Blocked` park until the waker fires. It
installs minimal signal handlers (atomic bits) and resets SIGPIPE for children.

## Embedding into pi-env

pi-env (pi-mono, `packages/env/daemon`) is a synchronous, threaded Rust daemon that
runs every `exec` as `bash -c` today (`research/pi-current.md`). psh plugs in at its
`exec` op:

- Advertised as a feature in `hello.features`; `exec { command, shell: "psh" }`
  selects it. `shellPath` still selects real bash.
- One thread per exec runs the driver loop: `step`; on `Blocked`, park on a
  `Mutex`+`Condvar` woken by the engine's waker or by the request's cancel. Cancel and
  timeout go through the `ControlHandle`; TERM, grace, KILL.
- Embedded, psh installs no signal handlers; every external leads its own process
  group or job, so a cancel reaches the whole tree.
- Output stays combined stdout+stderr in arrival order, with exit status, and goes
  through the daemon's existing decoding, windowing and spill. Exit codes follow
  pi-env's contract (`128+n` for signals).
- A `parse` op returns the AST summary and classification for UI and permissions, even
  where execution still uses real bash.

**Open:** how pi-env drives sessions. Each `exec` is a fresh shell today, and Durable's
replay assumes stateless commands. Options: keep it stateless; or a persistent session
(cwd, env, functions carried across calls) that would have to be journaled. Also open:
whether the Node-side `NodeExecutionEnv` gets psh as a sidecar binary or always goes
through the daemon (`research/pi-current.md` §6).

## WASM and the browser

- `psh-syntax` and `psh-classify` build for `wasm32-unknown-unknown` with no imports; a
  host can ship the parser alone (brush's parser was about 146 KB gzipped; ours should be
  smaller without peg).
- The whole shell builds for `wasm32-unknown-unknown` with a JS `Host`: a virtual fs
  (memory, OPFS, or a remote pi-env), commands supplied by the host (tools) instead of
  processes, utilities in-process. The JS side drives `step` from its event loop; a
  pending fs op or command resolves through the waker. Because the engine is sans-IO,
  no Asyncify or JSPI is needed.
- In the browser there is no git, python or node, so this mode is for exploration and
  editing unless the host supplies such commands.

**Open:** whether the browser mode matters before pi-env integration, and whether a WASI
build is wanted at all.

## psh-classify: understanding commands

The classifier runs on the same AST the engine executes, so a command is never parsed one
way for approval and another way for execution.

- For each simple command: the command name (literal or dynamic), argv words with spans
  (literal or which expansions they contain), redirections with targets, and nested
  commands (`$(…)`, `<(…)`, `eval`/`bash -c` strings where literal, `xargs`/`find -exec`
  /`env`/`timeout` wrapped commands).
- Categories from the corpus: read, mutate, VCS, build, network, install, long-running,
  unknown; plus effects: paths read and written, whether output is truncated (`| head`).
- **Conservative.** Anything dynamic (command name from a variable, `eval` of a
  non-literal, `source`, functions or aliases defined earlier, unknown programs, inline
  interpreters) is `unknown`, never "read-only". Redirections to files, `sed -i`, `tee`
  are writes.
- Per-utility knowledge (which flags write) lives in tables next to the utilities, and
  `psh-classify` reads them without depending on `psh-utils`. **Open:** where these
  tables live so both crates share them.

## Open questions

1. `no_std` or std for `psh-syntax` and `psh-engine`.
2. Platform bindings for `psh-os` (`libc`, `windows-sys`, or our own FFI).
3. How pi-env drives sessions (stateless vs persistent), and how Node-side execution gets
   psh.
4. Minimum supported Rust version: pi-env pins 1.96.0, so psh must build with it if it
   is linked in.
5. Windows output decoding: where it lives and whether `encoding_rs` is allowed.
6. Dependencies beyond rg/fd: jaq for `jq`, regex/globset reuse for `grep`/`find`.
7. Where the per-utility effect tables live.
8. Whether the WASM host is needed before pi-env integration.
9. License of psh itself (MIT, or MIT/Apache-2.0).
