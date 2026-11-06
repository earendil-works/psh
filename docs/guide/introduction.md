# What psh is

psh is a shell for coding agents. An agent writes a bash command, psh runs it on macOS,
Linux or Windows and returns what bash would have returned: output, errors and an exit
status. A host that runs agents can also ask psh what a command will do before running
it, embed psh as a library, or run it in a browser over a virtual file system. This page
explains what kind of command psh is built for, the parts it consists of, and how to
read the rest of the guide.

> **Not built yet.** psh is at the design stage. This guide describes the design; the
> status page will say what is built and measured.

> **In short.** psh runs bash as agents write it, with the semantics of GNU bash 5.3, the
> same way on every platform and without any Unix layer on Windows. Common utilities such
> as `grep`, `sed`, `head` and `timeout` run inside psh, and fall through to the real
> program for anything they don't support. One parser serves both running commands and
> understanding them.

## A command an agent writes

```sh
cd src && grep -rn "fn main" --include='*.rs' . 2>&1 | head -20
```

A command like this is typical. It changes directory, searches, merges error output into
the result and keeps only the first lines. Each part says something about what a shell
for agents has to do well:

| What agents do | What psh needs to do | Goal |
| --- | --- | --- |
| Use a narrow, predictable part of bash: `&&`, pipes, `2>&1`, here-docs | Get those exactly right first, matching bash | [agent-first](goals.md#agent-first), [bash's behavior](goals.md#bashs-behavior) |
| Run the same commands on macOS, Linux and Windows | Run natively everywhere, with no Git Bash on Windows | [portable](goals.md#portable) |
| Mix GNU and BSD flags (`sed -i ''`, `grep -P`, `timeout`) | Ship one set of utilities that accepts both | [consistent](goals.md#consistent) |
| Mostly read: about half of all commands change nothing | Let the host see that before running the command | [understandable commands](goals.md#understandable-commands) |
| Start servers, poll, forget processes; get cancelled | Track every process and kill whole trees | [control of execution](goals.md#control-of-execution) |
| Run inside a host process, sometimes in a sandbox | Embed with no async runtime; run over a virtual file system | [embeddable](goals.md#embeddable), [sandboxable](goals.md#sandboxable) |

## The shape of the system

psh is a set of Rust libraries and one program:

- **The parser** turns a command string into a syntax tree in which every node knows its
  byte range in the original text, and can print a tree back into a command. It has no
  dependencies and builds for WebAssembly on its own.
- **The classifier** walks that tree and reports what each command does: the programs it
  calls, the files it reads and writes, and a category such as read, mutate, build or
  network.
- **The engine** runs the tree. It is a state machine the host steps: each step does a
  bounded amount of work and returns, so the host keeps its thread. Subshells, pipeline
  stages and background jobs are virtual processes inside the engine; nothing forks.
- **Hosts** carry out every side effect the engine asks for: starting programs, reading
  files, waiting. There is one for Unix, one for Windows, one in JavaScript for the
  browser, and an in-memory one for tests.
- **Utilities** are in-process versions of common programs (`head`, `grep`, `sed`, `jq`,
  `timeout`, ...). Each one runs the real program, or fails with a clear message, when it
  meets a flag it does not support.
- **The `psh` program** runs `psh -c '…'` and scripts like `bash` does, and can print the
  syntax tree or the classification of a command.

## Reading the guide

The first part, "Using psh", is for people who run psh or embed it: the language it
accepts, its utilities, Windows, classification, embedding. The second part, "How it
works", explains the parser, the engine, the host interface and the process layers, and
how psh is tested against bash. [What psh optimizes for](goals.md) is the page the others
refer back to.
