# psh on Windows

psh runs natively on Windows, on x64 and on arm64. It does not need Git Bash, MSYS,
Cygwin or WSL. A script an agent writes for bash runs under `psh.exe` and starts native
Windows programs directly. This page explains what that looks like: which paths psh
accepts, how it finds commands, how arguments reach programs and batch files, how psh
starts, waits for and stops programs, what exit statuses and Ctrl-C mean, and what has
no Windows counterpart.

> **In short.** psh speaks bash on the outside and Windows on the inside. It accepts
> `C:/x`, `C:\x` and Git Bash's `/c/x`, finds commands through `PATH` and `PATHEXT` as
> cmd would, and quotes arguments so that programs and batch files get exactly what the
> script wrote. Every program runs in a Windows job object of its own, so stopping a
> command stops everything it started, and if psh dies its programs die with it.

Behaving the same on every system is one of psh's goals ([portable](goals.md#portable)),
and Windows is where it matters most: an agent's shell commands are written for bash, and
the usual way to run them on Windows is a second, emulated Unix layer with its own paths
and its own copies of the tools. psh is a native program instead. The Unix side of the
same machinery is on [Processes on Unix](processes.md).

## Paths

psh keeps paths with forward slashes and turns them into Windows paths only when it
talks to Windows. All of these name the same directory:

```sh
cd C:/Users/me/project
cd 'C:\Users\me\project'
cd /c/Users/me/project
```

A few paths that every bash script assumes are mapped:

| Path in the script | What Windows opens |
| --- | --- |
| `/dev/null` | `NUL` |
| `/tmp`, `/tmp/x` | the user's temp directory (`%TEMP%`, with long names) |
| `/c/x` | `C:\x` |
| `/dev/zero`, `/dev/urandom` | emulated: zeros, and random bytes from the system |

The shell shows a path as it was written: after `cd /tmp`, `pwd` prints `/tmp` and
`pwd -P` the temp directory (`C:/Users/me/AppData/Local/Temp`). `cd C:/Windows` and
`cd 'C:\Windows'` both leave `pwd` at `C:/Windows`, and `..` stops at the drive.

> **Not built yet.** Network paths (`//server/share`) are not accepted by `cd`.

The standalone `psh` also sets the variables scripts expect, when they are not set:
`HOME` is `%USERPROFILE%` and `TMPDIR` is the temp directory, both with forward
slashes. The variable Windows calls `Path` is `PATH` in the shell: Windows ignores the
case of environment names, bash does not, and scripts write `PATH`. `OSTYPE` is `msys`
and `MACHTYPE` `x86_64-pc-msys` (or `aarch64-pc-msys`), as Git Bash reports them, so a
script's `case $OSTYPE` takes its Windows branch; `UID` is 1000.

## Finding commands

A command name is looked up in `PATH`. The entries are separated by `;`, as Windows
writes them, and also by `:`, as bash scripts write them
(`PATH="$HOME/bin:$PATH"`); a colon right after a drive letter (`C:/x`) belongs to the
drive. The same splitting serves every lookup: running a command, `type`, `command -v`,
`hash` and `source`. In each directory psh tries the name with each extension in
`PATHEXT`, in its order (`.COM`, `.EXE`, `.BAT`, `.CMD`, ...), the way cmd does, so
`type -P cmd` prints the `cmd.exe` in System32 (`C:/WINDOWS/system32/cmd.exe`).

A name that already has one of those extensions is taken as it is (`npm.cmd`). A file
without an extension is a command only if it starts with `#!`. That matters for the
directories package managers fill: next to `tsc.cmd`, npm puts a `tsc` script for sh,
and `tsc` runs `tsc.cmd`, as it would from cmd.

Unlike cmd, psh never looks in the current directory for a bare name. `tool` runs a
`tool.exe` on `PATH`; a `tool.exe` in the current directory runs as `./tool`, as in bash.
The app aliases Windows puts in `WindowsApps` (`winget`, `python` from the Store) are found
like any other program. `command -p` searches Windows' own default path (System32, the
Windows directory, WBEM and Windows PowerShell), whatever `PATH` says.

