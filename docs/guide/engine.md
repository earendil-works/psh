# The engine: a stepped interpreter with virtual processes

When psh runs `cd src && grep -rn TODO . | head`, nothing forks and no thread of psh's
own waits for `grep`. This page explains who owns the thread a command runs on, how
subshells, pipelines and background jobs run side by side inside one engine, how a host
stops or signals a run, and what the engine asks of the host it runs on.

> **In short.** The engine is a tree walker written as Rust async functions, polled by
> a small scheduler of its own: the embedder calls `step` with a budget and parks its
> thread when the engine reports it is blocked. Subshells, pipeline stages, command
> substitutions and `&` jobs are virtual processes, tasks with their own copy of the
> shell's state that run concurrently and talk through in-memory pipes. Every side
> effect goes through a host interface, and a cancel drops every task, which releases
> everything the run held.

## Stepping a shell

An embedder makes a `Shell` from a `ShellConfig` (the host, the environment, the
working directory, descriptors 0 to 2, the nesting limit, the in-process utilities),
gives it a waker, starts a script and calls `step(budget)` until it is done:

| `Step` | Means | The embedder |
| --- | --- | --- |
| `Done(status)` | the script finished; reported once | reads the status |
| `Blocked` | every task waits on the host, or the shell is paused | parks until the waker fires |
| `Yield` | the budget is spent and there is more work | calls `step` again when it likes |
| `Idle` | no script is running | starts one |

The budget counts safepoints: one before every command and every loop iteration, so
`step(100)` returns after about a hundred commands even inside `while :; do :; done`.
The host's threads never run engine code; when an operation finishes there, they only
wake the waker. `std_host::drive` is the loop for an embedder with a thread to spare:
it parks on `Blocked` and returns the status. Two runs of one shell, the second
cancelled from another thread:

```rust
let mut cfg = ShellConfig::new(Rc::new(StdHost::new()));
cfg.env = vec![(b"PATH".to_vec(), b"/usr/bin:/bin".to_vec())];
cfg.stdout = Some(Rc::new(StdFile::stdio(1)?));
let mut shell = Shell::new(cfg);
shell.start(Script::command("x=hello; echo $x | tr a-z A-Z"));
let first = drive(&mut shell); // prints HELLO, returns 0

let control = shell.control();
std::thread::spawn(move || {
    std::thread::sleep(Duration::from_secs(1));
    control.cancel();
});
shell.start(Script::command("echo $x; sleep 30"));
let second = drive(&mut shell); // prints hello, returns 143 after a second
```

Variables, functions, the working directory and open descriptors persist from one run
to the next. A `Shell` stays on the thread that made it; its `ControlHandle` may be sent
anywhere.

