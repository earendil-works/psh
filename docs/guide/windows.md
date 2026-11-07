# psh on Windows

psh runs natively on Windows, on x64 and on arm64. It does not need Git Bash, MSYS,
Cygwin or WSL. A script an agent writes for bash runs under `psh.exe` and starts native
Windows programs directly. This page explains what that looks like: which paths psh
accepts, how it finds commands, how arguments reach programs and batch files, what exit
statuses and Ctrl-C mean, and what has no Windows counterpart.

> **In short.** psh speaks bash on the outside and Windows on the inside. It accepts
> `C:/x`, `C:\x` and Git Bash's `/c/x`, maps `/dev/null` and `/tmp`, and finds commands
> through `PATH` and `PATHEXT` as cmd would. It quotes arguments the way Windows programs
> parse them, runs `.cmd` and `.bat` files through cmd.exe with quoting that cannot be
> turned into extra commands, and kills a command together with everything it started.

Behaving the same on every system is one of psh's goals ([portable](goals.md#portable)),
and Windows is where it matters most: an agent's shell commands are written for bash, and
the usual way to run them on Windows is a second, emulated Unix layer with its own paths
and its own copies of the tools. psh is a native program instead.

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
| `/tmp`, `/tmp/x` | the user's temp directory (`%TEMP%`) |
| `/c/x` | `C:\x` |

The shell shows the path as it was written, and `pwd -P` shows where it really is:

```text
$ cd /tmp && pwd && pwd -P
/tmp
C:/Users/me/AppData/Local/Temp
```

`cd C:/Windows` and `cd 'C:\Windows'` both leave `pwd` at `C:/Windows`, and `..` stops
at the drive.

> **Not built yet.** Network paths (`//server/share`) are not accepted by `cd`.

The standalone `psh` also sets the variables scripts expect, when they are not set:
`HOME` is `%USERPROFILE%` and `TMPDIR` is the temp directory, both with forward
slashes. The variable Windows calls `Path` is `PATH` in the shell: Windows ignores the
case of environment names, bash does not, and scripts write `PATH`.

## Finding commands

A command name is looked up in `PATH`. The entries are separated by `;`, as Windows
writes them, and also by `:`, as bash scripts write them
(`PATH="$HOME/bin:$PATH"`); a colon right after a drive letter (`C:/x`) belongs to the
drive. In each directory, psh tries the name with each extension in `PATHEXT`, in its
order (`.COM`, `.EXE`, `.BAT`, `.CMD`, ...), the way cmd does:

```text
$ type -P cmd
C:/WINDOWS/system32/cmd.exe
```

A name that already has one of those extensions is taken as it is (`npm.cmd`). A file
without an extension is a command only if it starts with `#!`. That matters for the
directories package managers fill: next to `tsc.cmd`, npm puts a `tsc` script for sh,
and `tsc` runs `tsc.cmd`, as it would from cmd.

Unlike cmd, psh never looks in the current directory for a bare name. `tool` runs a
`tool.exe` on `PATH`; a `tool.exe` in the current directory runs as `./tool`, as in bash.
The app aliases Windows puts in `WindowsApps` (`winget`, `python` from the Store) are found
like any other program.

## Arguments, batch files and scripts

Windows passes a program one command line, and the program splits it into arguments
itself. psh builds that line with the quoting that the Microsoft C runtime (and Rust,
Python, Node and most other programs) parse back into exactly the arguments the script
had: `'a b'`, `''`, a quote or a trailing backslash arrive as they were written.

`.cmd` and `.bat` files run through `cmd.exe`, which has its own, much looser rules.
Quoting for it naively lets an argument run commands: `"&calc"` would start a calculator.
psh quotes every argument for cmd so that it arrives as one argument, guards `%` so that
`%PATH%` stays text, and refuses arguments that cmd cannot receive intact (line breaks)
with an error instead of passing them cut. An npm-style shim gets what the script wrote:

```text
$ printf '@echo [%%1] [%%2] [%%3]\r\n' > show.cmd
$ ./show.cmd 'a b' 'x&y' '100%'
["a b"] ["x&y"] ["100%"]
```

A file that starts with `#!` runs under its interpreter, looked up on `PATH`:
`#!/usr/bin/env node`, `#!/usr/bin/node` and `#!/bin/node` all run `node` (`node.exe`
or a `node.cmd` shim), with the script's path and its arguments.