## Arguments, batch files and scripts

Windows passes a program one command line, and the program splits it into arguments
itself. psh builds that line with the quoting that the Microsoft C runtime (and Rust,
Python, Node and most other programs) parse back into exactly the arguments the script
had: `'a b'`, `''`, a quote or a trailing backslash arrive as they were written.

`.cmd` and `.bat` files run through `cmd.exe`, which has its own, much looser rules.
Quoting for it naively lets an argument run commands: `"&calc"` would start a calculator.
psh quotes every argument for cmd so that it arrives as one argument, guards `%` so that
`%PATH%` stays text, and refuses arguments that cmd cannot receive intact (line breaks)
with an error instead of passing them cut. An npm-style shim, a `.cmd` file that runs a
program with `%*`, hands the program what the script wrote; the Windows process tests
check this one:

```sh
shim 'a b' 'x&y' '100%' 'q"uote'   # the program gets: a b | x&y | 100% | q"uote
```

A file that starts with `#!` runs under its interpreter, looked up on `PATH`:
`#!/usr/bin/env node`, `#!/usr/bin/node` and `#!/bin/node` all run `node` (`node.exe`
or a `node.cmd` shim), with the script's path and its arguments. An interpreter written
as a Windows path that exists is used as it is, and `python3`, which Windows' Python
does not install, falls back to `python`.

Scripts for `sh` or `bash` (`#!/bin/sh`, `#!/usr/bin/env bash`), and files with neither
a `#!` line nor a program extension, run in psh itself: in a fresh subshell that sees
only the exported variables, with the script's name as `$0` and its arguments, as bash
runs a file the system cannot start. Windows has no `/bin/sh`, and psh is the shell
anyway. A command that names `sh` or `bash` (`sh -c '...'`) is different: it runs
whatever `PATH` finds, such as Git's `bash.exe`, or fails with 127.

## How programs run

Every program psh starts goes through the same steps, so that nothing it does can
escape psh's control ([control of execution](goals.md#control-of-execution)):

- **A job object of its own.** The process is created suspended, put into a new job
  object, and only then resumed, so even its first instruction runs inside the job.
  Breaking away from the job is not allowed: everything the program starts, and
  everything those start, stays in it.
- **Killed when psh dies.** The jobs of a run are marked so that Windows ends their
  processes when psh's last handle to them closes. If psh crashes or is killed from
  the task manager, what it started dies with it. Unix has no counterpart to this.
  Programs started with `nohup` or `setsid`, and jobs a script `disown`s, are exempt.
- **Only standard handles are inherited.** A program receives exactly its standard
  input, output and error, nothing else psh has open. Two programs started at the
  same moment from different threads cannot catch each other's pipe ends.
- **No stray windows.** Embedded, each program leads a console process group of its
  own, so psh can send it a Ctrl-Break without touching anything else. When psh has no
  console (a daemon), programs start without a console window: nothing flashes on the
  screen.
- **Waiting without blocking.** A small helper thread waits on each program's process
  handle and wakes the shell when it ends. Holding the handle also keeps Windows from
  reusing the process id while the shell may still signal it.

`$!` is the program's process id, and `kill`, `wait` and `jobs` work as on Unix.
`kill 0` reaches every program of the run.

> **Trade-off.** A job object per program over one for the whole run. Stopping one
> command (a `timeout`, a `kill %2`) ends that command's tree and nothing else. The
> cost: a kernel object per program, and a short check of every job when a run ends.

## Exit statuses, Ctrl-C and stopping commands

A program's exit code is `$?`. A crash, which on Windows is an exit code too, shows as
the signal a Unix crash would give, so scripts that test for it work the same:

| How the program ended | `$?` |
| --- | --- |
| access violation, stack overflow, in-page error | 139 (SEGV) |
| illegal instruction | 132 (ILL) |
| integer or floating point fault | 136 (FPE) |
| `abort()`, a Rust panic with `panic=abort`, a failed assertion | 134 (ABRT) |
| killed by psh | 137 (KILL) |
| ended by Ctrl-C, or by psh's Ctrl-Break for a TERM | 130 (INT) or 143 (TERM) |

