# psh: Architecture

The design reference for psh. It describes the system as designed and built, follows the
agreed constraints in `research/README.md` §8 and the execution-core study in
`research/execution-core.md`, and cites the decision behind each choice (`Dnnn` in
`docs/decisions.md`). It is not a changelog: when the design changes, this file says what
is, and the decision log keeps the history. Parts not built yet are marked **Planned**;
open questions are marked **Open** and collected at the end. What is built and how well
it scores is in `README.md` and `scripts/status`.

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
- **Portable.** The same semantics on macOS, Linux and Windows; Windows is not a port
  done at the end.
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
then the rest, unranked among themselves. Examples of the ranking at work: one signal
numbering everywhere (D1904), byte-order `sort` in every locale (D1160), `cut -c`
counting characters (D1161), non-ASCII regex ranges by code point (D1271). Under all of
them: permissive licensing (D007), MIT OR Apache-2.0 (D032).

## Crates

```
crates/
  psh-syntax/    lexer, parser, AST with byte spans, printers,     no_std + alloc, no deps
                 arithmetic parser and evaluator
  psh-regex/     POSIX ERE/BRE with GNU semantics (D1270)          no_std + alloc, no deps
  psh-classify/  AST -> effects and summaries; per-utility tables  no_std + alloc, psh-syntax
  psh-engine/    the stepped interpreter; Host trait; vprocs;      std, psh-syntax + psh-regex
                 shell builtins; the utility API (UtilCtx)
  psh-os/        OS hosts: Unix and Windows                        psh-engine, libc / windows-sys
  psh-utils/     in-process utilities and their registry           psh-engine, psh-classify,
                                                                   psh-regex, ripgrep crates (D020),
                                                                   deser-json (rg --json, D1136)
  psh/           the `psh` binary                                  all of the above, deser-json,
                                                                   argument (D035)
  psh-spec/      the conformance runner (TESTING.md)               engine, syntax, os, utils
  psh-wasm/      the WASM modules and their ABI                    syntax, classify, engine,
                                                                   utils, deser-json
  psh-docs/      the guide: its writing rules, live examples and   engine, utils, syntax, classify,
                 the static site (D1120, D1360, D1361, D1400)     pulldown-cmark
third_party/
  grep-printer/  grep-printer 0.3.1 with a wasm-safe clock (D1135), via [patch.crates-io]
js/              the JS glue and host for psh-wasm (`psh.mjs`), its tests and spec driver
```

Every dependency of every crate, direct or transitive, is listed with its decision in
`scripts/deps-allowlist.txt`, checked by the gate (D1223). pi-env (in pi-mono) will
depend on `psh-engine`, `psh-os` and `psh-utils`, and `psh-classify` for a parse op.

**std or not (D023).** `psh-syntax`, `psh-regex` and `psh-classify` are `#![no_std]` +
`alloc`, so the parser and classifier build small for wasm. `psh-engine` uses std; its
`wasm32-unknown-unknown` build in the gate guards against OS access. The engine must
never force std into the parser: no std-only types in the parser's API, no parser
feature that turns std on.

**unsafe.** Forbidden workspace-wide (D1014), except in `psh-os` and in `psh-wasm`'s
export layer (the JS import and the `no_mangle` exports, D1110), where every block says
why it is sound.

**Toolchain (D025).** MSRV is 1.96, pi-env's pinned toolchain; development uses current
stable; the gate's `msrv` leg checks 1.96.

**JSON (D029).** All JSON in psh goes through `deser-json`. The parser crate does not
depend on it: `psh --parse` and `--classify` build their JSON in the `psh` crate
(`parse_json.rs`, `classify_json.rs`, D1047), and `psh-wasm` reuses those files.
The live examples' AST tab shows `psh_syntax::outline` (D1400): rows for people, with
wrappers of one child collapsed, rendered by `psh-docs` at build time and through the
WASM parser (`psh_parse`'s outline flag, `outline_to_json`) in the browser, where the
JSON view also comes from; `psh --parse --tree` prints the same rows as text.

## psh-syntax: the parser

- A hand-written lexer and recursive-descent parser over bytes. Only the AST vocabulary
  (the `[[ ]]` predicates, the parameter-expansion operations) is adapted from
  brush-parser, attributed (D008, D1046).
- **Byte spans on every node** (`u32` start and end into the source). Line and column
  are computed on demand from a line table. The classifier, the UI and error messages
  all point into the original string. Sources are bytes (D1233): a script that is not
  UTF-8 parses byte-exactly, and the `&str` APIs see a reversible UTF-8 view.
- **Arenas** (D1040): an `Ast` owns the source and holds lists, commands and here-docs
  in arenas addressed by ids; the engine keeps `(Arc<Ast>, id)` pairs. Literal word
  parts are spans, so parsing copies no text.
- What `bash -n` does not check is deferred, as in bash (D1041): backquote bodies,
  arithmetic (kept as text with expansions), invalid `${…}` ("bad substitution") and
  errors in unquoted here-doc bodies. `$(…)`, `<(…)` and `${ …; }` are parsed in place.
  Brace expansion is structure; tilde and globs stay text for the engine (D1042).
- **Arithmetic** (D1230): `parse_arith` parses the text of `$(( ))`, `(( ))`, `let` and
  subscripts after expansion; `eval_arith` evaluates it over an `ArithEnv` the engine
  implements, with bash's 64-bit semantics and messages.
- Parses bash 5.3 syntax only: no zsh, no POSIX-sh mode. extglob is a parse option
  (D1043). **Aliases** expand in the chunked parser when the engine passes its alias
  table (`shopt -s expand_aliases`): every node from an alias text has a span of its own
  beyond the script, mapped back with `Ast::original_span` (D1250).
- **Incremental**, as bash runs scripts: `Parser::next_chunk` yields one top-level line
  at a time, so `echo a` on line 1 runs before a syntax error on line 2 is reported
  (status 2); the options (`extglob`, aliases) may change between chunks. `--parse` and
  the classifier parse the whole input.
- **Printers.** `print` reprints a tree with words as written, so `parse(print(ast))`
  holds on every L0 input (D1044); `print_function_bash` prints a function exactly as
  bash 5.3's `declare -f` does (D1232). `outline` lists the tree for people, one row
  per node with its depth, kind and source text (D1400); `dump` is the tests' form.
- Nesting deeper than 100 is the parse error "nesting too deep", which fits wasm32's
  stack (D1045).

## psh-engine: the interpreter

A synchronous, sans-IO state machine (D005). The shape follows oh-my-pi's engine at the
level of architecture only (clean room, D007); the code is ours.

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
  nesting limit, `exit_grace`, the utility policy), `shell.set_waker(waker)`,
  `shell.start(Script)`, then `shell.step(budget) -> Step`: `Done(status)` (once),
  `Blocked` (everything waits on the host or the shell is paused; the waker fires when
  there is work), `Yield` (budget spent), `Idle` (no script). The budget counts
  safepoints: one before every command and loop iteration. Shell state (variables, cwd,
  fds) persists across runs of one `Shell`. `std_host::drive` is the park loop the `psh`
  binary and psh-spec use. `psh_engine::invocation` parses bash's command line (`-c`,
  `-s`, file, options), shared by the binary, psh-spec and psh-wasm.
