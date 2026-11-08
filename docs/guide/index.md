# psh

<p class="lead">A bash-compatible shell for the commands coding agents write. psh runs
them with GNU bash 5.3's behavior on macOS, Linux and Windows, without a Unix layer,
runs common utilities inside itself, and can tell its host what a command will do
before it runs.</p>

<p class="cta"><a class="button" href="introduction.html">Read the guide</a><a class="button secondary" href="https://github.com/earendil-works/psh">Source on GitHub</a></p>

The example below is psh itself, compiled to WebAssembly and running in this page over
an in-memory file system. Edit it and run it again; the Classification tab shows what
psh's classifier reports about the script without running it.

```sh live
printf '%s\n' src/main.rs src/lib.rs docs/guide.md tests/cli.rs > files.txt
grep -c '\.rs$' files.txt
cut -d/ -f1 files.txt | sort | uniq -c | sort -rn | head -2
rm files.txt
```
<!-- gen:run -->
```text
3
      2 src
      1 tests
```
<!-- /gen -->

<div class="features">

- **[Bash as agents write it](language.md)** Pipelines, redirections, here-documents,
  functions, arrays, every parameter expansion, `[[ ]]`, `set -e` with bash's
  exemptions, traps and background jobs, checked case by case against recorded bash.
- **[Portable](goals.md#portable)** Native on macOS, Linux and Windows. On Windows it
  needs no Git Bash, MSYS or WSL: it quotes arguments the way Windows programs read
  them and handles `.cmd` files, `PATHEXT` and Ctrl-C itself.
- **[Utilities in-process](utilities.md)** `grep`, `sed`, `awk`, `jq`, `rg`, `fd`,
  `find`, `xargs`, `sort`, `timeout` and dozens more behave the same everywhere, accept
  both GNU and BSD flags, and hand anything they don't support to the real program.
- **[Understandable commands](classify.md)** The parser keeps every byte's position, and
  the classifier lists each command a script would run, nested ones included, with the
  files it reads and writes and whether it only reads.
- **[Embeddable](embedding.md)** The engine is a state machine its host steps, so the
  host keeps its thread, with no async runtime. Cancelling a command kills its whole
  process tree.
- **[In the browser](wasm.md)** The whole shell builds for WebAssembly and runs over a
  file system and commands written in JavaScript, with no async transform of the module.

</div>

psh passes 93.8% of the Oils spec cases and every one of its 701 cases shaped like agent
commands on Linux; [Status in numbers](status.md) has the figures for every platform.
[What psh is](introduction.md) is the place to start.