Every other code is an exit status, cut to 0-255 as bash does.

When psh stops a command (a cancelled run, a timeout, `kill`), it sends TERM, INT, HUP
or QUIT as a Ctrl-Break to the program's console group, which console programs treat
as a request to quit. When the program exits, or after a grace period of two seconds,
psh ends its whole job: grandchildren included, nothing is left running. Where no
Ctrl-Break can be delivered (no console shared with psh, or a program in psh's own
console group), the job ends at once. KILL always ends the job at once. Other signals
(`USR1`, `ALRM`, ...) cannot be delivered on Windows, and `kill` reports an error. A
process that is not one of the run's is ended with `TerminateProcess`.

The standalone `psh` keeps its programs in its own console group, as cmd does, so
Ctrl-C in a console reaches the program in the foreground directly, and psh too: psh
cancels the rest of the script and exits with 130, or runs a `trap` the script set.
Ctrl-Break is TERM (143); closing the console window, logging off and shutting down
are HUP. An embedded psh installs no console handler at all; its host cancels runs
through the shell's control handle (see [Cancellation, timeouts and background
jobs](jobs.md)).

## What survives a run

| How it was started | When the run ends | When psh dies |
| --- | --- | --- |
| a plain `&` job, embedded | killed: Ctrl-Break, then the job ends | killed |
| a plain `&` job, standalone `psh` | keeps running, as with bash | killed |
| `nohup`, `setsid` | keeps running | keeps running |
| `cmd & disown` | keeps running | keeps running |

An embedder chooses whether a run's leftovers are killed (the default) or kept. The
standalone `psh` keeps them: before it exits, it lifts the kill-on-close mark from the
jobs it leaves behind.

## Pipes

`dir-tool | sort | other-tool` connects programs with real Windows pipes. Where the
shell itself reads or writes a program's end (`$(…)`, a here-document, a pipe into a
builtin or an in-process utility), psh never lets its own thread block in the kernel,
since the other end may be part of the same shell:

- **Private pipes** psh creates for one program are named pipes, since only those
  support overlapped I/O. A helper thread owns psh's end and moves bytes between it and
  a 64 KiB buffer; when the shell drops its end, a reader thread stops at once and a
  writer first hands over what it holds, so the program sees end of file and no byte
  is lost.
- **Shared pipes** (psh's own standard input when it is a pipe, pipes other processes
  hold too) are never read ahead: a thread blocked in `ReadFile` would take bytes meant
  for a program that reads after the shell (`read line; cat`). The shell reads only what
  `PeekNamedPipe` shows is there; one poller thread watches such pipes, every
  millisecond at first and backing off to 16 ms, and wakes the shell. Writes to them
  are synchronous, since their reader is another process.
- Regular files, `NUL` and consoles are read and written synchronously.

> **Trade-off.** Peeking over a reader thread for shared pipes. A program that reads
> stdin after a `read` gets the rest of it, as on Unix. The cost: Windows' anonymous
> pipes have no readiness event, so waiting on one is polling, with up to 16 ms of delay
> on an idle pipe.

## Text and encodings

psh works in UTF-8. Scripts with Windows line endings (`\r\n`) run as they are.
`grep` and `rg` treat `\r\n` as a line end on Windows, so `grep 'x$'` finds lines that
end in `x\r\n`; `--binary` (grep) and `--no-crlf` (rg) turn that off. `rg` and `fd`
print paths with `/` unless told otherwise (`--path-separator`).

> **Not built yet.** `$(…)` (with backquotes, `$(< file)` and `${ …; }`) and `read` drop
> the `\r` of a `\r\n` line end on a host that says its programs end lines that way. The
> Windows host does not say so yet, so today `x=$(cmd /c echo hello)` keeps a trailing
> `\r`.

