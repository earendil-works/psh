# Cancellation, timeouts and background jobs

A command an agent runs can hang, start a server, or ignore the signal meant to stop it.
This page says how a psh run stops: when the embedder cancels it, when a signal reaches
the shell, when `timeout` runs out, and when the script simply ends with jobs still
running. It also covers the background jobs themselves and the exit statuses that tell
these cases apart. The operating-system mechanics are on [processes.md](processes.md)
(Unix) and [windows.md](windows.md); the API is on [embedding.md](embedding.md).

> **In short.** A cancel ends a run at once with status 143: every part of the script is
> dropped and every program it started gets `TERM`, then `KILL` to its whole tree after
> a grace period. When an embedded run ends, whatever it left running is killed unless
> it was detached with `nohup`, `setsid` or `disown`. Signals, `timeout`, `wait` and the
> statuses `128+n` work as in bash, with Linux's signal numbers on every platform.

All of this serves [control of execution](goals.md#control-of-execution): the agent host
must be able to stop a command and know that nothing it started is left behind.

## The ways a run stops

| What happens | Who does it | Traps run | `$?` of the run |
| --- | --- | --- | --- |
| the script ends or calls `exit` | the script | `EXIT` | its status |
| a cancel | the embedder | none | 143 |
| a signal to the shell (`kill $$`, an OS signal to `psh`) | the script, another process | its trap; `EXIT` if it ends the run | `128+n` unless trapped or ignored |
| `timeout` runs out (stops one command) | the script | the program's own | 124 for that command |

## Cancelling a run

An embedder cancels through the shell's control handle, from any thread. A cancel is a
forced `TERM`: no trap runs, every task of the run is dropped at the next point the
engine checks (before each command, and before it polls a task), and the run ends with
status 143. Dropping is what cleans up. Every file and pipe the run held is closed, and
every program still running is abandoned:

1. it gets `TERM` (and `CONT`, in case it was stopped), sent to its whole process group
   or, on Windows, its job object, so grandchildren get it too;
2. `KILL` follows when the grace period ends (two seconds by default) or as soon as the
   program itself exits, whichever comes first, for whatever it left behind;
3. the host reaps it in the background. The cancelled run does not wait for any of this.

The next run starts from the shell's state before the cancelled one, so a half-done `cd`
or assignment does not leak into it. Starting a run clears every earlier cancel and
pending signal: a cancel that arrives just after a run ended never stops the next one.

A **pause** stops the shell at its next check until the embedder resumes it. It stops
the shell only: programs it already started keep running.

> **Trade-off.** A cancel kills over a cancel that asks. A cancel could deliver `TERM`
> to the shell and let the script's trap decide, as a signal does (below). The embedder
> that cancels has decided already, and a trap that loops would make cancel unreliable.
> The cost: a script's cleanup trap does not run on cancel; an embedder that wants it
> sends the shell a signal instead and cancels only if the run does not end.

## Signals sent to the shell

A signal aimed at the shell (`kill $$`, `kill 0`, or an embedder's signal call) is taken
before the next command. A trap for it runs, an ignored one (`trap '' SIG`) does nothing,
and any other signal whose default is to end a process runs the `EXIT` trap and ends the
run with `128+n`. With a trap, a program running in the foreground finishes first, as in
bash; a signal that ends the shell abandons it as a cancel does. `wait` gives way to
both:

```text
$ psh -c 'trap "echo bye" EXIT TERM; kill $$; echo after'
bye
after
bye
```

The `psh` command also turns the operating system's signals into the same thing. It
catches the signals a script may trap whose default is to end a process (`HUP`, `INT`,
`TERM`, `USR1`, `USR2`, `ALRM` and a few rarer ones; not `QUIT`, which bash ignores),
runs the script's trap or ends the run, and then dies by the signal itself, so its parent
sees what it would see with bash. Probed on macOS, from bash:

```text
$ psh -c '/bin/sleep 5; echo after' & sleep 0.5; kill -TERM $!; wait $!; echo $?
143
$ psh -c 'trap "echo caught TERM" TERM; /bin/sleep 1; echo after' & sleep 0.3; kill $!; wait $!
caught TERM
after
```

A signal ignored when `psh` started stays ignored and cannot be trapped, as in bash. An
embedded shell installs no signal handlers and does not inherit its host's ignored
signals.

## When a run ends

When the script ends, its background jobs get one more turn. What happens to programs
still running then depends on how psh runs:

- **Embedded**, the end of a run kills what it left: background jobs still going, and
  processes a finished program left behind (a server started with `npm start &` inside a
  script, a forgotten `sleep`). Each gets `TERM`, then `KILL` after the grace period. Only
  programs started with `nohup` or `setsid`, and jobs released with `disown`, survive.
  The embedder can also kill the run's leftovers on demand, from any thread.
- **Exit grace.** An embedder can give background jobs time to finish: the run then
  waits up to that long for them before they are abandoned, so a script that ends with
  `upload &` does not lose a quick upload to the kill.
- **Kept, as bash does.** An embedder can choose to leave every background job running.
  The `psh` command does that, and keeps every program it starts in its own process
  group, as a non-interactive bash does: a harness that kills psh's process group, or a
  terminal's Ctrl-C, reaches everything psh started, its surviving jobs included.

The reasons, and what `nohup`, `setsid` and `disown` do to a process, are on
[processes.md](processes.md#what-survives-a-run).

## `timeout`

`timeout` runs a command with a time limit and has GNU's options and statuses, on every
platform (`gtimeout`, the name Homebrew gives GNU's, is the same command):

| Form | Meaning |
| --- | --- |
| `timeout 5 cmd`, `timeout 1.5s cmd`, `2m`, `1h`, `1d` | the limit, with fractions and suffixes; `0` means none |
| `-s SIG`, `--signal=SIG` | the signal to report (`TERM` by default; name or number) |
| `-k DUR`, `--kill-after=DUR` | ask for `KILL` after `TERM` |
| `--preserve-status` | end with the command's status, `128+SIG` when it timed out |
| `--foreground` | accepted; psh never gives a command a terminal of its own |
| `-v`, `--verbose` | say on standard error which signal is sent |

| Status | When |
| --- | --- |
| the command's | it finished in time |
| 124 | it timed out |
| 137 | it timed out and the signal was `KILL` |
| 125 | `timeout`'s own error, such as `timeout 1x true` |
| 126, 127 | the command could not be run or was not found |

```text
$ psh -c 'timeout 0.2 sleep 5; echo $?; timeout -s KILL 0.2 sleep 5; echo $?'
124
137
$ psh -c 'timeout --preserve-status 0.2 sleep 5; echo $?; timeout -v 0.1 sleep 3'
143
timeout: sending signal TERM to command 'sleep'
```

When the limit runs out, `timeout` stops the command the way a cancel does: an
in-process command (here `sleep`, which is built in) ends at once, and a program is
abandoned (`TERM`, `KILL` after the grace period). Embedded, that reaches the program's
whole tree. In the `psh` command, where programs share psh's process group, it reaches
the program itself; its own children are left to the caller's group kill.

> **Trade-off.** One way to stop a command over GNU's signal plumbing. `-s` and `-k`
> choose the status and announce the signal, but the stop is always the host's `TERM`
> and `KILL`, whose grace period the host sets (two seconds). A program that needs
> exactly `SIGINT` on timeout does not get it. In exchange, `timeout` stops a command
> exactly as a cancel does, on Unix and Windows, in-process commands included.

## Background jobs

`cmd &` starts a job, `$!` names it, and `jobs`, `wait`, `kill %n` and `disown` work as
in bash:

```text
$ psh -c '/bin/sleep 10 & echo $!; jobs; kill %1; wait; echo $?'
91055
[1]+  Running                    /bin/sleep 10 &
0
$ psh -c '(exit 3) & sleep 0.1 & wait -n; echo $?; false & wait $!; echo $?'
3
1
```

- **`$!`** is the program's process id when the job is one simple command that runs a
  program, so `kill $!`, `wait $!` and pid files mean the program, as in bash. Any other
  job (a pipeline, `{ ...; } &`) runs inside the shell and gets an id of its own, larger
  than any real process id; `wait`, `kill` and `jobs` understand it.
- **A job that is an in-process utility** (`sleep 10 &`, `grep -r x . &`) has no OS
  process either, so its `$!` is such an id (`sleep 10 &` gives `4294306`, as `sleep` is
  built in), and only the shell itself can signal it: `sh -c "kill $!"` fails with "No such process".
- **`wait`** returns the job's status, also for a job already waited for; `wait -n`
  returns the next one to finish (`-p VAR` names it); `wait` with no jobs or an unknown
  pid gives 127, as in bash. A trapped signal interrupts `wait` with `128+n`.
- **`kill %n`** signals a job; a job that runs inside the shell ends at once with
  `128+n`, and the program it was running is left to finish, as a killed bash subshell's
  child would be.
- **`disown`** removes a job from the table; its programs also leave the run and are not
  killed when it ends.

Background jobs ignore `INT` and `QUIT`, as bash's do without job control. Job control
itself (`fg`, `bg`, `set -m`, stopped jobs) is not part of psh.

## Exit statuses

| `$?` | Meaning |
| --- | --- |
| `128+n` | killed by signal `n`, in Linux's numbering |
| 124 | `timeout` ran out |
| 126 | found but not runnable (`Permission denied`, a directory, a binary it cannot run) |
| 127 | command not found; also `wait` for an unknown job |
| 130 | `INT` (Ctrl-C) |
| 137 | `KILL`: what an abandoned program that ignored `TERM` ends with |
| 141 | `PIPE`: the writer of `yes \| head -1` |
| 143 | `TERM`, and every cancelled run |

Signal numbers are Linux's on every platform, macOS and Windows included, so a script's
`128+n` checks and `kill -10` mean the same everywhere; psh translates to the system's
own numbers when it sends a signal. On macOS, where `USR1` is natively 30:

```text
$ psh -c 'kill -l | head -2; kill -l USR1; kill -l 143'
 1) SIGHUP	 2) SIGINT	 3) SIGQUIT	 4) SIGILL	 5) SIGTRAP
 6) SIGABRT	 7) SIGBUS	 8) SIGFPE	 9) SIGKILL	10) SIGUSR1
10
TERM
```

macOS's bash prints its own numbers there (`USR1` is 30, and `sh -c 'kill -USR1 $$'`
gives 158 where psh gives 138). A signal the system lacks, such as `STKFLT` on macOS,
cannot be sent to a program. On Windows, crashes and Ctrl-C map onto the same statuses
(see [windows.md](windows.md)).

After a foreground program dies of a signal other than `INT` and `PIPE`, psh prints the
line bash prints, with glibc's descriptions on every platform:

```text
$ psh -c 'sh -c "kill -TERM \$\$"; echo $?'
Terminated                 sh -c "kill -TERM \$\$"
143
$ psh -c 'sh -c "kill -9 \$\$"; echo $?'
psh: line 1: 57586 Killed                     sh -c "kill -9 \$\$"
137
```

`TERM` gets the short form, every other signal the long one with the line and the
process id. The line is not printed for a command inside `$(...)` or a pipeline stage;
`wait` prints it for a background job that died of a signal other than `INT`, `PIPE` and
`TERM`, as bash does.

## Where to look

- [crates/psh-engine/src/lib.rs](../../crates/psh-engine/src/lib.rs): the control
  handle (cancel, pause, signal), the end of a run and the exit grace.
- [crates/psh-engine/src/signals.rs](../../crates/psh-engine/src/signals.rs): signal
  names, numbers and descriptions.
- [crates/psh-engine/src/jobs.rs](../../crates/psh-engine/src/jobs.rs) and
  [crates/psh-engine/src/builtins/jobs.rs](../../crates/psh-engine/src/builtins/jobs.rs):
  jobs, `$!`, `wait`, `kill`, `jobs`, `disown`.
- [crates/psh-utils/src/timeout.rs](../../crates/psh-utils/src/timeout.rs): `timeout`.
- [crates/psh-os/src/unix/mod.rs](../../crates/psh-os/src/unix/mod.rs) and
  [crates/psh-os/src/unix/spawn.rs](../../crates/psh-os/src/unix/spawn.rs): the grace
  period, abandonment and the run's leftovers
  ([crates/psh-os/src/windows/spawn.rs](../../crates/psh-os/src/windows/spawn.rs) on
  Windows).
- [crates/psh-os/src/unix/signals.rs](../../crates/psh-os/src/unix/signals.rs) and
  [crates/psh/src/main.rs](../../crates/psh/src/main.rs): the `psh` command's signals and
  its process group.
- [tests/process/](../../tests/process/): cancel, signal and leftover tests.
