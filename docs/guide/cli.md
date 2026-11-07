# Running psh: the command line

The `psh` command runs a script the way `bash` does, from `-c`, a file or standard input,
with bash's options. It also has a few modes of its own: it can show the syntax tree of a
script, say what a script would do, and list which of its built-in utilities run on this
machine. This page covers both halves, the exit statuses, and arguments that are not
UTF-8.

> **In short.** Anything you would pass to `bash` to run a script works with `psh`:
> `-c`, a file, stdin, `-euo pipefail`, `+o`, `-O extglob`, `--`. psh's own modes are long
> options that come first: `--parse`, `--classify` and `--utils`, plus `--help` and
> `--version`. The two halves report errors differently: bash's options in bash's words,
> psh's own with a usage line and a suggestion for a typo.

## Running a script

There are three ways to give psh a script, as with bash:

```text
$ psh -c 'echo "$0 got $1"' name a
name got a
$ psh script.sh a b
2 args: a b
$ echo 'echo from stdin $1' | psh -s x
from stdin x
```

- `-c COMMAND [NAME [ARGS...]]` runs `COMMAND`. `-c` is a flag, and the command is the
  first argument after the options, so `psh -euc 'cmd'` and `psh -c -- 'cmd'` both work.
  `NAME` becomes `$0` (it is `psh` without one) and appears in error messages; `ARGS`
  become `$1` and on.
- `FILE [ARGS...]` reads the script from `FILE`; `$0` is the file's name.
- Without `-c` or a file, the script is standard input. `-s` reads it from standard input
  even when arguments follow, which then become `$1` and on.

psh reads a script from standard input whole before it runs it. bash reads it as it
goes, so a command in the script can read the script's remaining lines as its own input;
in psh it cannot:

```text
$ printf 'read x; echo got $x\nsecond line\n' | psh
got
psh: line 2: second: command not found
```

(bash prints `got second line`.) Agents write scripts with `-c` or into files, where the
two agree.

## Shell options

The options that come before the script are bash's:

| Option | Effect |
| --- | --- |
| `-e`, `-u`, `-x`, ... | turn on the option of `set` with that letter (`-abefhkmnptuvxBCEHPT`) |
| `+e`, `+u`, ... | turn it off |
| `-o NAME`, `+o NAME` | turn the option of `set -o` on or off: `-o pipefail` |
| `-O NAME`, `+O NAME` | turn a `shopt` option on or off: `-O extglob` |
| `-c`, `-s` | where the script comes from, as above |

Options combine as with bash: `-euo pipefail` sets `errexit`, `nounset` and `pipefail`,
and `-o` takes the next argument as its name even inside a cluster. The first argument
that is not an option ends the options, and so do `--` and `-`. An unknown option or
option name is an error with status 2:

```text
$ psh -euo pipefail -c 'echo $-; false | true'; echo $?
ehuBc
1
$ psh -Z
psh: -Z: invalid option
Usage: psh [OPTIONS] [FILE [ARGS]...]
       psh [OPTIONS] -c COMMAND [NAME [ARGS]...]
       psh --parse [--compact] [-O extglob] [-c COMMAND | FILE]
       psh --classify [--compact] [-O extglob] [-c COMMAND | FILE]
       psh --utils [--json]

For more information, try 'psh --help'.
```

`-h` is bash's `set -h`, not a request for help.

Of bash's long options, psh takes `--norc`, `--noprofile` and `--noediting`, which change
nothing in a shell that reads no startup files and has no line editor, so scripts that
pass them keep working. Put them first, as bash requires. `--posix` is
refused, since psh has no POSIX mode. The others (`--login`, `--rcfile`, ...) are errors.

## psh's own modes

psh's own options are long options and come first on the command line, before any of
bash's. Each mode has its own help page (`psh --parse --help`).

| Command | What it prints | Status |
| --- | --- | --- |
| `psh --help` | the help page: the forms above, the modes, the shell options | 0 |
| `psh --version` | `psh` and its version | 0 |
| `psh --parse [-c COMMAND \| FILE]` | the syntax tree as JSON | 0 if the script parses, 2 if not |
| `psh --classify [-c COMMAND \| FILE]` | what the script does, as JSON | 0 if the script parses, 2 if not |
| `psh --utils [--json]` | the in-process utilities and what runs here | 0 |

