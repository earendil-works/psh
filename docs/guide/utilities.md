# Utilities that run in-process

When a command calls `head`, `sed`, `find` or `timeout`, psh does not start a program:
it runs its own implementation of that utility inside the shell. This page lists the
utilities psh has, explains when a real program runs instead, which flags the utilities
accept when GNU and BSD disagree, and what stays a real program. The flags of each
utility are listed on [Utilities and their flags](ref-utilities.md).

> **In short.** psh carries in-process versions of the common Unix utilities, the same
> code on every platform and in WebAssembly, and finds them after functions and builtins
> and before `PATH`. An option a utility does not implement runs the real program of that
> name, or fails with a clear message; an explicit path always runs the real program. The
> flags are the union of GNU and BSD, with GNU winning conflicts except where agents
> write the BSD form, decided by the arguments and never by the operating system.

## Why the shell has its own utilities

- **They exist everywhere.** Windows has no `head`, `sed` or `xargs`, and a browser has
  no processes at all. There the utilities are the toolset: a script that pipes through
  `sort | uniq -c` works without installing anything. That is the
  [portable](goals.md#portable) and [sandboxable](goals.md#sandboxable) goals.
- **One dialect.** macOS ships BSD tools, Linux GNU ones, and agents write both on the
  same machine. psh's `find` accepts `-printf` on macOS, its `sed -i ''` works on Linux,
  and its `timeout` exists on macOS at all. That is the [consistent](goals.md#consistent)
  goal.
- **No process to start.** A utility runs as part of the shell, over the same file
  system interface as everything else, so it works over a virtual file system, stops
  the moment a run is cancelled, costs no process start, and the classifier knows
  exactly which of its options write or run code.

Both dialects in one script, with `awk` and `jq` from the same toolset:

```sh live
printf 'b 2\na 1\nb 3\n' > data.txt
sed -i '' 's/ /=/' data.txt
sed -i 's/=/: /' data.txt
sort data.txt | awk -F': ' '{ sum[$1] += $2 } END { for (k in sum) print k, sum[k] }' | sort
seq 3 | jq -s 'map(. * 10)' -c
```
<!-- gen:run -->
```text
a 1
b 5
[10,20,30]
```
<!-- /gen -->

> **Trade-off.** Generous built-in utilities over always running the installed tool.
> They make the same command work on every platform. The cost: where psh's version
> shadows a better real program, it can differ from it in a detail, and psh carries a
> lot of utility code. The modes and the fall-through below exist to contain that.

## What is built

| Group | Utilities |
| --- | --- |
| Reading and text | `cat` `head` `tail` `wc` `cut` `paste` `tr` `sort` `uniq` `comm` `nl` `tee` `seq` |
| Programs | `awk` (also as `gawk`), `sed`, `jq` |
| Search | `grep` (with `egrep` and `fgrep`), `rg`, `fd`, `find` |
| Files | `ls` `stat` `du` `cp` `mv` `rm` `ln` `mkdir` `rmdir` `touch` `chmod` `mktemp` |
| Paths | `basename` `dirname` `realpath` `readlink` |
| Wrappers | `env` `xargs` `timeout` (also as `gtimeout`) |
| Comparing, checksums, encoding | `cmp` `diff`, `md5sum` `sha1sum` `sha224sum` `sha256sum` `sha384sum` `sha512sum`, `md5`, `shasum`, `base64` |
| Environment, time and lookup | `printenv` `which` `sleep` `date` |

`echo`, `printf`, `test`, `pwd`, `true`, `false` and `kill` are shell builtins, part of
the shell itself, not utilities. `rg` is ripgrep's own code and `fd` is fd's, built into
psh; `jq` runs on the jaq interpreter with jq's values, builtins and messages; `awk` is
a port of GoAWK. The other utilities are written for psh.

> **Not built yet.** `nice` and `nohup`. After them come process and host tools (`ps`,
> `pgrep`, `pkill`, `uname`, `nproc`, `hostname`, `id`, `df`), small text tools
> (`tac`, `rev`, `column`, `xxd`, `od`, `fold`, `split`, `join`, `expr`, `yes`), the
> rest of the checksum family (`b2sum`, `cksum`), `tree`, `patch` and archives (`tar`,
> `gzip`, `unzip`). Until a utility exists, its name runs whatever `PATH` finds, or is
> not found (127).

## When the utility runs, and when the real program does

Each utility has one of three modes:

| Mode | The utility runs | Used for |
| --- | --- | --- |
| always (`S`) | always | every utility above |
| `PATH` fallback (`F`) | only when `PATH` has no program of that name | tools whose real version is better (none now) |
| Windows shadow (`W`) | always on Windows; elsewhere as `PATH` fallback | names Windows uses for other programs, such as `tree`, `more`, `hostname` (none built yet) |

`find`, `sort` and `timeout` are names that Windows also uses (for programs that do
something else), but they are "always" on every platform: agents on macOS write GNU
`find`, which BSD's rejects, and psh's `timeout` stops a whole process tree the same way
everywhere. `awk` is "always" for the same reason: macOS's BSD awk and gawk differ in
details scripts depend on (83 of the 772 cases in the awk test suites psh runs), and one
gawk-compatible awk on every platform keeps a command's meaning the same.

Whatever the mode, a command that names a file runs that file: `/usr/bin/head`,
`./tool`, a name bound with `hash -p`, and `command -p head` run the real program.

## Options a utility does not implement

A utility never guesses at an option it does not know. It runs the real program of the
same name from `PATH` with the same arguments, so the result is what that program does.
On macOS, where `head` is BSD's:

```text
$ psh -c 'head --definitely-not-an-option x; echo $?'
head: unrecognized option `--definitely-not-an-option'
usage: head [-n lines | -c bytes] [file ...]
1
```

With no such program, the utility says so and exits with status 2:

```text
$ psh -c 'PATH=/nonexistent; head --definitely-not-an-option x; echo $?'
head: unsupported option '--definitely-not-an-option' (psh built-in head)
2
```

Other usage errors (a missing value, a bad number) are the utility's own, in GNU's words.
Some utilities hand over whole features the same way: `rg` runs the real ripgrep for
`--pre`, `-z` and `-P`, `grep` for a `-P` pattern with look-around, `xargs` for `-p`,
`rm`, `cp`, `mv` and `ln` for `-i` (psh's utilities never prompt), `stat -f` without a
format for file-system status, and `date` for setting the clock. Most utilities hand
`--help` and `--version` to the real program too. The full list is on
[Utilities and their flags](ref-utilities.md).

## Seeing what runs

`type` and `command -v` report what will run, even when a program of the same name is
on `PATH`; `type -t` says `builtin`. `which` prints the program on `PATH` first, so the
common `$(which env)` idiom still gives a path that runs:

```text
$ psh -c 'type cat; type -t cat; command -v cat; which cat; which -a cat'
cat is a psh utility
builtin
cat
/bin/cat
cat: shell builtin
/bin/cat
```

`which` prints `NAME: shell builtin` alone only when no program exists, as in a
browser. `type -P` searches `PATH`, as bash's does for its own builtins.

`psh --utils` lists every utility with its mode and what a command of its name runs on
this machine, with this `PATH` ([the command line](cli.md#pshs-own-modes); how to read
it is on [Utilities and their flags](ref-utilities.md#reading-psh---utils)).

## Turning utilities off

An embedder chooses the utilities of a shell when it creates one. It can give it none,
turn off all of them, the destructive group (`rm`, `mv`, `cp`, `ln`) as one, or single
names; a name that is off runs the real program, or is not found. The `psh` command
turns all of them on.

The variable `PSH_UTILS_FORCE` does the opposite for utilities in `PATH` fallback and
Windows shadow mode: a comma-separated list of names, or `all`, that run in-process even
when `PATH` has the program. No utility is in either mode now, so in the `psh` command
it changes nothing yet. psh takes it out of the environment, so commands never see it:

```text
$ PSH_UTILS_FORCE=all psh -c 'env | grep -c PSH_UTILS_FORCE'
0
```

## One dialect: GNU and BSD together

A utility accepts the GNU and the BSD flags wherever they do not conflict, and GNU's
long options with unambiguous prefixes, options mixed with file operands and `--`. Where
the two conflict, GNU's meaning wins, except where agents overwhelmingly write the BSD
form or GNU's behavior is a trap for them. The choice depends only on the arguments,
never on the platform, so a command means the same on a Mac and in a Linux container:

| Command | psh does |
| --- | --- |
| `head -n -N`, `tail -r` | both accepted, though each exists in one dialect only |
| `cp -R src/ dst` | GNU: copies the directory `src` itself |
| `wc`, `uniq -c` | GNU's column widths |
| `du` | GNU's 1 KiB units, not BSD's 512-byte blocks |
| `find` with no path | searches `.`, as GNU does |
| `xargs` with no input | BSD: the command does not run (`-r` is accepted) |
| `sort`, `comm`, `uniq` | byte order in every locale, as with `LC_ALL=C` |
| `cut -c` | characters, not bytes, in every locale (BSD's meaning) |
| `tr` | bytes when both sets are ASCII (GNU); characters when a set has a non-ASCII one |
| `sed -i` | `-i ''` and `-i.bak` take a suffix; a bare `-i` before a script and operands is GNU's, without a backup; `-i .bak` before a script is BSD's suffix |
| `stat -f` | with a `%` in its argument a BSD format; before plain file names GNU's file-system status |
| `stat -t` | BSD's time format when a BSD format is given or its argument has a `%`; else GNU's terse output |
| `mktemp -t` | an argument with `XXX` is GNU's template in `$TMPDIR`; any other is a BSD prefix; `-t` as the last word is GNU's flag |
| `date -r` | all digits and no such file: BSD's epoch seconds; else GNU's reference file |
| `ls -G`, `ls -T` | BSD's: `-G` is color (psh prints none, as macOS does off a terminal), `-T` full times in the long format; GNU's `-o` for long without the group |
| `base64 -i` | GNU's `--ignore-garbage` before an option or at the end, else BSD's input file; output wraps at 76 |
| `md5`, `shasum` vs `md5sum`, `sha256sum` | each name prints its own native format |
| `awk` | gawk's behavior where gawk and BSD awk differ; `for (k in a)` in insertion order |

The rules for `xargs`, `sort` and `cut` change what an everyday GNU command does, on
purpose. `xargs` with GNU's rule turns
`find … | xargs rm` with no matches into a usage error from `rm`. A byte-order `sort`
is the only order that is the same on every system, and `sort | uniq -c` or `comm` of
two sorted lists depend on all tools agreeing, not on dictionary order; the cost is that
`B` sorts before `a` in an English locale too. And GNU's `cut -c1-80` can split a
character in half, which breaks the next tool's UTF-8:

```text
$ psh -c 'echo é | cut -c1; echo é | tr é e; printf "b\na\nB\n" | sort'
é
e
B
a
b
```

The BSD rules exist because agents write those forms on macOS and expect them to work.
Here are the three forms of `sed -i` one after another (`t.bak` holds the file as it
was before the second edit):

```text
$ psh -c 'printf "a\n" > t; sed -i "" s/a/b/ t; sed -i.bak s/b/c/ t; sed -i s/c/d/ t; cat t t.bak; ls'
d
b
t
t.bak
```

`stat`, `date` and `mktemp` take either dialect's form in the same command line
(on macOS; `mktemp` names are random):

```text
$ psh -c 'stat -f "%N %z" f; stat -c "%n %s" f; TZ=UTC date -r 0; TZ=UTC date -r f +%F'
f 6
f 6
Thu Jan  1 00:00:00 UTC 1970
2026-11-08
$ psh -c 'export TMPDIR=/tmp; mktemp -t build; mktemp -t build.XXXXXX'
/tmp/build.0SvAHOWqSj
/tmp/build.irvF85
```

`mktemp` uses `$TMPDIR` (else `/tmp`) where macOS's own `mktemp -t` prefers a per-user
directory. `ls -lT` is BSD's full time:

```text
$ psh -c 'ls -lT f'
-rw-r--r-- 1 mitsuhiko wheel 6 Jan  2 03:04:00 2024 f
```

The checksum tools are never unified: scripts parse their output, and each format is
what that name prints on its home system:

```text
$ psh -c 'md5sum f; md5 f'
b1946ac92492d2347c6235b4d2611184  f
MD5 (f) = b1946ac92492d2347c6235b4d2611184
```

Values in messages are quoted as GNU quotes them for the locale: `‘x’` when it is UTF-8,
`'x'` otherwise (`cut: invalid field value ‘x’`). Commands that report on the system,
such as `uname` and `hostname`, report the real host: psh never pretends to be Linux.

## Utilities inside the shell

A utility runs where a shell builtin runs, so a few things look different from a
program:

- **Wrappers reach back into the shell.** `env`, `xargs`, `timeout`, `find -exec` and
  `fd -x` run utilities and the builtins that also exist as programs (`printf`, `echo`,
  `test`, `[`, `true`, `false`, `pwd`, `kill`), so `find . -name f -exec printf
  '[%s]\n' {} \;` needs no program at all. They never run shell functions, as in bash.
  `awk`'s `system()` and its pipes, and `sed`'s `e` command, run the command in a fresh
  psh shell, never in `/bin/sh`.
- **No process id.** A background job that is a utility (`sleep 5 &`) has no operating
  system process. Its `$!` is a job id of the shell's own, larger than any real process
  id; the shell's `kill`, `wait` and `jobs` understand it, another program cannot signal
  it:

  ```text
  $ psh -c 'sleep 5 & sh -c "kill $!"'
  sh: line 0: kill: (4294306) - No such process
  ```

- **Stopping.** A cancelled run or a `timeout` stops a utility at once, wherever it is;
  see [Processes on Unix](processes.md) for the programs it started.
- **In WebAssembly** every utility runs in-process, whatever its mode, since there is
  no program to fall back to (see [psh in the browser](wasm.md)). There, `rg`, `grep`,
  `fd` and `jq`, which are built on synchronous code, read their input to the end
  before they start, within a bound; past it, they fail with `NAME: input too large for
  the in-process NAME in this environment` and status 2 instead of hanging.

## What stays a real program

Some programs are never utilities: interpreters and language runtimes (`python`,
`node`), toolchains and build systems (`cargo`, `make`), package and environment
managers (`npm`, `pip`), and database clients and engines (`sqlite3`, `psql`). Running
them as the real program keeps them what agents expect, and visible to the classifier
(see [Understanding commands](classify.md)). Version control and network clients
(`git`, `curl`, `ssh`, `gh`, `docker`) are out of scope: psh does not implement them,
and an embedder that wants them where there are no processes, as in a browser, supplies
them as host commands.

## Where to look

- [crates/psh-utils/src/lib.rs](../../crates/psh-utils/src/lib.rs): the registry of
  utilities, their names, modes and the destructive group, and `PSH_UTILS_FORCE`.
- [crates/psh-engine/src/util.rs](../../crates/psh-engine/src/util.rs): the utility
  interface, the lookup with modes and off switches, and the fall-through to the real
  program.
- [crates/psh-engine/src/util/blocking.rs](../../crates/psh-engine/src/util/blocking.rs):
  utilities built on synchronous code, such as `rg`, `grep`, `fd` and `jq`.
- [crates/psh-utils/src/opts.rs](../../crates/psh-utils/src/opts.rs) and
  [crates/psh-utils/src/common.rs](../../crates/psh-utils/src/common.rs): the shared
  option parser, the unsupported-option message and message quoting.
- [crates/psh-utils/src/modes.rs](../../crates/psh-utils/src/modes.rs): file modes for
  `chmod` and `mkdir -m`.
- [crates/psh-utils/src/sed.rs](../../crates/psh-utils/src/sed.rs) and
  [crates/psh-utils/src/sed](../../crates/psh-utils/src/sed): `sed`, its two `-i` forms
  and its script compiler.
- [crates/psh-utils/src/awk](../../crates/psh-utils/src/awk) and
  [crates/psh-utils/src/jq](../../crates/psh-utils/src/jq): the two language
  interpreters.
- [crates/psh-utils/src/grep](../../crates/psh-utils/src/grep),
  [crates/psh-utils/src/rg](../../crates/psh-utils/src/rg),
  [crates/psh-utils/src/fd](../../crates/psh-utils/src/fd),
  [crates/psh-utils/src/find](../../crates/psh-utils/src/find) and
  [crates/psh-utils/src/search](../../crates/psh-utils/src/search): the search
  utilities and the walk and pattern layer they share.
- [crates/psh-utils/src/ls.rs](../../crates/psh-utils/src/ls.rs),
  [crates/psh-utils/src/stat.rs](../../crates/psh-utils/src/stat.rs),
  [crates/psh-utils/src/mktemp.rs](../../crates/psh-utils/src/mktemp.rs) and
  [crates/psh-utils/src/date.rs](../../crates/psh-utils/src/date.rs): the utilities
  with BSD forms decided by their arguments.
- [crates/psh-utils/src/sum.rs](../../crates/psh-utils/src/sum.rs): the checksum tools.
- [tests/utils/](../../tests/utils/): the behavior cases for each utility, checked
  against GNU and BSD recordings.
