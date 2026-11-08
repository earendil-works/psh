# Processes on Unix

On macOS, Linux and Android, psh starts real programs, waits for them, connects them
with pipes, signals them and cleans up after them. This page explains how it does that
without blocking the thread the shell runs on, how a cancel reaches every process a
command started, what survives a run, and where psh differs from bash on purpose.

> **In short.** Every program psh starts is spawned with clean signal dispositions and,
> when psh is embedded, in a session of its own, so a cancel can kill its whole tree.
> Nothing the shell waits for blocks its thread: exits are watched by small helper
> threads, pipes and FIFOs by one readiness thread, and two programs in a pipeline share
> a real pipe. When a run ends, what it left running is killed unless it was started with
> `nohup`, `setsid` or `disown`.

Everything here serves one goal above all: [control of execution](goals.md#control-of-execution).
An agent's command may hang, start a server, or fork a dozen helpers; the agent host
must be able to stop it and know that nothing is left behind.

## The shell never waits in the kernel

The engine runs every part of a script on one thread: pipeline stages, subshells and
background jobs are virtual processes inside it, not forks. If that thread blocked in
`read(2)` on a pipe whose writer is another virtual process of the same shell, the shell
would deadlock. So psh never makes a call that can wait on something the shell itself
has to do first:

- **Exits.** A small pool of helper threads waits for the programs psh started, one per
  running program, and wakes the shell when one ends. The wait leaves the exited
  process in place (a zombie) until the shell has collected its status, so a signal
  sent to its process group can never reach an unrelated process that reused its id.
- **Pipes, FIFOs, terminals.** One thread watches every descriptor the shell is waiting
  on (with `select(2)` on macOS, where `poll(2)` does not report a FIFO whose writer
  closed, and `poll(2)` elsewhere) and wakes the shell when it is ready. The shell only
  reads or writes once a descriptor is ready, and writes to a shared pipe at most
  `PIPE_BUF` bytes at a time, which a ready pipe always accepts.
- **Descriptors are never made non-blocking** when other processes share them: that flag
  belongs to the open file, and switching it would change how the programs sharing it
  behave. Only pipe ends psh alone holds are non-blocking.
- **FIFO opens**, which wait until the other end is opened, run on a helper thread.

> **Trade-off.** A readiness thread over a thread per pipe. One thread serves every pipe
> of the process, and a read never takes more bytes than the reader asked for, so a
> program that reads the shell's input after a `read` gets the rest of it. The cost: a
> write to a shared pipe goes in `PIPE_BUF` pieces with a readiness check in front of
> each.

## Pipes between programs

`seq 1 1000000 | grep 7 | wc -l` connects the three programs with real kernel pipes:
no byte passes through psh. A pipe between two of the shell's own commands is an
in-memory buffer instead, and a pipe becomes a kernel pipe the moment a program is
started on one of its ends while it holds no buffered bytes. From then on the shell's own
commands write to and read from the kernel pipe too, so the order of the bytes is kept:

```sh
{ echo a; seq 2 3; echo b; } | cat
```

prints `a 2 3 b` in that order, whichever command started first. A here-document is an
in-memory buffer that is fed to the program that reads it, so a body larger than a pipe
holds never makes psh wait before the program has started.

A program reading from a pipe whose writers are all gone sees end of file; one writing
into a pipe whose readers are gone gets `SIGPIPE`, which psh resets to its default for
every program it starts (Rust programs, psh included, ignore it). `yes | head -1` ends.

A pipeline stage that is a single program hands its descriptors to it, as bash does when
it runs the last command of a subshell in place: the stage keeps no pipe end open, so
`producer | head -1` stops the producer as soon as `head` exits.

## Where programs run, and how a cancel reaches them

| Situation | Where each program runs | What a cancel or a timeout reaches |
| --- | --- | --- |
| psh embedded in an agent host | a new session (and process group) per program | each program's whole tree |
| the `psh` command | psh's own process group, as with bash | everything in the group, when the caller signals it |

Embedded, psh gives every program a session of its own, so a program cannot touch a
terminal and a cancel can signal everything it started, grandchildren included. A
cancel ends the run at once: each running program gets `SIGTERM` (and `SIGCONT`, in case
it was stopped), and `SIGKILL` when it has not exited after a grace period (two seconds
by default) or as soon as it exits, for whatever it left in its group. The host then
reaps it in the background; the shell does not wait for that.

The `psh` command keeps its programs in its own process group instead, as a
non-interactive bash does. Whoever runs psh can then stop the whole tree by signalling
psh's group, the way a terminal's Ctrl-C or `timeout` does.

## What survives a run

When an embedded run ends, psh kills what it left running: background jobs that are
still going, and processes a finished program left in its session (a server started
with `npm start &`, a forgotten `sleep`). The ways to keep a process alive are explicit:

```sh
nohup python -m http.server 8000 >server.log 2>&1 &   # leaves the run
setsid ./worker &                                       # leaves the run
./long-job & disown                                     # leaves the run
./long-job &                                            # killed when the run ends
```

An embedder can choose bash's behavior instead and leave every background job running.
The `psh` command does that: its background jobs outlive it, as with bash, in psh's
process group, where a caller's group kill still reaches them.

> **Trade-off.** Killing leftovers over leaving them. Agents start servers and forget
> them; on a machine an agent works on for days that means dozens of stray processes. The
> cost: a script that relied on a plain `&` job outliving the shell needs `nohup`,
> `setsid` or `disown`.

## Background jobs, `$!`, `wait` and `kill`

A job that is one simple command is its program: `$!` is the program's process id, so

```sh
./server & echo $! > server.pid
kill "$(cat server.pid)"
```

works as in bash, even from a later run. A job that is more than one command
(`{ build; test; } &`, a pipeline) runs inside the shell; its `$!` is an id of its own,
larger than any real process id, which `wait` and `kill` understand. `kill` of such a job
ends it at once, as killing bash's subshell would; the program it was running is left to
finish, as that subshell's child would be.

`wait` returns a job's status, also for a job that has already been waited for, and is
interrupted by a signal the script traps (status `128+n`). `jobs` lists jobs in bash's
columns, and background jobs ignore `SIGINT` and `SIGQUIT`, as bash's do without job
control. `fg`, `bg` and `set -m` (job control) are not part of psh.

## Signals

Embedded, psh installs no signal handlers at all: the host cancels a run through the
shell's control handle. The `psh` command catches the signals a script may trap (`HUP`,
`INT`, `TERM`, `USR1`, `USR2`, `ALRM` and a few more), and only on the thread that runs
the shell. The handler only marks the signal; at its next command the shell runs the
script's trap, ignores the signal, or ends with it, and psh then dies by the signal so
that its parent sees what bash would show. A program the shell waits for in the
foreground is abandoned (as above) when a signal ends the shell; with a trap set, the
shell waits for the program first, as bash does. `kill $$` and `kill 0` reach the
shell's traps in the same way.

