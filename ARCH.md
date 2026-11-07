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
- **Bash's behavior.** What bash 5.3 prints and returns is the target, including error
  messages. Deviations are recorded decisions.
- **Consistent.** One shell and one utility dialect everywhere: locally, over pi-env, in
  the browser.
- **Understandable commands.** The parser is a product: hosts classify, summarize and
  approve commands with the exact parser that runs them.
- **Control of execution.** Cancel, timeouts and process-tree kill with no orphans; the
  embedder owns the thread.
- **Embeddable.** No async runtime, near-zero dependencies, a size budget.
- **Sandboxable.** Every side effect goes through a `Host`, so the same engine runs over
  a virtual file system in WASM.

Ranking when goals conflict (D033): agent-first, then portable, then bash's behavior,
then the rest, unranked among themselves. Under all of them: permissive licensing
(D007), MIT OR Apache-2.0 (D032).

## Crates

```
crates/
  psh-syntax/    lexer, parser, AST with byte spans, printer      no_std + alloc, zero deps
  psh-classify/  AST -> effects and summaries                     no_std + alloc, psh-syntax only
  psh-regex/     POSIX ERE/BRE with GNU semantics (D1270)         no_std + alloc, zero deps
  psh-engine/    the stepped interpreter; Host trait; vprocs      std, psh-syntax + psh-regex, wasm32
  psh-os/        OS hosts: unix and windows                       libc, windows-sys
  psh-utils/     in-process utilities (grep, sed, head, jq, ...)  ripgrep crates (D020)
  psh/           the `psh` binary: -c, scripts, --parse, --classify
  psh-spec/      the conformance runner (TESTING.md)
  psh-wasm/      later: the WASM build and its JS host
```

Dependency direction: `psh-syntax` ← `psh-classify`, `psh-syntax` + `psh-regex` ←
`psh-engine` ← `psh-os`, `psh-utils` ← `psh`, `psh-spec`, `psh-wasm`. pi-env depends on `psh-engine`, `psh-os`
and `psh-utils` (and `psh-classify` for a parse op); it lives in pi-mono, not here.

**std or not (D023).** `psh-syntax` (and `psh-classify`) are `#![no_std]` + `alloc`, so
the parser builds small and clean for wasm. `psh-engine` uses std; its
`wasm32-unknown-unknown` build in the gate guards against OS access. The engine must
never force std into the parser: no std-only types in the parser's API, no parser
feature that turns std on.

**Toolchain (D025).** MSRV is 1.96, pi-env's pinned toolchain; development uses current
stable. A gate leg checking 1.96 is a follow-up; until then, no features newer than 1.96.

**JSON (D029).** All JSON in psh (`psh --parse`, `psh --classify`, `rg --json`, any
later JSON input or output) goes through the `deser-json` crate (deser 0.10.x). The
parser crate does not depend on it; the AST's JSON form is produced outside
`psh-syntax` unless deser-json works under no_std.

## psh-syntax: the parser

- Copied from brush-parser (MIT, attributed) as a starting point, with peg and winnow
  replaced by a hand-written lexer and recursive-descent parser (D008). brush's other
  dependencies (bon, cached, miette, thiserror, tracing, utf8-chars) go too.
- **Byte spans on every node** (`u32` start and end into the source). Line and column
  are computed on demand from a line table. The classifier, the UI and error messages
  all point into the original string.
- **Arenas** (D1040): an `Ast` owns the source and holds lists, commands and here-docs
  in arenas addressed by ids; the engine keeps `(Arc<Ast>, id)` pairs. Literal word
  parts are spans, so parsing copies no text.
- What `bash -n` does not check is deferred, as in bash (D1041): backquote bodies,
  arithmetic (kept as text with expansions), invalid `${…}` ("bad substitution") and
  errors in unquoted here-doc bodies. `$(…)`, `<(…)` and `${ …; }` are parsed in place.
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
  `Parser::next_chunk` yields those lines; the parse options (`extglob`) may change
  between chunks, as `shopt` does in bash (D1043).

## psh-engine: the interpreter

A synchronous, sans-IO state machine (D005). The shape follows oh-my-pi's engine at the
level of architecture only (clean room, D007); the code is ours or brush's.

**How it runs (D1050).** The interpreter is a tree walker over the arena AST written as
Rust `async fn`s; recursion goes through boxed futures. The engine owns a small
scheduler (`sched.rs`, no crate, no threads): every vproc and helper task is a boxed
future in a slab, each with its own `std::task::Waker`. Waking a task (from the engine
thread, or from a host thread that finished an operation) queues its id and, unless a
step is running, wakes the embedder's waker; only woken tasks are polled. The compiler
generates the state machines, so expansion can wait in the middle of a word
(`"${x:=a}$(cmd)"`) and every compound command can wait anywhere. This is not an async
runtime in D005's sense: no runtime crate, no threads, the embedder steps and parks.

- **API (D1051).** `Shell::new(ShellConfig)` (host, env, cwd, stdio as `HostFile`s,
  nesting limit), `shell.set_waker(waker)`, `shell.start(Script)`, then
  `shell.step(budget) -> Step`: `Done(status)` (once), `Blocked` (everything waits on
  the host or the shell is paused; the waker fires when there is work), `Yield` (budget
  spent), `Idle` (no script). The budget counts safepoints: one before every command.
  Shell state (variables, cwd, fds) persists across runs. `std_host::drive` is the
  park loop the `psh` binary and psh-spec use.