> **Trade-off.** An engine the embedder steps over one that owns a thread or an async
> runtime. The host decides when psh runs and on which thread, a browser's event loop
> can drive it, and psh brings no runtime along: the [embeddable](goals.md#embeddable)
> goal. The cost: the engine must never block, so every wait goes through the host and
> comes back as a wakeup.

## A tree walker made of async functions

The interpreter walks the syntax tree (kept in arrays, see [The parser](syntax.md)) with
plain recursive code written as Rust `async fn`s. Every place where a command may wait
for the host, a pipe or another task is an `.await`, and the compiler builds the state
machine that stops there and resumes later. Waits happen in awkward places:
`"${x:=a}$(cmd)"` assigns `x`, then runs `cmd` and waits for it in the middle of one
word, and a compound command may wait anywhere in its body.

A scheduler of a few hundred lines polls these futures. Every task (a virtual process
or a helper) is a boxed future with a waker of its own; waking it queues its id and,
unless a step is running and will see the queue anyway, wakes the embedder. A step polls
only woken tasks, so a stage blocked on a full pipe costs nothing until its reader makes
room. The engine has no threads and no runtime crate: `std::task::Waker` is its whole
interface to the outside. In a release build, a script of 100,000 `true` commands runs
in about 0.2 seconds, start-up and parsing included.

> **Trade-off.** Async functions over hand-written frame machines. The compiler
> generates the states for every command and expansion, which by hand would be several
> times the code and easy to get wrong. The cost: nested commands use native stack (see
> below), and every engine path must cope with an operation that is not finished yet.

## Virtual processes

bash forks for a subshell, each pipeline stage, `$(…)`, `<(…)` and a `&` job. psh
runs each as a task that owns a copy of its parent's state: variables, functions,
options, traps, positional parameters, the working directory, the descriptor table and
`$?`. What the copy changes stays in it. A `&` job has an id of its own, larger than
any real process id, which `$!`, `wait` and `kill` understand; a subshell has no
process id, so `$BASHPID` in one is an error:

```sh
$ psh -c 'x=1; { x=2; echo "in stage: $x"; } | cat; echo "after: $x"'
in stage: 2
after: 1
```

The stages of a pipeline always run concurrently: with bounded pipes between them, a
stage that waited for the previous one to finish would deadlock on output larger than a
pipe holds. Since nothing forks, the same engine runs on Windows, which has no fork, and
in WebAssembly, which has no processes ([portable](goals.md#portable),
[sandboxable](goals.md#sandboxable)). How tasks meet real programs is on
[Processes on Unix](processes.md) and [psh on Windows](windows.md).

## Pipes inside the engine

A pipe between two virtual processes is an in-memory buffer of 64 KiB, as on Linux,
that counts its readers and writers. Reading an empty pipe waits while a writer is left,
writing a full one waits while a reader is left, and a writer whose readers are all gone
ends with status 141, as a process killed by `SIGPIPE` does:

```sh
$ psh -c 'yes | head -1; echo "${PIPESTATUS[@]}"'
y
141 0
```

A program started on such a pipe needs a real descriptor. If the buffer is empty at that
moment, the pipe becomes an operating system pipe of the host, so `a | b` of two
programs moves no byte through psh. Otherwise, or on a host without such pipes, the
engine pumps bytes between its buffer and a host pipe; the pumps belong to the wait for
the program and stop with it. A here-document or here-string is a pipe filled with the
expanded body and without a writer, so nothing is written into a kernel pipe before the
command that reads it runs. In redirections, `/dev/stdin`, `/dev/stdout`, `/dev/stderr`
and `/dev/fd/N` always mean the virtual process's own descriptors: an embedded shell's
descriptors are not its host process's, and Windows has no such files.

## Cancelling, pausing and signalling a run

A `ControlHandle` sets bits in one atomic word that the engine checks at every safepoint
and before it polls a task:

- **`cancel`** is a forced `TERM`: every task is dropped, no trap runs, and the run ends
  with 143. Dropping is what cleans up, because every resource a host hands out is
  released when dropped: a running program is abandoned (`TERM`, a grace period, `KILL`,
  reaped in the background) and every file and pipe is closed. The next run starts from
  the shell's state before the cancelled one.
- **`pause`** stops the shell at its next safepoint; `step` answers `Blocked` until
  `resume`. The programs it started keep running.
- **`signal(n)`** sends a signal to the shell, which takes it at its next safepoint: a
  trap runs, an ignored signal does nothing, any other runs the `EXIT` trap and ends the
  run with `128+n`. A foreground command finishes first; waits and blocking reads give
  way. `kill $$` goes the same way:

```sh
$ psh -c 'trap "echo got USR1" USR1; kill -USR1 $$; echo next'
got USR1
next
```

`start` clears the control word and pending signals, so a cancel that arrives just after
a run ended never reaches the next one. When a script ends, its background jobs get one
more round; then their programs are released to outlive the shell, as in bash, or, with
an exit grace in the config, waited for up to that long and then abandoned (see
[what survives a run](processes.md#what-survives-a-run)).

## Stack and the nesting limit

Futures nested inside one task are polled recursively, so a function call, `eval` or
`source` deepens the native stack: about 3 KiB per function call level in a release
build, 15 KiB in a debug build. Subshells, pipeline stages and substitutions are tasks
of their own and do not. The nesting limit (`ShellConfig::max_depth`, 1000 by default)
counts function calls and subshells, and fails with a message instead of overflowing:

```sh
$ psh -c 'f() { f; }; f; echo "status $?"'
psh: line 1: f: maximum function nesting level exceeded (1000)
```

A native embedder runs the engine on a thread with `std_host::ENGINE_STACK` (about
20 MiB, of which only touched pages cost memory); `std_host::engine_thread` makes one.
The WebAssembly module links a 16 MiB stack and caps the depth at 768.

## Finding a command

A name is looked up as functions, shell builtins, `bash` and `sh` (which run in psh
itself, below), psh's in-process utilities (only when the embedder passed them in; see
[Utilities that run in-process](utilities.md)), then `PATH`, one entry at a time,
through the host. A found program is remembered in a hash
table that belongs to the `PATH` value it was found under. A name with a slash, a hashed
path and `command -p` run the program directly.

```sh
$ psh -c 'type cd cat git'
cd is a shell builtin
cat is a psh utility
git is /usr/bin/git
```

When the host cannot start a file (on Unix, an executable without `#!`; on Windows,
`sh` and `bash` scripts and files with neither an interpreter line nor an executable
extension), it answers "run this in the shell", and the engine runs the file as bash
runs one the kernel rejects: in a fresh subshell that sees only exported variables, with
the file name as `$0`.

A command named `bash` or `sh`, without a slash, runs in psh as a new shell, also in an
engine without utilities and in the browser, where no bash exists. It starts afresh as a
new bash process would: its variables are built from the exported ones (`SHLVL` one
higher), with no functions, aliases, traps or options of the caller. It is a virtual
process with a task of its own: `$!` of `bash -c '…' &` names it, and `kill`, `wait`,
`timeout` and a cancel reach it as they reach a program; a signal sent to it runs its
trap. It has no process id: a script that reads `$$` or `BASHPID` runs under the real
program on `PATH` (never psh itself, `ShellConfig::own_program`), and without one
reading them is an error. `ShellConfig::nested_shells = false` leaves both names to
`PATH`.

## Errors that end the shell, and errors that abandon a line

An error that **ends the shell** (or the subshell it happens in) is `exit`, `${x?}`, a
write into a closed pipe, or a feature psh does not run. An error that **abandons the
line** skips the rest of the current top-level line, sets `$?` and goes on with the next
one, as bash does for a bad substitution, an arithmetic error, an assignment to a
read-only variable and the nesting limit above:

```sh
$ printf 'echo $((1/0)); echo same line\necho next line\n' > s; psh s
s: line 1: 1/0: division by 0 (error token is "0")
next line
$ psh -c 'set -o posix; echo after'; echo "status $?"
psh: line 1: set -o posix: not supported yet
status 2
```

> **Trade-off.** Stopping on an unsupported feature over running on. A script whose
> `set -e` or option was not understood must not continue as if it had been. The cost:
> some scripts that would happen to work by carrying on stop instead.

## The host interface

The engine does no I/O itself. Every side effect goes through one Rust trait, `Host`,
whose operations may finish later: file system calls return futures, open files have
`poll_read` and `poll_write`, a started program has `poll_wait`. Paths are bytes,
absolute and `/`-separated (a Windows drive stays in front: `C:/x`); the host converts
them at its boundary. Methods beyond the core have defaults that answer "unsupported" or
a neutral value, so a small host implements little:

| Group | What it covers |
| --- | --- |
| Core (required) | open, stat, access, read a directory, canonicalize, resolve a command, spawn, sleep, `$$` |
| Processes | operating system pipes, `kill`, `PATH` entries, the standard `PATH`, native paths for utilities that use the OS file system |
| File system changes | `mkdir`, remove, rename, links, modes and times, the rest of `stat` |
| Facts | user and group ids, host name, `OSTYPE` and friends, umask, clock and time zone, user and group names, home directories |

The Unix host (macOS, Linux, Android) and the Windows host run the `psh` program, with
process groups or job objects ([Processes on Unix](processes.md),
[psh on Windows](windows.md)). The std host uses the standard library's process API for
simple embedders and tests; it has no process groups and kills with `KILL` only. The
test host has an in-memory file system, fake commands written as futures and captured
output; in its latency mode every operation is pending once before it completes, which
proves no engine path expects an immediate answer, and it counts live processes, pipes
and files so tests can check that a cancel leaks nothing. The JavaScript host has an
in-memory file system, commands written in JavaScript and timers, and is driven from the
JavaScript event loop ([psh in the browser](wasm.md)).

## Where to look

- [crates/psh-engine/src/lib.rs](../../crates/psh-engine/src/lib.rs): `Shell`,
  `ShellConfig`, `Step`, `ControlHandle`;
  [crates/psh-engine/src/sched.rs](../../crates/psh-engine/src/sched.rs): tasks,
  wakers, the budget and safepoints.
- [crates/psh-engine/src/exec.rs](../../crates/psh-engine/src/exec.rs): lists,
  pipelines, compound and simple commands, command lookup.
- [crates/psh-engine/src/pipe.rs](../../crates/psh-engine/src/pipe.rs): pipes between
  virtual processes and their switch to operating system pipes.
- [crates/psh-engine/src/host.rs](../../crates/psh-engine/src/host.rs): the `Host`
  trait; [crates/psh-engine/src/std_host.rs](../../crates/psh-engine/src/std_host.rs):
  the std host, `drive` and `engine_thread`;
  [crates/psh-engine/src/testhost.rs](../../crates/psh-engine/src/testhost.rs): the
  test host.
- [crates/psh/src/main.rs](../../crates/psh/src/main.rs): the `psh` program driving the
  engine on a thread of its own.
