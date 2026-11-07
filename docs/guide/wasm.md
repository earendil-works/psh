# psh in the browser

psh builds for WebAssembly and runs in any JavaScript host: a browser page, a worker,
node. A page can classify the commands an agent writes without a server, or run them in
a shell whose files live in memory and whose extra commands are JavaScript functions.
This page explains the two modules, how they talk to JavaScript, the API in
`js/psh.mjs`, and what a shell without an operating system can and cannot do.

> **In short.** One module only parses and classifies; the other is the whole shell with
> its utilities, over a host written in JavaScript. The engine never waits inside
> WebAssembly: every file, pipe, timer and command is a request the JavaScript side
> answers at once or later, so the module needs no async transform. There are no
> programs to start, so a command that is neither a builtin, a utility nor a JavaScript
> command is "command not found", and the host learns which names were missing.

## Two modules

| Module | What it has | Size (gzipped) |
| --- | --- | --- |
| `psh-parser.wasm` | `parse` and `classify` | about 390 KB (150 KB) |
| `psh.wasm` | the same, plus the shell and all utilities | about 4.3 MB (1.5 MB) |

The parser module is for hosts that only want to understand commands. Its two functions
return exactly the JSON `psh --parse` and `psh --classify` print (see
[Understanding commands](classify.md)), so a check in a browser and one on the command
line agree. The shell module runs the utilities in sandbox mode: every utility runs
in-process, whatever its mode elsewhere, since there is no real program to prefer (see
[Utilities that run in-process](utilities.md)).

`scripts/build-wasm` builds both for `wasm32-unknown-unknown` into `target/wasm/`;
`--sizes` prints only their sizes:

```text
$ scripts/build-wasm --sizes
psh.wasm          4255930 bytes, 1494528 gzipped
psh-parser.wasm    389947 bytes,  148924 gzipped
```

## Why there is no wasm-bindgen, Asyncify or JSPI

A shell waits all the time: for a file, a pipe, a timer, a command. In WebAssembly the
usual ways to wait for JavaScript are Asyncify, which rewrites the module to unwind and
resume its stack, and JSPI, which not every JavaScript engine has. psh needs neither: an
embedder steps its engine, and every host operation may finish later (see
[The engine](engine.md)). The module imports one function,
`host_call(kind, op, arg, ptr, len)`. Every host operation (open, stat, read, write,
spawn, sleep, ...) is a call with a fresh operation id. The JavaScript side answers by
calling the export `psh_complete(op, errno, value, len)`, either during the call or later
from its event loop. The answer wakes the waiting task, the module asks JavaScript to
step the shell again, and the step continues the script. Bytes cross through two
buffers: `psh_in` returns where JavaScript writes the input of the next call, and
`psh_out_ptr`/`psh_out_len` point at the last result. Files, pipes and commands live on
the JavaScript side; the module only holds integer handles to them.

> **Trade-off.** A hand-written ABI of integers and buffers over generated bindings.
> The module stays one ordinary binary that any JavaScript engine instantiates, with no
> glue generator and no stack rewriting, which also keeps it small. The cost: the
> payload layouts are written twice, in Rust and in `js/psh.mjs`, and must change
> together; the module carries a version number that the glue checks when it loads.

## Running a command from JavaScript

`js/psh.mjs` is the glue and a default host, `MemHost`. `loadPsh` takes the module's
bytes, a `Response` or a compiled `WebAssembly.Module`, and either module:

```js
import { readFile } from "node:fs/promises";
import { loadPsh, MemHost } from "./psh.mjs";

const parser = await loadPsh(await readFile("psh-parser.wasm"));
const c = parser.classify('git commit -m "$(rm -rf build)"');
console.log(c.ok, c.json.primary, c.json.read_only, c.json.commands.map((x) => x.name));

const host = new MemHost();
const psh = await loadPsh(await readFile("psh.wasm"), { host });
host.fs.writeFile("/work/notes.txt", "cherry\napple\nbanana\n");
host.command("shout", async (ctx) => {
  const text = await ctx.stdin.text();
  await ctx.stdout.write(`${ctx.args.join(" ")}: ${text.toUpperCase()}`);
  return 0;
});

const r = await psh.run("sort notes.txt | head -2 | shout first two; git status", {
  cwd: "/work",
});
console.log(r.status, JSON.stringify(r.stdout), JSON.stringify(r.stderr));
console.log(host.missing.includes("git"));
```