- **`!Send`.** A `Shell` holds `Rc`s: create and step it on one thread. That fits
  pi-env's thread per exec, psh-spec's worker threads and the JS event loop.
  `ControlHandle` is `Send + Sync`.
- **Stack (D1050, D1059).** Nested futures are polled recursively, so nesting within one
  task (compound commands, function calls, `eval`, `source`) uses native stack: about
  3 KiB per function call level in release builds and 15 KiB in debug builds. Compound
  and simple commands dispatch through plain functions returning boxed futures, and a
  list item that is a single command skips the and-or and pipeline frames.
  `ShellConfig::max_depth` (default 1000) bounds functions plus subshells and fails with
  "maximum function nesting level exceeded" instead of overflowing; native embedders run
  the engine on a thread with `std_host::ENGINE_STACK` (about 20 MiB, only touched pages
  cost); the wasm module links a 16 MiB stack and caps the depth at 768 (D1111).
  Subshells, pipeline stages and substitutions are tasks of their own and do not deepen
  the stack.
- **Virtual processes.** Subshells, pipeline stages (including compound commands and
  functions), `$(…)`, `<(…)` and `&` jobs are vprocs: a task owning a copy of the
  parent's state (`Vp`: variables, functions, options, traps, positional parameters,
  cwd, fd table, `$?`). Nothing forks; this is also what makes Windows and WASM
  possible. Pipeline stages run concurrently, never one after another (pipe buffers
  would deadlock). A stage that is one external command lets go of its descriptors once
  the external has them, as bash's exec of the stage would (D1902).
- **Control (D1051, D1284).** A `ControlHandle` (an atomic control word: cancel, pause,
  pending signals) is checked at every safepoint and before every poll. `cancel` is a
  forced TERM: every task is dropped, the run ends with 143, and dropping is what
  releases everything, since every host resource is released by `Drop` (externals
  abandoned: TERM, grace, KILL, background reap; files and pipes closed). `signal(sig)`
  sets a pending signal that the shell's own vproc takes at its next safepoint: a trap
  runs, an ignored signal does nothing, otherwise the default action (EXIT trap, then
  `128+n`); waits and blocking reads of the shell give way to such signals (D1903,
  D1306). `start` clears the control word, so a late cancel never leaks into the next
  run. A nested `bash`/`sh` has a pending-signal mask of its own, which `kill` and its
  `ExtProc` set; each vproc's `sigs` names the mask it takes signals from (D1411).
- **Signals are numbered as on Linux** everywhere, in `kill -l`, `trap`, `$?`; hosts
  map them to the OS's numbers at their boundary (D1904).
- **Values are bytes (D1053).** Variables, words and paths are `Vec<u8>`; char-wise
  operations (`${#x}`, patterns) decode UTF-8 when `LC_ALL`/`LC_CTYPE`/`LANG` says
  UTF-8, else work on bytes. Messages are `$0: line N: …` as in bash.
- **Pipes between vprocs (D1054)** are in-memory engine objects (64 KiB, reader and
  writer counts; writing without readers ends the vproc with status 141, its SIGPIPE).
  An external gets a host pipe for a descriptor that is an engine object, and the
  engine pumps between the two; pumps are polled inside the wait for the external, so a
  cancel drops them too. Input pumps stop when the external exits; the command completes
  when the external has exited *and* its output pumps reached EOF. Descriptors that
  share one engine object (`|&`, `2>&1` into a pipe) share one host pipe. A pipe whose
  buffer is empty when an external starts on one of its ends becomes an OS pipe of the
  host (`Host::pipe`, D1068): externals get the OS end directly (no pump between two
  externals), vprocs write to it and read through the engine buffer. Pipes with
  buffered bytes (here-docs) and hosts without OS pipes keep pumps.
- **Here-docs and here-strings (D1054)** are pipes preloaded with the expanded body and
  no writer: nothing is written into a kernel pipe before its consumer runs.
- **`/dev/stdin`, `/dev/stdout`, `/dev/stderr`, `/dev/fd/N`** in redirections always
  refer to the vproc's descriptors (embedded, the process's are not the shell's; D1055).
  `{var}>file` takes a descriptor from 10 up and outlasts the command (D1326).
- **Command lookup** (D1083, D1291, D1410): functions, then shell builtins, then a bare
  `bash` or `sh` (psh itself, below), then psh's utilities (when the embedder passed a
  policy), then `PATH` through the host, one entry at a time (`Host::path_entries`,
  D1910), remembered in a hash table that belongs to the `PATH` value it was filled
  under. A name with a slash, a hashed path and `command -p` (the host's standard path,
  D1332) run the program. A program the host cannot start itself (no `#!` on Unix;
  `sh`/`bash` scripts on Windows) comes back as `RunInShell` and runs in a fresh psh
  subshell, as bash runs a file `execve` rejects (D1300).