- **`!Send`.** A `Shell` holds `Rc`s (a `PhantomData<*const ()>` makes it explicit):
  create and step it on one thread. That fits pi-env's thread per exec, psh-spec's
  worker threads and the JS event loop. `ControlHandle` is `Send + Sync`.
- **Stack (D1050, D1059).** Nested futures are polled recursively, so nesting within one
  task (compound commands, function calls, later `eval`/`source`) uses native stack:
  measured about 3 KiB per function call level in release builds and 15 KiB in debug
  builds (`examples/stackprobe.rs DEPTH BYTES func`). To keep that low, compound
  commands and simple commands dispatch through plain functions returning boxed
  futures, and a list item that is a single command skips the and-or and pipeline
  frames. `ShellConfig::max_depth` (default 1000) bounds functions plus subshells and
  fails with "maximum function nesting level exceeded" instead of overflowing; embedders
  run the engine on a thread with `std_host::ENGINE_STACK` (about 20 MiB, only touched
  pages cost). On wasm32 (1 MiB stack by default) use a limit around 200 or link a
  larger stack. Subshells, pipeline stages and substitutions are tasks of their own and
  do not deepen the stack.
- **Virtual processes.** Subshells, pipeline stages (including compound commands and
  functions), `$(…)`, `<(…)` and `&` jobs are vprocs: a task owning a copy of the
  parent's state (`Vp`: variables, positional parameters, cwd, fd table, `$?`).
  Nothing forks; this is also what makes Windows and WASM possible. Pipeline stages run
  concurrently, never one after another (pipe buffers would deadlock).
- **Control (D1051).** A `ControlHandle` (an atomic control word: cancel, pause) is
  checked at every safepoint and before every poll. Cancel drops every task: that is
  the virtual TERM's default action (traps come later), the run ends with 143, and
  dropping is what releases everything, since every host resource is released by
  `Drop` (externals abandoned: TERM, grace, KILL, background reap; files and pipes
  closed). `start` clears the control word, so a late cancel never leaks into the next
  run; after a cancel the shell continues from its state before the cancelled run.
- **Wakeups.** `std::task::Waker` only. Every pending thing the engine polls (an
  external process, a host fs op, a pipe, a timer) is polled with the task's waker.
- **Values are bytes (D1053).** Variables, words and paths are `Vec<u8>`; char-wise
  operations (`${#x}`, patterns) decode UTF-8 when `LC_ALL`/`LC_CTYPE`/`LANG` says
  UTF-8, else work on bytes. Messages are `$0: line N: …` as in bash.
- **Pipes between vprocs (D1054)** are in-memory engine objects (64 KiB, reader and
  writer counts; writing without readers ends the vproc with status 141, its SIGPIPE).
  Externals get host pipes, and the engine pumps between those and its objects; the
  pumps are polled inside the wait for the external, so a cancel drops them too. Input
  pumps stop when the external exits (so a vproc writing to it gets EPIPE); the command
  completes when the external has exited *and* its output pumps reached EOF.
  Descriptors that share one engine object (`|&`, `2>&1` into a pipe) share one host
  pipe, so ordering survives. A pipe whose buffer is empty when an external starts on
  one of its ends becomes an OS pipe of the host (`Host::pipe`, D1068): externals get
  the OS end directly (no pump between two externals), vprocs write to it and read
  through the engine buffer, refilled from it. Pipes with buffered bytes (here-docs)
  and hosts without OS pipes keep pumps.
- **Here-docs and here-strings (D1054)** are pipes preloaded with the expanded body and
  no writer: nothing is written into a kernel pipe before its consumer runs. Large
  seekable bodies through a host temp file are a later option.
- **`/dev/stdin`, `/dev/stdout`, `/dev/stderr`, `/dev/fd/N`** in redirections always
  refer to the vproc's descriptors (embedded, the process's are not the shell's; D1055).
- **Shell builtins** (`cd`, `export`, `set`, `trap`, `read`, `printf`, `test`, `kill`,
  `wait`, ...) are part of the engine, async functions on the calling vproc. Job control
  is limited to `&`, `wait`, `kill`, `$!`, `jobs`, `disown` (D002): a job that is one
  simple command is its program (`$!` is its OS pid), any other job a vproc with a pid
  of its own that a signal ends at once (D1900). When a script ends, detached
  vprocs (`cmd &`) get one more round; then, by `ShellConfig::exit_grace`, their
  externals still running are released (`None`, bash parity: background jobs outlive
  the shell) or waited for up to the grace and then abandoned (embedders that must
  leave nothing behind; psh-spec uses 500 ms).
- **Expansion** (brace expansion, tilde, parameters, command substitution, arithmetic
  as 64-bit, word splitting, globbing, quote removal) is engine code over the AST;
  globbing reads directories through the host. Arithmetic is psh-syntax's
  `eval_arith` with the vproc as its `ArithEnv`. Features not implemented yet fail
  loudly (`X: not supported yet`, the script ends with 2) rather than doing something
  else (D1057).
