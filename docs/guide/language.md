# The language: bash as agents write it

psh runs bash scripts the way `bash -c` runs them: non-interactive GNU bash 5.3, with
its quoting, expansions, arrays, functions, options and traps. This page says what that
covers, how psh reads a script, how a script can tell it runs under psh, and what
happens when a script uses one of the few parts of bash psh leaves out.

> **In short.** A script that runs under `bash -c` runs under psh with the same output,
> the same errors and the same exit status: the whole non-interactive language, down to
> errexit's exemptions and the order of an associative array's keys. Interactive
> features, POSIX mode, job control proper, `coproc` and `select` are not run; using
> one prints `not supported yet` and ends the shell with status 2, never a quiet
> difference. psh sets `BASH_VERSION` to the bash it follows and `PSH_VERSION` beside
> it, so scripts that check for bash run and scripts that care can tell.

The full list of syntax and builtins is on [Supported syntax and shell
builtins](ref-syntax.md), the places where psh deliberately differs on [Differences
from bash](differences.md), and how the parser works on [The parser](syntax.md). The
examples below are real output of `psh -c '...'`, run on macOS.

## What runs

Everything a non-interactive bash script uses, which is what agents write
([bash's behavior](goals.md#bashs-behavior)):

| Area | Covers |
| --- | --- |
| Commands | simple commands, pipelines (`!`, `\|&`, `PIPESTATUS`), lists with `&&` `\|\|` `;` `&`, `{ }`, `( )` |
| Redirections | `<` `>` `>>` `>\|` `<>` `&>` `&>>`, `n>&m`, `n>&-`, `{var}>file`, here-documents (`<<`, `<<-`, quoted delimiters), here-strings (`<<<`) |
| Quoting | `\`, `'…'`, `"…"`, `$'…'`, `$"…"` |
| Expansions | brace, tilde, parameter with every operator, `$(…)` and backquotes, `<(…)` and `>(…)`, `$((…))`, word splitting on `IFS`, globbing, `${ list; }` and `${\| list; }` |
| Globbing | `*` `?` `[…]`, extglob groups, globstar `**`, nullglob, failglob, dotglob, nocaseglob |
| Variables | indexed and associative arrays, `declare` with its attributes (`-i -l -u -n -r -x -a -A`), `local`, namerefs, bash's dynamic variables (`RANDOM`, `SECONDS`, `BASHPID`, `FUNCNAME`, `LINENO`, ...) |
| Compound commands | `if`, `while`, `until`, `for`, `for ((…))`, `case` (with `;&` and `;;&`), `[[ … ]]` with `=~` and `BASH_REMATCH`, `(( … ))`, functions |
| Options | all of `set -o` and all 59 `shopt` options of bash 5.3 are accepted; `errexit`, `nounset`, `pipefail`, `xtrace`, `noclobber`, `noglob`, `extglob`, `globstar`, `lastpipe`, `inherit_errexit` and the rest that matter in a script take effect |
| Traps | `EXIT`, `ERR`, `DEBUG`, `RETURN` and signals |
| Builtins | the non-interactive ones: `read`, `printf` (with `-v`), `mapfile`, `getopts`, `eval`, `source`, `exec`, `command`, `type`, `trap`, `wait`, `kill`, `jobs`, `disown`, `pushd`/`popd`, `ulimit`, `umask`, ... |

A few of these at once:

```sh
$ psh -c 'f=src/main.rs; echo ${f##*/} ${f%.rs} ${f/main/lib} ${#f} ${f^^} ${x:-def}'
main.rs src/main src/lib.rs 11 SRC/MAIN.RS def
$ psh -c 'echo {a,b}{1..3}; echo "$(echo hi | tr a-z A-Z)" $((2**10 % 7))'
a1 a2 a3 b1 b2 b3
HI 2
$ psh -c 'paste <(echo a) <(echo b)'
a	b
$ psh -c 'x="a  b c"; printf "<%s>" $x; echo'
<a><b><c>
$ psh -c 'shopt -s globstar; echo **/*.rs'
a/b/y.rs a/x.rs
$ psh -c 'shopt -s failglob; echo *.md; echo after'
psh: line 1: no match: *.md
```

Arrays, functions and the tests behave as in bash, including the order in which an
associative array lists its keys, which bash derives from its hash table and which psh
reproduces, since scripts print `${!h[@]}` without sorting it:

```sh
$ psh -c 'declare -A h=([k]=v [x]=y); h[z]=w; for k in "${!h[@]}"; do echo "$k=${h[$k]}"; done'
z=w
x=y
k=v
$ psh -c 'f() { local x=in; echo "$x $1"; return 3; }; x=out; f arg; echo "$? $x"'
in arg
3 out
$ psh -c '[[ x.rs =~ \.(rs)$ ]] && echo "${BASH_REMATCH[1]}"; (( n = 3 * 4, n > 10 )) && echo $n'
rs
12
```

Every builtin is psh's own; a name like `read` or `type` never falls through to a
program of the same name. Common utilities such as `grep`, `sed` and `sort` also run
inside the shell; that is the subject of [Utilities that run in-process](utilities.md).

## Errors, errexit and traps

`set -e` follows bash 5.3 exactly, exemptions included, because a script that depends
on errexit stopping it must stop at the same command. errexit is off inside an `if` or
`while` condition, for every `&&`/`||` operand but the last, for a `!` pipeline, and in
everything those run, functions included. `{ }`, `if`, loops and `case` fail only
through a command inside them. A command substitution runs without errexit unless
`inherit_errexit` is set:

```sh
$ psh -c 'set -e; false && true; echo "after &&"; f() { false; echo "in f"; }; f || :; false; echo unreached'
after &&
in f
$ echo $?
1
$ psh -c 'set -e; x=$(false; echo sub); echo "x=$x"'
x=sub
```

The `ERR` trap fires where errexit would look, whether or not `-e` is on, and `EXIT`
runs once when the shell ends:

```sh
$ psh -c 'trap "echo cleanup" EXIT; trap "echo failed at \$LINENO" ERR; false; echo end'
failed at 1
end
cleanup
```

Errors that bash treats as fatal to the current line, such as a bad substitution or a
division by zero in `$((…))`, abandon the rest of the line and go on with the next;
errors bash treats as fatal to the shell, such as `${x?}` or `set -u` on an unset
variable, end it with bash's status. `set -x` traces in bash's format, quoting included.

## How psh reads a script

**Line by line.** Like bash, psh parses a complete command line, runs it and only then
parses the next. A syntax error on line 2 comes after line 1 ran, with bash's message:

```sh
$ psh -c $'echo one\nfi\necho three'
one
psh: -c: line 2: syntax error near unexpected token `fi'
psh: -c: line 2: `fi'
```

This is also why `shopt -s extglob` and `alias` take effect from the next line, as in
bash. Aliases expand only with `shopt -s expand_aliases`, which is off under `-c`:

```sh
$ psh -c $'shopt -s expand_aliases; alias hi="echo hello"; hi\nhi'
psh: line 1: hi: command not found
hello
```

`eval` and `source` (or `.`) read their text the same way, with positional parameters
for a sourced file.

**Bytes, and the locale decides characters.** A script, its variables, arguments and
command output are bytes, and psh passes them on unchanged, so `$'\xff'` and binary
output survive. Operations that count characters (`${#x}`, `${x:1:1}`, `?` in a
pattern, case conversion) decode UTF-8 when `LC_ALL`, `LC_CTYPE` or `LANG` (the first
one set) names a UTF-8 locale, and count bytes otherwise; a byte that is not valid
UTF-8 counts as one character. As in bash, assigning the variable takes effect at once:

```sh
$ psh -c 'x=héllo; LC_ALL=en_US.UTF-8; echo ${#x} ${x:1:1}; LC_ALL=C; echo ${#x}'
5 é
6
```

## Background jobs

`cmd &`, `$!`, `wait`, `kill` (also `kill %1`), `jobs` and `disown` work as in a bash
script. A job that is a single command is that program, so `$!` is its process id and
pid files mean what they say:

```sh
$ psh -c 'sleep 5 & kill $!; wait $!; echo "status $?"'
status 143
```

How jobs, timeouts and cancellation fit together is on [Cancellation, timeouts and
background jobs](jobs.md).

## Telling psh from bash

psh sets `BASH_VERSION` and the readonly `BASH_VERSINFO` to the bash it follows, and
`PSH_VERSION` to its own version:

```sh
$ psh -c 'echo $BASH_VERSION $PSH_VERSION; echo "${BASH_VERSINFO[@]}"'
5.3.9(1)-release 0.0.0
5 3 9 1 release aarch64-apple-darwin25.6.0
```

Installer and agent scripts often check `BASH_VERSION` and re-run themselves with
`bash` when it is missing, and on Windows there may be no bash to run. Claiming the
version psh matches lets those scripts run; one that then uses something psh lacks
fails loudly on that feature. A script that needs to know tests `[[ -n $PSH_VERSION ]]`.
`BASH_VERSINFO[5]` is the machine type the host reports.

## What psh does not run

psh is a shell for scripts. Everything that exists for a person at a terminal, and the
alternative dialects, is out of scope:

| Feature | What psh does |
| --- | --- |
| Line editing, completion: `bind`, `complete`, `compgen`, `compopt` | `NAME: not supported yet`, status 2 |
| History: `fc` | `fc: not supported yet`, status 2 |
| History: `history` | accepts its arguments and shows nothing: a script's shell keeps no history |
| Interactive shell: `psh -i` | refused on the command line (`-i: invalid option`), status 2 |
| Prompts: `PS1`, `PS2` | inherited values are dropped, as a non-interactive bash does; assigning them is harmless |
| POSIX mode: `set -o posix`, `psh --posix` | `set -o posix: not supported yet`, status 2; `--posix: POSIX mode is not supported` |
| Restricted mode: `set -r` | `set -r (restricted mode): not supported yet`, status 2 |
| Job control: `fg`, `bg`, `suspend` | `NAME: not supported yet`, status 2 |
| Job control: `set -m` | accepted and shown in `$-`, with no effect |
| `coproc` | `coproc: not supported yet`, status 2 |
| `select` | `select: not supported yet`, status 2 |
| Loadable builtins: `enable -f` | `enable -f: not supported yet`, status 2 |

An unsupported feature ends the shell or the subshell it runs in. It does not skip the
command and go on:

```sh
$ psh -c 'set -o posix; echo after'
psh: line 1: set -o posix: not supported yet
$ echo $?
2
$ psh -c '( fg ); echo "after subshell $?"'
psh: line 1: fg: not supported yet
after subshell 2
```

> **Trade-off.** Failing over carrying on. A script that asked for POSIX mode, or set
> up a coprocess, and then ran on with different semantics would produce wrong results
> that look right; for an agent that reads the output, a clear error it can act on is
> better. The cost: a script that would mostly have worked stops at the first such
> feature. When that matters, the embedder can run real bash instead.

These constructs still parse, so the classifier and `psh --parse` see them
([understandable commands](goals.md#understandable-commands)); only running them fails.
The parser's own limits, such as a nesting limit far above what scripts reach, are on
[The parser](syntax.md).

`time` is the one gap that does not fail loudly: `time cmd` runs `cmd` but prints no
timing report.

## Where to look

- [crates/psh-engine/src/exec.rs](../../crates/psh-engine/src/exec.rs): running
  commands, pipelines and compound commands, and where `coproc` and `select` are refused.
- [crates/psh-engine/src/expand.rs](../../crates/psh-engine/src/expand.rs): word
  expansion and the `not supported yet` error.
- [crates/psh-engine/src/pattern.rs](../../crates/psh-engine/src/pattern.rs): the
  pattern matcher for globs, `case` and `${x#…}`, with extglob groups.
- [crates/psh-engine/src/vars.rs](../../crates/psh-engine/src/vars.rs) and
  [crates/psh-engine/src/arrays.rs](../../crates/psh-engine/src/arrays.rs): variables,
  attributes, arrays and bash's associative array order.
- [crates/psh-engine/src/options.rs](../../crates/psh-engine/src/options.rs): `set` and
  `shopt` options.
- [crates/psh-engine/src/builtins/](../../crates/psh-engine/src/builtins/): the builtins;
  [mod.rs](../../crates/psh-engine/src/builtins/mod.rs) lists bash's builtins and
  refuses the ones psh leaves out, [set.rs](../../crates/psh-engine/src/builtins/set.rs)
  refuses POSIX and restricted mode, [trap.rs](../../crates/psh-engine/src/builtins/trap.rs)
  handles traps.
- [tests/](../../tests/): the behavior cases, checked against recordings of bash 5.3.
