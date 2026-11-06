# Execution core: what oh-my-pi learned about tokio/async, and what to build

Source: `~/Development/oh-my-pi` at `75e7801e24` (2026-11-06).

Clean-room note: under our policy for GPL-derived code, the bash-derived engine (`crates/pi-shell/src/bash`) is described here only at the level of architecture and public API shape. Nothing below copies or paraphrases its code. The OS host (`src/sys`, `src/process`, `src/host.rs`, `src/session.rs`) is pi's own MIT code. I read its module docs and describe mechanisms in prose.

## Short answer

They dropped tokio as the **interpreter's executor and as its process/signal layer**. They did **not** drop the `Future`/`Waker` model.

- The new engine (landed 2026-11-03, `44711a78c1`) is a synchronous, step-driven VM of cooperatively scheduled virtual processes. It is executor-agnostic: it uses only `std::task::{Waker, Context, Poll}` and has no runtime, no threads and no tokio.
- The OS host underneath uses plain threads and OS primitives:
  - pooled waiter threads blocking in `waitid`, with pidfd + `poll` on Linux;
  - one process-wide readiness thread (`poll`, or `select` on macOS);
  - Windows waiter threads on `WaitForMultipleObjects`, plus thread-pool overlapped I/O on named pipes;
  - signal handlers that only set atomic bits.
- tokio survives only at the **embedding edge**: the N-API session driver, timers in the embedded host, provider (JS) filesystem futures, and the builtin utilities (`spawn_blocking`).
- The standalone `obash` binary drives the engine with a thread parker and no tokio runtime. tokio code is still linked in only because of an optional `Handle::try_current()` probe.

---

## 1. Where tokio is used in `pi-shell` today

Hits per area (`grep -c tokio`, and `async fn`/`.await` lines):

| Area | tokio refs | Where / why |
|---|---|---|
| `src/bash/**` (engine: parser, IR, VM, builtins, zsh) | 11, **all in `bash/trace/viz/runtime_tests.rs`** (`#[tokio::test]`) | None in non-test code. No `async fn`, no `std::thread`. The 33 "await" hits are identifiers (`awaiting(call)`). |
| `src/process/**` (obash / `omp bash` process shell host) | 0 | Thread parker, timer thread, signal bitmask |
| `src/bin/obash` | 0 | – |
| `src/sys/**` (OS host: spawn, pumps, fs, relay) | 20 | `relay.rs` tests (19), plus one optional `tokio::runtime::Handle::try_current()` in `sys/pump/orphan.rs:46`: if a runtime exists, adopted output work is spawned there; otherwise a process-wide set is advanced by pollers |
| `src/host.rs` (`PiHost`, embedded agent host) | 6 | `Host::timer` is a tokio `sleep_until` task that wakes the engine waker (`host.rs:665`); builtin tasks and VFS provider ops run on the session runtime |
| `src/session.rs` | 73 (many in tests) | The *driver*: one task on a caller-supplied runtime steps the shell, `select!`s on the engine waker, a `Notify` bell and tool deadlines; cancel escalation via `tokio::time::timeout` (`session.rs:410-430`, `560`, `723-740`) |
| `src/shell.rs` | 49 | N-API-facing `Shell::run`/`execute_shell`: output readers (`tokio::fs::File`), `tokio::spawn` + `select!` for cancel and timeout, drain windows |
| `src/vfs.rs` | 2 | provider-path fs ops as tasks on the runtime (`VfsFs`), returning `Pending` to the engine |
| `src/extras.rs`, `src/tools/` | 8 | builtin (`pi-builtins`) tasks spawned on the runtime; `git` layer worker |
| `src/git/`, `src/proctree.rs`, `src/cancel.rs` | 29 | git guard worker, process-tree kill helpers, `CancelToken` (`tokio::sync::Notify`) |
| `src/suites`, `tests/` | 90 | tests |

**Engine core.** The engine is synchronous with a cooperative scheduler (`crates/pi-shell/src/bash/mod.rs` module docs, `bash/vm/mod.rs` header):

