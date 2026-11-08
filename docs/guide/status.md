# Status in numbers

This page collects psh's measured numbers: how much of bash's behavior it matches on each
platform, how large its parts are, what a build costs in bytes, and how fast a few
everyday things run, and at the end what is not built yet and the gaps known today. Each
table says what it counts. How the cases are recorded and compared is on [How psh is
tested](testing.md); what psh deliberately does differently is on [Differences from
bash](differences.md).

> **In short.** On Linux, at commit `3475d0aa`, psh passes 93.8% of the Oils spec cases
> and 90.0% of brush's, and every one of its 701 agent-shaped cases, 994 utility cases
> and 34,598 parser inputs; macOS is slightly lower and Windows clearly lower, mostly
> because cases call programs that platform lacks or prints differently. psh has 58
> utility names and 53 of bash's 61 builtins in about 128,000 lines of Rust and 43
> third-party crates, and its binary is 11.9 MB as built, 5.0 MB optimized for size and
> stripped, and 4.3 MB as the browser module.

## How to read the numbers

- **A case** is a script with its input, arguments and files. GNU bash 5.3 ran every case
  once in a pinned Linux container, and its stdout, stderr, exit status and the files it
  left are committed. A case **passes** when psh's stdout, exit status and files match
  that recording, after run-specific paths (the case's directory, the home directory)
  are normalized. stderr is compared too and scored apart: the "+ stderr" columns count
  the cases that also match there, so the wording of error messages can catch up
  without holding back the language.
- **Active, skip and later.** Every case belongs to one group. Skipped cases test things
  psh will never do (an interactive shell, history, completion, `select`, zsh-only
  idioms) or have results bash itself does not reproduce (process ids, clock times);
  they are not run and not counted. Later cases are run and reported apart. The
  percentage is passing active cases over all active cases.
- **An L0 input** passes when the parser accepts exactly what `bash -n` accepts, and the
  tree it builds, printed back to text and parsed again, comes out unchanged.
- **One recording for every platform.** macOS and Windows are compared against the same
  Linux recording, so their numbers include every way their own programs differ from
  Debian's GNU ones. Each platform keeps a list of those differences, case by case,
  with the reason.

## Linux

At commit `3475d0aa`, on Linux, against the recording:

| Layer | What it is | Active | Pass | % | Not counted |
| --- | --- | ---: | ---: | ---: | ---: |
| L0 parse | every case script, the shape corpus, the parser's own snippets | 34,598 | 34,598 | 100% | – |
| L1 Oils | the Oils project's spec tests | 2,511 | 2,355 | 93.8% | 234 skip |
| L2 brush | the brush shell's test cases | 3,171 | 2,855 | 90.0% | 465 skip, 51 later |
| L3 agent | cases shaped like real agent commands | 701 | 701 | 100% | – |
| L4 utilities | cases per in-process utility, recorded from GNU and BSD | 994 | 994 | 100% | – |

The L0 inputs are the 8,127 case scripts of L1 to L4, the 26,234 skeletons of the shape
corpus and 237 snippets written for the parser. The Oils and brush cases that still fail
are spread thinly over many files, a few in each: corners of `declare`, `trap`, signals,
`type` and the `extdebug` option, NUL bytes, and similar.

## macOS

The same code, measured for this page on macOS (Apple Silicon), against the same
recording:

| Layer | Active | Pass | % | + stderr |
| --- | ---: | ---: | ---: | ---: |
| L0 parse | 34,598 | 34,598 | 100% | – |
| L1 Oils | 2,511 | 2,302 | 91.7% | 2,225 |
| L2 brush | 3,171 | 2,815 | 88.8% | 2,708 |
| L3 agent | 701 | 699 | 99.7% | 697 |
| L4 utilities | 994 | 994 | 100% | 989 |

What fails here and not on Linux is the platform: the two agent cases pipe into `od`,
which psh does not carry and whose BSD output differs from GNU's; others depend on where
`ls` and `cat` live (macOS has no merged `/usr`), on `SIGUSR1` being 30, on root's home
being `/var/root`, or on `/dev/full`, which macOS lacks. 35 cases that test the host
itself are held to their result on Linux only.

## Windows

On Windows, natively, against the same recording:

| Layer | Active | Pass | % |
| --- | ---: | ---: | ---: |
| L1 Oils | 2,511 | 2,132 | 85% |
| L2 brush | 3,171 | 2,579 | 81% |
| L3 agent | 701 | 283 | 40% |
| L4 utilities | 433 | 357 | 82% |

Most agent cases that fail call a program Windows does not have (`sh`, `ps`, Python),
not a shell feature psh lacks there. Such a case may fail without being listed;
every other Windows difference is listed with its reason. [psh on Windows](windows.md)
explains what runs differently there.

## What psh contains

| What | Count | Notes |
| --- | ---: | --- |
| Utility names | 58 | 49 utilities; `gawk`, `gtimeout`, `egrep`, `fgrep` and the `sha*sum` family are further names of one of them; `psh --utils` lists them |
| Shell builtins | 53 of 61 | bash 5.3's list, as `enable -a` prints it |
| Third-party crates | 43 | every one, direct or transitive, on every target |

The 58 names are on [Utilities and their flags](ref-utilities.md) and the builtins on
[Supported syntax and shell builtins](ref-syntax.md). The eight builtins psh does not
run belong to an interactive shell: `bg`, `fg`, `bind`, `compgen`, `complete`,
`compopt`, `fc` and `suspend`. psh knows their names, so no program of that name runs in
their place; using one stops the script with status 2, since what follows likely depends
on it:

```text
$ psh -c 'type fg; fg; echo not reached'
fg is a shell builtin
psh: line 1: fg: not supported yet
$ echo $?
2
```

The parser, the classifier, the regular expression engine and the engine have no
third-party dependencies at all; the OS layer has `libc` and `windows-sys`. Nearly all
the others come with the utilities: 32 crates of ripgrep's set (its search, walking and
regex crates and what they pull in, `libc` and `windows-sys` among them) for `rg`, `fd`
and `grep`, and `jaq-core` with four small crates for `jq`. `deser-json` and three
crates of its own write JSON (`psh --parse`, `rg --json`), and the `psh` program's
command line adds two more.

