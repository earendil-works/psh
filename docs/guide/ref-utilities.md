# Utilities and their flags

This page lists every utility psh runs in-process: its names, its mode, whether it is in
the destructive group, which tool's behavior it follows, and the options it hands to the
real program or does not support. Why psh has its own utilities, how the modes work and
how GNU and BSD flags are reconciled is on [Utilities that run in-process](utilities.md).

> **In short.** psh has 58 utility names, all in mode `S`: they run in-process on every
> platform, whatever `PATH` holds. Each follows one reference tool (GNU's in most
> cases, the upstream program for `rg`, `fd` and `jq`) and accepts the BSD flags that
> do not conflict with it. An option a utility does not implement runs the real program
> of that name from `PATH`, or fails with `unsupported option` and status 2.

## Reading `psh --utils`

`psh --utils` prints the list for this machine, with this `PATH`. The answer comes from
the shell's own command lookup, the one `type` uses, so it is what a script gets. On a
Mac (an excerpt):

```text
$ psh --utils
NAME       MODE  DESTRUCTIVE  RUNS     PATH
awk        S     no           psh      /usr/bin/awk
gawk       S     no           psh
jq         S     no           psh      /usr/bin/jq
find       S     no           psh      /usr/bin/find
timeout    S     no           psh
gtimeout   S     no           psh
...
rm         S     yes          psh      /bin/rm
...
fd         S     no           psh      /opt/homebrew/bin/fd
rg         S     no           psh      /opt/homebrew/bin/rg
...
mktemp     S     no           psh      /usr/bin/mktemp
```

| Column | Meaning |
| --- | --- |
| `NAME` | one name of a utility; a utility with several names (`awk` and `gawk`) has a row for each |
| `MODE` | `S`: always in-process; `F`: in-process only when `PATH` has no program of that name; `W`: always on Windows, `F` elsewhere |
| `DESTRUCTIVE` | `yes` for the destructive group (`rm`, `mv`, `cp`, `ln`), which an embedder can turn off as one |
| `RUNS` | what a command of that name runs here: `psh` (the utility), `program` (the one in `PATH`) or `builtin` (a shell builtin of that name) |
| `PATH` | the program `PATH` finds for the name, whether or not it runs; empty when there is none |

Every utility is `S` today, so `RUNS` says `psh` for every name in the `psh` command. The
`PATH` column still matters: it is the program that runs when a utility falls through,
and the one an explicit path (`/usr/bin/find`) or `command -p` runs. An empty `PATH`
column means a fall-through has nothing to run, so the option fails instead.

`psh --utils --json` prints the same as an object with a `utilities` array:

```text
$ psh --utils --json
{
  "utilities": [
    {
      "name": "awk",
      "utility": "awk",
      "mode": "S",
      "destructive": false,
      "runs": "psh",
      "path": "/usr/bin/awk"
    },
    {
      "name": "gawk",
      "utility": "awk",
      "mode": "S",
      "destructive": false,
      "runs": "psh",
      "path": null
    },
...
```

`utility` is the main name of the utility the row belongs to: `timeout` for
`gtimeout`, `grep` for `egrep`, `md5sum` for `sha256sum`.

## How to read the tables

- **Names**: every name the utility answers to.
- **D**: `yes` when the utility is in the destructive group. Other utilities can write
  too (`sed -i`, `touch`, `chmod`, `tee`, `find -delete`); the group is what an embedder
  can switch off as one.
