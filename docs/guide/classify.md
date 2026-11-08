# Understanding commands: parsing and classification

A host that runs commands an agent wrote often wants to report what a command does: show
a person a one-line summary, list the files it reads and writes, or know whether it only
reads, so that running it again after a crash changes nothing. psh answers that with the
same parser that runs the command, so the report and the shell can never disagree about
what the text means. The classifier is for reporting: psh is not a sandbox, and the
classification is not a permission check. This page explains what the classifier
reports, what counts as read-only, why it errs on the side of "unknown", and how a host
uses it.

> **In short.** The classifier walks the syntax tree of a command and lists every
> program it would run, including those hidden in `$(…)`, `bash -c '…'` or behind
> `timeout`, with the files each one reads and writes and a category such as read,
> mutate, build or network. A command is read-only only if every part of it provably
> only reads. Anything the classifier cannot see through, such as a command name in a
> variable or an inline Python script, is "unknown", never read-only.

## What a host gets

`psh --classify` prints the result as JSON; `psh --parse` prints the syntax tree it is
built on. For a commit message that hides a deletion:

```sh
psh --classify -c 'git commit -m "$(rm -rf build)"'
```

the answer (shortened) is:

```json
{
  "ok": true,
  "primary": "vcs",
  "categories": ["mutate", "vcs"],
  "read_only": false,
  "truncated": false,
  "commands": [
    { "name": "git", "span": [0, 31], "parent": null, "via": "script",
      "categories": ["vcs"], "words": [ … ] },
    { "name": "rm", "span": [17, 29], "parent": 0, "via": "command-substitution",
      "categories": ["mutate"], "words": [ … ] }
  ],
  "writes": [ { "path": "build", "span": [24, 29], "command": 1 } ]
}
```

For the whole command, it reports:

- **categories**, and the **primary** one for a short summary;
- **read_only**: true only when the command provably only reads;
- **commands**: every simple command that would run, in order, each with its name, its
  words (as written, with the value after quote removal when the word has no
  expansions), its redirections, its categories, which command it runs inside of
  (`parent`) and how (`via`), and, for an unknown command, the reason;
- **reads** and **writes**: the paths it reads and writes, with the position of each in
  the text;
- **truncated**: whether the output is cut short, as in `| head -20`.

