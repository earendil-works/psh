# Supported syntax and shell builtins

This page lists bash 5.3's syntax construct by construct and says whether psh runs it,
then lists the shell builtins psh has, the bash builtins it lacks, and which shell
options change behavior. For how the language feels in practice, read
[language.md](language.md); for the deliberate differences and their reasons, read
[differences.md](differences.md).

> **In short.** psh parses all of bash 5.3's syntax and runs nearly all of it: every
> redirection form, every parameter operator and transformation, arithmetic, `[[ ]]`,
> globs and extglob. It does not run `select` and `coproc`, the completion, history
> editing and job-control builtins, or POSIX mode; these fail with
> `NAME: not supported yet` and stop the script with status 2. Most `set` and `shopt`
> options that matter to scripts take effect; the rest are stored and reported but
> change nothing.

## How to read the tables

Every row marked **Supported** was checked by running the example with psh and with
bash 5.3 and comparing output and exit status. These probes ran on macOS. Each row
shows one example; the behavior cases in the test suite go much further.

- **Supported**: psh prints what bash prints.
- **Partial**: psh runs it, with the gap the row names.
- **Not supported**: psh parses it, then fails at run time with an error and stops the
  script.

An unsupported feature never runs on with bash semantics silently skipped. It prints
one line and ends the script with status 2, because what follows in a script usually
depends on it:

```
$ psh -c 'echo before; coproc cat; echo after'
before
psh: line 1: coproc: not supported yet
$ echo $?
2
```