- **Follows**: the tool whose behavior, output and messages psh reproduces, with the
  flags it takes from the other dialect. Where GNU and BSD conflict, GNU wins unless
  the [dialect rules](utilities.md#one-dialect-gnu-and-bsd-together) say otherwise.
- **Falls through or not supported**: options and forms psh hands to the real program.
  On top of these, any option a utility does not know falls through, and so do
  `--help` and `--version` for every utility except `rg`, `fd` and `jq`, which print
  their own.

The mode column is left out of the tables: it is `S` for every row.

A fall-through runs the program `PATH` finds with the same arguments. Without one, the
utility fails with status 2. These are psh's messages with `PATH` empty, on macOS:

```text
$ psh -c 'PATH=/nonexistent; find . -ls; xargs -p echo </dev/null; rg --pre cat x'
find: unsupported option '-ls' (psh built-in find)
xargs: unsupported option '-p' (psh built-in xargs)
rg: unsupported --pre (psh built-in rg)
```

## Reading and text

| Names | D | Follows | Falls through or not supported |
| --- | --- | --- | --- |
| `cat` | no | GNU (`-A -b -e -E -n -s -t -T -u -v`; BSD's letters mean the same) | |
| `head` | no | GNU (`-n -N`, `-c -N`, `-q -v -z`, the old `-5` form) | |
| `tail` | no | GNU (`-f`, `-F`, `--pid`, `--retry`, `+N`), BSD's `-r` | |
| `wc` | no | GNU, with GNU's column widths | `--files0-from` |
| `cut` | no | GNU, BSD's `-w`; `-c` counts characters in every locale | |
| `paste` | no | GNU: every line ends in a newline, `-d ''` means no delimiter | |
| `tr` | no | GNU on bytes when both sets are ASCII; characters when a set has a non-ASCII one; BSD's `-C -u` | |
| `sort` | no | GNU's keys and orders (`-n -g -h -V -M -R`), byte order in every locale; `-S -T --parallel --batch-size` are accepted and ignored | `--compress-program`, `--files0-from`, `--random-source`, `--debug` |
| `uniq` | no | GNU (counts in 7 columns), byte comparison; `-s -w` count characters | |
| `comm` | no | GNU, BSD's `-i`; byte comparison | |
| `nl` | no | GNU (`-b -h -f` styles `a t n`, `-n ln rn rz`, logical pages) | the style `pREGEX` |
| `tee` | no | GNU (`-a -p --output-error`); `-i` is accepted, since the shell owns signals | |
| `seq` | no | GNU (`seq 0` prints nothing, `-s` taken literally), BSD's `-t` | |

## Programs

| Names | D | Follows | Falls through or not supported |
| --- | --- | --- | --- |
| `awk`, `gawk` | no | POSIX awk with the gawk extensions agents use (`gensub`, `match` with an array, `strftime`, regex `RS` and `RT`); gawk 5.2 where gawk and BSD awk differ; `for (k in a)` in insertion order; `system()` and pipes run in psh | every option but `-F -f -v -i` (`--lint`, `--version`, ...) |
| `sed` | no | GNU sed 4.9's language, options and messages; both forms of `-i`; `e` and `s///e` run their command in a fresh psh shell; `--sandbox` refuses `e`, `r` and `w` | `--debug`, `--help`, `--version` |
| `jq` | no | jq 1.7.1: its values, builtins, output, messages and exit statuses | not supported: modules (`import`, `include`; `-L` is accepted and ignored), `.5` literals |

## Search

| Names | D | Follows | Falls through or not supported |
| --- | --- | --- | --- |
| `grep`, `egrep`, `fgrep` | no | GNU grep 3.11; messages say `grep:` for all three; recursion visits names in sorted order | `-T`, BSD's `-O -S -p`, a `-P` pattern the built-in engine rejects (look-around, back-references) or several `-P` patterns, unknown values of `--color`, `-d`, `-D` |
| `rg` | no | ripgrep 15.2.0, its own code; `--json` included; on Windows `/` separators by default | `--pre`, `-z`/`--search-zip`, `-P`/`--pcre2`, `--engine auto`, `--debug`, `--trace` |
| `fd` | no | fd 10.5.1; results in path order; `-x` commands run one after another; on Windows `/` separators by default | `--changed-within`/`--changed-before` with a duration or a date (only `@timestamp` runs in-process), `--owner` with a user or group name (and `--owner` on Windows), `--hyperlink` |
| `find` | no | GNU (`-printf`, `-regextype`, `-type f,l`, `-newerXY`, `-delete`, `-execdir`) and BSD's extra options (`-E`, `-s`, `-x`, `-d`); no path means `.`; entries in sorted order | primaries psh does not implement: `-ls`, `-ok`, `-okdir`, `-fprint`, `-fprintf`, `-fls`, `-fstype` |

## Files

| Names | D | Follows | Falls through or not supported |
| --- | --- | --- | --- |
| `ls` | no | GNU's output and options, byte order; BSD's `-G` (no color is printed) and `-T` (full times in the long format); `-o` is GNU's | any color (`--color=always`), `--time=birth`, BSD's `-@ -O -e`; GNU's `-T COLS` is not accepted |
| `stat` | no | GNU's `-c`, `--printf`, `-t` and default output; BSD's `-f FORMAT` (with a `%`), `-t TIMEFMT`, `-n`, `-q` | `-f` before plain file names (GNU's file-system status), BSD's `-l -s -x -r` |
| `du` | no | GNU (1 KiB units, `-d`, `--max-depth`, `--exclude`), BSD's `-A -g`; hard links count once | BSD's `-I` |
| `cp` | yes | GNU (`cp -R src/ dst` copies `src` itself); BSD's `-c -X` make plain copies | `-i`, numbered backups (`--backup=numbered`, `existing`) |
| `mv` | yes | GNU; across file systems it copies and removes | `-i`, numbered backups |
| `rm` | yes | GNU's rules: `.` and `..` refused, `/` needs `--no-preserve-root`, links removed, never followed | `-i`, `-I` |
| `ln` | yes | GNU, `-n` (BSD's `-h`); on Windows a directory link is a junction when a symbolic link is not allowed, never a copy | `-i`, numbered backups |
| `mkdir` | no | GNU (`-p -m -v`) | `-Z` |
| `rmdir` | no | GNU (`-p -v --ignore-fail-on-non-empty`) | |
| `touch` | no | GNU's `-d` date strings, POSIX `-t` and `-r`, BSD's `-f` (ignored) | a `-d` date psh does not parse |
| `chmod` | no | GNU (octal and symbolic modes, `-R -v -c -f --reference`) | BSD's `-h` |
| `mktemp` | no | GNU ∪ BSD: `-t` by the rule on the dialect page, several templates as BSD, fewer than three `X` fail as GNU; files `600`, directories `700`, in `$TMPDIR` or `/tmp` | |

## Paths

| Names | D | Follows | Falls through or not supported |
| --- | --- | --- | --- |
| `basename` | no | GNU and BSD agree (`-a -s -z`) | |
| `dirname` | no | GNU (several names, `-z`) | |
| `realpath` | no | GNU (`-e -m -L -P -s -q -z --relative-to --relative-base`); no operand resolves `.`, as BSD does | |
| `readlink` | no | GNU (`-f -e -m -n -q -s -v -z`) | |

## Wrappers

| Names | D | Follows | Falls through or not supported |
| --- | --- | --- | --- |
| `env` | no | GNU (`-i -u -0 -C DIR -S STRING`) | the signal options, `-v` |
| `xargs` | no | GNU, but no command runs on empty input (as BSD; `-r` is accepted); GNU's statuses 123 to 127 | `-p`, `--show-limits`, `--process-slot-var`, BSD's `-J -R -S` |
| `timeout`, `gtimeout` | no | GNU (`-s -k --preserve-status --foreground -v`; statuses 124, 137, 125); stops the whole process tree | |

The wrappers, `find -exec` and `fd -x` run utilities, the builtins that also exist as
programs (`printf`, `echo`, `test`, `[`, `true`, `false`, `pwd`, `kill`) and programs,
never shell functions.

## Comparing, checksums and encoding

| Names | D | Follows | Falls through or not supported |
| --- | --- | --- | --- |
| `cmp` | no | GNU (`differ: byte N, line L`, `-b -l -s -i -n`) | |
| `diff` | no | GNU diffutils' formats (normal, `-u`, `-c`, `-y`, `-q`), `-r` over directories, `--color`, `-p`/`-F` | an `-I` or `-F` pattern psh cannot translate, an `-x` pattern it cannot read |
| `md5sum`, `sha1sum`, `sha224sum`, `sha256sum`, `sha384sum`, `sha512sum` | no | GNU (`HASH  FILE`, `--tag`, `-c` with GNU's messages, `-z`) | |
| `md5` | no | macOS (`MD5 (FILE) = HASH`, `-q -r -p -s`, `-c HASH`) | |
| `shasum` | no | the Perl `shasum` (`-a ALG`, `-c` takes each line's algorithm from the hash's length) | |
| `base64` | no | GNU's encoding (wraps at 76) and decoding; BSD's `-i FILE`, `-o FILE`, `-D`, `-b N` | |

`-c` of every checksum tool reads both line formats.

## Environment, time and lookup

| Names | D | Follows | Falls through or not supported |
| --- | --- | --- | --- |
| `printenv` | no | GNU (several names, `-0`) | |
| `which` | no | GNU and BSD agree (`-a -s`); prints the program on `PATH` first, `NAME: shell builtin` when there is none | |
| `sleep` | no | GNU durations (fractions, `s m h d`, several operands add up, `inf`) | |
| `date` | no | GNU ∪ BSD: GNU's `-d`, `-I`, `-R`, `--rfc-3339`, `+FORMAT` with `%N` and `%:z`; BSD's `-j`, `-f INFMT` with `-j`, `-v`; `-r` by the rule on the dialect page; the zone is the command's `TZ` | setting the clock (`-s`, a date operand without `-j`) |

## Not utilities

`echo`, `printf`, `test`, `[`, `pwd`, `true`, `false` and `kill` are shell builtins,
listed on [Supported syntax and shell builtins](ref-syntax.md). What psh does not have
yet (`nice`, `nohup`, `uname`, `tac`, `tree`, archives, ...) runs whatever `PATH` finds,
or is not found (127); the list is on [Utilities that run in-process](utilities.md).

## Where to look

- [crates/psh-utils/src/lib.rs](../../crates/psh-utils/src/lib.rs): the registry with
  every name, mode and the destructive group.
- [crates/psh/src/utils.rs](../../crates/psh/src/utils.rs): `psh --utils` and its JSON.
- [crates/psh-utils/src/common.rs](../../crates/psh-utils/src/common.rs) and
  [crates/psh-utils/src/textio.rs](../../crates/psh-utils/src/textio.rs): the
  fall-through and the `unsupported` message.
- [crates/psh-utils/src/](../../crates/psh-utils/src/): one module per utility; each
  module's header lists its options and the forms it hands to the real program.
- [tests/utils/](../../tests/utils/): the behavior cases for each utility.