- **Errors (D1058).** How control leaves a command is `Unwind`: `Exit` ends the
  (sub)shell (`exit`, `${x?}`, SIGPIPE, unsupported features), `Abort` abandons the rest
  of the top-level line and continues with the next one (bad substitution, arithmetic
  errors, readonly assignments: bash's behavior), `Return`, `Break(n)`, `Continue(n)`.
- **Options and errexit (D1280, D1281).** `Vp::opts` holds `set` and `shopt` options
  per vproc. errexit is checked in `run_command_with` after simple commands, subshells,
  `((…))` and `[[ ]]`, and in `run_pipeline_stages` after multi-command pipelines;
  `Vp::no_errexit` suspends it in conditions, `&&`/`||` operands but the last and `!`
  pipelines. Pipeline stages run unchecked (`Body::Stage`).
- **Traps and signals (D1283, D1284).** `Vp::traps`; EXIT runs when a (sub)shell ends,
  ERR beside errexit, DEBUG/RETURN around commands and function calls. Signals to the
  shell come in through `ControlHandle::signal(sig)` (Linux numbering): a pending mask
  in `Shared`, taken by the shell's own vproc at its next safepoint (trap, ignore, or
  the EXIT trap and `128+sig`). `kill` on the shell and the standalone driver's OS
  handlers feed it (0006); `cancel` stays a forced TERM.
- **Functions and scope (D1059).** Functions live in `Vp::funcs` (an `Rc` map shared
  with subshells until one changes it) as `(Rc<Code>, CmdId)`; lookup order is
  function, builtin, external. A call pushes a `Frame` that records the old value of
  every variable its `local`s (and prefix assignments) shadow and restores them on
  return: dynamic scope with one flat variable table. `Vp::loops` counts the loops of
  the current function body (reset per call and per subshell) for `break`/`continue`.

### Engine internals other work builds on

Language work in parallel issues (arrays and expansion, `[[ ]]`/`test`, shell options
and errexit, builtins) extends these; restructuring them needs a decision first:
- `sched.rs` (tasks, wakers, budget, safepoints) and the `Shell`/`Step`/`ControlHandle`
  API; one safepoint per command and per loop iteration.
- `Vp` as the whole per-vproc state, cloned for subshells; `Unwind` as the only
  non-local control flow; `R = Result<i32, Unwind>`.
- The dispatch shape in `exec.rs`: `run_list` → `run_command` (safepoint) →
  `run_simple` / `run_compound` returning boxed futures; `prepare_simple` (expansion,
  assignments) then `run_argv` (function, builtin, external). Keep new compound
  commands as their own boxed async fns (stack per level, D1059).
- `expand.rs`'s `Fields` model (quoted, literal and unquoted text; `pat` for globbing)
  and its entry points (`expand_words`, `expand_single`, `expand_pattern`,
  `expand_target`, `expand_heredoc`, `expand_decl_words`).
- `FdObj`/`FdTable` and the redirection save/restore in `with_redirects`/`run_simple`;
  pipes and pumps (`pipe.rs`, `fd::Pump`).
- Builtins are `fn(&mut Vp, &[Vec<u8>]) -> LocalBoxFuture<R>` in `builtins::find`;
  output through `builtins::write_fd`/`out` (SIGPIPE as status 141).
- Variables are `Vars` (name → `Var { value: Value, exported, readonly, integer, case,
  nameref }`, `Value::{Unset, Str, Indexed, Assoc}`, D1260), used through the `Vp`
  accessors (`var`, `set_var`, `assign`, `assign_elem`, `assign_array`, `get_elem`,
  `unset_elem`, `is_set`, `resolve`), never by poking `Var` fields for assignments.
  Associative arrays keep bash's iteration order (D1261). Declaration builtins take
  `DeclArg`s (D1262). Dynamic variables (`$RANDOM`, `$SECONDS`, …) are `Var`s marked
  `dynamic`, refreshed by `Vp::refresh_dynamic` when expansion or arithmetic reads them
  (D1263); `$_` is set in `run_command_with` from `Vp::last_args`.

### The Host trait

Every side effect goes through one trait (`host.rs`, D1052), with pollable operations
so that any of them may be `Pending`. File system operations return
`HostFuture<T> = Pin<Box<dyn Future<Output = T>>>`; paths are bytes, absolute, `/`
separated, converted at the host's boundary.

- `spawn(SpawnSpec) -> Spawned`: argv (with `argv[0]` as written), the resolved
  program, cwd, the complete env, and the child's descriptors as `ChildFd` (`File`: a
  file of this host, passed as is; `Pipe`: a new pipe whose other end the engine pumps;
  `Dup(n)`: the same as descriptor n; `Null`). `ExtProc` has `poll_wait(cx)`,
  `signal(sig)`, `release()` (keep running, reap in the background), and drop =
  abandon (TERM, grace, KILL, background reap).
- File system: `open` (`OpenOptions`), `stat`/`lstat`, `access`, `read_dir`,
  `canonicalize`; open files are `HostFile`s with `poll_read`/`poll_write` and
  `is_terminal`. To come with their features: mkdir, remove, rename, symlink, temp
  files, umask, FIFOs.
- `path_entries(PATH)` splits a `PATH`-like value as the platform does (`:`; Windows `;`,
  D1910), and `resolve(name, PATH, cwd)` does command lookup (PATH, PATHEXT on Windows); a hook
  where an embedder can veto or rewrite a command at execution time is planned
  (approval at exec time).