Under node this prints:

```text
true vcs false [ 'git', 'rm' ]
127 "first two: APPLE\nBANANA\n" "psh: line 1: git: command not found\n"
true
```

`parse` and `classify` return `{ ok, json }`. `psh.run(source, options)` runs one
command in a fresh shell and returns `status`, `stdout` and `stderr` (strings), and
`stdoutBytes` and `stderrBytes`; it takes the options of both calls below.

For state that lasts, `psh.createShell({ cwd, env, stdin, onStdout, onStderr,
maxDepth })` makes a shell whose `run(source, options)` runs scripts one after another,
keeping variables, functions and the working directory. `onStdout` and `onStderr` get
each chunk as it is written. A run takes `options` (command-line options as `psh` takes
them: `["-e", "-o", "pipefail"]`), `arg0` and `args` (`$0` and the positional
parameters), `budget` (about how many commands one step may run) and `signal`, an
`AbortSignal` that cancels the run with status 143. `shell.free()` releases the shell
and everything it holds.

## Commands written in JavaScript

`host.command(name, async ctx => status)` adds a command. Every `PATH` lookup finds it
as `/usr/bin/NAME`, so `type shout` prints `shout is /usr/bin/shout`. The function gets
a context:

| Field | What it is |
| --- | --- |
| `argv`, `args` | the words as strings, with and without the command name; `argvBytes` as bytes |
| `env`, `cwd` | the command's environment (an object) and working directory |
| `stdin` | `read(max)` (a chunk, or `null` at the end), `bytes()`, `text()`, and `for await` |
| `stdout`, `stderr` | `write(text or bytes)`; it rejects with `EPIPE` when nobody reads |
| `fs`, `path(p)` | the host's file system, and `p` made absolute against `cwd` |
| `signal` | an `AbortSignal`, aborted when the shell signals the command |

The returned number is the exit status. A function that throws prints `NAME: message` on
its standard error and exits with 1; one that stops on `EPIPE` counts as killed by
SIGPIPE (141). Commands may wait on anything: a timer, a `fetch`, a message from the
page.

A utility of the same name runs before a JavaScript command, as it runs before a program
on `PATH` natively: `host.command("cat", ...)` is never called, and `type cat` says
`cat is a psh utility`. A JavaScript command is the way to add what psh does not have,
such as a `git` that talks to a server.

Every name the shell looked up and did not find is appended to `host.missing`, once per
lookup, so a host can tell a missing program from a command that failed.

## The file system, the clock and user names

`MemHost` keeps its files in a `MemFs`: directories, files and symlinks, with modes,
hard links and times in nanoseconds, but no permission checks. A new host has
`/dev/null`, `/dev/zero`, `/dev/full` and `/dev/urandom`, a sticky `/tmp`, and empty
`/bin`, `/etc`, `/home`, `/usr/bin`, `/usr/lib` and `/var`. `host.fs` offers
`writeFile(path, data, mode)` (which makes the parents), `readFile`, `mkdir`,
`symlink`, `remove`, `lookup`, and `walk(path)` for snapshots. `MemFs.lazyDir(load)`
makes a directory whose entries are read on first use and `mount(path, node)` puts it
anywhere, which is how a host shows part of a real file system without copying it.

