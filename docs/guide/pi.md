# psh in pi

pi is the coding agent psh was built for: a command-line agent, a mobile app, and the
agent harness and daemon behind them. Every shell command pi's agents run goes through
one tool, and that tool is where psh fits. This page explains how pi runs commands
today, how psh plugs into pi's daemon, what pi can use already, and how another agent
host can do the same.

> **In short.** pi runs every command as a fresh `bash -c` with whatever shell it finds,
> and knows nothing about the command beyond its text. psh is designed to run inside
> pi-env, pi's small synchronous daemon, as the in-process shell of its `exec`
> operation, with a `parse` operation that tells pi what a command does before it runs.
> Until that lands, the `psh` binary, its JSON output and the parser's WebAssembly
> module already work for pi and for any other agent host.

## How pi runs commands today

Every shell command, whether the model's tool call or a user's `!cmd`, starts a new
process: `bash -c '<command>'`. Which bash that is depends on where pi runs:

| Where | Shell |
| --- | --- |
| macOS | `/bin/bash`, which is bash 3.2 |
| Linux | `/bin/bash`, else `bash` on `PATH`, else `sh` |
| the mobile app, locally | `$SHELL`, so zsh on most Macs |
| Windows | Git Bash, or the WSL `bash.exe` when that comes first on `PATH` |

The lookup exists three times, in the command-line agent, in the harness and in the
daemon, and the copies are kept in sync by hand. The tool contract is the same
everywhere: a command string (and an optional timeout) in; out comes stdout and stderr
combined in arrival order, cut to the last 2000 lines or 50 KB with the rest in a spill
file, and the exit code, `128+n` when a signal ended the command.

pi has no shell parser. The user interface shows the raw command string, or its first
line. Permission and plan-mode checks are extensions that match regular expressions
against the text, which `$(…)`, `eval`, `bash -c` or a `;` after a harmless prefix get
around. The harness marks its shell tool as unsafe to replay after a crash, because it
cannot tell a command that reads from one that writes.