- `sleep` (timers), `pid` (`$$`), `now` (clock), `home_dir(user)` (`~user`, D1322).
  To come: user and host names, signals to process groups (`kill`).

Hosts:

| Host | Where | Notes |
| --- | --- | --- |
| `psh-os::unix` | macOS, Linux, Android; `psh` and psh-spec there | Below (D1060–D1062) |
| `psh-os::windows` | Windows (x64, arm64); `psh` and psh-spec there | Below (D1074–D1079) |
| `psh_engine::std_host` | simple embedders and tests | std `Command`, a waiter thread per child polling `try_wait`, helper threads per pump, KILL only, no process groups; regular files and the shell's own stdio are read and written synchronously |
| test host | `psh_engine::testhost` | In-memory fs, fake commands written as futures, captured stdio; a latency mode makes every op `Pending` once to prove no engine path expects synchronous answers; counts live processes, pipes and files for leak checks |
| JS host | `psh-wasm` | Virtual fs, host-provided commands, driven from the JS event loop |

## psh-os: the OS hosts

From `research/execution-core.md` §5; the pitfalls listed there are requirements.

**Unix.**
- Spawn (D1060): `posix_spawn` with a session of its own per external (`SETSID`),
  `SETSIGDEF` for every catchable signal (SIGPIPE back to default), an empty signal
  mask, `CLOEXEC_DEFAULT` on macOS, `addchdir_np` where available (glibc ≥ 2.29, found
  with `dlsym`); a bare `fork`+`execve` elsewhere and for a umask or ignored signals
  `posix_spawn` cannot express. The standalone `psh` keeps externals in its own
  process group, as bash does (D1067). A file without `#!` runs in `/bin/sh` until the engine runs it itself
  (D1063).
- Wait: a pooled waiter thread per running external in `waitid(..., WNOWAIT)`, which
  wakes the engine. No SIGCHLD handler. Exited group leaders stay unreaped while their
  group may still be signalled (no pid reuse). One party reaps, under the lifecycle
  lock (`psh_os::lifecycle`, D1062). pidfd parking on Linux is a later optimization.
- Files and pumps (D1061): no thread per pump. The engine's pumps poll host files; our
  private pipe ends to externals are `O_NONBLOCK`, and I/O that would wait registers the
  waker with one process-wide readiness thread (`poll`; `select` on macOS, where `poll`
  misses FIFO writer close). Shared and inherited descriptors (the shell's stdio,
  redirections) keep their flags: a zero-timeout readiness check before each read or
  write, writes to streams capped at `PIPE_BUF`. FIFO opens run on a helper thread.
  Timers run on one process-wide thread.
- Kill: TERM (and CONT) to the group, KILL to the group when the leader exits or after
  a grace period (default 2 s), then a reap (D1065).
- Runs (D1067): embedded, a host is one run; dropping it kills what the run left
  (released background jobs, what finished externals left in their sessions) unless it
  was detached (`nohup`, `setsid`) or disowned; `Leftovers::Keep` leaves it, as bash.
  Standalone, everything stays in psh's group, which a harness's group kill reaches. `UnixHost::leftovers()` lists
  groups with live members (leak checks).
- Signals: embedded, install no handlers at all. Standalone `psh` (D1903): the
  trappable signals are caught on the engine thread only; the handler sets the signal's
  bit in the shell's pending mask (traps run, D1284) and a watcher thread wakes the
  shell; ended by a default action, psh dies by the signal after its abandoned
  externals are killed. `kill $$` sets the mask directly. Children start with every signal at its default (SIGPIPE
  included) except those ignored when psh started and those the engine asks to ignore.

**Windows.**
- Spawn (D1074): `CreateProcessW` suspended, assigned to a Job Object of its own (no
  breakaway), then resumed; only the three stdio handles inherited
  (`PROC_THREAD_ATTRIBUTE_HANDLE_LIST`). `Placement::Group` (default):
  `CREATE_NEW_PROCESS_GROUP`, so TERM can be a ctrl-break; `Inherit` (standalone psh)
  keeps psh's console group. Without a console, `CREATE_NO_WINDOW`: nothing flashes.
  The run's jobs kill their processes when their last handle closes, so a dying psh
  takes them along; detached and disowned externals are exempt, and `Leftovers::Keep`
  clears the flag when the host is dropped. Descriptors above 2 are not passed.
- Programs: `.exe`/`.com` directly; `.bat`/`.cmd` through `cmd.exe` with the hardened
  quoting Rust std uses (BatBadBut), refusing line breaks; MSVCRT argv quoting
  otherwise. A file starting with `#!` runs under its interpreter, looked up on the
  child's `PATH` (`/usr/bin/env X`, `/bin/X`, `/usr/bin/X` all mean X); `sh`/`bash`
  scripts and files without an interpreter need psh itself (an engine hook, D1079).
- Wait: a waiter thread per external on its process handle; holding the handle keeps
  the pid from being reused.