- `Shell::step(cx, Budget)` runs ready virtual processes round-robin, up to a fixed op slice each.
- It returns one of: a script finished, a host callout is needed, everything is blocked, paused, yield (budget spent), suspend-ready, or idle.
- Every IR op is a safepoint. A control word set from any thread (cancel, pause, signal, suspend) is checked before each op (`bash/vm/control.rs` docs).
- The same control object *is* the engine's `Waker`. Every pending thing the engine polls (an external process, a native task, a pending fs op, a file read) is polled with a `Context` built from it.
- The host waits however it likes and then calls `step` again.
- Subshells, pipelines, command substitutions and background jobs are virtual processes inside one thread, not forks or OS threads.

**Three drivers of the same engine:**

1. `obash` / process shell (`src/process/mod.rs`, `drive` and `wait_runnable`): a plain loop on one thread.
   - Before each step it dispatches caught OS signals.
   - On "blocked/paused" it parks in `sys::Parker`, re-checking about every 50 ms.
   - On "yield" it checks for background exits without parking.
2. Embedded agent shell (`src/session.rs`): a tokio task. It is the same loop with `await` on the engine waker, a `Notify` and a deadline, plus `yield_now()` on "yield".
3. Test kit (`src/testkit`): `FakeHost` + `MemFs`, fully deterministic, with no OS and no runtime. `MemFs::with_latency` makes *every* fs op `Pending`, to prove no engine path expects synchronous answers.

## 2. How they do processes, signals, pipes, here-docs and cancellation now

All of this is in `crates/pi-shell/src/sys` (module docs in `sys/mod.rs`, `sys/spawn/mod.rs`, `sys/spawn/unix.rs`, `sys/spawn/windows.rs`, `sys/park.rs`, `sys/ready.rs`, `sys/pump.rs`, `sys/pump/os.rs`, `sys/lifecycle.rs`, `sys/relay.rs`, `sys/fs.rs`).

### Child spawn and wait