Each of these is a goal of psh's. Agents meet bash 3.2 on one machine and zsh on
another, where an unmatched glob is an error ([consistent](goals.md#consistent)). On
Windows, killed pipelines leave processes running for hours and console output arrives
in the wrong code page ([portable](goals.md#portable),
[control of execution](goals.md#control-of-execution)). And a host that can only match
patterns cannot approve or summarize safely
([understandable commands](goals.md#understandable-commands)).

## pi-env, the daemon

pi-env is a small Rust daemon that runs commands and file operations for pi, locally or
on a remote machine. It is synchronous: each `exec` gets its own threads, and there is
no async runtime. Its stripped release build is about 1.9 MB. It ships for eight targets
(Linux with musl, Android, macOS and Windows, each on x64 and arm64), is copied over SSH
under a name derived from its SHA-256 hash, and is verified before every start. Its
first message, `hello`, lists the features the daemon offers.

This is the host psh's [embeddable](goals.md#embeddable) goal is shaped by: the engine
brings no async runtime, a host steps it from a thread it owns, psh builds with the same
Rust toolchain as the daemon, and every dependency counts against the binary's size.

## psh inside pi-env

> **Not built yet.** The engine's embedding interface exists and the test runner uses
> it; the daemon side is designed and not written.

The plan plugs psh in at the daemon's `exec` operation:

- The daemon lists `psh` in `hello`'s features, and `exec { command, shell: "psh" }`
  runs the command in psh. `shellPath` still selects a real bash, zsh or Cygwin, for
  users who want their own shell.
- One thread per `exec` creates a shell and drives it: step, and park while the shell
  waits on the operating system. A cancel or a timeout goes through the shell's control
  handle: `SIGTERM`, a grace period, then `SIGKILL`.
- Embedded, psh installs no signal handlers. Every program it starts leads a session of
  its own on Unix and runs in a job object of its own on Windows, and dropping the host
  at the end of the call kills what the run left behind, so nothing outlives the call
  unless it was started with `nohup`, `setsid` or `disown`
  ([Processes on Unix](processes.md#what-survives-a-run),
  [psh on Windows](windows.md#exit-statuses-ctrl-c-and-stopping-commands)).
- Output stays combined in arrival order and goes through the daemon's existing
  decoding, windowing and spill, so pi's tool output does not change shape. Exit codes
  follow the same contract: `128+n` for a signal.
- psh's in-process utilities are a cargo feature of the daemon, on by default. A build
  for a size-constrained target can leave them out; every command is then a real
  program, as with bash.
- Execution in Node (the harness's local environment and the mobile app's local shell)
  goes through the daemon to reach psh, not through a separate binary or a Node addon.

> **Trade-off.** A fresh shell per `exec` over a session that lives across calls. The
> harness may replay a tool call after a crash, which is only sound when a call depends
> on nothing but its own text; agents already write `cd dir && …` in every command
> because `bash -c` never kept state either. The cost: `cd`, `export` or a function
> defined in one call does not reach the next one.

## What a command does, before it runs

> **Not built yet.** The classifier exists and is described in
> [Understanding commands](classify.md); the daemon's `parse` operation and pi's use of
> it are planned.

A `parse` operation returns the classification of a command, even where execution still
uses a real bash. A typical agent command:

```sh
psh --classify -c 'cd src && grep -rn foo . | head -20'
```

gives (shortened):

```json
{
  "ok": true,
  "error": null,
  "primary": "read",
  "categories": ["read"],
  "read_only": true,
  "truncated": true,
  "commands": [
    { "span": [0, 6], "name": "cd", "parent": null, "via": "script",
      "primary": "read", "categories": ["read"], "reason": null, "words": [ … ] },
    { "span": [10, 24], "name": "grep", "parent": null, "via": "script",
      "primary": "read", "categories": ["read"], "reason": null, "words": [ … ] },
    { "span": [27, 35], "name": "head", "parent": null, "via": "script",
      "primary": "read", "categories": ["read"], "reason": null, "words": [ … ] }
  ],
  "reads": [ { "path": ".", "span": [23, 24], "source": null, "command": 1 } ],
  "writes": [],
  "sources": []
}
```

With that, pi can:

- summarize a command in the interface ("read 3 files") instead of printing its first
  line;
- run a read-only command without asking, under the classifier's
  [conservative rule](classify.md#read-only-and-why-the-classifier-is-conservative);
- check permissions on the commands and paths in the tree instead of on the text;
- mark a read-only command as safe to replay in the harness.

An approval hook at run time is planned as well: the host sees each command after
expansion, just before it runs, and can refuse or rewrite it. That covers what the
classifier has to call unknown, such as `$cmd` or `eval "$x"`, whose meaning only exists
at run time.

## What pi can use today

These exist and need no change to the daemon:

- **The `psh` binary as `shellPath`.** `psh -c '…'` takes what `bash -c '…'` takes.
  Standalone, psh keeps the programs it starts in its own process group, as a
  non-interactive bash does, so pi's group kill on cancel reaches all of them, and
  background jobs outlive psh as they outlive bash.
- **`psh --parse` and `psh --classify`**, which print the syntax tree and the
  classification as JSON, with byte ranges into the original text.
- **The parser's WebAssembly module**, which offers the same two functions to
  TypeScript without a process: `psh.parse(source)` and `psh.classify(source)` return
  the same JSON ([psh in the browser](wasm.md)).
- **The read-only rule**, through any of the three.

## For another agent host

Nothing above is specific to pi. Another host has the same three ways in, in order of
effort:

1. **Run the binary** where it runs bash today, and keep its timeout and group kill.
2. **Load the WebAssembly modules**: the parser module to classify in JavaScript, or the
   whole shell over a virtual file system, with commands the host writes in JavaScript.
3. **Embed the engine** in a Rust host, as pi-env will. A shell holds reference-counted
   state, so it is created and stepped on one thread; its control handle can be sent to
   another thread to cancel it. A host that runs one command and returns combined
   output looks like this on Unix:

```rust
use std::io::Read;
use std::rc::Rc;
use std::time::{Duration, Instant};
use psh_engine::std_host::{drive_until, engine_thread};
use psh_engine::{HostFile, Script, Shell, ShellConfig};
use psh_os::unix::{UnixConfig, UnixFile, UnixHost};

let (mut reader, writer) = std::io::pipe()?;
let deadline = Instant::now() + Duration::from_secs(120);
let engine = engine_thread(move || {
    // The default: a session per program, leftovers killed when the host drops.
    let host = Rc::new(UnixHost::new(UnixConfig::default()));
    let mut cfg = ShellConfig::new(host);
    cfg.utilities = psh_utils::policy(false);
    cfg.env = env;
    cfg.cwd = cwd;
    // One pipe for both streams: output arrives combined, in order.
    let out: Rc<dyn HostFile> = Rc::new(UnixFile::new(writer));
    cfg.stdout = Some(out.clone());
    cfg.stderr = Some(out);
    // Background jobs get half a second to finish, then they are killed.
    cfg.exit_grace = Some(Duration::from_millis(500));
    let mut shell = Shell::new(cfg);
    shell.start(Script::command(&command));
    drive_until(&mut shell, Some(deadline)) // None: timed out and cancelled
})?;
let mut output = Vec::new();
reader.read_to_end(&mut output)?;
let status = engine.join().unwrap();
```

`drive_until` is the simplest driver: it parks the thread while the shell waits. A host
with its own event loop gives the shell a waker instead and calls `step` when it fires.

## Where to look

- [crates/psh-engine/src/lib.rs](../../crates/psh-engine/src/lib.rs): `Shell`,
  `ShellConfig`, `Step` and `ControlHandle`, the embedding interface.
- [crates/psh-engine/src/std_host.rs](../../crates/psh-engine/src/std_host.rs):
  `engine_thread` and the `drive` and `drive_until` loops.
- [crates/psh-os/src/unix/mod.rs](../../crates/psh-os/src/unix/mod.rs) and
  [crates/psh-os/src/windows/mod.rs](../../crates/psh-os/src/windows/mod.rs): the OS
  hosts and what happens to a run's leftovers.
- [crates/psh-spec/src/engine.rs](../../crates/psh-spec/src/engine.rs): an in-process
  embedder, the test runner, with a leak check after every case.
- [crates/psh/src/main.rs](../../crates/psh/src/main.rs): the standalone binary's
  setup, and [crates/psh/src/classify_json.rs](../../crates/psh/src/classify_json.rs):
  the classification as JSON.
- [crates/psh-wasm/src/lib.rs](../../crates/psh-wasm/src/lib.rs) and
  [js/psh.mjs](../../js/psh.mjs): the WebAssembly modules and their JavaScript glue.