## Test cases

| Set | Size |
| --- | ---: |
| Oils spec cases | 2,745 (2,511 active) |
| brush cases | 3,687 (3,171 active) |
| Agent-shaped cases | 701 |
| Utility cases | 994 |
| Parser inputs (L0) | 34,598 |
| Process tests (Rust, some per platform) | 69 |
| Labelled commands for the classifier | 645 |

The shape corpus holds 26,234 syntax skeletons of real agent commands, with every
private word replaced by a placeholder. They stand for 122,848 of the 295,401 commands
in the corpus they come from, and they cover every sequence of four syntax tokens that
occurs in it. The only words a skeleton keeps come from a reviewed list of 1,422
(command names, subcommands, options, well-known variables). On that corpus, the
classifier calls 52.1% of the commands read-only ([Understanding
commands](classify.md)).

## Code

Lines of Rust per crate, comments, blank lines and tests included:

| Crate | Lines | What it is |
| --- | ---: | --- |
| `psh-syntax` | 10,546 | the parser and printer |
| `psh-classify` | 5,099 | the classifier and the utilities' option tables |
| `psh-regex` | 2,134 | POSIX regular expressions |
| `psh-engine` | 25,210 | the interpreter, builtins, expansion, jobs |
| `psh-os` | 8,380 | the Unix and Windows hosts |
| `psh-utils` | 64,016 | the in-process utilities |
| `psh-wasm` | 1,369 | the WebAssembly modules |
| `psh` | 2,555 | the program |
| `psh-spec` | 8,031 | the conformance runner |
| `psh-docs` | 620 | the guide checker |
| **Total** | **127,960** | |

The utilities' count holds a good share of other projects' code, carried with its
copyright notice: of the 17,292 lines of `rg`, 16,784 come from ripgrep's command line
and the small crates it uses there, patched where they meet psh; about 3,300 lines of
`fd` come from fd and its color crates; and `awk` (8,049 lines) is a port of GoAWK's
interpreter. ripgrep's output crate is a patched copy of its own
(9,350 lines, outside the ten crates). The test runners under `tests/` add 1,516 lines.

## Size

| Artifact | Size | Gzipped |
| --- | ---: | ---: |
| `psh` binary, the release profile | 11.9 MB | – |
| `psh` binary, fat LTO, one codegen unit, stripped, optimized for size | 5.0 MB | 2.5 MB |
| `psh.wasm`: the shell and the utilities | 4.3 MB | 1.5 MB |
| `psh-parser.wasm`: the parser and the classifier | 0.39 MB | 0.15 MB |

Both WebAssembly modules are built optimized for size, with LTO and stripped. For the
native binary that configuration is a measurement, not a profile of the workspace.

## Speed