`--parse` and `--classify` read standard input when neither `-c` nor a file is given.
Both take `--compact`, which prints the JSON on one line instead of indented, and
`-O extglob`, which parses with bash's extended patterns (`!(x)`, `@(a|b)`) on, as
`shopt -s extglob` would earlier in the script. Neither runs anything.

- `--parse` prints every node of the tree with its byte span in the input. Its format
  and its errors are on [the parser page](syntax.md#a-tree-with-byte-spans).
- `--classify` prints every command the script would run, the files each reads and
  writes, its categories and whether the whole script only reads. Its format is on
  [the classification page](classify.md#what-a-host-gets).

```text
$ psh --classify --compact -c 'rm -rf build'
{"ok":true,"error":null,"primary":"mutate","categories":["mutate"],"read_only":false, ...
```

`--utils` lists every name of psh's [in-process utilities](utilities.md): the utility's
mode (`S` always in-process, `F` only when `PATH` has no such program, `W` always on
Windows), whether it is in the destructive group, and what a command of that name runs
here, with this `PATH` and `PSH_UTILS_FORCE`. The answer comes from the shell's own
command lookup, the one `type` uses, so it is what a script would get. On a Mac:

```text
$ psh --utils
NAME       MODE  DESTRUCTIVE  RUNS     PATH
awk        F     no           program  /usr/bin/awk
gawk       F     no           psh
...
timeout    S     no           psh
cat        S     no           psh      /bin/cat
rm         S     yes          psh      /bin/rm
...
```

`PATH` is the program `PATH` finds for the name, whether or not it runs. With `--json`,
the same list is an object with a `utilities` array of `name`, `utility` (the
utility's main name: `timeout` for `gtimeout`), `mode`, `destructive`, `runs` (`psh`,
`program` or `builtin`) and `path` (or `null`).

A typo in psh's own options gets a suggestion:

```text
$ psh --pars -c 'echo hi'
error: unexpected argument '--pars' found

  tip: a similar option exists: '--parse'

Usage: psh [OPTIONS] [FILE [ARGS]...]
...
```

> **Trade-off.** Two styles of error message over one. The options bash has keep bash's
> messages, so a harness that matches them or a person who knows bash sees what they
> expect; psh's own options are not bash's, so their errors can be more helpful. The
> cost: an error looks different depending on which half of the command line it is in.

## Exit statuses

| Situation | Status |
| --- | --- |
| the script ran | its exit status, as with bash |
| the script was ended by a signal | psh ends by the same signal (the parent sees 128 + n in a shell) |
| the script file does not exist | 127 |
| the script file cannot be read (a directory, no permission) | 126 |
| a command line error, bash's or psh's | 2 |
| `--parse`, `--classify` on a script that does not parse | 2 |
| `--help`, `--version` | 0 |

## Arguments that are not UTF-8

On Unix an argument is a string of bytes, and psh passes them on unchanged: a `-c`
command, a file name, `$1` and the arguments of `--parse` keep every byte, whether or not
it is valid UTF-8. `--parse` reports spans as byte offsets into the input as given
([scripts that are not UTF-8](syntax.md#scripts-that-are-not-utf-8)). `--classify` works
on text: each byte that is not valid UTF-8 becomes U+FFFD, which moves the offsets after
it.

On Windows the command line is UTF-16 text, and psh turns it into UTF-8; a broken
surrogate becomes U+FFFD.

## The environment

psh passes its environment to the script, with one exception: `PSH_UTILS_FORCE`, a
comma-separated list of utility names (or `all`) that run in-process even when `PATH`
has the program, is psh's own and is taken out
([turning utilities off](utilities.md#turning-utilities-off)).

## Where to look

- [crates/psh/src/cli.rs](../../crates/psh/src/cli.rs): psh's own options, the help
  pages and the split between psh's options and bash's.
- [crates/psh-engine/src/invocation.rs](../../crates/psh-engine/src/invocation.rs):
  bash's invocation rules, shared with the conformance runner.
- [crates/psh/src/main.rs](../../crates/psh/src/main.rs): running a script, `--parse`
  and `--classify`.
- [crates/psh/src/utils.rs](../../crates/psh/src/utils.rs): `--utils`.