- Kill: TERM/INT/HUP/QUIT = `CTRL_BREAK_EVENT` to the external's group, or
  `TerminateJobObject` when no break can be delivered; KILL = `TerminateJobObject`.
  Abandon = TERM, then `TerminateJobObject` when the external exits or the grace ends:
  the whole tree dies. Crash exit codes are reported as signals (access violation as
  SEGV, fail-fast as ABRT).
- Files (D1077): pump pipes are private named pipes whose overlapped end a helper
  thread owns (cancelled at once on drop). Shared pipes (the shell's stdio,
  `Host::pipe`) are read only as far as `PeekNamedPipe` shows data, with one poller
  thread for readiness; writes to them are synchronous. Regular files and consoles are
  synchronous.
- `umask()` is `0o022`; a creation `mode` without write bits sets the read-only
  attribute; other mode bits have no Windows meaning (D1911).
- Decoding (D1078): optional (`Decode::Ansi`), in the host's pump pipes: child output
  read by the engine that is not UTF-8 is converted from the ANSI code page.

**Windows rules** (from `research/landscape.md` §3):
- Paths: forward slashes internally; accept `C:\x`, `C:/x` and `/c/x` in builtins and
  `cd`; convert to native form only at the OS boundary. No MSYS-style rewriting of argv
  to external programs. `\\?\` for long paths.
- `/dev/null` maps to `NUL` and `/tmp` to the user's temp directory;
  `/dev/stdin|stdout|stderr` are emulated in redirections. `HOME` defaults to
  `USERPROFILE`, `TMPDIR` to the temp directory, both `/`-separated (D1075).
- Command lookup (D1076): PATH entries separated by `;`, and by `:` except after a
  drive letter; PATHEXT, case-insensitive; a bare name only when it is a `#!` script;
  cwd is not searched for bare names. `test -x` decides by extension or `#!`.
- Shebangs: `./script` with `#!` is run by the named interpreter (`/usr/bin/env X`
  resolves X on PATH); `#!/bin/sh` and `#!/bin/bash` run in psh itself.
- Environment names are case-insensitive for the OS environment, case-sensitive for
  shell variables.
- Output of native programs in the ANSI code page can be decoded where the engine reads
  it, in the host (D1078, an option, off by default).
- CRLF: scripts with `\r\n` parse; `$(…)` and `read` strip a trailing `\r` from native
  output.
- Utilities whose names collide with System32 programs (`find`, `sort`, `timeout`,
  `tree`, `more`, `hostname`, `whoami`) are always the builtin on Windows.
- `ln -s`: symlink, else a junction for directories, else a clear error; never a silent
  copy.

Platform bindings: `psh-os` uses `libc` and `windows-sys`, as pi-env does (D024).

**Windows CI (D027).** The gate's Windows leg runs on the owner's Parallels VM via
`prlctl exec` until the repository has a remote, then on GitHub Actions.

## psh-utils: in-process utilities

"Builtin utilities" (or just utilities) are in-process implementations of external
programs (`grep`, `sed`, `head`, `jq`, `timeout`, ...), distinct from the shell builtins
in the engine. The list, its tiers and the policies below come from
`research/builtins.md` §4 (D010, D014–D022, D028, D034).

Why have them at all, in order of weight: **portability** (Windows has none of these
tools and WASM has no processes, so the utility set *is* the toolset there),
**consistency** (one flag dialect everywhere), and **no fork** (faster, works over a
virtual fs, cancels cooperatively, and the classifier knows their effects). Their cost
is size, maintenance, and above all divergence where a builtin shadows a better real
program, which the policies below exist to contain.

### Interface

