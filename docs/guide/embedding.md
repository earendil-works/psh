# Embedding psh in a host

A Rust program can run psh inside its own process: an agent host's daemon, a test
runner, an editor. This page shows how to create a shell, run scripts in it, step it
from a thread you own, stop or signal it from another thread, and run agent commands
the way a daemon does, with combined output, a deadline and nothing left behind.

> **In short.** You build a `Shell` from a `ShellConfig` (a host, the environment, the
> working directory, descriptors 0 to 2 and the in-process utilities), start a script
> and call `step` until it reports `Done`, parking the thread while it is `Blocked`. A
> `Shell` stays on the thread that made it; its `ControlHandle` cancels, pauses or
> signals it from anywhere. With the operating system hosts, dropping the host ends the
> run and kills what it left running.

Embedding is what the [embeddable](goals.md#embeddable) goal is about: psh brings no
async runtime and no threads of its own, and the embedder decides when it runs.

## A first shell

```rust
let mut cfg = ShellConfig::new(Rc::new(StdHost::new()));
cfg.env = vec![(b"PATH".to_vec(), b"/usr/bin:/bin".to_vec())];
cfg.stdout = Some(Rc::new(StdFile::stdio(1)?));
let mut shell = Shell::new(cfg);
shell.start(Script::command("x=hello; echo $x | tr a-z A-Z"));
let status = drive(&mut shell); // prints HELLO, returns 0
```

`ShellConfig::new` gives closed stdio, an empty environment, `/` as the working
directory and no in-process utilities. The fields:

| Field | What it is | Default |
| --- | --- | --- |
| `host` | every side effect goes through it ([The host interface](host.md)) | (required) |
| `env` | the environment, as byte pairs | empty |
| `cwd` | the working directory, absolute, `/`-separated (`C:/x` on Windows) | `/` |
| `stdin`, `stdout`, `stderr` | descriptors 0 to 2, as files of the host; `None` is closed | closed |
| `max_depth` | the nesting limit for function calls and subshells | 1000 |
| `exit_grace` | how long background jobs may finish after the script ends | `None` |
| `utilities` | psh's in-process utilities and their off switches | none |

Pass `psh_utils::policy(false)` as `utilities` to get every utility of
[Utilities that run in-process](utilities.md); `policy(true)` is for a host without real
programs (a sandbox, WebAssembly), where utilities that normally defer to a program on
`PATH` always run. The returned policy has off switches: `off_all`, `off_destructive`
(`rm`, `mv`, `cp`, `ln` as one group) and `off_names`, a list of single names. A name
that is off runs the real program, or is not found. With the default, empty policy every
command that is not a function or a builtin is a program, as with bash.

`exit_grace` decides about `cmd &` jobs still running when the script ends. `None`
releases them, as bash does; `Some(grace)` waits up to `grace` for them and then
abandons them (`TERM`, then `KILL`). Whether released programs survive is the host's
choice (below).

## Starting a script

`Script::command(src)` is `psh -c src`: `$0` is `psh` and syntax errors say `-c:`. For
anything else fill in the struct. Its `options` take the command-line options of `set`
and `shopt` (`-e`, `+x`, `-o NAME`, `-O NAME`), and
`psh_engine::invocation::parse_args` turns a bash-style argument list into these
fields:

```rust
shell.start(Script {
    source: b"echo \"$0: $1\"; false | true; echo not reached".to_vec(),
    arg0: b"job".to_vec(),
    args: vec![b"first".to_vec()],
    dash_c: true,
    options: vec![b"-e".to_vec(), b"-o".to_vec(), b"pipefail".to_vec()],
});
```

This prints `job: first` and ends with status 1: under `pipefail` the pipeline fails,
and `-e` stops the script. The source is bytes and need not be UTF-8.

A shell keeps its state between runs: variables, functions, options, the working
directory and open descriptors. A cancelled run is the exception: the next one starts
from the state before it. `start` panics if a script is still running, and clears
earlier cancel, pause and signal requests, so a cancel that arrives just after a run
ended never reaches the next one.

## Stepping

`step(budget)` runs ready work for about `budget` commands (one safepoint before every
command and every loop iteration) and says how it stopped:

| `Step` | Means | The embedder |
| --- | --- | --- |
| `Done(status)` | the script finished; reported once | reads the status |
| `Blocked` | everything waits on the host, or the shell is paused | waits for the waker |
| `Yield` | the budget is spent and there is more work | steps again when it likes |
| `Idle` | no script is running | starts one |

The waker given to `set_waker` fires when a `Blocked` shell can go on. The host's
threads only wake it; engine code runs inside `step` and nowhere else. A loop of your
own looks like this:

```rust
struct Unpark(std::thread::Thread);
impl Wake for Unpark {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

shell.set_waker(Waker::from(Arc::new(Unpark(std::thread::current()))));
let status = loop {
    match shell.step(1_000) {
        Step::Done(status) => break status,
        Step::Yield => {}                     // do other work, then step again
        Step::Blocked => std::thread::park(), // the waker unparks us
        Step::Idle => break 0,
    }
};
```

A host with an event loop of its own hands the shell a waker that queues an event, and
calls `step` from there. `std_host::drive` is the loop above, and `drive_until(shell,
deadline)` also cancels the shell at a deadline and returns `None` when it did.

A `Shell` holds reference-counted state and is not `Send`: create it and step it on
one thread. Function calls, `eval` and `source` use native stack, up to the nesting
limit, so give that thread `std_host::ENGINE_STACK` (about 20 MiB, of which only
touched pages cost memory); `std_host::engine_thread(f)` starts one. With a smaller
stack, lower `max_depth` to match (`ShellConfig::STACK_PER_LEVEL` per level).

## Controlling a run

`shell.control()` returns a `ControlHandle`: cheap to clone, `Send` and `Sync`.

| Call | Effect |
| --- | --- |
| `cancel()` | ends the run at once with 143: every task is dropped, no trap runs, the programs it started are abandoned |
| `pause()` / `resume()` | stops the shell at its next safepoint; `step` answers `Blocked` until `resume`; started programs keep running |
| `signal(n)` | delivers signal `n` (Linux numbering) at the next safepoint: a trap runs, an ignored signal does nothing, any other runs the `EXIT` trap and ends the run with `128+n` |
| `wake()` | wakes the waker without changing anything |

`signal_word()` is the atomic mask behind `signal`; setting a bit in it is
async-signal-safe, for an embedder that forwards its own signals from a handler and
then calls `wake`. After a run, `shell.killed_by()` names the signal whose default
action ended it, so a program that embeds psh as its shell can die by that signal too.
A script that traps `USR1` and `EXIT` and counts `i` up in an endless loop, sent `USR1`
and then `TERM` from another thread, and a second run of the same shell afterwards:

```text
got USR1
exiting
status 143 killed_by Some(15)
i=78195
status 0
```

The second run sees `i`: a run that a signal ended keeps its state, unlike a cancelled
one.

## Running agent commands from a daemon

A daemon that runs one agent command per call gives each call a thread of its own, a
fresh host and shell on it, one pipe for both output streams so they arrive combined
and in order, and a deadline. This runs on Unix as written:

```rust
use std::io::Read;
use std::os::unix::ffi::OsStringExt;
use std::rc::Rc;
use std::time::{Duration, Instant};

use psh_engine::std_host::{drive_until, engine_thread};
use psh_engine::{HostFile, Script, Shell, ShellConfig};
use psh_os::unix::{UnixConfig, UnixFile, UnixHost};

/// Runs one command: combined output, and the status (`None`: it timed out).
fn exec(command: String, cwd: Vec<u8>, timeout: Duration)
    -> std::io::Result<(Option<i32>, Vec<u8>)>
{
    let env: Vec<(Vec<u8>, Vec<u8>)> = std::env::vars_os()
        .map(|(k, v)| (k.into_vec(), v.into_vec()))
        .collect();
    // psh-os's pipe: close-on-exec, made so no other thread's spawn inherits it.
    let (mut reader, writer) = psh_os::unix::pipe()?;
    let deadline = Instant::now() + timeout;
    let engine = engine_thread(move || {
        // A session per program, leftovers killed when the host drops (the defaults),
        // and a `$$` of its own, so `kill 0` cannot reach the embedder's process group.
        let host = Rc::new(UnixHost::new(UnixConfig {
            pid: Some(99_999_999),
            ..UnixConfig::default()
        }));
        let mut cfg = ShellConfig::new(host);
        cfg.env = env;
        cfg.cwd = cwd;
        cfg.utilities = psh_utils::policy(false);
        let out: Rc<dyn HostFile> = Rc::new(UnixFile::new(writer));
        cfg.stdout = Some(out.clone());
        cfg.stderr = Some(out);
        // Background jobs get half a second after the script ends, then are abandoned.
        cfg.exit_grace = Some(Duration::from_millis(500));
        let mut shell = Shell::new(cfg);
        shell.start(Script::command(&command));
        drive_until(&mut shell, Some(deadline))
        // The shell and its host drop here: the pipe's write end closes, and whatever
        // the run left running is killed.
    })?;
    let mut output = Vec::new();
    reader.read_to_end(&mut output)?;
    let status = engine.join().expect("engine thread");
    Ok((status, output))
}
```

Run on macOS with a small `main` that prints the output and the status:

```text
$ exec 'echo out; echo err >&2; exit 3'
out
err
status Some(3) after 803.2µs
$ exec 'echo start; sleep 30'          # a two-second timeout
start
status None after 2.0s
$ exec '/bin/sleep 300 & echo "job $!"'
job 84523
status Some(0) after 508.5ms
```

The `sleep 300` job is gone afterwards: half a second of `exit_grace`, then it was
abandoned. Read the pipe while the shell runs, as here, or a command that writes more
than a pipe holds waits forever. The read ends when every writer is closed, so a
`nohup cmd &` that keeps writing to the pipe keeps the reader waiting until it exits;
detached jobs should redirect their output.

The OS hosts' settings decide what a run may leave behind:

| `UnixConfig` field | Means | Default |
| --- | --- | --- |
| `placement` | `Session`: each program leads a session of its own, so a cancel reaches its whole tree; `Inherit`: programs stay in the embedder's process group, as with bash | `Session` |
| `leftovers` | when the host drops, `Kill` signals every session of the run (`TERM`, `KILL` after the grace period); `Keep` leaves them running | `Kill` |
| `grace` | time between `TERM` and `KILL` when a program is abandoned | 2 s |
| `pid` | `$$`; when set, `kill` cannot reach the embedder's process or its group (`kill 0` and `kill -1` included) | the real pid |
| `detached_survive` | `nohup` and `setsid` jobs outlive the run; off for a test runner, where nothing may survive | on |
| `inherit_ignored` | signals the embedder ignores stay ignored in programs, as bash keeps them; off, so a daemon started under `nohup` does not pass that on | off |

`WindowsConfig` has the same fields except `inherit_ignored`, with `Placement::Group`
(a console process group per program) as its default, and `decode`, which can convert
programs' output from the ANSI code page. On Windows, pass the environment through
`psh_os::windows::shell_env`, which names `Path` as `PATH` and sets `HOME` and `TMPDIR`,
and give `cwd` with forward slashes. How the two hosts place and stop programs is on
[Processes on Unix](processes.md) and [psh on Windows](windows.md).

A daemon that keeps one shell for several calls ends a run's leftovers without dropping
the host: keep an `Rc<UnixHost>` next to the shell and call `kill_all()`, or hand
`killer()`, a `Send` handle, to another thread. Abandoned programs are killed by helper
threads, which die with the process, so a daemon that exits right after a run calls
`psh_os::unix::settle(timeout)` first. An embedder that also starts programs of its own
with `std::process::Command` while shells run holds `psh_os::unix::FD_LOCK` for writing
around each spawn; on macOS that keeps a shell's pipe from leaking into the program.

## Choosing a host

| Host | For |
| --- | --- |
| `psh_os::unix::UnixHost`, `psh_os::windows::WindowsHost` | real programs with process groups or job objects, cancel of whole trees, leftovers killed |
| `psh_engine::std_host::StdHost` | simple embedders and tests on the standard library alone; no process groups, kills with `KILL` only |
| `psh_engine::testhost::TestHost` | tests: in-memory files, fake commands, captured output, counters for leak checks |
| your own | a virtual file system, a remote machine, commands of your own |

What each one implements, and what a host of your own must provide, is on
[The host interface](host.md). For tests, `TestHost::config()` returns a config wired to
the test host's captured stdio, and `host.stdout()` returns what the script printed.

## From JavaScript

A JavaScript host does not embed the Rust crates directly. The `psh-wasm` crate builds
the engine for `wasm32-unknown-unknown` with a host whose every operation is a request
to JavaScript, and `js/psh.mjs` steps it from the event loop over an in-memory file
system with commands written in JavaScript. See [psh in the browser](wasm.md).

## In an agent host's daemon

> **Not built yet.** No agent host's daemon embeds psh yet; the plan, much like the
> `exec` example above, is described on [psh in one agent host](pi.md).

## Where to look

- [crates/psh-engine/src/lib.rs](../../crates/psh-engine/src/lib.rs): `Shell`,
  `ShellConfig`, `Script`, `Step`, `ControlHandle`.
- [crates/psh-engine/src/std_host.rs](../../crates/psh-engine/src/std_host.rs):
  `StdHost`, `StdFile`, `drive`, `drive_until`, `engine_thread`, `ENGINE_STACK`.
- [crates/psh-engine/src/invocation.rs](../../crates/psh-engine/src/invocation.rs):
  bash's command line turned into a script and its options.
- [crates/psh-utils/src/lib.rs](../../crates/psh-utils/src/lib.rs): `policy` and the
  registry of utilities;
  [crates/psh-engine/src/util.rs](../../crates/psh-engine/src/util.rs): `UtilPolicy`.
- [crates/psh-os/src/unix/mod.rs](../../crates/psh-os/src/unix/mod.rs) and
  [crates/psh-os/src/windows/mod.rs](../../crates/psh-os/src/windows/mod.rs):
  `UnixConfig`, `WindowsConfig`, `kill_all`, `killer`;
  [crates/psh-os/src/lib.rs](../../crates/psh-os/src/lib.rs): `Leftovers`.
- [crates/psh/src/main.rs](../../crates/psh/src/main.rs): the `psh` program, an
  embedder that keeps bash's process placement;
  [crates/psh-spec/src/engine.rs](../../crates/psh-spec/src/engine.rs): the test runner,
  an embedder that checks every case for leaked processes.