Signals ignored when psh started stay ignored, in psh and in the programs it starts, as
bash keeps them. An embedded shell does not inherit its host's ignored signals: a daemon
started under `nohup` does not make every agent command ignore `SIGHUP`.

After a foreground program dies of a signal other than `SIGINT` and `SIGPIPE`, psh
reports it on standard error as bash does:

```text
Terminated                 sh -c 'kill -TERM $$'
psh: line 2: 41237 Killed                     sh -c 'kill -9 $$'
```

## Scripts without `#!`

A file that is executable but has no interpreter line is not something the kernel can
start. bash runs such a file itself, and so does psh: in a fresh subshell that sees only
the exported variables, with the file name as `$0` and the arguments as `$@`. A file
whose first line contains a NUL byte is refused as a binary, with bash's message.

## `bash` and `sh` in a command

Agents' commands call a shell again: `bash -c '…'`, `bash -n script.sh`,
`xargs -I{} sh -c '…' _ {}`, `find … -exec bash -c '…' _ {} \;`. A command named `bash`
or `sh`, without a slash, runs in psh itself, so it gets psh's utilities, cancellation
and bash 5.3's behavior instead of whatever `bash` the system has (bash 3.2 on macOS,
none on Windows). It is a new shell, as a new bash process is: only exported variables
reach it, `exit` and `set -e` end only it, and its options and traps stay inside it.
`sh` runs with bash's rules too, since psh has no POSIX mode.

```sh
x=plain; export y=exported
bash -c 'echo "x=${x-unset} y=$y $0 $1"' name arg   # x=unset y=exported name arg
```

It runs inside the shell but behaves as a process: `$!` of `bash -c '…' &` names it,
`kill`, `wait` and `timeout` reach it, and a signal sent to it runs its trap or ends it,
as with a program; `timeout` and a cancel stop it and what it is running. `$PPID` in it
is the caller's `$$`.

What it does not have is a process id. A script that reads `$$` or `BASHPID` (psh looks
before it runs one, into `$(…)`, here-documents and literal `bash -c '…'` inside it)
runs under the real program `PATH` finds instead, never under psh itself, so pid files,
`kill -0 $$` and `/proc/$$` keep working:

```sh
bash -c 'echo $$ > app.pid; exec ./server'   # the real bash: app.pid names a process
```

Without a real `bash` (Windows, the browser), or when the use only shows at run time
(`eval`, `source`), reading `$$` or `BASHPID` there is an error that ends that shell, as
`${x:?}` does: `bash: line 1: $$: this shell runs inside psh and has no process id`.
`BASHPID` in a subshell of any shell is the same error; the shell's own `$$` and
`BASHPID` are real.

A script on its standard input (`curl … | sh`, `bash <<'EOF'`, `bash -s a b`) is read
one command at a time, as bash reads it, so its commands can read the lines that follow
them.

What still runs the program: a path (`/bin/bash`, `./bash`, `/opt/homebrew/bin/bash`),
`command -p bash`, a name bound with `hash -p`, an option psh lacks (`-i`, `--rcfile`,
`--posix`), and scripts that name their interpreter (`#!/bin/bash`), which the kernel
starts as before. The `psh` command sends `bash` and `sh` to `PATH` again when
`PSH_NESTED_OFF` is set; an embedder does so through its configuration.

## Where to look

- [crates/psh-os/src/unix/spawn.rs](../../crates/psh-os/src/unix/spawn.rs): spawning,
  waiting, abandonment, leftovers.
- [crates/psh-os/src/unix/ready.rs](../../crates/psh-os/src/unix/ready.rs) and
  [crates/psh-os/src/unix/file.rs](../../crates/psh-os/src/unix/file.rs): the readiness
  thread and files that never block the shell.
- [crates/psh-os/src/unix/signals.rs](../../crates/psh-os/src/unix/signals.rs): the
  `psh` command's signal handling.
- [crates/psh-engine/src/pipe.rs](../../crates/psh-engine/src/pipe.rs): in-memory pipes
  that become kernel pipes.
- [crates/psh-engine/src/jobs.rs](../../crates/psh-engine/src/jobs.rs) and
  [crates/psh-engine/src/builtins/jobs.rs](../../crates/psh-engine/src/builtins/jobs.rs):
  jobs, `$!`, `wait`, `kill`, `jobs`, `disown`.
- [tests/process/](../../tests/process/): the process tests, each with a check that
  nothing it started is left.