A utility is an async function `run(&mut UtilCtx, argv) -> i32` (D1080, D1081), run on
the vproc that runs the command, where shell builtins run, natively and in WASM alike.
`UtilCtx` (`psh_engine::util`, versioned) gives it the command's descriptors (reads
awaited, stdout buffered and flushed before reads, before stderr writes and when full),
cwd, environment, the Host's file operations, `sleep`, a safepoint (`check`) for long
loops, `spawn` for the wrappers (`xargs`, `find -exec`, `env`, `timeout`, `nice`,
`nohup`) and `run_program` for falling through to the real program. Cancel is dropping
the future. Code built on synchronous crates (ripgrep's, jaq) is the exception: an
async front end parses its arguments (and falls through if it must), then
`UtilCtx::run_blocking` runs a `fn(&mut SyncCtx, T) -> i32` body with std `Read`/`Write`
stdio and a synchronous file system (D1130). When the Host names the process's own file
system (`Host::native_paths`) the body runs on a worker thread whose stdio are bounded
queues the engine pumps, and it may use `std::fs` (the `ignore` walker) on the OS paths;
otherwise (WASM, sandbox and test hosts) it runs inline, with stdin read first when it
asks for it (within a bound), output collected and written after, and Host calls that
must answer at once. The registry (psh-utils) is passed in `ShellConfig::utilities`; the
engine looks a name up after functions and builtins, before PATH (D1083). Ported code
(uutils calls `std::io::stdout()` and `std::fs` directly) is refactored onto this
context, as oh-my-pi did; re-executing the psh binary per utility, as brush does, is not
an option because it fails in WASM and breaks cancellation (D018).

### Shadowing modes (D015)

Each utility has one mode:

| Mode | Meaning | Used for |
| --- | --- | --- |
| **S, always** | Always the builtin | Where psh can be at least as good as GNU and BSD for agent use |
| **F, PATH fallback** | The builtin only if no program of that name is on PATH, or in WASM/sandbox mode | Tools whose real versions are better, or whose exact output is the point (`gzip -9c \| wc -c`, `tar`, `file`) |
| **W, Windows-only shadow** | S on Windows, F elsewhere | Names that collide with incompatible System32 programs: `find`, `sort`, `timeout`, `tree`, `expand`, `more`, `fc`, `hostname`, `whoami`. On Windows the builtin must win, or `find . -name x` runs the wrong program |

Around the modes:

- An explicit path (`/usr/bin/grep`) always runs the real program.
- **Fall-through** (D010): a utility that meets a flag or form it does not implement
  runs the real program of that name from PATH if there is one, or fails with a clear
  message naming the unsupported flag. It never silently produces different output.
- **Off switches** per session: all utilities, the destructive group (`rm mv cp ln dd`),
  or single names.
- `command -v`, `type` and `which` report utilities truthfully (`jq: shell builtin`).
- `env NAME`, `xargs NAME`, `find -exec NAME` and `timeout NAME` dispatch back into the
  shell, so utilities (and `printf`, `echo`, `test`, `true`, `false`, `pwd`, `kill`,
  which need standalone forms) stay reachable from them.

### Dialect (D016)

- Accept the **union of GNU and BSD** flags where they don't conflict.
- Where they conflict, **GNU semantics**, except where agents overwhelmingly write the
  macOS form (`sed -i ''`: 5,307 BSD vs 144 GNU in the corpus; `stat -f FORMAT`). Those
  are resolved by deterministic rules on the arguments, not by the host OS:

| Command | Rule |
| --- | --- |
| `sed -i` | `''` next, or text attached (`-i.bak`): a suffix. Otherwise, if the next argument parses as a script and operands follow: GNU form. A bare `.`-prefixed argument without command characters followed by a script: BSD suffix |
| `stat -f` | Argument containing `%`: BSD format. `-f` alone with file operands: GNU file-system mode. `-c` and BSD `-f`/`-t` both offered |
| `date -r` | All digits and not an existing file: BSD epoch seconds; otherwise GNU reference file |
| `base64 -i` | Followed by a non-option operand: BSD input file; otherwise GNU ignore-garbage |
| `mktemp -t` | Argument containing `XXX`: GNU template; otherwise BSD prefix |
| `cp -R src/` | GNU (copies the directory), documented |
| `wc`, `uniq -c` | GNU column widths |
| `find` with no path | `.` (GNU) |
| `head -n -N`, `tail -r` | Single-dialect forms, both accepted |
| `md5`/`shasum` vs `md5sum`/`sha256sum` | Output format keyed on the name; never unified |

  Decided in their utility issues (D034), falling through until then: `ls -G`/`-T`
  (BSD color / full time vs GNU no-group / tab size; research suggests the macOS
  meaning), `xargs` on empty input (research recommends BSD: don't run, `-r` accepted as
  a no-op), and `sort`'s default collation (byte order vs locale).
- `uname`, `hostname`, `id` and friends report the real host. psh never pretends to be
  Linux.

### Tiers (D017)

- **Tier 0**, shell builtins: part of the engine (`cd`, `export`, `read`, `mapfile`,
  `printf`, `test`, `kill`, `wait`, `trap`, ...), not up for debate.
- **Tier 1**, day one, 62 names: the top corpus names that are not excluded, everything
  the agent read/edit/search loop needs, the wrappers (which must be builtins so they
  can wrap builtins and own process trees), and `fd`, which the owner moved up (D019):
  `head tail cat ls cp mv rm mkdir rmdir touch ln chmod pwd basename dirname realpath
  readlink mktemp stat du grep egrep fgrep rg fd sed awk sort uniq wc cut tr paste nl tee
  seq comm printf echo test true false sleep find xargs diff cmp jq base64 md5sum
  sha1sum sha256sum sha512sum shasum md5 date env printenv timeout nice nohup which`.
  `timeout` comes first: it is the most common "command not found" (389) in the corpus.
  `find` is W; `awk` is F until conformant; the rest are S.
- **Tier 2**, soon after (~45 names): process tools (`ps pgrep pkill`, per OS), host
  info (`uptime nproc id uname whoami hostname df`), `tree column tac rev strings od
  xxd hexdump expr yes truncate fold expand unexpand join split`, the rest of the
  checksum family, `shuf patch`, archives in F mode (`tar gzip gunzip zcat unzip`),
  `sponge ts ifne`, `dos2unix unix2dos iconv`, `install link unlink sync mkfifo logname
  tty`.
- **Tier 3**, on demand, mostly F: `file bc dc yq` (decompressors as tar codecs),
  `zip dd numfmt fmt pr csplit tsort factor uuidgen getopt envsubst chown chgrp clear
  pidwait pidof free`; `ast-grep` only as an optional feature.

### Sources (D018)

Port mature permissive code rather than write new, above all for the four big ones:

- `rg`, `fd`, `grep` and `find` on the ripgrep crates (next section), `sed` from uutils sed, `jq` on jaq, and **`awk`**,
  which has no mature permissive Rust implementation: port goawk (MIT, passes the
  onetrueawk tests), with onetrueawk (the macOS awk) and its `T.*` tests as reference.
  awk runs in F mode until it passes the onetrueawk and goawk suites.
- Most of the rest (~45 of Tier 1) from uutils coreutils, findutils and diffutils.
- Ours: the wrappers (`timeout nice nohup env xargs`, on psh's process model), `which`,
  `xxd` (written from the man page; xxd itself is under vim's license), `tree` (GNU tree
  is GPL), `column`, `strings`.
- GPL implementations (GNU coreutils, sed, grep, gawk, tree, patch, busybox, moreutils)
  are never read (D007); BSD implementations are fine as references.

**Crates (D028).** `jq` depends on jaq (copied instead only if its issue shows size
demands it). Crates for the date parser (`date -d`, `touch -d`, `find -newermt`,
`fd --changed-*`), tar/zip and process listing (`sysinfo`) are decided, and recorded, in
the utility issue that needs them. The search crates, including `encoding_rs`, are
D020/D021; there is no PCRE backend, so `grep -P` accepts what the regex crate parses
and falls through otherwise.

### Search: rg, fd, grep and find on the ripgrep crates (D020, D021)

`rg` and `fd` are embedded by owner requirement, on the upstream crates, the one
standing dependency exception (D006). Neither ripgrep nor fd has a library entry point,
and fd's `main` is unusable in-process (it changes the process cwd, installs a Ctrl-C
handler, needs threads, pulls clap and jiff). Source: `research/builtins.md` §5. All of
this is D020, confirmed by the owner (D021).

**Crates.** Exactly: `grep-matcher`, `grep-regex`, `grep-searcher`, `grep-printer`
(`default-features = false`, so no serde), `ignore`, `globset`, `regex` (plus
`regex-automata`/`regex-syntax` directly where needed), and what they pull in: about 27
crates on macOS, 30 on Windows, 25 on wasm32. Not `grep-cli` (process-centric helpers;
psh owns TTY, stdin and decompression) and not `grep-pcre2` (C code, breaks the WASM
build). `encoding_rs` arrives with `grep-searcher`. Marginal size of rg+fd+grep+find:
about 0.5–0.75 MB on top of the regex stack (1.2–1.4 MB), which `sed`, `[[ =~ ]]` and
`find -regex` need anyway. Vendoring both CLIs verbatim would cost 3.7–4.7 MB.

**rg: copy ripgrep's flag layer ("B+").** `crates/core/flags/*` (defs, lowargs, hiargs,
parse) and the glue in `search.rs`/`main.rs` are copied into `psh-utils` with
attribution (MIT OR Unlicense), so the whole flag surface and its `--help` text stay a
mechanical diff from upstream. The seams are patched: per-invocation message/error state
instead of statics, cwd/env/stdio/TTY from `UtilCtx` (also for `RIPGREP_CONFIG_PATH`),
`--pre` and `-z` through psh's command runner, no `process::exit`, no global logger.
`lexopt` is copied (about 1k lines, no dependencies). Search and printing use
`grep-searcher` and `grep-printer`; `rg --json` writes its JSON through `deser-json`
(D029), since `grep-printer`'s JSON printer needs serde. Broken pipe ends the search with
status 0, so `rg … | head` stops early.