Scripts for `sh` or `bash` (`#!/bin/sh`, `#!/usr/bin/env bash`), and files with neither
a `#!` line nor a program extension, run in psh itself, in a subshell with the script's
arguments, as bash runs a file the system cannot start. Windows has no `/bin/sh`, and
psh is the shell anyway:

```text
$ printf '#!/bin/sh\necho sh $1 $#\n' > s.sh && ./s.sh a b
sh a 2
```

## Exit statuses, Ctrl-C and stopping commands

A program's exit code is `$?`. A crash, which on Windows is an exit code too, shows as
the signal a Unix crash would give, so scripts that test for it work the same:

| How the program ended | `$?` |
| --- | --- |
| access violation, stack overflow | 139 (SEGV) |
| `abort()`, a Rust panic with `panic=abort`, a failed assertion | 134 (ABRT) |
| killed by psh | 137 (KILL) |
| stopped by Ctrl-C or Ctrl-Break | 130 (INT) or 143 (TERM) |

Every program psh starts runs in a Windows job object together with everything it
starts in turn. When psh stops a command (a cancelled run, a timeout, `kill`), it first
sends a Ctrl-Break, which console programs treat as a request to quit, and after a
short grace period ends the whole job: grandchildren included, nothing is left running.
Where no Ctrl-Break can be delivered (a program without a console shared with psh), the
job ends at once.

In a console, Ctrl-C reaches the program in the foreground, as it does under cmd, and
it reaches psh: psh cancels the rest of the script and exits with 130, or runs a `trap`
the script set. Ctrl-Break is TERM (143), closing the console window is HUP.

Background jobs (`cmd &`) keep running when the standalone `psh` ends, as with bash. If
psh itself is killed, what it started dies with it, except commands started with `nohup`
or detached with `disown`. An embedder chooses whether a run's background jobs are
killed when the run ends.

## Text and encodings

psh works in UTF-8. Scripts with Windows line endings (`\r\n`) run as they are. Output of
native programs passes through unchanged:

> **Not built yet.** `$(…)` and `read` will drop the `\r` of a `\r\n` line end, so
> `x=$(cmd /c echo hello)` gives `hello`. Today it gives `hello` followed by `\r`.

Programs that print in the system's ANSI code page instead of UTF-8 (older tools, some of
the System32 ones) can be decoded where psh reads their output, for `$(…)` and pipes into
shell code: output that is not valid UTF-8 is converted from the ANSI code page. An
embedder turns this on; it is off by default, since most programs an agent runs print
UTF-8 and a guess can garble them.

## What has no Windows counterpart

| On Unix | On Windows |
| --- | --- |
| descriptors above 2 for a program (`exec 3>f; cmd`) | not passed: Windows programs only receive standard input, output and error |
| `umask` | always `022`; nothing applies it |
| file modes (`chmod`, a creation mode) | only "writable or not": the read-only attribute |
| `test -x` | true for directories, files with a `PATHEXT` extension and `#!` scripts |
| `/bin/sh` | none; psh is the shell |

> **Not built yet.** Most utilities scripts expect (`ls`, `cat`, `grep`, `sed`, ...) will
> run inside psh, so they exist on Windows without any extra install. The names that
> Windows already uses for different programs (`find`, `sort`, `timeout`, `tree`, `more`,
> `hostname`, `whoami`) will always be psh's own on Windows, so `find . -name x` does
> what the script means. Until then such a command runs whatever `PATH` finds, or is not
> found (127).

## Where to look

- The Windows host: [crates/psh-os/src/windows/mod.rs](../../crates/psh-os/src/windows/mod.rs)
- Starting, waiting for and stopping programs:
  [crates/psh-os/src/windows/spawn.rs](../../crates/psh-os/src/windows/spawn.rs)
- Argument and batch file quoting:
  [crates/psh-os/src/windows/cmdline.rs](../../crates/psh-os/src/windows/cmdline.rs)
- Paths and `PATH`: [crates/psh-os/src/windows/path.rs](../../crates/psh-os/src/windows/path.rs)
- Pipes and files: [crates/psh-os/src/windows/file.rs](../../crates/psh-os/src/windows/file.rs)
- Ctrl-C and Ctrl-Break: [crates/psh-os/src/windows/signals.rs](../../crates/psh-os/src/windows/signals.rs)
- The Windows process tests: [tests/process/windows.rs](../../tests/process/windows.rs)