A few rough timings, best of three, on macOS (Apple M5 Max) with a release build, beside
Homebrew's bash 5.3 on the same machine:

| What | psh | bash |
| --- | ---: | ---: |
| start and exit: `-c true` | 2.9 ms | 4.4 ms |
| `true;` 100,000 times in one command (parsing included) | 81 ms | 22 ms |
| `for ((i=0;i<100000;i++)); do :; done` | 137 ms | 89 ms |
| `x=$(echo hi)` 200 times | 5 ms | 148 ms |
| `/bin/echo x` 200 times | 208 ms | 346 ms |
| `seq 200000 \| sort -r \| head -1` | 15 ms | 45 ms |

psh interprets plain loops more slowly than bash, about 1 µs per simple builtin
command. It wins wherever bash forks: `$(…)` of a builtin runs inside psh with no
process, starting a program uses `posix_spawn`, and `seq`, `sort` and `head` run
in-process. One slow path stands out: an unquoted word with an unmatched `[`, such as
the `[` of `[ "$i" -lt 5 ]`, reads the current directory every time it is expanded, so
`[ … ]` in a hot loop costs far more than `test …` or `[[ … ]]`.

## What is not built yet

- **Running inside an agent host's daemon.** An embedding interface for a long-lived
  daemon (both output streams in one, cancellation and a time limit per call), running
  commands through it, and the classifier in the host's user interface.
  [One agent host](pi.md) describes how it plans to use psh.
- **Shipping.** Release builds for every target, a size budget with cargo features for
  the heavy groups of utilities (search, `jq`, `awk`) so that an embedder can leave them
  out, and continuous integration on machines other than the development one.
- **Robustness for embedding.** Limits on output size and memory, a stress layer that
  cancels runs at random points, fuzzing of the engine, and benchmarks against bash.
- **Output control for agents.** Shortening long output and handling commands that run
  for a long time are not designed yet.
- **More utilities.** `nice`, `nohup`, process and host tools, small text tools and
  archives, as [Utilities that run in-process](utilities.md#what-is-built) lists.

## Known gaps

- On Windows, `$(…)` and `read` keep the `\r` of native programs' output: the Windows
  host does not strip CR line ends yet.
- `timeout -s SIG` stops the command with TERM and then KILL, not with SIG; `-k` and
  `--foreground` are accepted and do nothing. In the `psh` program, `timeout` and
  cancelling reach the program itself but not its children; the embedded hosts stop
  whole process trees.
- `time` prints no timing report, and `diff <(…) <(…)` fails with "Bad file descriptor".
- `set -m`, `set -o vi`, `set -o emacs`, `set -o history` and `FUNCNEST` are accepted
  and have no effect.
- psh ignores an inherited logical `$PWD`: under a symlinked `/tmp` it prints
  `/private/tmp/…` where bash prints `/tmp/…`.
- `fd --changed-within` and `--changed-before` run the real `fd`.
- In a `sed` regular expression with `\xHH` bytes, `.` matches a single byte, where
  GNU sed matches nothing.

## Where to look

- [tests/spec.toml](../../tests/spec.toml): the groups and their reasons;
  [tests/scores/status.tsv](../../tests/scores/status.tsv): the scores over time;
  [crates/psh-spec/src/status.rs](../../crates/psh-spec/src/status.rs): how the
  scoreboard is computed (`psh-spec --status`, `psh-spec --l0`).
- [tests/oils/expected-failures.txt](../../tests/oils/expected-failures.txt),
  [tests/brush/expected-failures.txt](../../tests/brush/expected-failures.txt) and
  [tests/agent/expected-failures.macos.txt](../../tests/agent/expected-failures.macos.txt):
  the cases that may fail, shared and per platform.
- [crates/psh-engine/src/builtins/mod.rs](../../crates/psh-engine/src/builtins/mod.rs):
  the builtins and bash's list;
  [crates/psh-utils/src/lib.rs](../../crates/psh-utils/src/lib.rs): the utilities.
- [scripts/deps-allowlist.txt](../../scripts/deps-allowlist.txt): every dependency and
  why it is allowed; [NOTICE](../../NOTICE): the code taken from other projects.
- [tests/shapes/shapes.txt](../../tests/shapes/shapes.txt) and
  [tests/shapes/vocabulary.txt](../../tests/shapes/vocabulary.txt): the shape corpus;
  [tests/classify/cases.txt](../../tests/classify/cases.txt): the classifier's cases.
- [scripts/build-wasm](../../scripts/build-wasm): builds both WebAssembly modules and,
  with `--sizes`, prints their sizes.