**fd: rewrite the CLI, copy the logic.** fd's command line is re-implemented on psh's
own argument parser; its filters (`filter/size.rs`, `time.rs`, `owner.rs`), `exec/`
(`-x`/`-X` templates, run through the command runner) and `fmt/` (`{}`, `{/}`, `{//}`,
`{.}`, `{/.}`) are copied with attribution (MIT/Apache). clap, jiff, lscolors, ctrlc,
nix, etcetera, normpath, faccess, argmax and crossbeam-channel are not used.
`--changed-within/-before` use psh's shared date parser. `--base-directory` never
changes the process cwd. fd's tests and documented flags are the conformance target.

**One walker seam, two implementations.**
- *Native:* `ignore::WalkBuilder`/`WalkParallel` with absolute roots (parent-directory
  ignore discovery canonicalizes against the *process* cwd), `current_dir(cwd)`, display
  paths mapped back to the user's spelling, global gitignore located from the shell's
  `HOME`/`XDG_CONFIG_HOME`/`GIT_CONFIG_*`, not the process's, and a cancel check in each
  visitor. The parallel walker uses scoped, always-joined threads, which fits the
  utility thread rule.
- *Virtual fs (WASM, sandboxes):* psh's own single-threaded walker over the `UtilCtx`
  fs. It reuses `ignore`'s in-memory matchers (`gitignore`, `overrides`, `types`) and
  re-implements only the ignore chain (`.ignore` > `.gitignore` > `.git/info/exclude` >
  global, parent directories), about 600–800 lines copied from `ignore/src/dir.rs` with
  attribution.
- A **parity test** runs both walkers natively over the same fixture trees with the same
  flags and requires identical results.

**Sorted output.** When rg or fd search with more than one thread, output is
path-sorted by default (real rg's order is arbitrary, so sorted output is a valid rg
output). This keeps goldens and replay stable. ripgrep's `--sort path` forces a single
thread, so psh merges results in order instead; the cost is measured in 0013.