- **Nested shells** (D1410–D1412, `util/nested.rs`): a bare `bash`/`sh` (on Windows also
  `/bin/sh`, `/bin/bash`, `/usr/bin/sh`, `/usr/bin/bash`) runs in psh, also without a
  utility policy and from the wrappers (`env`, `xargs`, `find -exec`, `timeout`); `type`
  reports it like a utility; `ShellConfig::nested_shells` turns it off. Its command line
  is `invocation::parse_args`; options psh lacks run the program from `PATH`. It starts
  afresh (`exec::start_afresh`: variables rebuilt from the exported ones as `Shell::new`
  builds them, `SHLVL` one higher, no functions, aliases, traps or options beyond
  `SHELLOPTS`/`BASHOPTS`) and is a virtual process: an `ExtProc` over a task of its own,
  started where programs are spawned, with a job id for `$!` and a pending-signal mask
  of its own (`Vp::sigs`), so jobs, `kill`, `wait` and abandoning treat it as a program
  and its traps run. It has no `$$` (D1413): a script that reads `$$` or `BASHPID`
  (`pidscan.rs` over its parsed tree) runs under the real program on `PATH` that is not
  psh itself (`ShellConfig::own_program`); without one, reading them is an expansion
  error, as is `BASHPID` in any subshell. Shebang scripts are not intercepted.
- **Shell builtins** (`cd`, `export`, `set`, `trap`, `read`, `printf`, `test`, `kill`,
  `wait`, ...) are async functions on the calling vproc. Assignments in front of any
  builtin are temporary and exported (D1290). Job control is limited to `&`, `wait`,
  `kill`, `$!`, `jobs`, `disown` (D002): a job that is one simple command is its
  program (`$!` is its OS pid), any other job a vproc with a pid of its own (D1900); a
  job that is an in-process utility has no OS pid (D1085). When a script ends, detached
  vprocs get one more round; then, by `ShellConfig::exit_grace`, their externals still
  running are released (bash parity: background jobs outlive the shell) or waited for
  up to the grace and then abandoned (embedders that must leave nothing behind).
- **Expansion** (brace expansion, tilde with `Host::home_dir` for `~user`, parameters,
  command substitution, arithmetic, word splitting, globbing, quote removal) is engine
  code over the AST; globbing reads directories through the host. One pattern matcher
  with options serves globs, `case`, `${x#…}` and `[[ == ]]`, with backtracking extglob
  groups (D1321). `[[ =~ ]]` and every other regex is psh-regex (D1272). `$(…)` drops
  a `\r` before each trailing newline it removes when `Host::strips_crlf()` (D1940).
- **Variables (D1260–D1264, D1320, D1328, D1329).** `Var { value, exported, readonly,
  integer, case, nameref }` with `Value::{Unset, Str, Indexed, Assoc}`; associative
  arrays iterate in bash's order (D1261). Dynamic variables (`RANDOM`, `SECONDS`,
  `FUNCNAME`, `BASH_SOURCE`, `GROUPS`, ...) are computed when read. psh sets
  `BASH_VERSION=5.3.9(1)-release` and `PSH_VERSION` (D1264); `UID`, `PPID`, `OSTYPE` and
  friends come from `Host::info()`.
- **Options, errexit, traps (D1280–D1285).** `set` and `shopt` options live per vproc;
  `set -o posix` is refused (POSIX mode is out of scope). errexit follows bash 5.3's
  exemptions exactly. Traps cover EXIT, ERR, DEBUG, RETURN and signals; `set -x` traces
  as bash does.