Every position is a byte range into the original text, so a user interface can
highlight exactly the word that writes a file. This is what the
[understandable commands](goals.md#understandable-commands) goal asks for.

The live example below runs a script and also classifies it: its Classification tab
lists each command with its categories and how it comes to run, and the paths the
script reads and writes. There is no `git` where live examples run, so running it fails
at `git`; the classification needs nothing but the text and appears without waiting
for execution to finish.

```sh live
git commit -m "$(rm -rf build)"
find . -name '*.log' | xargs rm
timeout 600 cargo test
```
<!-- gen:run -->
```text
psh: line 1: git: command not found
psh: line 3: cargo: command not found
(exit status 127)
```
<!-- /gen -->

## Categories

| Category | Means | Examples |
| --- | --- | --- |
| read | reads files or state, changes nothing outside the shell | `ls`, `grep -rn x .`, `sed -n 1,20p f`, `git status`, `cd`, `ps` |
| network | talks to the network | `curl https://…`, `gh pr view`, `git ls-remote` |
| build | builds, tests or lints, running the project's code | `cargo test`, `npm run build`, `make`, `pytest` |
| long-running | runs until stopped | `npm run dev`, `tail -f log`, `cmd &` |
| mutate | changes files or processes | `rm`, `sed -i`, `> out.txt`, `kill`, `curl -o file` |
| vcs | changes a repository | `git commit`, `git push`, `gh pr create` |
| install | installs or removes packages | `npm install`, `cargo add`, `brew install` |
| unknown | cannot be known without running it | `$cmd`, `python3 - <<'EOF'`, `./script.sh`, `eval "$x"` |

A command can have several categories: `git push` is vcs and network, `npm run dev` is
build and long-running. The primary category is the last one in the table that is
present, so a command that builds and writes a file shows as mutate, and anything with
an unknown part shows as unknown.

## Read-only, and why the classifier is conservative

A host uses the read-only flag in two places. A harness that replays tool calls after a
crash replays a read-only command, because running it again changes nothing; a summary
shows it as a command that changes nothing. Both are only right if "read-only" is a
promise: every command in it is known, reads only, and nothing is written except to
`/dev/null` and the standard streams.

Anything the classifier cannot see through makes the command unknown:

- a command name that is not literal (`x=rm; $x -rf build`), `eval` or `bash -c` of a
  non-literal string, `source`;
- inline interpreters and scripts (`python3 -c`, `node -e`, `python3 - <<'EOF'`,
  `./run.sh`), and programs psh has no knowledge of;
- functions and aliases defined in the same script: `ls() { rm -rf /; }; ls` is not a
  listing;
- setting a variable that changes which program runs (`PATH=/tmp:$PATH ls`,
  `LD_PRELOAD=…`, `GIT_SSH_COMMAND=…`);
- a word from a variable where an option could be, for a program that has an option
  that writes or runs code: `rg "$pattern" src` is unknown, because a pattern of
  `--pre=./x` makes `rg` run `./x`; `grep "$pattern" src` is read, because no `grep`
  option writes or runs anything; and `cat "src/$f"` is read, because a word that starts
  with `src/` cannot be an option;
- `sed` scripts with the `e` command, awk programs that call `system` or redirect
  `print`, `git` with `-c` or an alias as the subcommand, shells that load startup files
  (`bash -l`), and `xargs` or `find -exec` splicing file names into `sh -c`.

> **Trade-off.** False "unknown" over false "read-only". An unknown command is treated as
> one that may change things: it is not replayed, and its summary does not claim it is
> harmless. A wrong "read-only" would replay a deletion after a crash and tell the user
> that nothing changed. The cost: some harmless commands are unknown, such as
> `sed -n "$(grep -n x f | cut -d: -f1),+5p" f`.

## Commands inside commands

The classifier follows every way bash runs a command inside another one:

- `$(…)`, backquotes, `<(…)` and `>(…)`, in any word, including assignment values,
  redirection targets, `${x:-…}` defaults and the bodies of unquoted here-documents;
- literal shell code in strings: `bash -c '…'`, `eval '…'`, `trap '…' EXIT`, alias
  bodies, and a literal here-document fed to a shell (`bash <<'EOF'`); its commands
  carry `source` (an index into `sources`), and their positions are into that string;
- wrappers: `env`, `timeout`, `nice`, `nohup`, `command`, `exec`, `sudo`, `uv run`,
  `xargs` and `find -exec` run the command that follows them, which is reported as a
  command of its own with the wrapper as its parent.

So `timeout 600 cargo test` is build, `echo ok && bash -c 'rm x'` is mutate, and
`find . -name '*.log' | xargs rm` reports the `rm`.

## Variables with a known value

Agents often name a path once and use it several times:

```sh
S=src/lib.rs; sed -n 1,40p $S; rg -n "fn main" $S
```

The classifier substitutes a variable's value when it can be sure of it: every
assignment to the variable anywhere in the script gives it the same literal value, and
one of them has certainly run before the use, as an earlier command of an enclosing list
in the same shell. This command is read-only. These are not, because the value at the
use cannot be known:

- `S=/x; if c; then S=-i; fi; sed $S s/a/b/ f` (another value in a branch);
- `cd x && S=/x; sed -n 1p $S` (the assignment does not run when `cd` fails, and `S`
  then comes from the environment);
- `(S=/x); sed -n 1p $S` (assigned in a subshell);
- `S=/x; read S; …`, `for S in …`, `unset S`, `((S++))`, `${S:=…}` (other writes);
- any script with `eval`, `source`, `trap`, aliases or name references (`declare -n`),
  which can assign variables out of sight.

An unquoted variable whose value contains spaces or glob characters is not substituted,
since bash would split or expand it. `$HOME` and `$PWD` are not substituted, but a word
that starts with them is known to be a path, not an option.

## Using the classifier from a host

From Rust, the `psh-classify` crate has the same information as the JSON. It builds
without the standard library and for WebAssembly, so a host can classify where it cannot
run anything, with the [embeddable](goals.md#embeddable) goal's constraints.

```rust
use psh_classify::{Category, classify};

let c = classify("cd src && grep -rn TODO . | head -20");
assert!(c.read_only());
assert!(c.truncated);

let c = classify("git commit -m \"$(rm -rf build)\"");
assert_eq!(c.primary(), Category::Vcs);
for cmd in &c.commands {
    println!("{:?} {:?} {:?}", cmd.name, cmd.categories.primary(), cmd.reason);
}
```

A script that does not parse is unknown, with the parser's error in `error`.

## How it does on real commands

On a corpus of about 295,000 commands run by coding agents, the classifier calls 52.1%
read-only. A separate study of the same corpus, which labelled commands by the programs
they call without psh's conservative rules, counted 51.7% as pure reads; the two agree
on about 151,000 commands. Most of the difference is in both directions at once: psh
counts `ps`, `sleep`, `command -v` and `--version` queries as reads, and refuses the
dynamic-option cases above.

## Where to look

- [crates/psh-classify/src/lib.rs](../../crates/psh-classify/src/lib.rs): the result
  types and the entry points.
- [crates/psh-classify/src/walk.rs](../../crates/psh-classify/src/walk.rs): the walk
  over the syntax tree and nested commands.
- [crates/psh-classify/src/rules.rs](../../crates/psh-classify/src/rules.rs): the rules
  for programs with subcommands, wrappers, shells and interpreters.
- [crates/psh-classify/src/utils.rs](../../crates/psh-classify/src/utils.rs): what psh
  knows about each utility's options and operands, shared with the in-process
  utilities.
- [crates/psh-classify/src/vars.rs](../../crates/psh-classify/src/vars.rs): variables
  with a known value.
- [tests/classify/cases.txt](../../tests/classify/cases.txt): the labelled cases,
  including the adversarial ones.