**grep shares the engine, not the rg front end.** Shared: the `grep-regex` matcher,
`grep-searcher` (context, binary detection, `-z`, encodings), the walker, and the
0/1/2 exit rules (`-q` beats errors). GNU-specific: a POSIX BRE/ERE → `regex-syntax`
translator (`\( \) \{ \}`, GNU `\+ \? \|`, leading `*` literal, `[[:class:]]`, bracket
rules; oh-my-pi's `bre.rs` is an MIT reference); GNU output through psh's own `Sink`
(`file:line:text`, `file-line-text` context, `--` separators, `Binary file X matches`,
the `-c -l -L -H/-h` defaults), since `grep-printer`'s standard printer is rg-shaped;
GNU recursion (`-r` without a path means `.`, hidden files searched, `.gitignore`
ignored, `-R` follows symlinks, `--include/--exclude/--exclude-dir` as basename fnmatch
via `globset` with `literal_separator(false)`).

**find** uses the same walker with all ignore filtering off; `-name`/`-path` go through
`globset` in fnmatch mode, `-regex` through the translator.

**Regex gaps** (shared by grep and sed):
- *Backreferences* (`\(a\)\1`): the regex crate lacks them. Plan: a small backtracking
  matcher of psh's own implementing `grep_matcher::Matcher`, used only when the
  translator sees a backreference. Until it exists, such patterns fall through to the
  host program (D010), or fail clearly where there is none (Windows, WASM).
- *Leftmost-longest:* Rust regex is leftmost-first, POSIX is leftmost-longest. Line
  selection is the same, but `grep -o`, `--color`, `-b` and `sed s///` can pick a
  different span (`echo abcd | grep -o 'ab\|abcd'`: POSIX `abcd`, Rust `ab`). Plan:
  `regex-automata` with `MatchKind::All` and a forward search to the last match end for
  those modes (needs a spike). Until then, in those modes a conservative check (for
  example, any alternation in the pattern) makes the utility fall through; the spike
  settles the exact rule.
- *PCRE-only syntax* (`grep -P` lookaround): no PCRE; what `regex-syntax` parses is
  accepted, the rest falls through.
- *Locale:* UTF-8 Unicode semantics for classes and ranges, documented; this matches GNU
  in a UTF-8 locale.

**Windows.** Default `/` path separators for rg and fd (as `--path-separator /`; an
explicit flag overrides). `psh.exe` carries a `longPathAware` manifest like ripgrep's.
Color is ANSI or none, from the `UtilCtx` TTY state, never the console API. Gitignore
matching stays case-sensitive (ripgrep's default). `grep` and `rg` treat CRLF as a line
end by default on Windows (ripgrep's `--crlf` behavior; D030); explicit flags override.

**WASM.** The `grep-*` crates, `globset`, `regex` and the `ignore` matchers build for
`wasm32-unknown-unknown` and work over `UtilCtx` readers; only the `ignore` walker does
not (every fs call is `Unsupported`), which is what the virtual-fs walker replaces.

**In pi-env** the utilities (and with them the regex stack) are a cargo feature of the
daemon build, on by default (D031).

### Never builtins and out of scope (D014, D022)

- **Never builtins:** database clients and stateful engines (`sqlite3`, `psql`, `mysql`,
  `redis-cli`, `postgres`), interpreters and language runtimes, toolchains and build
  systems, package and environment managers.
- **Out of scope for now:** `git`, `curl`/`wget`, and ssh/rsync-like network and VCS
  clients (`ssh scp sftp rsync nc gh aws gcloud kubectl docker podman` and the like).
  They are not utilities and nobody works on them, but they are not "never": psh may
  later hook them, as host-provided commands or by interception.
- **Not utilities, no work planned** (proposals from `research/builtins.md` §4.5, not
  owner rules): interactive/TTY programs (`less vim top watch tmux fzf`), privileged and
  OS-specific system tools (`sudo launchctl lsof pbcopy`), heavy domain tools (`ffmpeg
  pandoc 7z`), compressors in compress mode when a real one exists, benchmarking tools,
  `shred`, `mkfs`.
- An embedder may supply any such command through the host (in WASM, like a
  fetch-backed `curl`); those are host commands, not psh utilities.

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

**Sessions (D026).** Each `exec` gets a fresh psh shell; nothing (cwd, variables,
functions) persists across calls for now, which keeps Durable's replay model intact.
Node-side execution (`NodeExecutionEnv` and friends) uses psh through the daemon, not a
separate binary or addon, so local p2 execution moves from Node spawning to the daemon.
The utilities are a daemon cargo feature, on by default (D031).

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
- Per-utility knowledge (which flags write, which options take a value, what the
  operands are) lives in `psh_classify::utils` (D1090): no_std data that the utilities
  depend on and check their own option parsing against in their tests.

## Open questions

1. Where Windows output decoding lives (engine output layer or host).
2. *(Decided, D1090: in `psh_classify::utils`.)*
3. Whether the WASM host is needed before pi-env integration, and whether a WASI build
   is wanted at all.

Decided and recorded: std vs no_std (D023), platform bindings (D024), MSRV (D025),
pi-env sessions (D026), Windows CI (D027), utility crates (D021, D028), JSON (D029),
CRLF on Windows (D030), pi-env feature (D031), license (D032), goal ranking (D033),
never-builtins and out-of-scope clients (D022), dialect points (D034).