- **Errors (D1057, D1058).** How control leaves a command is `Unwind`: `Exit` ends the
  (sub)shell (`exit`, `${x?}`, SIGPIPE, unsupported features), `Abort` abandons the rest
  of the top-level line and continues with the next one (bad substitution, arithmetic
  errors, readonly assignments: bash's behavior), `Return`, `Break(n)`, `Continue(n)`.
  A construct the engine does not run prints `X: not supported yet` and ends the shell
  with 2: never a silent different behavior.

### Engine internals other work builds on

Parallel language work extends these; restructuring them needs a decision first:
- `sched.rs` (tasks, wakers, budget, safepoints) and the `Shell`/`Step`/`ControlHandle`
  API.
- `Vp` as the whole per-vproc state, cloned for subshells; `Unwind` as the only
  non-local control flow; `R = Result<i32, Unwind>`.
- The dispatch shape in `exec.rs`: `run_list` → `run_command` (safepoint) →
  `run_simple` / `run_compound` returning boxed futures; `prepare_simple` (expansion,
  assignments) then `run_argv` (function, builtin, utility, external). Keep new
  compound commands as their own boxed async fns (stack per level, D1059).
- `expand.rs`'s `Fields` model (quoted, literal and unquoted text; `pat` for globbing)
  and its entry points (`expand_words`, `expand_single`, `expand_pattern`,
  `expand_target`, `expand_heredoc`, `expand_decl_words`).
- `FdObj`/`FdTable` and the redirection save/restore in `with_redirects`/`run_simple`;
  pipes and pumps (`pipe.rs`, `fd::Pump`).
- Builtins are `fn(&mut Vp, &[Vec<u8>]) -> LocalBoxFuture<R>` in `builtins::find`;
  output through `builtins::write_fd`/`out` (SIGPIPE as status 141). Declaration
  builtins take `DeclArg`s (D1262).
- Variables through the `Vp` accessors (`var`, `set_var`, `assign`, `assign_elem`,
  `assign_array`, `get_elem`, `unset_elem`, `is_set`, `resolve`), never by poking `Var`
  fields; dynamic variables refreshed by `Vp::refresh_dynamic` (D1263).

### The Host trait

Every side effect goes through one trait (`host.rs`, D1052), with pollable operations
so that any of them may be `Pending`. File system operations return
`HostFuture<T> = Pin<Box<dyn Future<Output = T>>>`; paths are bytes, absolute, `/`
separated (a Windows drive stays in front: `C:/x`, D1323), converted at the host's
boundary. Methods with a default answer `Unsupported` (changes) or a neutral value
(queries), so a small host needs only the core.

| Group | Methods |
| --- | --- |
| Core (required) | `open(path, OpenOptions)` (with a creation `mode`, D1069), `stat(path, follow)`, `access`, `read_dir`, `canonicalize`, `resolve(name, PATH, cwd)`, `spawn(SpawnSpec)`, `sleep`, `pid` |
| Processes | `pipe()` (OS pipes, D1068), `kill(pid, sig)`, `path_entries(PATH)` (D1910), `standard_path()` (D1332), `native_paths()` (the OS path of a host path, if the file system is the process's: lets blocking utilities use `std::fs`, D1130) |
| File system changes (D1150) | `stat_extra`, `mkdir`, `remove_file`, `remove_dir`, `rename`, `symlink`, `hard_link`, `read_link`, `set_mode`, `set_times` |
| Facts | `info()` (uid, euid, groups, ppid, hostname, `OSTYPE`/`MACHTYPE`/`HOSTTYPE`, D1301), `umask()`, `now()`, `utc_offset`, `user_name`, `group_name`, `home_dir(user)` (D1322) |

`spawn` takes a `SpawnSpec` (program, argv with `argv[0]` as written, the complete env,
cwd, the child's descriptors as `ChildFd::{File, Pipe, Dup, Null}`, signals to start
ignored, a umask, `detach` for `nohup`/`setsid`) and returns the external as an
`ExtProc` (`poll_wait`, `signal`, `release`: keep running and reap in the background;
`disown`: leave the run; `pid`; drop = abandon: TERM, grace, KILL, background reap) plus
the engine's ends of its pipes. A spawn error carrying `RunInShell` asks the engine to
run the file itself (D1300). Open files are `HostFile`s (`poll_read`, `poll_write`,
`is_terminal`, `kind`).

`Host::strips_crlf()` makes the CRLF rule of `read` and `$(…)` a host property (D1292,
D1940); the Windows host does not return true yet (a small follow-up). There is no
exec-time approval hook: classification serves reporting only (D036).

Hosts:

| Host | Where | Notes |
| --- | --- | --- |
| `psh_os::unix::UnixHost` | macOS, Linux, Android; `psh` and psh-spec there | Below (D1060–D1069) |
| `psh_os::windows::WindowsHost` | Windows (x64, arm64); `psh` and psh-spec there | Below (D1074–D1078, D1911–D1919) |
| `psh_engine::std_host` | simple embedders and tests | std `Command`, a waiter thread per child, helper threads per pump, KILL only, no process groups |
| `psh_engine::testhost` | engine tests | In-memory fs, fake commands written as futures, captured stdio; a latency mode makes every op `Pending` once; counts live processes, pipes and files for leak checks |
| JS host (`MemHost`, `js/psh.mjs`) | `psh-wasm` | In-memory fs, commands written in JS, timers; driven from the JS event loop (D1112) |

## psh-os: the OS hosts

From `research/execution-core.md` §5; the pitfalls listed there are requirements.

**Unix.**
- Spawn (D1060): `posix_spawn` with a session of its own per external (`SETSID`),
  `SETSIGDEF` for every catchable signal (SIGPIPE back to default), an empty signal
  mask, `CLOEXEC_DEFAULT` on macOS, `addchdir_np` where available (glibc ≥ 2.29, found
  with `dlsym`); a bare `fork`+`execve` elsewhere and for what `posix_spawn` cannot
  express. `ENOEXEC` (no `#!`) answers `RunInShell` (D1300).
- Wait (D1062): pooled waiter threads block in `waitid(WEXITED | WNOWAIT)` and wake the
  engine. No SIGCHLD handler. A process is never reaped while its `ExtProc` exists, so
  signals cannot hit a recycled pid; one party reaps, under the lifecycle lock
  (`psh_os::lifecycle`).
- Files and pumps (D1061): no thread per pump. Our private pipe ends to externals are
  `O_NONBLOCK`, and I/O that would wait registers the waker with one process-wide
  readiness thread (`poll`; `select` on macOS, where `poll` misses FIFO writer close).
  Shared and inherited descriptors (the shell's stdio, redirections) keep their flags:
  a zero-timeout readiness check before each read or write, writes to streams capped at
  `PIPE_BUF`. FIFO opens run on a helper thread; timers on one process-wide thread.
  Helper threads start with every asynchronous signal blocked (D1901).
- Kill (D1065): TERM and CONT to the group, KILL to the group when the leader exits or
  after a grace period (default 2 s), then a reap.
- Runs (D1067): embedded, a host is one run; dropping it kills what the run left
  (released background jobs, what finished externals left in their sessions) unless it
  was detached (`nohup`, `setsid`) or disowned; `Leftovers::Keep` leaves it, as bash.
  `UnixHost::leftovers()` lists groups with live members; psh-spec's leak check turns
  `detached_survive` off so even `nohup` jobs count (D1305). Standalone, externals stay
  in psh's own process group, which a harness's group kill reaches.
- Signals: embedded, install no handlers at all. Standalone `psh` (D1903, D1302): the
  trappable signals are caught on the engine thread only; the handler sets the signal's
  bit in the shell's pending mask (traps run, D1284) and a watcher thread wakes the
  shell; ended by a default action, psh dies by the signal after its abandoned externals
  are killed. Children start with every signal at its default except those the engine
  asks to ignore and, standalone only (`inherit_ignored`), those ignored when psh
  started (D1304).
- umask, creation modes and the clock (D1069): the host reads the process's umask once
  and never changes it; a creation mode gives exact bits.

**Windows.**
- Spawn (D1074): `CreateProcessW` suspended, assigned to a Job Object of its own (no
  breakaway), then resumed; only the three stdio handles inherited
  (`PROC_THREAD_ATTRIBUTE_HANDLE_LIST`), so descriptors above 2 are not passed.
  `Placement::Group` (default): `CREATE_NEW_PROCESS_GROUP`, so TERM can be a
  ctrl-break; `Inherit` (standalone psh) keeps psh's console group. Without a console,
  `CREATE_NO_WINDOW`: nothing flashes. The run's jobs kill their processes when their
  last handle closes, so a dying psh takes them along; detached and disowned externals
  are exempt, and `Leftovers::Keep` clears the flag when the host is dropped.
- Programs: `.exe`/`.com` directly; `.bat`/`.cmd` through `cmd.exe` with the hardened
  quoting Rust std uses (BatBadBut), refusing line breaks; MSVCRT argv quoting
  otherwise. A file starting with `#!` runs under its interpreter, looked up on the
  child's `PATH` (`/usr/bin/env X`, `/bin/X`, `/usr/bin/X` all mean X); `sh`/`bash`
  scripts and files without an interpreter run in psh itself (`RunInShell`, D1300).
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
- File system changes (D1915): read-only files are removed like others, a symlink to a
  directory is removed as the link, `symlink` makes a junction to a directory when
  Windows refuses symlinks for lack of privilege and a clear error for a file, never a
  copy. `umask()` is `0o022`; a creation mode without write bits sets the read-only
  attribute; other mode bits have no Windows meaning (D1911). Execute bits come from
  the file itself (directories, `PATHEXT` extensions, `#!`); `chmod +x` changes nothing
  and nothing emulates it (D1919).
- Facts (D1913): `OSTYPE=msys`, `MACHTYPE=x86_64-pc-msys` or `aarch64-pc-msys`, as Git
  Bash reports, so agents' scripts take their Windows branch; uid, euid and groups
  1000. `standard_path()` is Windows' default system path (D1917).
- Decoding (D1078): optional (`WindowsConfig::decode = Decode::Ansi`), in the host's
  pump pipes: child output read by the engine that is not UTF-8 is converted from the
  ANSI code page. Off by default.
- Leak check (D1918): psh-spec's Windows host counts `nohup` jobs too; a job with a live
  process after the grace period fails the case.

**Windows rules** (from `research/landscape.md` §3):
- Paths: forward slashes internally, a drive in front (`C:/x`); `C:\x`, `C:/x` and
  `/c/x` are accepted; conversion to native form only at the OS boundary. No MSYS-style
  rewriting of argv to external programs. `\\?\` for long paths.
- `/dev/null` maps to `NUL`, `/tmp` to the user's temp directory, `/c/…` to `C:/…`;
  `/dev/stdin|stdout|stderr` are emulated in redirections. `HOME` defaults to
  `USERPROFILE`, `TMPDIR` to the temp directory, both `/`-separated, and the OS's `Path`
  is the shell's `PATH` (D1075). Null-device aliases (`NUL`, case-insensitive, also
  with an extension or trailing spaces) resolve to `NUL` before long-path conversion;
  explicit verbatim paths remain literal for artifact removal (D1370).
- Command lookup (D1076, D1910): `PATH` entries separated by `;`, and by `:` except
  after a drive letter, split by the host; `PATHEXT`, case-insensitive; a bare name only
  when it is a `#!` script; no implicit cwd search. `test -x` decides by extension or
  `#!`.
- Environment names are case-insensitive for the OS environment, case-sensitive for
  shell variables.
- CRLF: scripts with `\r\n` parse; `$(…)` (also backticks, `$(< f)`, `${ …; }`) drops
  the `\r` before each trailing newline it removes, as `read` does, on a host whose
  `strips_crlf()` is true (D1292, D1940). **Planned:** the Windows host does not
  override it yet, so on Windows both keep the `\r` today. `grep` and `rg` treat CRLF
  as a line end (D030).
- Utilities whose names collide with System32 programs are always the builtin on
  Windows: `find`, `sort` and `timeout` are mode S everywhere (D1172), the others
  (`tree`, `expand`, `more`, `fc`, `hostname`, `whoami`) mode W.

**Windows CI (D027, D1070–D1073, D1916).** The gate's last leg runs fmt, clippy, the
tests (arm64 natively, x64 under emulation), the MSRV check and L4 on the owner's
Parallels VM via `prlctl exec`, one run at a time in one shared checkout (TESTING.md
"Windows"). It moves to GitHub Actions once the repository has a remote.

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
`UtilCtx` (`psh_engine::util`, `UTIL_API_VERSION` 3; every change is a decision) gives
it:

- the command's descriptors: stdin reads awaited; stdout buffered and flushed when full,
  at exit, before every stdin read, before every stderr write and before `spawn`
  (D1081);
- cwd, the command's environment, `utf8()` (the locale), `is_terminal`, `now`, `umask`,
  `utc_offset`;
- the Host's file operations resolved against the cwd, including the D1150 changes
  (`mkdir`, `rename`, `symlink`, `set_times`, ...);
- `sleep`, and `check()`, a safepoint for long loops;
- `spawn(argv, env)` for the wrappers (`xargs`, `find -exec`, `env`, `timeout`, `nice`,
  `nohup`): a child vproc running a standalone builtin, a utility or a program, never a
  function; `SpawnEnv` sets its stdin and cwd (D1170, D1203);
- `spawn_shell(script)`, a command string run as `sh -c` would, in a fresh psh shell on
  the same scheduler, never `/bin/sh` (`awk`'s `system()` and pipes, D1191);
- `run_program`, the fall-through to the real program (D1082);
- `run_blocking(options, arg, body)` for code built on synchronous crates (ripgrep's,
  jaq): the body is a `fn(&mut SyncCtx, T) -> i32` with std `Read`/`Write` stdio and a
  synchronous file system (D1130). When the host has `native_paths` it runs on a worker
  thread whose stdio are bounded queues the engine pumps, and may use `std::fs` on the
  OS paths; otherwise (WASM, sandbox and test hosts) it runs inline, with stdin read
  first when asked (within a bound), output collected and written after, and host calls
  that must answer at once.

Cancel is dropping the future. Each utility declares its options as data (`opts::Spec`),
parsed by one shared GNU-style parser; a test compares every spec with the classifier's
table for that utility, so the classifier and the implementation agree on which flags
write (D1082, D1090). Ported code (uutils calls `std::io::stdout()` and `std::fs`
directly) is rewritten onto this context; re-executing the psh binary per utility, as
brush does, fails in WASM and breaks cancellation (D018).

### Registry, modes and fall-through

psh-utils' `REGISTRY` lists each utility with its names, mode and whether it is in the
destructive group; `psh_utils::policy(sandbox)` turns it into the `UtilPolicy` an
embedder puts in `ShellConfig::utilities` (the `psh` binary and psh-spec natively,
psh-wasm with `sandbox: true`; a bare engine has none) (D1083). Each utility has one
mode (D015, D1172):

| Mode | Meaning | Used for |
| --- | --- | --- |
| **S, always** | Always the builtin | Where psh can be at least as good as GNU and BSD for agent use: nearly all, including `find`, `sort` and `timeout` |
| **F, PATH fallback** | The builtin only if no program of that name is on PATH, or in sandbox mode | Tools whose real versions are better, or whose exact output is the point: `awk` until its suites pass; later `tar`, `gzip`, `file` |
| **W, Windows-only shadow** | S on Windows, F elsewhere | Names that collide with incompatible System32 programs: `tree`, `expand`, `more`, `fc`, `hostname`, `whoami` |

Around the modes:

- An explicit path (`/usr/bin/grep`), a hashed path and `command -p` run the real
  program.
- **Fall-through** (D010, D1082): an option outside a utility's spec runs the real
  program of that name from PATH if there is one, else `NAME: unsupported option 'X'
  (psh built-in NAME)` and status 2. It never silently produces different output.
- **Off switches** per session: all utilities, the destructive group (`rm mv cp ln dd`),
  or single names; `force_names` (`PSH_UTILS_FORCE`) is the inverse, for tests of F and
  W utilities (D1192).
- **Reporting.** `type` says `NAME is a psh utility`, `type -t` says `builtin`,
  `command -v` prints the name: what runs, even when a program of the same name is on
  PATH (D1083, D1151). `which` prints the program's path first, for `$(which ls)`
  idioms, and `NAME: shell builtin` only when no program exists (D1200).
- `env`, `xargs`, `find -exec`, `timeout`, `nice`, `nohup` dispatch back into the
  shell, so utilities and the standalone builtins (`printf echo test [ true false pwd
  kill`) stay reachable from them. In wasm the utilities shadow JS host commands of the
  same name.
- A background job that is a utility has no OS pid (D1085).

### Dialect (D016)

- Accept the **union of GNU and BSD** flags where they don't conflict.
- Where they conflict, **GNU semantics**, except where agents overwhelmingly write the
  macOS form (`sed -i ''`: 5,307 BSD vs 144 GNU in the corpus; `stat -f FORMAT`), or
  where GNU's behavior is a trap for agents (D033). Conflicts are resolved by
  deterministic rules on the arguments, never by the host OS:

| Command | Rule |
| --- | --- |
| `sed -i` | `''` next, or text attached (`-i.bak`): a suffix. Otherwise, if the next argument parses as a script and operands follow: GNU form. A bare `.`-prefixed argument without command characters followed by a script: BSD suffix |
| `stat -f` | Argument containing `%`: BSD format. `-f` alone with file operands: GNU file-system mode. `-c` and BSD `-f`/`-t` both offered |
| `date -r` | All digits and not an existing file: BSD epoch seconds; otherwise GNU reference file |
| `base64 -i` | Before an option or at the end: GNU `--ignore-garbage`; else BSD's input file; `-o FILE`, `-D`, `-b N` are BSD's; output wraps at 76 (D1202) |
| `mktemp -t` | Argument containing `XXX`: GNU template; otherwise BSD prefix |
| `cp -R src/` | GNU (copies the directory), documented |
| `wc`, `uniq -c` | GNU column widths |
| `find` with no path | `.` (GNU) |
| `xargs` with no input | BSD: the command does not run; `-r` accepted (D1170) |
| `sort`, `comm`, `uniq` | Byte order in every locale, as `LC_ALL=C` (D1160) |
| `cut -c` | Characters, not bytes, in every locale (BSD's meaning, D1161) |
| `tr` | Bytes when both sets are ASCII (GNU); UTF-8 characters when a set has a non-ASCII character (D1162) |
| `head -n -N`, `tail -r` | Single-dialect forms, both accepted |
| `md5`/`shasum` vs `md5sum`/`sha256sum` | Output format keyed on the name; never unified (D1201) |
| `awk` | gawk's behavior where gawk and BSD awk differ; `for (k in a)` in insertion order (D1193) |

  `ls -G`/`-T` take the macOS meaning: BSD color, printed as none, and full times in the
  long format (D1152).
- Values in messages are quoted as GNU quotes them for the locale (`‘x’` in UTF-8,
  `'x'` otherwise), one helper for all utilities (D1163).
- `uname`, `hostname`, `id` and friends report the real host. psh never pretends to be
  Linux.

### Tiers (D017)

- **Tier 0**, shell builtins: part of the engine (`cd`, `export`, `read`, `mapfile`,
  `printf`, `test`, `kill`, `wait`, `trap`, ...), not up for debate.
- **Tier 1**, 62 names: the top corpus names that are not excluded, everything the
  agent read/edit/search loop needs, the wrappers (which must be builtins so they can
  wrap builtins and own process trees), and `fd`, which the owner moved up (D019):
  `head tail cat ls cp mv rm mkdir rmdir touch ln chmod pwd basename dirname realpath
  readlink mktemp stat du grep egrep fgrep rg fd sed awk sort uniq wc cut tr paste nl tee
  seq comm printf echo test true false sleep find xargs diff cmp jq base64 md5sum
  sha1sum sha256sum sha512sum shasum md5 date env printenv timeout nice nohup which`.
  `timeout` came first: it is the most common "command not found" (389) in the corpus.
  All are S, `awk` too (D1194).
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

Port mature permissive code rather than write new:

- `rg`, `fd`, `grep` on the ripgrep crates (next section); `sed` on psh-regex; `jq` on
  jaq (a dependency, D028); **`awk`** is a port of GoAWK (MIT), checked against the
  vendored onetrueawk and GoAWK suites (D1193), mode S (D1194).
- Most of the rest from uutils coreutils, findutils and diffutils, rewritten onto
  `UtilCtx`.
- Ours: the wrappers (`timeout nice nohup env xargs`, on psh's process model), `find`'s
  walk (D1171), `which`, the checksums (MD5, SHA-1, SHA-2, D1201), `xxd` (written from
  the man page; xxd itself is under vim's license), `tree` (GNU tree is GPL), `column`,
  `strings`.
- GPL implementations (GNU coreutils, sed, grep, gawk, tree, patch, busybox, moreutils)
  are never read (D007); BSD implementations are fine as references.

Crates for the date parser (`date -d`, `touch -d`, `find -newermt`, `fd --changed-*`),
tar/zip and process listing (`sysinfo`) are decided in the utility issue that needs them
(D028).

### Regular expressions: psh-regex and the regex crate

`psh-regex` (D1270) is GNU's POSIX regex semantics (ERE and BRE with GNU's extensions,
back-references, GNU's errors) in a linear-time engine where there are no
back-references, verified against glibc's `regcomp`/`regexec` (4,053 recorded results).
It is the regex of `[[ =~ ]]`, `sed`, `awk` and `find -regex`. The search utilities use
it as their parser (D1131): `psh_regex::to_rust_syntax` translates a POSIX pattern into
`regex-syntax` for the fast matcher (`grep-regex`), and says when that would be wrong:
a pattern with back-references, or an `ambiguous` one (an alternation, or more than one
variable-length part), where leftmost-first and leftmost-longest can choose different
spans. Those use `psh_regex::Regex` for spans; line selection is the same either way.
Non-ASCII bracket ranges compare code points (D1271). There is no PCRE: `grep -P` and
`rg -P` accept what the regex crate parses and fall through otherwise.

### Search: rg, fd, grep on the ripgrep crates (D020, D021)

`rg` and `fd` are embedded by owner requirement, on the upstream crates, the one
standing dependency exception (D006). Neither ripgrep nor fd has a library entry point,
and fd's `main` is unusable in-process (it changes the process cwd, installs a Ctrl-C
handler, needs threads, pulls clap and jiff). Source: `research/builtins.md` §5.

**Crates.** Exactly: `grep-matcher`, `grep-regex`, `grep-searcher`, `grep-printer`
(`default-features = false`, so no serde; patched in `third_party/` for a clock that
works on wasm, D1135), `ignore`, `globset`, `regex`, `regex-automata`, `regex-syntax`,
and what they pull in. Not `grep-cli` and not `grep-pcre2` (C code, breaks the WASM
build). `encoding_rs` arrives with `grep-searcher`.

**rg** (D1133) is ripgrep 15.2.0's `crates/core` (flags, main, search, haystack,
messages) and lexopt, copied with every change marked `psh:`, kept in ripgrep's
formatting so it stays a reviewable diff from upstream. Small stand-ins replace anyhow,
textwrap and grep-cli. The seams are patched: argument parsing, help and
`RIPGREP_CONFIG_PATH` through the utility's context; message and error state per run;
no `process::exit`, no logger; cwd, environment, TTY (headings, line numbers, color
from `TERM`/`NO_COLOR`) from `SyncCtx`. It runs as a blocking utility. Memory maps are
off. `--json` is grep-printer's JSON printer with serde replaced by deser-json, byte for
byte the real rg's output (D1136). The real rg runs for `--pre`, `-z`, `-P`, `--engine
auto` and `--debug`/`--trace`. The version reads `ripgrep
15.2.0 (psh)`; `--help` is upstream's.

**Walker seam (D1132).** `search::walk::Walker` runs one of two walks with identical
results, checked by a parity test over fixture trees (nested ignore files, negations,
parent-directory rules, hidden files, symlinks and loops, globs, types, limits, errors):
- *Native*, when the file system is the process's: `ignore::Walk` over absolute OS
  roots, `current_dir` the shell's cwd, paths mapped back to the user's spelling. Not
  native when the process's environment names another global gitignore than the
  shell's, or with explicit ignore files.
- *Virtual* otherwise (WASM, sandboxes): `ignore`'s traversal and ignore chain
  re-implemented over `WalkHost`, with `ignore`'s matchers; the chain is adapted from
  `ignore/src/dir.rs` (attributed).
- **Order** is depth first with each directory's entries sorted by name, in both, so
  output is deterministic. A multithreaded rg keeps the walk sequential on its own
  thread and searches files on scoped search threads, printing in walk order (at most
  4×threads ahead): the output equals the single-threaded output.

**fd** (D1138) is fd 10.5.1: its command line re-implemented on psh's option parser
(flags, aliases, overrides and conflicts of its clap definition, clap's wording for
errors), its filters, `fmt/`, `exec/`, output, patterns and receiver rules copied with
attribution, and what it uses of lscolors and nu-ansi-term adapted; none of fd's other
dependencies. The walk is the walker seam in a blocking utility, with the base
directory as the walk's cwd (`--base-directory` never changes the process cwd).
Results print in path order (fd's own order when its search ends within its 100 ms
buffering). `-x`/`-X`/`-l` collect the results; the async front end then runs the
commands through `spawn`, one after another (fd runs `-x` in parallel, in no order).
The real fd runs for `--changed-*` with a duration or date (until the shared date
parser; `@timestamp` is psh's), `--owner` with names and `--hyperlink`.

**grep** (`grep`, `egrep`, `fgrep`; D1137) is GNU grep 3.11's command line and output,
written for psh, on the same matcher, `grep-searcher` and walker as rg, with its own
front end and a GNU `Sink`: POSIX patterns through psh-regex (above; back-references and
leftmost-longest spans by `psh_regex` itself, GNU's ERE extensions as a pre-pass),
`file:line:text`, `--` between groups, GREP_COLORS, `grep: FILE: binary file matches`
on stderr, GNU recursion (`-r` without a path means `.`, hidden files searched, no ignore
files, `-R` follows symlinks, `--include`/`--exclude` by GNU's suffix rules). It runs as
a blocking utility; `-P` patterns the regex crate rejects and options psh lacks (`-T`)
run the real grep.

**find** walks on its own, over the utility's asynchronous file system (D1171): its
actions (`-exec`, `-delete`, `-prune`, `-quit`) interleave with the walk, which the
synchronous walker seam cannot do on the inline backend. Depth first, each directory's
entries sorted by name. `-name`/`-path` go through `globset` in fnmatch mode, `-regex`
through psh-regex.

**Windows.** Default `/` path separators for rg and fd (an explicit flag overrides);
CRLF as a line end for `grep` and `rg` unless `--crlf`/`--no-crlf` is given (D030).
Color is ANSI or none, never the console API. `psh.exe` needs a `longPathAware`
manifest like ripgrep's.

**In pi-env** the utilities (and with them the regex stack) will be a cargo feature of
the daemon build, on by default (D031).

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

`psh -c CMD [ARGS]`, `psh SCRIPT [ARGS]`, a script on stdin (`Script::stdin`: read a
line at a time as it runs, D1414), with bash's options (`-e`,
`-o pipefail`, `-O extglob`, ...), parsed by `psh_engine::invocation`. Its own modes:
`psh --parse [--compact] [-O extglob] [-c CMD | FILE]` prints the AST as JSON with spans
(D1047); `psh --classify` prints the classification; `psh --utils [--json]` lists the
utilities and what each name runs here, asked from a shell configured as `psh -c` is
(D1341). Modes, flags and help are parsed
with the `argument` crate, while bash's invocation rules (`+o`, combined `-euc`) stay
with the invocation parser, which the conformance runner shares (D035): `argument` reads
the leading `--name` arguments that are not bash's, and the first other argument starts
bash's part (D1340). The binary installs the utility policy (with
`PSH_UTILS_FORCE`, D1192), drives the engine with `std_host::drive` (step; on `Blocked`
park until the waker fires), keeps its externals in its own process group (D1067),
catches the trappable signals on the engine thread and dies by a fatal one (D1903).

## Embedding into pi-env (Planned, 0010)

pi-env (pi-mono, `packages/env/daemon`) is a synchronous, threaded Rust daemon that
runs every `exec` as `bash -c` today (`research/pi-current.md`). psh plugs in at its
`exec` op:

- Advertised as a feature in `hello.features`; `exec { command, shell: "psh" }`
  selects it. `shellPath` still selects real bash.
- One thread per exec runs the driver loop: `step`; on `Blocked`, park on a
  `Mutex`+`Condvar` woken by the engine's waker or by the request's cancel. Cancel and
  timeout go through the `ControlHandle`; TERM, grace, KILL.
- Embedded, psh installs no signal handlers; every external leads its own session or
  job, and dropping the host kills what the run left (D1067), so a cancel reaches the
  whole tree and nothing outlives the call unless it was detached.
- Output stays combined stdout+stderr in arrival order, with exit status, and goes
  through the daemon's existing decoding, windowing and spill. Exit codes follow
  pi-env's contract (`128+n` for signals).
- A `parse` op returns the classification for UI summaries and replay (D036), even where execution
  still uses real bash.
- **Sessions (D026).** Each `exec` gets a fresh psh shell; nothing (cwd, variables,
  functions) persists across calls, which keeps Durable's replay model intact.
  Node-side execution (`NodeExecutionEnv` and friends) reaches psh through the daemon,
  not a separate binary or addon. The utilities are a daemon cargo feature, on by
  default (D031).

## WASM and the browser

`psh-wasm` (D1110) builds two modules for `wasm32-unknown-unknown`, without
wasm-bindgen, Asyncify or JSPI (`scripts/build-wasm`, `--sizes` prints their sizes):

- **`psh-parser.wasm`**: `psh_parse` and `psh_classify`, JSON out exactly as `psh
  --parse`/`--classify` print it, for hosts that only want to understand commands.
- **`psh.wasm`**: the same plus the shell, with the utilities in sandbox mode (every
  utility runs in-process; blocking utilities run inline, D1080).

The ABI is a plain `extern "C"` one: one import, `psh.host_call(kind, op, arg, ptr,
len)`, carries every host operation as a request with an id, and the JS side answers
with the export `psh_complete`, at once or later from its event loop; the waiting future
wakes the engine, which asks JS to step the shell again. Data crosses through an input
and an output buffer. `ABI_VERSION` 3 (D1114, D1115) covers invocation options, the
clock, creation modes and the D1150 file system changes. The module links a 16 MiB stack
(D1111).

`js/psh.mjs` is the glue plus a default host, `MemHost` (D1112): an in-memory file
system with devices and the usual system directories, unbounded pipes, timers, and
commands written in JS (`host.command(name, async ctx => status)`), found under
`/usr/bin/NAME`; a missing command is recorded in `host.missing`. Hosts with other
storage (OPFS, a remote pi-env) implement the same methods. In the browser there is no
git, python or node, so this mode is for exploration and editing unless the host
supplies such commands. psh-spec runs every case in `psh.wasm` under node (D1113).

## psh-classify: understanding commands

The classifier runs on the same AST the engine executes (D013), so a command is never
parsed one way for a report and another way for execution.

- For each simple command: the command name (literal or dynamic), argv words with spans
  (literal or which expansions they contain), redirections with targets, its category
  and effects (paths read and written, output truncated by `| head`).
- **Nested scripts** (D1093): literal shell code in strings (`bash -c`, `eval`, `trap`
  handlers, alias bodies, backquotes, a here-document fed to a shell) is parsed and
  walked like the script; wrapped commands (`env`, `timeout`, `xargs`, `find -exec`,
  `sudo`, ...) are commands of their own with the wrapper as parent.
- **Categories** (D1091): read, network, build, long-running, mutate, vcs, install,
  unknown; the primary one is the highest in that order. `read_only()` holds when the set
  is just {read}, nothing is written and the script parsed.
- **Conservative** (D1092): dynamic command names, `eval`/`bash -c` of non-literals,
  `source`, inline interpreters, local and unknown programs, functions and aliases
  defined anywhere in the script, assignments to variables that change what runs
  (`PATH`, `LD_*`, `GIT_SSH_COMMAND`, ...), a dynamic word that could be an option of a
  utility with an option that writes or runs code (`sed "$x"`, `rg "$x"`) are
  `unknown`, never read-only. Literal variables are tracked where every write is the
  same literal and the assignment has definitely run (D1240); `$HOME`/`$PWD` paths are
  not options (D1242); `sqlite3` is classified from its literal SQL (D1241).
- Per-utility knowledge (which options take a value, what the operands are, which
  options write or run code) lives in `psh_classify::utils` (D1090): no_std data that
  the utilities depend on and check their own option parsing against.

## Open questions

1. Whether the WASM host (and a WASI build) matters before pi-env integration, and
   which storage a browser host uses.
2. ~~Where the exec-time approval hook sits~~: dropped (D036). Classification is for
   reporting only; there is no permission policy layer.