The host answers a few questions besides files, and each is a method a host may
replace: `now` (milliseconds since the epoch; `new MemHost({ now })`, used for
`$EPOCHSECONDS` and dates), `userName(uid)` and `groupName(gid)` (by default uid and gid
0 are `root` and 1000 is `user`), and `utcOffset(epochSecs)` (by default the JavaScript
engine's local zone). With `now` set to 2025-01-02 03:04:05 UTC, `utcOffset` returning
3600 and `userName` answering `ada` for 1000:

```text
$ awk 'BEGIN { print strftime("%F %T %z") }'
2025-01-02 04:04:05 +0100
$ touch /tmp/f; find /tmp/f -printf '%u %g %m\n'
ada user 644
```

A host with other storage (the browser's origin private file system, a remote machine)
implements the same methods; the module cannot tell the difference.

## What works, and what does not

The language works as it does natively: pipelines, substitutions, here-documents,
functions, arrays, traps, background jobs and `wait`. Pipeline stages, subshells and
`$(…)` are tasks inside the engine, not processes, so nothing needs `fork`. What is
missing follows from having no operating system:

- **No programs.** Nothing outside the shell exists unless the host adds it, so `git`,
  `python`, `node` and `make` are "command not found" (127), as are utilities psh does
  not have yet (`ls`, `grep`, `sed`). This mode suits exploring and editing files, not
  building a project, unless the host supplies such commands.
- **Signals cannot stop JavaScript.** A JavaScript function cannot be interrupted from
  outside. When the shell signals a JavaScript command (`kill`, `timeout`, a cancelled
  run), the command counts as killed at once, its `signal` is aborted with the signal's
  number, and whatever it writes later goes nowhere. A function that should stop early
  has to watch `ctx.signal`.
- **No processes.** `$$` is the host's `pid` (4242 by default) and `$OSTYPE` is
  `unknown`. A few signal cases (a subshell signalling itself, a shell killed by a
  signal) differ from bash, since there is no process for a signal to end.
- **Blocking utilities read their input first.** `rg` is built on synchronous code,
  which runs inside the step in WebAssembly. It reads its standard input to the end
  before it searches, within a bound of 64 MiB; past it, it fails instead of hanging:

  ```text
  $ seq 10000000 | rg 99999 | head -1; echo ${PIPESTATUS[@]}
  rg: input too large for the in-process rg in this environment
  141 2 0
  ```

  Natively the same pipeline prints `99999` and ends as soon as `head` has its line.

## Stack and nesting

A function call deepens the stack of the engine (see
[The engine](engine.md#stack-and-the-nesting-limit)), and WebAssembly's default stack
for Rust is 1 MiB, far too small. The shell module links a 16 MiB stack, the same budget
a native embedder gives the engine, and caps the nesting limit at 768 levels, three
quarters of the stack at 16 KiB per level. A deeper recursion is an error, not a trap:

```text
$ f() { f; }; f
psh: line 1: f: maximum function nesting level exceeded (768)
```

`createShell({ maxDepth })` may set a lower limit, never a higher one. Linear memory
starts above the stack, so an instance takes about 17 MiB before it allocates anything.
A panic inside the module is reported to JavaScript, which throws `psh panicked: ...`;
the instance cannot be used after that, and the host loads a new one.

## How it is tested

Every conformance case also runs in the shell module under node, through `MemHost`.
Each case gets a fresh host with its working directory and home mounted lazily from
disk, and what it changes is written back, so it is checked against the same recorded
bash output as the native run. A case that needed a program and did not find one may
fail without being listed; every other difference is in a list of known differences of
its own, with its reason, held like the native ones (see
[How psh is tested](testing.md)). Most entries are about signals. A smoke test of the
JavaScript API runs alongside.

## Where to look

- [crates/psh-wasm/src/lib.rs](../../crates/psh-wasm/src/lib.rs): the exports, the
  buffers, parse and classify.
- [crates/psh-wasm/src/shell.rs](../../crates/psh-wasm/src/shell.rs): creating and
  stepping a shell; the nesting limit.
- [crates/psh-wasm/src/host.rs](../../crates/psh-wasm/src/host.rs) and
  [crates/psh-wasm/src/ops.rs](../../crates/psh-wasm/src/ops.rs): the host interface
  over `host_call`, and the operations in flight.
- [js/psh.mjs](../../js/psh.mjs): the JavaScript glue, `MemHost` and `MemFs`.
- [js/test/smoke.mjs](../../js/test/smoke.mjs) and
  [js/test/spec-driver.mjs](../../js/test/spec-driver.mjs): the API's tests and the
  conformance cases under node.
- [scripts/build-wasm](../../scripts/build-wasm): builds both modules.