**Unix spawn** (`sys/spawn/unix.rs` header):
- `posix_spawn` is used when it can express the spec: `SETSID`/`SETPGROUP`, `adddup2`, `addchdir_np`, `SETSIGDEF` for every non-ignored signal (this restores `SIGPIPE`, which Rust ignores), an empty mask, and `CLOEXEC_DEFAULT` on macOS.
- Otherwise it uses std `Command` with an async-signal-safe `pre_exec`. This is needed for a different umask, ignored signals, or `setsid`.
- A bare `fork` is used when the child itself must apply redirections. Opening a FIFO may block, and only the child may wait for it.
- glibc before 2.29 lacks `addchdir_np` (CHANGELOG #18046; doc commit `ea30823121`).
- Placement modes (`Sessions`): `Detach`, where every external is a session leader (the default for the embedded agent shell, so cancels reach whole trees and nothing touches pi's terminal); `Share`, which uses process groups for job control; and `Inherit`.

**Unix wait** (`sys/spawn/mod.rs` "Lifecycle", `Exits` enum at `spawn/mod.rs:116`):
- `Exits::Thread` (default, any executor). A **pooled waiter thread** per running external blocks in `waitid(..., WNOWAIT)`, records the status, and wakes the engine waker.
  - The pool reuses threads with small stacks. The doc comment says this saves "about a tenth of a spawn's cost on Linux".
  - `WNOWAIT` leaves the process unreaped, so an exited pipeline leader keeps its process group alive for later `setpgid` joins, and a group signal can never hit a recycled pid while the handle exists.
  - Stops and continues are consumed with non-reaping `WSTOPPED|WCONTINUED|WNOHANG` queries. macOS reports stops even for `WEXITED`; this was measured.
- `Exits::Park` (Linux only, used by standalone `obash`). The engine thread waits itself. `Parker::park` blocks in `poll(2)` on an eventfd (the waker) plus **one pidfd per awaited external**. The doc comment says a waiter thread costs "two more thread switches per external". It falls back to a waiter thread for job-controlled externals, because a pidfd reports exits, not stops.
- macOS and other Unixes always use waiter threads. They do *not* use kqueue `EVFILT_PROC` for exits; kqueue is used only for timers.
- **There is no SIGCHLD handler for waiting.** brush's SIGCHLD-subscription race (below) is gone by construction.

**Windows** (`sys/spawn/windows.rs` header and `wait_loop`):
- `CreateProcessW` creates the child suspended in a new console process group, assigns it to a **Job Object per job**, then resumes it.
- Only the three stdio handles are inherited (`PROC_THREAD_ATTRIBUTE_HANDLE_LIST`). Batch files use std's hardened `cmd.exe` quoting (`sys/cmdline.rs`).
- Wait: a **waiter thread per external** blocks in `WaitForMultipleObjects` on {process handle, wake event}, with a timeout equal to the pending KILL deadline.
- Signals map onto the job:
  - KILL = `TerminateJobObject`;
  - TERM/INT/HUP/QUIT = `CTRL_BREAK_EVENT` to the console group, then KILL after the grace period;
  - stop/cont = `NtSuspendProcess`/`NtResumeProcess` on exactly the members suspended (`sys/job.rs`, a platform-neutral state machine tested everywhere).

**Lifecycle / abandonment** (`sys/lifecycle.rs`):
- An external is complete only when it has exited *and* its output pumps reached EOF.
- Dropping the `ExtProc` after exit reaps it. Dropping it while the process is running *abandons* it: TERM to the group now, KILL after `SpawnConfig::grace` (default 2 s; the embedded host uses `TERM_GRACE` = 1 s) on a small "kill" thread, then a background reap. This is the explicit replacement for tokio's `kill_on_drop`.
- `nohup`/detached externals are *released*: no signal, background reap.
- Leak checks (`leftovers`/`kill_leftovers`) run after every test case.

### Signals

- **Process shell** (`src/process/signals.rs`). The handler is async-signal-safe and only ORs a bit into a static `AtomicU64`. The drive loop drains the bits at scheduler boundaries into the engine's *virtual* signal bus (traps run at safepoints).
  - A signal during `Parker::park` ends the park via `EINTR` on Linux; elsewhere it is noticed within the 50 ms re-check.
  - On Linux, ignored dispositions are read from `/proc/self/status` `SigIgn`.
  - When embedded in Bun (`omp bash`), `RuntimeSignals` temporarily resets the runtime's dispositions for a fixed set of shell signals (HUP INT QUIT TERM PIPE ALRM TSTP TTIN TTOU CHLD WINCH CONT) to `SIG_DFL` and restores them afterwards.
  - `obash` uses a C `main` (`#![no_main]`) so Rust's runtime never sets `SIGPIPE` to ignored (`src/bin/obash/main.rs` docs).
- **Embedded agent shell** (`src/host.rs` header). It installs **no OS signal handlers at all** and never reports a terminal. Every external leads its own session. Cancellation is a control-word request: the engine delivers a virtual TERM to its vprocs (traps run, status 143), then KILL after the grace period (137). Abandoned externals get group TERM→KILL from the spawner. `kill` to pi itself, its group or its ancestors is refused.
- `tokio::signal` no longer exists anywhere in `crates/` (the last uses were removed in `44711a78c1`).

### Pipes and I/O

- **Between virtual processes**, pipes are in-memory engine objects. There are no OS pipes and no threads.
- **Engine ↔ external**: *pumps* (`sys/pump.rs`). A pump is a polled state machine, **not a thread**. It is driven from the external's `poll_wait` with the polling task's waker, moves a bounded burst per poll, and then yields.
  - Output runs to EOF of all write-end holders.
  - On `EPIPE` from the engine side it closes the OS read end, so the child gets a real `SIGPIPE`.
  - One shared OS pipe per engine fd object ("feed"), so `cat < <(…)` and `diff <(a) <(b)` don't race pumps for the same bytes.
- **OS side of a pump** (`sys/pump/os.rs` header):
  - Unix: our private end is non-blocking (it's never handed to a child, so `O_NONBLOCK` affects nobody). I/O that would wait registers the waker with the readiness thread. Timers are `timerfd` on Linux and kqueue timers on macOS/BSD, watched the same way.
  - Windows: anonymous pipes can't do overlapped I/O, so each pump pipe is a **private named-pipe instance**. Our server end is overlapped and bound to the system thread pool (`CreateThreadpoolIo`); the child's client end is an ordinary synchronous handle. Completion callbacks only record the result and wake. Dropping an end cancels the I/O (`CancelIoEx`) without waiting.
- **Readiness thread** (`sys/ready.rs`). One process-wide thread waits on all watched fds plus a wake pipe. It wakes each waker once and forgets it. It uses `poll(2)`, **except on macOS, where it uses `select(2)`**: "neither poll nor kqueue reports a FIFO whose last writer closed (measured: revents 0, no EV_EOF)", and macOS `poll` answers `POLLNVAL` for terminals. Fds at or above `FD_SETSIZE` fall back to `poll` with a 20 ms recheck.
- **Never set `O_NONBLOCK` on inherited or shared fds** (`ready.rs`, `sys/fs.rs` header). The flag lives on the open file description, which children and other processes share. Instead, `StdFile` asks the readiness thread first and only does I/O that cannot block. Reason given: "the other end of a FIFO, a /dev/fd/N or a socket may be a vproc of the same engine, so a read(2) or write(2) that waits in the kernel would deadlock the scheduler."
- **FIFO opens** that would block run on a helper thread and return `Pending` (`sys/fs.rs` header).
- **Output after exit / grandchildren** (`sys/pump/orphan.rs`, commit `5e19a91013`). Output pumps still running when their `ExtProc` is dropped (`nohup`, abandoned processes, a leftover grandchild holding the pipe) are *adopted*. They become a runtime task when one exists, or are advanced by every poller and by the drive loop before it parks. The session layer bounds the post-exit drain: EOF, or 250 ms idle, or 2 s maximum (`docs/natives-shell-pty-process.md`).
- **Windows output decoding** (`sys/relay.rs`, `src/output_decode.rs`). A per-program relay pipe is polled on the shell's own task (no thread) and decoded UTF-8 → ANSI code page per program.

### Here-docs

- Here-doc and here-string bodies are engine fd objects. An external consumer is fed through the same non-blocking pump or feed machinery, so the writer never blocks the thread that launches the consumer.
- When the consumer must seek, or the body is too large for a pipe, the engine asks the host for a temp file (`TempPurpose::HereDoc`, `bash/fs.rs` API; `sys/fs.rs` header). On Windows, process substitution uses `\\.\pipe\obash-<pid>-<n>`.
- The brush-era fix (below) used a fire-and-forget writer thread. The new design removes the problem structurally.

### Cancellation and timeout

- **Engine**: `ControlHandle::cancel` sets the control word, and it takes effect at the next op safepoint. Externals being waited on are dropped, which abandons them (TERM→KILL).
- **Session** (`src/session.rs` docs): TERM first, then KILL if not done after `TERM_GRACE` (1 s). A cancel reaches the engine only from the driver, while its run is in flight, so "a late cancel never leaks into the next command".
- **N-API layer**: `CancelToken` from `timeoutMs` + `AbortSignal`, then descendant cleanup, a 2 s graceful window (5 s on Windows), and finally an abort of the task (`docs/natives-shell-pty-process.md` "Cancellation").
- **Builtins**: a cancel flag plus EOF on a blocked stdin read; async builtins are dropped (status 130).

## 3. What the history says about async

### brush era (vendored `brush-core` 0.4, 2026-02-01 to 2026-11-03)

brush's interpreter is async end to end: `async-recursion` and `async-trait` for every nested construct, tokio processes and `tokio::signal` (from `git show 44711a78c1^:crates/vendor/brush-core/Cargo.toml` and `src/sys/unix/signal.rs`, which had `tstp_signal_listener`, `chld_signal_listener`, `await_ctrl_c`). The vendored brush-core/parser had about 120 `tokio` references.

Commit subjects (`git log --grep`):

- `8506fbdf52` 2026-06-25, **"fix(coding-agent): switched ctrl-z handler to SIGSTOP-self to defeat brush tokio SIGTSTP hijack"**. Body: "brush-core's Process::wait calls tokio::signal::unix::signal(SIGTSTP) … Per tokio's documented contract, the first call for a SignalKind permanently replaces the kernel-default handler for the lifetime of the process. So once omp has executed any bash tool call … SIGTSTP's default "stop" action is gone".
- `0b7270c701` 2026-10-01, **"fix(brush-core): report pipeline stages that stop before the wait starts"**. Body: "ChildProcess::wait subscribed to SIGCHLD only when it began waiting, and a tokio signal listener never sees a signal delivered before it subscribed. Every pipeline stage is spawned before the first one is waited on … run_string hung." This is PR #14023.
- `ae927b6f81` 2026-05-23, **"fix(pi-shell): unblock bash heredocs >4 KiB on Windows and >64 KiB on macOS"**. Body: "wrote the entire heredoc/here-string body into an anonymous pipe synchronously before handing the reader to the downstream command … deadlocked the writer forever". The fix was a fire-and-forget writer thread.
- `57becd56ee` 2026-02-12, "fix(brush): prevented zombie processes by tracking reap state and implementing Drop trait". It "Enabled kill_on_drop for tokio child processes".
- `6d3a7dfbb2` 2026-03-25, "fix: prevent subshell output read from blocking async runtime (#502)". It moved `read_to_string` into `spawn_blocking`.
- `9d5508980f` 2026-04-01, "perf(pi-natives): optimized pipe I/O to use async instead of blocking workers". Quote: "Replaced blocking Tokio worker with non-blocking async I/O for pipe reading to prevent stalling the runtime when child processes keep pipes open."
- `588055e6d0` 2026-10-12, "fix(shell): unblocked read on process substitution". Quote: "Moved blocking read input to a Tokio blocking worker so queued process-substitution producers can run." This fixes #15501.
- `50ca4cad2c` / `f4216501c4` 2026-06-14, "fix(pi-natives): avoid deadlock by deferring tokio runtime install" and "fix(natives): avoided linux addon load hang".

`packages/natives/CHANGELOG.md` entries:

- 18.11.x (line 366): "Fixed embedded-shell `read` in a pipeline stage or async job queueing a second blocking worker while holding its own, which could hang every such read once the bounded blocking pool filled (#15509)".
- line 379: "Fixed `read` and `while read` hanging on process-substitution input (#15501)".
- 18.4.10 (line 616): "Fixed the embedded shell sometimes hanging on a pipeline with a stage that stopped (for example with `kill -STOP $$`) before the shell began waiting on it; the pipeline now becomes a stopped job (#14023)".
- 17.4.0 (line 958): "Updated compound blocks (`{ ...; }`, `(...)`) and shell-function pipeline stages to run concurrently with other pipeline stages, preventing head-of-line blocking and pipe buffer deadlocks." brush ran them in sequence.
- 15.5.0 (line 1570): the heredoc deadlock (same text as `ae927b6f81`).
- 15.5.10 (line 1540): background jobs pinned the JS thread at 200 % CPU because each kernel `read(2)` became one `ThreadsafeFunction::call`. The fix was coalescing into batches of up to 64 KiB.
- 15.13.1 (line 1414): "`pi-natives` deadlocking at addon load (`dlopen` hang) … The load-time Tokio runtime install … ran inside `#[module_init]`, which executes while the dynamic-loader lock is held; building the multi-thread runtime there eagerly spawns worker threads … deadlocks the whole load".
- 15.12.6 (line 1425): "napi-rs builds its own Tokio runtime with one eagerly-spawned worker per CPU, and that spawn _panics_ rather than erroring" (Windows OS error 1455).
- 18.13–18.14 (lines 119, 150–151), brush-era fixes: "built-in bash commands such as `cat`, `grep`, `head`, `wc`, `read`, and `mapfile` hanging at the end of FIFO input on macOS". "a shell redirect to or from a FIFO whose other end is a background job, such as `echo hi > f & cat < f`, hanging the command". Both are the class of bug that the new `ready.rs` (`select` on macOS) and the FIFO-open helper thread address structurally.

### Switch to their own engine

- `44711a78c1` 2026-11-03, "feat(shell): replaced brush with pi's own bash engine in pi-shell". The body lists functional fixes (per-run stdio, EXIT traps, late background output, `env -i`, xargs signal status, FIFO and umask handling). **It states no explicit "remove tokio" rationale.** The engine came pre-built from another workspace. The design rationale is in the module docs quoted above: "nothing blocks a thread"; "a read(2) … that waits in the kernel would deadlock the scheduler"; pidfd parking saving "two more thread switches per external".
- `5e19a91013` 2026-11-03, "feat(pi-shell/sys): added orphan handling for detached process output". Quotes: "Migrate Windows named-pipe server ends to overlapped I/O with `CreateThreadpoolIo` for non-blocking stream lifecycle" and "Integrate orphan polling into `Parker` and `NativeSpawner` to ensure background work advances in synchronous execution contexts".
- 18.17.0 CHANGELOG: "`Shell` and `executeShell` now run pi-shell's Bash engine with Bash 5.3 semantics" and "Native shell is faster across parsing, functions, expansion, redirections, arithmetic, loops, and external command startup."

**Why tokio is kept at all.** It is kept because of the *embedding*, not the engine:

- N-API async tasks need a runtime anyway (`pi-natives/src/task.rs`, `lib.rs`).
- JS filesystem providers are inherently async callbacks (`pi-vfs` provider futures).
- Builtins use `spawn_blocking` and async utilities (§4).

`session.rs` documents that it runs fine on a **current-thread** runtime: "Nothing blocks a thread: a current-thread runtime drives the driver, the provider futures … external spawning and output pumps together." The cost: `pi-vfs`'s `BlockingFs` uses `block_in_place` + `Handle::block_on`, and "On a current-thread runtime's own thread tokio rejects this with a panic — a contract violation" (`crates/pi-vfs/src/lib.rs` header).

## 4. How `pi-builtins` uses async

`crates/pi-builtins/src/host.rs` defines three kinds of utility:

- `Utility` (`host.rs:71`), most of them: a `clap` model plus a **synchronous** `run(self, &mut Host) -> i32`, "Called on a blocking thread, so blocking reads, rayon, and long filesystem walks are all fine". `run_utility` runs it with `tokio::task::spawn_blocking` (`host.rs:2134`).
  - On cancel, a cancel flag makes blocked stdin reads return EOF, and the command runner is dropped. The blocking thread "cannot be dropped, so it is always awaited: no detached thread keeps writing to the builtin's descriptors" (`host.rs:2140-2147`).
  - Status 130 on cancel, unless `OWNS_CANCELLATION` (rsync).
- `AsyncUtility` (`host.rs:129`): `grep`, `rg`, `fd`, `find`, `ast-grep`. The body is a `Future` awaited on the calling task. It "MUST NOT block". Cancellation drops it. Output goes to memory-backed writers flushed at unit boundaries. Input is read via `stdio::Source`; Unix fds become readiness-driven through tokio `AsyncFd` on a private dup, each syscall gated by a zero-timeout `poll` (`docs/native-crates.md`). Background: `37d3f1d0c0` 2026-10-29, "feat: migrated filesystem operations to asynchronous runtime", which introduced `AsyncUtility` and drop-cancelable filesystem futures, plus the vendored `grep-searcher` `search_push` for chunk-fed search.
- `Process` (`host.rs:2551`): `ps`, `top`, `sleep`, `pgrep`, `pkill`, `pidwait`, with async bodies.
- `async-trait` appears only in tests (`host.rs:3757`).

In the engine, each builtin is a host `Task` (`ExtraTask`, `crates/pi-shell/src/extras.rs`) spawned on the session's tokio runtime and polled by the engine like an external. Pi-builtins is therefore the main reason `pi-shell` still needs tokio in the embedded path.

**The wasm tokio cfg** (`crates/pi-builtins/Cargo.toml:150`) is a leftover from brush. It arrived in `2ee9943563` (2026-08-08, "refactor: unify builtins in one place") from the vendored `brush-builtins/Cargo.toml:221`; `brush-core` had the same, with wasm stubs in `sys.rs` and `interp.rs`. Nothing builds pi-builtins for wasm. The roughly 21 `cfg(not(any(unix, windows)))` fallbacks are uutils/brush heritage.

## 5. Recommendation for psh, embedded in pi-env

Our constraints: about zero external crates; MIT brush code may be copied with attribution; pi-env is synchronous and threaded with no async runtime.

### Core decision: a sans-IO, step-driven engine on `std::task` only

Copy the *shape* that oh-my-pi converged on. This is architecture, not code:

1. **The engine is a synchronous state machine.** `step(budget) -> {Done, NeedsHost(call), Blocked, Yield}`, plus a thread-safe `ControlHandle` (atomic control word: cancel, pause, signal) checked at every safepoint (statement or op boundary). Subshells, pipelines, `$(…)` and `&` jobs are *virtual processes* inside the engine, not threads and not forks.
2. **Use `std::task::Waker` as the only wake mechanism.** It is zero-dependency and lets the same engine run under a pi-env thread loop, under a tokio task (if someone embeds psh in an async host), or under a wasm/JS host.
3. **Put every side effect behind one `Host` trait** with *pollable* operations (spawn → handle with `poll_wait`; fs ops may return `Pending`; files expose `poll_read`/`poll_write`). This gives us a deterministic fake host for conformance tests and a JS host for WASM later.
4. **Driver in pi-env**: one thread per shell session runs `loop { dispatch signals; step(); on Blocked park(timeout) }`. The parker is a `Mutex`+`Condvar` (or `thread::park`) woken by the waker. On Linux it can optionally `poll` an eventfd plus pidfds.

**Do not copy brush-core's interpreter or sys layer.** It is async-recursive (`async-recursion`/`async-trait`) and built on `tokio::process` and `tokio::signal`, which is exactly the part that bit oh-my-pi. brush-parser (MIT) is a candidate for copying, but note it depends on `peg` and `winnow` (plus miette, cached, bon, tracing). We'd have to vendor `peg`'s generated output or rewrite the grammar by hand.

### Per-platform primitives

| Concern | Linux | macOS | Windows |
|---|---|---|---|
| spawn | `posix_spawn` (SETSID/SETPGROUP, adddup2, addchdir_np when glibc ≥ 2.29, SETSIGDEF incl. SIGPIPE); `fork`+`exec` fallback for umask, ignored signals, child-side FIFO redirects | same; `POSIX_SPAWN_CLOEXEC_DEFAULT` | `CreateProcessW` suspended → `AssignProcessToJobObject` → `ResumeThread`; `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`; std-style argv quoting + batch-file hardening |
| exit wait | pooled waiter thread `waitid(WEXITED\|WSTOPPED\|WCONTINUED, WNOWAIT)`; later optimization: pidfd in the driver's `poll` | pooled waiter thread (oh-my-pi did not use kqueue `EVFILT_PROC`; it's an option, but waiter threads are simpler and proven) | waiter thread on `WaitForMultipleObjects(process, wake_event)` with the KILL deadline as timeout (or `RegisterWaitForSingleObject` to use the system pool) |
| process groups / kill | own session or group per external; TERM → grace → KILL to the group; reap later | same | Job Object per job: KILL = `TerminateJobObject`; TERM = `CTRL_BREAK_EVENT` then KILL; pause via `NtSuspendProcess` |
| engine ↔ external pipes | private non-blocking end + one readiness thread (`poll`) | same, but **`select`** for FIFOs and ttys (poll/kqueue miss FIFO writer close) | private **named-pipe** instance with overlapped server end (thread-pool IO or one IOCP thread); the child gets a sync client handle |
| external ↔ external pipes | plain `pipe2(O_CLOEXEC)` passed directly; no pumping | same | anonymous pipe passed directly |
| signals (standalone `psh`) | handler sets atomic bits plus a self-pipe/eventfd write to wake the parker; drain at safepoints; C `main` or reset SIGPIPE to `SIG_DFL` | same (self-pipe) | `SetConsoleCtrlHandler` → atomic bits + event |
| signals (embedded in pi-env) | **install no handlers**; cancel = control word + group signals | same | same |

A simpler v1 is acceptable for pi-env, which is already threaded. Instead of a readiness thread plus overlapped I/O, a **bounded helper thread per pump** (blocking `read`/`write` into an engine pipe object with a waker) is easy to get right. Only cross the engine↔OS boundary when a builtin or virtual process is on one side of a pipe. Keep the trait shape pollable so we can swap in the readiness thread or IOCP later without engine changes.

### Pitfalls to design for from day one

1. **Never block the engine thread in the kernel** on a pipe, FIFO, `/dev/fd/N`, socket or tty. The peer may be a virtual process of the same engine. This was oh-my-pi's FIFO, process-substitution and `read` deadlock class (#15501, #15509, macOS FIFO EOF, `echo hi > f & cat < f`).
2. **Never write a here-doc or here-string body synchronously before starting the consumer.** Feed it through the same pollable pipe machinery, or use a temp file when large or seekable (`ae927b6f81`).
3. **No signal-subscription races.** Don't learn about child state changes via late-subscribed SIGCHLD listeners. Use per-pid waits that cannot miss an event: `waitid`, pidfd, or a process handle (`0b7270c701`, #14023).
4. **Don't replace process-wide signal dispositions as a side effect of running a command.** That was the tokio SIGTSTP hijack, `8506fbdf52`. When embedded, install nothing. When standalone, install minimal bit-setting handlers and reset children to `SIG_DFL` (especially SIGPIPE).
5. **Don't set `O_NONBLOCK` on inherited or shared descriptors.** Use our own private ends, or readiness checks.
6. **Avoid pid reuse.** Keep exited leaders unreaped (`WNOWAIT`) while their group may still be joined or signaled. Hold process handles on Windows. Reap in the background after abandonment.
7. **Drop/abandon semantics are explicit.** Abandoned means TERM → grace → KILL plus a background reap. Detached (`nohup`) means released. A command completes only when exit *and* output EOF are both observed. Output from grandchildren still holding the pipe is adopted and drained with bounds (250 ms idle / 2 s maximum at the session layer).
8. **Pipeline stages, including compound blocks and functions, must run concurrently.** Sequential execution deadlocks on pipe buffers (17.4.0 changelog line 958).
9. **Cancellation must be cooperative at safepoints and stay scoped to its run.** A late cancel must not hit the next command (`session.rs`). Builtins need a cancel flag that unblocks their stdin reads. Never detach a thread that still writes to the command's fds.
10. **Runtime and FFI hygiene** (relevant if psh is ever exposed through N-API): don't spawn threads in module init (`dlopen` loader-lock deadlock); treat thread-spawn failure as recoverable (Windows commit limit); coalesce output callbacks (the 200 % CPU bug).
11. **macOS quirks**: `poll`/kqueue miss FIFO writer close and return `POLLNVAL` for ttys, so use `select` with an `FD_SETSIZE` fallback. `waitid(WEXITED)` reports stops.
12. **Windows quirks**: anonymous pipes can't be overlapped. `.bat`/`.cmd` need cmd.exe quoting hardening (CVE-2024-24576). Console ctrl events need a shared console, so fall back to killing the job. Decode output per producer for non-UTF-8 code pages.
13. **Builtins**: keep the bodies synchronous behind a small `Host` (stdio fds, cwd, env, cancel flag, command runner for `xargs`/`find -exec`) and run them on a thread pool. That is the oh-my-pi `Utility` model without tokio. Reserve pollable or async bodies for the few that must stream without a thread (grep/find over huge trees), if ever.
14. **Testing**: build the deterministic fake host and an in-memory fs where every op can be `Pending` from the start. Run each conformance script under injected pause/resume at every k-th safepoint, as oh-my-pi's skeleton suite does. Leak-check process groups after each test.