> **Trade-off.** Stopping over continuing. An agent that reads `not supported yet` and
> status 2 knows to rewrite the command, which serves the
> [agent-first](goals.md#agent-first) goal and keeps [bash's
> behavior](goals.md#bashs-behavior) honest: psh either does what bash does or says it
> cannot. The cost: a script that uses one unsupported line fails as a whole, even
> where bash would have printed a warning and gone on.

The parser accepts everything `bash -n` accepts, including the constructs psh does not
run, so classifying and printing a command never depends on what the engine can run.

## Commands and lists

| Construct | Example | psh |
| --- | --- | --- |
| Simple command, assignments in front | `x=1 sh -c 'echo $x'` | Supported |
| Lists `;`, `&&`, `\|\|`, newline | `true && echo a \|\| echo b; echo c` | Supported |
| Background `&` | `{ echo bg; } & wait` | Supported |
| Comments | `echo a # b` | Supported |
| Pipeline `\|` | `printf 'b\na\n' \| sort \| head -1` | Supported |
| Pipeline with stderr `\|&` | `{ echo err >&2; } \|& cat` | Supported |
| Negation `!` | `! false; echo $?` | Supported |
| `PIPESTATUS` | `false \| true \| (exit 3); echo ${PIPESTATUS[@]}` | Supported |
| `time`, `time -p`, `TIMEFORMAT` | `time sleep 0.2` | Partial: runs the pipeline, prints no timing report |

An assignment in front of any builtin, special or not, lasts only for that command and
is exported to it, as in bash outside POSIX mode: `x=2 :; echo "[$x]"` prints `[]`.

## Redirections

| Construct | Example | psh |
| --- | --- | --- |
| Output `>`, append `>>`, input `<` | `echo a > f; echo b >> f; cat < f` | Supported |
| Read and write `<>` | `cat <> f` | Supported |
| Force over `noclobber` `>\|` | `set -C; echo c >\| f` | Supported |
| Duplicate `n>&m`, `n<&m` | `{ echo e >&2; } 2>&1` | Supported |
| Close `n>&-`, `n<&-` | `{ echo x >&-; } 2>/dev/null` | Supported |
| Move `n>&m-` | `exec 4>&1; echo hi >&4-` | Supported |
| Both streams `&>`, `&>>`, `>&word` | `{ echo o; echo e >&2; } &> f` | Supported |
| Descriptor into a variable `{var}>`, `{var}<`, `{var}>&-` | `exec {fd}> f; echo hi >&$fd; exec {fd}>&-` | Supported |
| Redirections on `exec` alone | `exec 3< /dev/null` | Supported |
| Redirections on compound commands and functions | `f() { echo x; } > out` | Supported |

`{var}>` takes the lowest free descriptor from 10 up, stores its number in `var` and
stays open after the command, as in bash with `varredir_close` off.

## Here-documents and here-strings

| Construct | Example | psh |
| --- | --- | --- |
| Here-doc with expansion `<<E` | `$HOME` and `$((1+1))` expand in the body | Supported |
| Quoted delimiter `<<'E'`, `<<"E"` | body taken literally | Supported |
| Tab-stripping `<<-E` | leading tabs removed | Supported |
| Several on one line | `cat <<A; cat <<B` | Supported |
| Inside `$( )` and in a pipeline | `x=$(cat <<E ... E)`, `cat <<E \| tr a b` | Supported |
| Here-string `<<<` | `cat <<< "hi $HOME"` | Supported |

## Compound commands

| Construct | Example | psh |
| --- | --- | --- |
| Subshell `( )` | `x=1; (x=2; echo $x); echo $x` | Supported |
| Group `{ }` | `{ echo a; echo b; } \| wc -l` | Supported |
| `if`/`elif`/`else` | `if false; then ...; elif true; then ...; fi` | Supported |
| `while`, `until` | `while ((i<3)); do ...; done` | Supported |
| `for name in words`, `for name` | `for x; do echo $x; done` | Supported |
| Arithmetic `for (( ; ; ))` | `for ((i=0;i<3;i++)); do ...; done` | Supported |
| `case` with `\|`, `(pat)`, `;;`, `;&`, `;;&` | `case a in a) echo 1;& b) echo 2;;& *) echo 3;; esac` | Supported |
| Arithmetic command `(( ))` | `((2>1)) && echo y` | Supported |
| Conditional command `[[ ]]` | `[[ a < b ]]` | Supported |
| `select` | `select x in a b; do ...; done` | Not supported: `select: not supported yet` |

## Functions and coprocesses

| Construct | Example | psh |
| --- | --- | --- |
| `name() body` | `f() { echo f "$@"; }` | Supported |
| `function name`, `function name()` | `function g { echo g; }` | Supported |
| Any compound body | `f() ( x=1; echo $x )` | Supported |
| Recursion, `local`, `return` | `f() { local x=l; return 3; }` | Supported |
| `FUNCNAME`, `BASH_SOURCE`, `BASH_LINENO`, `LINENO` | `f() { echo $LINENO ${FUNCNAME[0]}; }` | Supported |
| Printing a function (`type f`, `declare -f f`) | bash's layout | Supported |
| `coproc`, `coproc NAME` | `coproc cat` | Not supported: `coproc: not supported yet` |

## Quoting

| Construct | Example | psh |
| --- | --- | --- |
| Single quotes | `echo 'a $x "b"'` | Supported |
| Double quotes with `\$`, `\"`, `\\`, backquotes | ``echo "a $x \$x \"q\" `echo b`"`` | Supported |
| Backslash | `echo a\ b \$x` | Supported |
| ANSI-C quoting `$'...'` | `$'a\tb\x41\u00e9\n'` | Supported |
| Locale quoting `$"..."` | `echo $"hello"` | Supported, without a message catalog (as bash without one) |

## Parameters and variables

| Construct | Example | psh |
| --- | --- | --- |
| `$x`, `${x}` | `echo ${x}y` | Supported |
| Positional `$1`, `${10}`, `$#`, `$*`, `$@` | `set -- a b; echo "$@"` | Supported |
| Special `$?`, `$$`, `$!`, `$-`, `$0`, `$_` | `echo $-` | Supported |
| Indexed arrays, `a+=( )`, negative indices, `unset 'a[0]'` | `a=(1 2 3); a+=(4); echo ${a[-1]}` | Supported |
| Associative arrays | `declare -A m=([k]=v)` | Supported |
| Compound assignment with indices | `a=([2]=x [0]=y)` | Supported |
| Append `+=` to strings and integers | `declare -i n=1; n+=2` | Supported |
| Name references `declare -n` | `declare -n r=t; r=6` | Supported |
| `RANDOM`, `SRANDOM`, `SECONDS`, `EPOCHSECONDS`, `EPOCHREALTIME`, `BASHPID`, `BASH_VERSINFO` | `echo $BASHPID` | Supported |

## Parameter expansion operators

| Operator | Example (`x=a.b.c` unless set) | psh |
| --- | --- | --- |
| `${x:-w}`, `${x-w}` default | `${u:-d}` | Supported |
| `${x:=w}`, `${x=w}` assign default | `${u:=d}` | Supported |
| `${x:?w}`, `${x?w}` error if unset | `${u:?missing}` prints `u: missing` | Supported |
| `${x:+w}`, `${x+w}` alternative | `${x:+s}` | Supported |
| `${#x}`, `${#a[@]}` length | `${#x}` | Supported |
| `${x:off}`, `${x:off:len}`, negative offset and length | `${x: -2}`, `${x:1:-1}` | Supported |
| Slices of `$@` and arrays | `${@:2:2}`, `${a[@]:1}` | Supported |
| `${x#p}`, `${x##p}` remove prefix | `${x#*.}` is `b.c` | Supported |
| `${x%p}`, `${x%%p}` remove suffix | `${x%%.*}` is `a` | Supported |
| `${x/p/r}`, `${x//p/r}` replace | `${x//./-}` | Supported |
| `${x/#p/r}`, `${x/%p/r}` anchored replace | `${x/#a/S}` | Supported |
| `${x/p}` delete | `${x/b}` | Supported |
| `&` in the replacement (`patsub_replacement`) | `${x/b/[&]}` | Supported |
| `${x^}`, `${x^^}`, `${x^^[ab]}` upper case | `${x^^}` | Supported |
| `${x,}`, `${x,,}` lower case | `${x,,}` | Supported |
| `${x~}`, `${x~~}` toggle case | `${x~~}` | Supported |
| Operators over arrays | `${a[@]/b/X}`, `${a[@]^^}`, `${a[@]#a}` | Supported |

`nocasematch` applies to `${x/p/r}` but not to `${x#p}` and `${x%p}`, as in bash.

## Transformations `${x@op}`

| Operator | Means | psh |
| --- | --- | --- |
| `@Q` | quoted for reuse as input | Supported |
| `@E` | backslash escapes expanded, as in `$'...'` | Supported |
| `@P` | expanded as a prompt string | Supported |
| `@A` | an assignment or `declare` that recreates the variable | Supported |
| `@a` | the variable's attributes | Supported |
| `@u`, `@U`, `@L` | first letter upper, all upper, all lower | Supported |
| `@K`, `@k` | key-value pairs of an array, quoted or as separate words | Supported |

## Indirection and names

| Construct | Example | psh |
| --- | --- | --- |
| `${!x}` indirect | `y=val; x=y; echo ${!x}` | Supported |
| `${!prefix*}`, `${!prefix@}` names with a prefix | `${!abc*}` | Supported |
| `${!a[@]}`, `${!a[*]}` keys of an array | `${!m[@]}` | Supported |

## Other expansions

| Construct | Example | psh |
| --- | --- | --- |
| Command substitution `$( )`, nested | `$(echo a $(echo b))` | Supported |
| Backquotes, nested with escaped backquotes | ``echo `echo a \`echo b\` ` `` | Supported |
| Function substitution `${ list; }` | `y=${ x=2; echo out; }` runs in the current shell | Supported |
| Value substitution `${\| list; }` | `y=${\| REPLY=r; }` | Supported |
| Arithmetic `$(( ))`, `$[ ]` | `$((1+2*3))` | Supported |
| Process substitution `<( )`, `>( )` | `cat <(echo in)` | Supported |
| Tilde `~`, `~/x`, `~user`, after `=` in assignments | `x=~/y` | Supported |
| Brace lists and sequences | `a{b,c}d`, `{05..10..2}`, `{a..e..2}` | Supported |
| Word splitting with `IFS` | `IFS=': '; for w in $x` | Supported |
| History expansion `!!` | `set -H; echo !!` | Not performed, as in bash running a script |

## Arithmetic

All of these work in `$(( ))`, `(( ))`, `let`, array indices, `for (( ))` and
substring offsets.

| Operators | Example | psh |
| --- | --- | --- |
| `+ - * / % **` | `$((2**10))` | Supported |
| Unary `- + ! ~` | `$((~x))` | Supported |
| `++ --`, prefix and postfix | `$((x++))` | Supported |
| `< <= > >= == !=` | `$((1<=1))` | Supported |
| `&& \|\|` | `$((1&&0))` | Supported |
| `& \| ^ << >>` | `$((1<<4))` | Supported |
| `?:` and `,` | `$((1?2:3))`, `$((x=1, x+5))` | Supported |
| `= += -= *= /= %= <<= >>= &= \|= ^=` | `((x+=2, x<<=2))` | Supported |
| Bases `0x1f`, `010`, `2#101`, `36#z`, `64#_` | `$((2#101))` | Supported |
| Variables by name, recursively, and array elements | `a=2; b=a; echo $((b+1))` | Supported |
| Errors | `$((1/0))` prints `division by 0 (error token is "0")` | Supported |

## `[[ ]]` operators

| Operators | Example | psh |
| --- | --- | --- |
| `-z -n`, `==`/`=` and `!=` as patterns | `[[ abc == a* ]]`; quoted parts match literally | Supported |
| `=~` with `BASH_REMATCH` | `[[ ab12 =~ ^([a-z]+)([0-9]+)$ ]]` | Supported |
| `<`, `>` string order | `[[ a < b ]]` | Supported |
| `-eq -ne -lt -le -gt -ge` | `[[ 1 -lt 2 ]]` | Supported |
| `!`, `&&`, `\|\|`, `( )` | `[[ ! ( a == b \|\| c == d ) && x ]]` | Supported |
| File tests `-e -f -d -r -w -x -s -L` and the rest | `[[ -f a.txt ]]` | Supported |
| `-nt -ot -ef` | `[[ a.txt -ef a.txt ]]` | Supported |
| `-v`, `-R` | `[[ -v x && -R r ]]` | Supported |
| `-o optname`, `-t fd` | `[[ -o errexit ]]` | Supported |

The right side of `==` and `!=` accepts extglob patterns even without `shopt -s
extglob`, as in bash. `=~` is an extended regular expression; `nocasematch` makes it
ignore case.

## Globs and extglob

| Construct | Example | psh |
| --- | --- | --- |
| `*`, `?` | `echo *.txt ?.log` | Supported |
| Brackets, negation, classes | `[ab].txt`, `[!a].txt`, `[[:alpha:]].log` | Supported |
| No match leaves the word | `echo *.nope` | Supported |
| `nullglob`, `failglob` | `shopt -s failglob; echo *.nope` | Supported |
| `dotglob`, `globskipdots` | `shopt -s dotglob; echo .h*` | Supported |
| `globstar` `**` | `shopt -s globstar; echo **/*.txt` | Supported |
| `nocaseglob` | `shopt -s nocaseglob; echo A.TXT` | Supported |
| `GLOBIGNORE` | `GLOBIGNORE=a.txt; echo *.txt` | Supported |
| Extglob `?( ) *( ) +( ) @( ) !( )` in words, `case` and patterns | `echo !(*.txt\|d)` | Supported |
| `nocasematch` in `case` and `[[ ]]` | `case X in x) ...` | Supported |

`shopt -s extglob` takes effect from the next line, as in bash: on the line that turns
it on, `@(a|b)` is still a syntax error.

## Aliases

| Construct | Example | psh |
| --- | --- | --- |
| `alias`, `unalias`, expanded under `shopt -s expand_aliases` | `alias ll='echo L'` (next line) `ll x` | Supported |
| Off by default in a script | `ll x` without the shopt: `command not found` | Supported |
| Alias text ending in a blank expands the next word | `alias s='sudo '` | Supported |
| A quote opened in the alias text, closed in the script | `alias q='echo "a'` then `q b"` | Not supported: psh reports `unexpected EOF` |

## Shell builtins

psh has these builtins. They run inside the shell, never as a program of the same name.
A builtin's arguments, messages and statuses follow bash.

| Builtin | Notes |
| --- | --- |
| `:`, `true`, `false` | |
| `.`, `source` | with arguments; searches `PATH` as bash does |
| `[`, `test` | the same tests as `[[ ]]` without patterns |
| `alias`, `unalias` | |
| `break`, `continue`, `return`, `exit`, `shift` | |
| `builtin`, `command` | `command -v`, `-V`, `-p` |
| `caller` | |
| `cd`, `pwd` | `-L`, `-P`, `cd -`; `CDPATH` is not searched |
| `declare`, `typeset`, `local` | all attributes (`-a -A -i -l -u -n -r -x -g -t`, `-f`, `-F`, `-p`); `local -` is not supported |
| `dirs`, `pushd`, `popd` | |
| `disown`, `jobs`, `kill`, `wait` | job specs (`%1`), `kill -l`, `wait -n`, `wait -p`; see [jobs.md](jobs.md) |
| `echo`, `printf` | `echo -n -e -E`, `xpg_echo`; `printf -v`, `%q`, `%b`, `%(fmt)T` |
| `enable` | lists and `-n` turns a builtin off; `-f` and `-d` (loadable builtins) are not supported |
| `eval`, `exec` | `exec` with only redirections changes the shell's descriptors; `exec -a` |
| `export`, `readonly`, `unset` | `export -n`, `unset -f`, `unset -v` |
| `getopts` | |
| `hash` | `-r`, `-t` |
| `help` | psh's own short help texts |
| `history` | accepts bash's options; a script keeps no history, so it prints nothing |
| `let` | |
| `logout` | `logout: not login shell: use 'exit'`, status 1, as bash in a script |
| `mapfile`, `readarray` | `-t`, `-C`, `-c` and the rest |
| `read` | `-r -a -d -n -N -p -s -t -u -e -i` |
| `set`, `shopt` | see [Shell options](#shell-options) |
| `times` | prints zero times |
| `trap` | signals, `EXIT`, `ERR`, `DEBUG`, `RETURN`, `-p`, `-l` |
| `type` | `-t`, `-a`; an in-process utility shows as `ls is a psh utility` |
| `ulimit`, `umask` | `ulimit -a` and setting limits; `umask -S` |

`type` and `command -V` show an in-process utility as its own kind (`sort is a psh
utility`), and `command -v sort` prints the bare name; see [utilities.md](utilities.md).

### Bash builtins psh does not have

These names are still builtins in psh: they fail with the error below and stop the
script with status 2, instead of running a program of the same name (macOS ships
`/usr/bin/fg` and others as wrappers). `enable` lists them, as bash does.

| Builtin | psh says | bash in a script |
| --- | --- | --- |
| `bg`, `fg` | `bg: not supported yet` | `bg: no job control`, status 1 |
| `suspend` | `suspend: not supported yet` | `suspend: cannot suspend: no job control`, status 1 |
| `bind` | `bind: not supported yet` | a warning that line editing is off |
| `fc` | `fc: not supported yet` | works on the empty history |
| `complete`, `compgen`, `compopt` | `compgen: not supported yet` | `compgen -W` and `complete` work |
| `enable -f`, `enable -d` | `enable -f: not supported yet` | loads a shared object |

## Shell options

`set` and `shopt` know every option bash 5.3 has: `set -o` and `shopt` list all of them
with bash's defaults, `$-`, `SHELLOPTS` and `BASHOPTS` follow them, and `shopt -q` and
`[[ -o ]]` report them. The command line takes the same options (`psh -e -o pipefail -O
extglob -c ...`; see [cli.md](cli.md)). Each subshell has its own copy. An unknown name
is an error, as in bash (`set: nosuch: invalid option name`).

"Takes effect" below means the engine acts on the option; "stored" means `set` and
`shopt` accept and report it, and nothing else changes.

### `set -o`

| Option | psh |
| --- | --- |
| `allexport` (`-a`) | Takes effect |
| `errexit` (`-e`) | Takes effect |
| `errtrace` (`-E`) | Takes effect |
| `functrace` (`-T`) | Partial: functions inherit `DEBUG` and `RETURN` traps; bash runs the `DEBUG` trap once more at a function call |
| `noclobber` (`-C`) | Takes effect |
| `noexec` (`-n`) | Takes effect, from the next line (in bash too) |
| `noglob` (`-f`) | Takes effect |
| `nounset` (`-u`) | Takes effect |
| `pipefail` | Takes effect |
| `verbose` (`-v`) | Takes effect |
| `xtrace` (`-x`) | Takes effect |
| `braceexpand` (`-B`) | Stored: `set +B` does not turn brace expansion off |
| `hashall` (`-h`) | Stored: `set +h` does not turn hashing off |
| `keyword` (`-k`) | Stored: `f b=1` passes `b=1` as an argument |
| `physical` (`-P`) | Stored: `cd` and `pwd` stay logical; use `cd -P` and `pwd -P` |
| `privileged` (`-p`), `notify` (`-b`), `monitor` (`-m`), `onecmd` (`-t`) | Stored |
| `emacs`, `vi`, `history`, `histexpand` (`-H`), `ignoreeof`, `interactive-comments`, `nolog` | Stored; they concern interactive shells |
| `posix` | Not supported: `set -o posix: not supported yet`, status 2; `psh --posix` is refused too |
| `-r` (restricted mode) | Not supported: `set -r (restricted mode): not supported yet` |

POSIX mode is refused rather than ignored: a script that asks for POSIX semantics and
gets bash's would differ silently.

### `shopt`

| Option | psh |
| --- | --- |
| `dotglob`, `extglob`, `failglob`, `globskipdots`, `globstar`, `nocaseglob`, `nullglob` | Takes effect |
| `nocasematch` | Takes effect in `case`, `[[ ]]` and `${x/p/r}` |
| `expand_aliases` | Takes effect, from the next line |
| `inherit_errexit` | Takes effect |
| `lastpipe` | Takes effect |
| `patsub_replacement` | Takes effect (on by default) |
| `xpg_echo` | Takes effect |
| `extdebug` | Partial: fills `BASH_ARGC` and `BASH_ARGV` for functions; `declare -F` prints no file and line, and a failing `DEBUG` trap does not skip the command |
| `cdable_vars`, `execfail`, `gnu_errfmt`, `localvar_inherit`, `shift_verbose`, `bash_source_fullpath` | Stored: probes show bash acting on them and psh not |
| `array_expand_once`, `assoc_expand_once`, `globasciiranges`, `localvar_unset`, `noexpand_translation`, `sourcepath`, `varredir_close`, `checkhash` | Stored |
| `compat31` to `compat44` | Stored |
| `autocd`, `cdspell`, `checkjobs`, `checkwinsize`, `cmdhist`, `complete_fullquote`, `direxpand`, `dirspell`, `extquote`, `force_fignore`, `histappend`, `histreedit`, `histverify`, `hostcomplete`, `huponexit`, `interactive_comments`, `lithist`, `login_shell`, `mailwarn`, `no_empty_cmd_completion`, `progcomp`, `progcomp_alias`, `promptvars`, `restricted_shell` | Stored; they concern interactive or login shells |

`shopt -o` reaches the `set -o` options, so `shopt -s -o posix` is refused the same way.

## Where to look

- [crates/psh-syntax/src/parser.rs](../../crates/psh-syntax/src/parser.rs) and
  [crates/psh-syntax/src/ast.rs](../../crates/psh-syntax/src/ast.rs): the grammar and
  the tree every construct above parses into.
- [crates/psh-engine/src/exec.rs](../../crates/psh-engine/src/exec.rs): commands,
  pipelines, redirections, and the `not supported yet` stop for `select` and `coproc`.
- [crates/psh-engine/src/expand.rs](../../crates/psh-engine/src/expand.rs): parameter
  operators, transformations, substitutions and globbing.
- [crates/psh-engine/src/arith.rs](../../crates/psh-engine/src/arith.rs),
  [crates/psh-engine/src/cond.rs](../../crates/psh-engine/src/cond.rs) and
  [crates/psh-engine/src/pattern.rs](../../crates/psh-engine/src/pattern.rs):
  arithmetic, `[[ ]]`, and the one pattern matcher for globs, `case` and extglob.
- [crates/psh-engine/src/builtins/mod.rs](../../crates/psh-engine/src/builtins/mod.rs):
  the table of builtins, bash's full list, and the refusal of the missing ones.
- [crates/psh-engine/src/options.rs](../../crates/psh-engine/src/options.rs): every
  `set` and `shopt` option with its default, and the ones the engine reads.
