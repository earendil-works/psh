# Differences from bash

psh runs the commands agents write the way GNU bash 5.3 runs them, and its utilities
behave like the GNU or BSD tools of the same name. This page lists the places where psh
behaves differently on purpose: for each one, what bash (or the tool) does, what psh
does, and why. Unfinished features are not on this page; they are listed on
[Supported syntax and shell builtins](ref-syntax.md) and [Status in numbers](status.md).

> **In short.** psh follows [bash's behavior](goals.md#bashs-behavior) unless being
> [agent-first](goals.md#agent-first) or [portable](goals.md#portable) asks for
> something else, and those two rank above it. So psh stops a script at a feature it
> does not have, numbers signals as Linux does on every system, and runs its own
> utilities, in one GNU-plus-BSD dialect, even when the system has its own. Most other
> differences come from psh living inside one process: subshells and utility jobs have
> no process id of their own, and `ulimit` limits only the programs psh starts.

Unless a section says otherwise, the examples ran on macOS against Homebrew's bash 5.3,
whose `sort`, `cut`, `awk` and `xargs` are BSD's. Where the example names GNU's output,
that came from GNU's tools in a Debian container.

## The language

| What | bash | psh | Why |
| --- | --- | --- | --- |
| A feature psh does not have (`coproc`, `select`, `compgen`, `complete`, `fg`, `bg`, `enable -f`, `set -r`) | runs it | prints `NAME: not supported yet` and ends the shell (or subshell) with status 2 | a script that runs on after a step psh skipped, such as a `set -e` it did not apply, can do damage; a loud stop never does |
| POSIX mode (`set -o posix`, `--posix`) | switches to POSIX rules | refused as a missing feature | POSIX mode is out of scope; going on with bash's rules after a script asked for POSIX ones would be a silent difference |
| `BASH_VERSION`, `BASH_VERSINFO` | its own version | `5.3.9(1)-release`, the version psh follows, plus `PSH_VERSION` | installer and agent scripts check `BASH_VERSION`, and some start `bash` when it is missing, which may not exist (Windows); `PSH_VERSION` lets a script detect psh |
| `RANDOM` | bash's generator | another generator: the same seed gives other numbers | no script should depend on bash's numbers |
| `BASH`, `HISTCMD` | set | not set | psh does not know a path to itself as a program, and has no history |
| `OSTYPE` on macOS | the kernel release of the machine bash was built on (`darwin25.1.0`) | the release of the running kernel (`darwin25.6.0`) | the running system is the more useful fact |
| Nesting of commands and words | no limit | more than 100 levels is the syntax error `nesting too deep` | the parser never overflows its stack, even one of 1 MiB; real scripts stay far below |
| Function calls | no limit unless `FUNCNEST` is set; deep recursion crashes bash | at most 1000 levels of functions and subshells together (768 in the browser), then `maximum function nesting level exceeded`; `FUNCNEST` does not change it | a runaway recursion is an error message, not a crash of the host that embeds the shell |
| Arithmetic | evaluates while it parses | parses the whole expression first | see below |
| Bracket ranges with non-ASCII letters in `=~` (`[à-ÿ]`) | an error in glibc's `C.UTF-8` locale | compared by code point | an agent's `[[ $x =~ [à-ÿ] ]]` should match; other UTF-8 locales behave this way too |
| `/dev/stdin`, `/dev/stdout`, `/dev/stderr`, `/dev/fd/N` | the system's files where they exist | always the shell's own descriptors: in redirections, `source` and file operands of utilities | an embedded shell's descriptors are not its process's, and Windows has no such files |
| Messages | translated by `LC_MESSAGES`; start with `bash:` | always English; start with `psh:` (or the script's name) | psh carries no translations |

A missing feature stops the script at that point:

```text
$ psh -c 'coproc cat; echo after'; echo $?
psh: line 1: coproc: not supported yet
2
```

Bash prints `after` and exits 0. `set -o posix` gives
`psh: line 1: set -o posix: not supported yet`, and `psh --posix` refuses to start with
`POSIX mode is not supported` and status 2.

### Arithmetic is parsed before it runs

Bash evaluates `$(( ))` and `(( ))` while it reads them, so a part before a syntax error
has already run. psh parses the whole expression first. A syntax error therefore wins
over an earlier runtime error, and no side effect before it happens:

```text
$ psh -c 'x=1; (( x++ , 1 + )); echo "x=$x"'
psh: line 1: ((: x++ , 1 + : arithmetic syntax error: operand expected (error token is "+ ")
x=1
```

Bash prints the same message and then `x=2`. For `$(( 1/0 + ))` bash reports
`division by 0`, psh the syntax error. The messages and error tokens are otherwise
bash's, including its look-ahead. The parser and the evaluator have limits of their own:
more than 100 nested parentheses, or values that refer to values more than 2048 levels
deep, fail with `expression recursion level exceeded`, where bash goes deeper.

## Signals and processes

| What | bash | psh | Why |
| --- | --- | --- | --- |
| Signal numbers | the system's (on macOS `USR1` is 30) | Linux's on every system: `kill -l`, `trap -l`, `kill -10`, and `128+n` for a program a signal ended | a script behaves the same on every system ([portable](goals.md#portable)); psh translates at the system boundary |
| A signal the system lacks (`SIGSTKFLT`, `SIGPWR` on macOS) | not a signal name | `kill: SIGSTKFLT: no such signal on this system`, status 1 | the name exists in psh's numbering, but nothing can deliver it |
| Signal descriptions (`User defined signal 2`) | the system's texts | glibc's texts | one wording everywhere |
| `INT`, `TERM`, `HUP` sent to the standalone `psh` | waits for the foreground program; after `INT` it goes on if the program did not die of it | stops the foreground program and every program of the run at once, then dies by the signal | an agent host that sends `TERM` or `INT` wants the run to stop now, not when the program finishes |
| A `( … )` subshell when the shell is killed | the subshell's process may run on | stops with the shell | a subshell is part of the shell, not a process |
| `bash` and `sh` in a command (`bash -c '…'`, `sh script.sh`) | the program `PATH` finds | psh itself, as a new shell; `sh` with bash's rules; a path such as `/bin/bash` still runs the program | the same shell and utilities everywhere, also on Windows and in the browser, and a cancel reaches it |
| `$$` and `BASHPID` in such a `bash` or `sh` | its process id | a script that reads them runs under the real program `PATH` finds instead; without one (Windows, the browser), or when the use only shows at run time (`eval`, `source`), reading them is an error that ends that shell: `$$: this shell runs inside psh and has no process id` | it runs inside psh and has no process id; a made-up one would reach `/proc/$$`, `ps -p $$` or a pid file and silently name no process |
| `BASHPID` in a subshell (`( … )`, `$( … )`, a pipeline stage) | the subshell's process id | the same error; `$$` and the shell's own `BASHPID` are real | a subshell is not a process |
| `$PPID` in such a `bash` or `sh` | its parent's process id | the caller's `$$` | the caller is its parent |
| `SHLVL` when bash runs the last command of `-c`, `( … )` or `$( … )` in place | the nested shell keeps it | the nested shell counts one more level; `exec bash` keeps it in both | psh does not imitate that optimization |
| `$!` of a background job that is more than one program, or a psh utility | a process id | a job id above every real process id (`4294306`); the shell's `kill`, `wait` and `jobs` know it, other programs cannot signal it | such a job has no operating system process |
| Background programs when an embedded run ends | they outlive the shell | the host ends them with the run, unless they were detached (`nohup`, `setsid`, `disown`) | an embedder must be able to end a run completely; the standalone `psh` leaves them, as bash does |
| `ulimit` | limits bash's own process and, through it, everything it starts | limits only the programs psh starts; redirections, builtins and utilities inside psh are not limited | an embedded shell must not lower the limits of the program that embeds it |

Signal numbers on macOS:

```text
$ psh -c 'kill -l | head -2; kill -l USR1; sh -c "kill -USR2 \$\$"; echo $?'
 1) SIGHUP	 2) SIGINT	 3) SIGQUIT	 4) SIGILL	 5) SIGTRAP
 6) SIGABRT	 7) SIGBUS	 8) SIGFPE	 9) SIGKILL	10) SIGUSR1
10
psh: line 1: 95087 User defined signal 2      sh -c "kill -USR2 \$\$"
140
```

Bash on macOS lists `7) SIGEMT` and `10) SIGBUS`, prints `30` for `USR1`, reports
`User defined signal 2: 31` and exits `159`.

How the two shells react to a signal shows with `/bin/sleep 3; echo after $?` and a
`TERM` sent to the shell's process after one second: psh ends after one second, killed
by `TERM`, and stops `sleep` with it. Bash ends after three seconds, when `sleep` is
done. With `INT` instead, bash prints `after 0` and exits 0, since `sleep` never got the
signal; psh again ends at once. A trapped signal behaves as in bash in both shells: the
trap runs when the foreground program is done. See
[Cancellation, timeouts and background jobs](jobs.md) and
[Processes on Unix](processes.md).

A utility job has no process for another program to signal:

```text
$ psh -c 'sleep 5 & sh -c "kill $!"; echo $?'
sh: line 0: kill: (4294306) - No such process
1
```

`ulimit` and an in-process utility, here psh's `head`, then the system's:

```text
$ psh -c 'ulimit -f 1; head -c 4000 /dev/zero > big; wc -c < big'
4000
$ psh -c 'ulimit -f 1; /usr/bin/head -c 4000 /dev/zero > big; wc -c < big'
psh: line 1:  4552 File size limit exceeded   /usr/bin/head -c 4000 /dev/zero > big
1024
```

Bash stops both at 1024 bytes. Where the system has no such limits (Windows, the
browser), `ulimit` rejects the option with status 2, as bash does for a limit its system
lacks.

## Utilities

psh runs its own implementation of `find`, `sort`, `awk`, `sed`, `grep`, `timeout` and
the other utilities on [Utilities that run in-process](utilities.md), also when the
system has a program of that name. An explicit path (`/usr/bin/find`) or `command -p`
runs the system's program, and an option psh does not implement runs it too. Where GNU
and BSD disagree, the choice depends on the arguments, never on the system; GNU wins
unless agents write the BSD form or GNU's behavior is a trap. The flags are on
[Utilities and their flags](ref-utilities.md).

| What | the system's tool | psh | Why |
| --- | --- | --- | --- |
| Which `find`, `timeout`, `awk`, ... runs | the program on `PATH`: BSD's on macOS, none for `timeout` there, other programs on Windows | psh's own, on every system | agents on macOS write GNU's `find -printf` and `timeout`; one behavior everywhere ([consistent](goals.md#consistent)) |
| `type`, `type -t`, `command -v` | bash: `find is /usr/bin/find`, `file` | `find is a psh utility`, `builtin`, `find` | they say what runs |
| `which` | the program on `PATH` | the program on `PATH` too, though psh's utility runs; `NAME: shell builtin` only when there is no program | `$(which env)` must give a path that runs |
| `sort`, `comm`, `uniq` | the locale's collation | byte order in every locale, as with `LC_ALL=C` | the only order that is the same everywhere; `sort \| uniq -c` and `comm` need every tool to agree |
| `cut -c` | GNU: bytes in every locale; BSD: characters in a UTF-8 locale | characters in every locale (`-b` stays bytes) | agents mean characters (`cut -c1-80`), and a split character breaks the next tool's UTF-8 |
| `tr` | GNU: bytes; BSD: characters in a UTF-8 locale | bytes while both sets are ASCII; characters once a set has a non-ASCII one | GNU's behavior where it is sane; GNU's `tr é e` corrupts other characters |
| `xargs` with empty input | GNU runs the command once without arguments | BSD: runs nothing; `-r` is accepted | with GNU's rule `find … \| xargs rm` with no matches is an error from `rm` |
| `awk` on macOS | BSD's awk | gawk's behavior where gawk and BSD awk differ | macOS agents otherwise get an awk that differs in many details |
| `awk` `for (k in a)` | an order of the implementation's hash table | insertion order | awk leaves the order open; insertion order is stable and what people expect |
| `awk`'s `system()` and pipes, `sed`'s `e` | run `/bin/sh` | run the command in psh | the same shell as the script; there is no `/bin/sh` on Windows |
| `sed` on macOS | BSD sed | GNU sed 4.9's language and messages; `-i ''` and `-i.bak` are accepted as BSD writes them | agents write GNU's sed and macOS's `-i ''` alike |
| `grep` on macOS | BSD grep | GNU grep 3.11's behavior | GNU on conflict |
| Order of `find`, `grep -r`, `rg` and `fd` results | GNU's `find` and `grep -r` walk in the order of the directory; `rg` and `fd` in the order their threads finish | sorted by name | the same output on every file system |
| `fd` with a pattern that starts with a dot | an error on Unix only | an error everywhere | a command behaves the same everywhere |
| `TZ=UTC date -d '2024-05-01 10:00 -3 hours' +%T` | GNU reads `-3` as a zone: `14:00:00` | three hours earlier: `07:00:00` | GNU's reading is a quirk nobody means |
| `date -j -v+1m` (BSD) | macOS clamps the day: Jan 31 is Feb 29 | the same; GNU's `-d '+1 month'` still gives Mar 2 | each form keeps the meaning of its dialect |
| `date` without zone files (Windows, the browser) | — | the host's offset; `%Z` is `UTC` or the numeric offset | there is no zone database to read |
| `diff` | GNU's algorithm | Myers' algorithm tuned to GNU's choices: GNU's hunks for 102 of 108 real file pairs, equally small ones with another alignment for the rest; past 2048 changes it gives up being minimal | GNU's code cannot be used, and an unbounded search would make very different files slow |
| `jq` with several files | one JSON text over all of them | each file its own text | a value split across two files is an error |
| `base64` | BSD does not wrap | wraps at 76, as GNU | GNU on conflict |
| `cmp` | BSD: `differ: char 1` | GNU: `differ: byte 1, line 1` | GNU on conflict |
| `ls -G`, `ls -T` | GNU: no group, tab size | BSD: color (psh prints none), full times | agents write only the BSD forms |
| `mktemp` | BSD creates a name with fewer than three `X`s as written | fails (GNU); several templates are made one by one (BSD) | both dialects, GNU on conflict |
| `md5` and `md5sum`, `shasum` and `sha256sum` | each on its home system | each name prints its own native format | scripts parse the output |

On macOS, where `sort` and `cut` are BSD's and `LANG` is English:

```text
$ printf 'b\nA\na\nB\n_x\n-y\nä\nz\n' | LC_ALL=en_US.UTF-8 sort | tr '\n' ' '
-y A B _x a b z ä
$ awk 'BEGIN { a["z"]; a["b"]; a["m"]; a["a"]; for (k in a) printf "%s ", k; print "" }'
z b m a
$ which find; type find; type -t find
/usr/bin/find
find is a psh utility
builtin
```

The system's `sort` gives `_x -y a A ä b B z`, and both macOS's awk and gawk print
`m z a b`. GNU's `echo héllo | cut -c1-2` prints `h` and half of `é`; psh prints `hé`.

In the browser, where there are no threads, `grep`, `rg` and `jq` read their standard
input to the end before they work on it, within a bound of 64 MiB. Past it they fail
with `NAME: input too large for the in-process NAME in this environment` and status 2,
never hanging; natively they stream. See [psh in the browser](wasm.md).

## Platforms

### Windows

There is no bash on Windows to compare with; these are the places where psh on Windows
differs from bash on Unix. [psh on Windows](windows.md) explains each of them.

| What | bash on Unix | psh on Windows | Why |
| --- | --- | --- | --- |
| Descriptors above 2 (`exec 3>f; cmd`) | passed to the program | not passed, and no error | Windows programs only receive standard input, output and error |
| Scripts starting with `#!/bin/sh` or `#!/bin/bash`, or with no interpreter line | run by `/bin/sh` or bash | run by psh itself | there is no `/bin/sh` |
| `/bin/sh`, `/bin/bash`, `/usr/bin/sh`, `/usr/bin/bash` in a command | run those programs | run psh itself, as bare `sh` and `bash` do | those files do not exist on Windows |
| Programs when the shell dies | may run on | end with it, unless detached or disowned | Windows can tie them to the shell; Unix cannot |
| A program a signal ended | `128+n` | Windows programs do not die by signals: psh's own stop is `KILL` (137), a crash is reported as `SEGV`, `ILL`, `FPE` or `ABRT` | the statuses scripts test for keep their meaning |
| `chmod +x`, `test -x` | execute bits | execute bits come from the file (directories, `PATHEXT` extensions, `#!` scripts); `chmod +x` changes nothing | a record kept by psh alone would disagree with what actually runs |
| `rm` in a read-only directory | refused | allowed | Windows does not protect a directory's entries, and psh does not pretend to |
| `PATH` | split on `:` | split on `;`, and on `:` unless it follows a drive letter | both Windows' and bash's way of writing it work |
| `/dev/null`, `/tmp`, `/c/x` | files | mapped to `NUL`, the user's temp directory and `C:\x` | scripts assume them |
| `OSTYPE`, `MACHTYPE` | the system's | `msys`, `x86_64-pc-msys`, as Git Bash reports | scripts take their Windows branch |
| `grep`, `rg` | `$` matches before `\n` | `\r\n` is a line end too | Windows files end lines with `\r\n` |

> **Not built yet.** On a host whose programs end lines with `\r\n`, `$(…)` (with
> backquotes, `$(< file)` and `${ …; }`) and `read` drop the `\r` before a line end they
> remove. The shell has the rule, but the Windows host does not switch it on yet, so
> `x=$(cmd /c echo hello)` keeps a trailing `\r`. Elsewhere a `\r` is data, as in bash.

### The browser

In WebAssembly nothing outside the shell exists unless the host adds it, and there are
no processes: `$$` is a number the host chooses, `OSTYPE` is `unknown`, the nesting
limit is 768 levels, `ulimit` has no limits to set, and the blocking utilities read
their input first, as above. See [psh in the browser](wasm.md).

## Where to look

- [crates/psh-engine/src/expand.rs](../../crates/psh-engine/src/expand.rs): the
  `not supported yet` error;
  [crates/psh-engine/src/builtins/set.rs](../../crates/psh-engine/src/builtins/set.rs):
  POSIX and restricted mode refused.
- [crates/psh-engine/src/lib.rs](../../crates/psh-engine/src/lib.rs) and
  [crates/psh-engine/src/state.rs](../../crates/psh-engine/src/state.rs):
  `BASH_VERSION`, `PSH_VERSION`, `RANDOM`, `BASHPID` and the default nesting limit.
- [crates/psh-syntax/src/parser.rs](../../crates/psh-syntax/src/parser.rs): the nesting
  limit; [crates/psh-syntax/src/arith.rs](../../crates/psh-syntax/src/arith.rs) and
  [crates/psh-syntax/src/arith_eval.rs](../../crates/psh-syntax/src/arith_eval.rs):
  arithmetic, parsed then evaluated.
- [crates/psh-regex/src](../../crates/psh-regex/src): the regex engine, with ranges by
  code point.
- [crates/psh-engine/src/builtins/trap.rs](../../crates/psh-engine/src/builtins/trap.rs)
  and [crates/psh-engine/src/signals.rs](../../crates/psh-engine/src/signals.rs): the
  Linux signal table;
  [crates/psh-os/src/unix/signum.rs](../../crates/psh-os/src/unix/signum.rs): the mapping
  to the system's numbers;
  [crates/psh-os/src/unix/signals.rs](../../crates/psh-os/src/unix/signals.rs) and
  [crates/psh/src/main.rs](../../crates/psh/src/main.rs): how the standalone `psh` takes
  signals and dies by them.
- [crates/psh-engine/src/builtins/ulimit.rs](../../crates/psh-engine/src/builtins/ulimit.rs):
  `ulimit` and the limits given to programs.
- [crates/psh-utils/src/sort.rs](../../crates/psh-utils/src/sort.rs),
  [crates/psh-utils/src/cut.rs](../../crates/psh-utils/src/cut.rs),
  [crates/psh-utils/src/tr.rs](../../crates/psh-utils/src/tr.rs),
  [crates/psh-utils/src/xargs.rs](../../crates/psh-utils/src/xargs.rs),
  [crates/psh-utils/src/which.rs](../../crates/psh-utils/src/which.rs),
  [crates/psh-utils/src/awk](../../crates/psh-utils/src/awk),
  [crates/psh-utils/src/sed](../../crates/psh-utils/src/sed),
  [crates/psh-utils/src/date.rs](../../crates/psh-utils/src/date.rs) and
  [crates/psh-utils/src/diff_algo.rs](../../crates/psh-utils/src/diff_algo.rs): the
  utilities named above.
- [crates/psh-engine/src/util/blocking.rs](../../crates/psh-engine/src/util/blocking.rs):
  blocking utilities and the 64 MiB input bound.
- [crates/psh-os/src/windows/spawn.rs](../../crates/psh-os/src/windows/spawn.rs): how
  Windows programs start, what they inherit and how their exit codes are read.