Programs that print in the system's ANSI code page instead of UTF-8 (older tools, some of
the System32 ones) can be decoded where psh reads their output, for `$(…)` and pipes into
shell code: output that is not valid UTF-8 is converted from the ANSI code page. An
embedder turns this on; it is off by default, since most programs an agent runs print
UTF-8 and a guess can garble them. Output that goes from a program straight to a file,
another program or the console is never touched.

## What has no Windows counterpart

| On Unix | On Windows |
| --- | --- |
| descriptors above 2 for a program (`exec 3>f; cmd`) | not passed: Windows programs only receive standard input, output and error |
| `umask` | always `022`; nothing applies it |
| file modes (`chmod`, a creation mode) | only "writable or not": the read-only attribute |
| `chmod +x`, `test -x` | execute bits come from the file: directories, `PATHEXT` extensions and `#!` scripts; `chmod +x` changes nothing |
| `ln -s` | needs Developer Mode or administrator rights; without them a link to a directory is a junction and a link to a file fails |
| file names with `* ? \| < > " \ :` or a trailing dot | refused by Windows |
| signals other than INT, TERM, HUP, QUIT and KILL | cannot be sent |
| `/bin/sh`, `/etc`, `/usr` | none; psh is the shell |

The utilities psh runs in-process (`ls`, `cat`, `grep`, `sed`, `awk`, `find`, `xargs`,
`sort`, `diff`, `jq`, `rg`, `fd`, `date`, `stat`, `mktemp`, `timeout` and the others on
[Utilities that run in-process](utilities.md)) exist on Windows without any extra
install; `psh --utils` lists them. `find`, `sort` and `timeout`, names Windows uses for
different programs, are psh's own on every platform, so `find . -name x` does what the
script means. Programs psh does not provide (`uname`, `od`, `make`, `python3`, ...) run
if `PATH` has them, and are not found (127) otherwise.

> **Not built yet.** psh's own versions of the other names Windows uses for different
> programs (`tree`, `more`, `hostname`, `whoami`). For now these run Windows' programs.

## How well it works

psh's tests run on Windows 11 (arm64, and x64 under emulation) before a change to its
code lands, against the same expected output as on the other platforms, recorded with
bash on Linux. There, psh passes 2,132 of the Oils spec cases (85%), 2,579 of brush's (81%), 357 of 433
utility cases and 283 of 701 agent-style commands; most of the agent cases that fail need
a program Windows lacks. Every other Windows difference is listed with its reason. After
each case, the test runner checks the case's jobs: a process still alive after the
grace period is a leak, and the case fails. The numbers are on [Status in
numbers](status.md), the method on [How psh is tested](testing.md).

## Where to look

- The Windows host: [crates/psh-os/src/windows/mod.rs](../../crates/psh-os/src/windows/mod.rs)
- Starting, waiting for and stopping programs, job objects, exit codes:
  [crates/psh-os/src/windows/spawn.rs](../../crates/psh-os/src/windows/spawn.rs)
- Argument and batch file quoting:
  [crates/psh-os/src/windows/cmdline.rs](../../crates/psh-os/src/windows/cmdline.rs)
- Paths and `PATH`: [crates/psh-os/src/windows/path.rs](../../crates/psh-os/src/windows/path.rs)
- Pipes and files: [crates/psh-os/src/windows/file.rs](../../crates/psh-os/src/windows/file.rs)
  and the poller for shared pipes,
  [crates/psh-os/src/windows/poller.rs](../../crates/psh-os/src/windows/poller.rs)
- File system changes: [crates/psh-os/src/windows/fs.rs](../../crates/psh-os/src/windows/fs.rs)
- Ctrl-C and Ctrl-Break: [crates/psh-os/src/windows/signals.rs](../../crates/psh-os/src/windows/signals.rs)
- The Windows process tests: [tests/process/windows.rs](../../tests/process/windows.rs)
  and [crates/psh-os/src/windows/tests.rs](../../crates/psh-os/src/windows/tests.rs)
- The Windows differences of each test layer:
  [tests/agent/expected-failures.windows.txt](../../tests/agent/expected-failures.windows.txt)
  and its siblings in `tests/oils/`, `tests/brush/` and `tests/utils/`
