# 0005: Minimal interpreter

status: in-progress
agent: engine
branch: issue/0005-minimal-interpreter
depends-on: 0002, 0004
touches: crates/psh-engine/, crates/psh/ (driver), crates/psh-spec/ (in-process engine), tests/**/expected-failures.txt
guide: engine.md (not written yet; list it as a follow-up)
decisions: D1050–D1059

## Goal
The first version of the stepped engine (ARCH.md "psh-engine", D005), enough that L1, L2
and L3 start moving: simple commands, pipelines, `&&`/`||`/`;`, redirections, variables
and assignments, basic expansion, exit status. It fixes the engine's shape (step API,
control handle, waker, Host trait, virtual processes) that everything later builds on.

## Acceptance criteria
- [ ] `Shell::step(budget) -> Step` (`Done`, `Blocked`, `Yield`, `Idle`), a
      `ControlHandle` checked at safepoints (cancel works), wakeups via
      `std::task::Waker` only. No threads inside the engine, no runtime.
- [ ] A first `Host` trait (spawn with `poll_wait`, fs ops that may return `Pending`,
      files with `poll_read`/`poll_write`, timers, env) and a test host with an
      in-memory fs and fake processes, including a mode where every op is `Pending`
      once before it completes.
- [ ] Virtual processes: pipeline stages run concurrently, in-memory pipes between
      them; subshells `( )` as vprocs.
- [ ] Language: simple commands with assignments, redirections (`>`, `>>`, `<`, `2>&1`,
      `&>`, `n>&m`, here-docs, here-strings), `&&`, `||`, `;`, `!`, `{ }`, `( )`,
      `$?`, parameter expansion `$x ${x} ${x:-y}`, quoting and quote removal, word
      splitting on IFS, `$(…)`. Shell builtins: `echo`, `printf` (basic), `true`,
      `false`, `:`, `cd`, `pwd`, `export`, `unset`, `exit`, `test`/`[`.
- [ ] A simple Unix host good enough for the runner: std `Command` spawn with a waiter
      thread per child that wakes the engine, helper threads for pipes. 0006 replaces
      it; keep it small.
- [ ] `psh -c CMD` and `psh FILE` run through a standalone driver (step, park on
      `Blocked`).
- [ ] psh-spec runs psh in-process; ratchets updated; the Result records the scores.
- [ ] The engine builds for `wasm32-unknown-unknown` (gate).

## Notes
- Read `research/execution-core.md` §5 "Pitfalls" before designing; all 14 points
  apply. Never block the engine thread in the kernel; never write a here-doc before its
  consumer starts.
- brush-core's expansion code may be copied with attribution; its interpreter and sys
  layer are async and must not be (D008).
- Keep the Host trait minimal and say in the log what you expect 0006 and the utilities
  to need; the trait will change.

## Log

### 2026-11-07 part 1
- Design (ARCH.md "psh-engine" rewritten, D1050–D1057): async fns over the AST polled by
  the engine's own scheduler (approved by the coordinator with four conditions, all in
  D1050); Step API and ControlHandle (D1051); Host trait v1 (D1052); values are bytes
  (D1053); in-memory pipes, pumps polled inside the external's wait, here-docs as
  preloaded pipes (D1054); emulated /dev/fd names (D1055); psh-spec in-process (D1056);
  loud "not supported yet" (D1057).
- `psh-engine`: `sched.rs` (tasks, wakers, budget, safepoints), `host.rs`, `pipe.rs`,
  `fd.rs` (fd objects, pumps), `state.rs` (Vp, vars), `exec.rs` (lists, and-or,
  pipelines, `{ }`, `( )`, simple commands, assignments, redirections incl. `n>&m`,
  `&>`, here-docs/strings, externals, `&`), `expand.rs` (brace, tilde, parameters with
  `- = ? +` (colon), `# ## % %%`, `/ // /# /%`, case ops, `${#x}`, `${!x}`, `@Q/E/U/L/u`;
  `$(…)`, backquotes, IFS splitting, globbing via the host, quote removal),
  `pattern.rs` (glob matcher), `escape.rs`, builtins (`echo printf true false : cd pwd
  export readonly unset exit set -- shift return test [`), `testhost.rs`, `std_host.rs`,
  `invocation.rs`.
- Test host: in-memory fs, fake commands as futures (`cat yes head wc argv env hang`
  ...), latency mode; every engine test runs with and without latency and checks no
  task, process or pipe is left. Cancel test counts live resources after cancel.
- `psh` binary: `-c`, file, stdin; engine thread with `ENGINE_STACK`.
- psh-spec in-process; `$$` fake; `$PPID` cases via the binary (`kill -USR1 $PPID` killed
  the runner before). Background externals are released (not killed) when the script
  ends: `touch bg_done &` was flaky otherwise. `|&` into an external now shares one pipe
  (`ChildFd::Dup`); two pipes raced (oils/pipeline/10 was flaky).
- 5 cases passed only with the stub (status 2 by accident): added to the ratchets with
  reasons, coordinator-approved: oils/alias/21, oils/alias/25 (need aliases),
  brush/brush/external_commands/16 (`"$0"` is not on PATH for psh), oils/smoke/14 and
  brush/compat/redirection/22 (platform: GNU `ls` exits 2, macOS BSD `ls` 1).
- Stack: 95 nested `{ }` need 256–512 KiB in release, 1–2 MiB in debug.
- Speed: 100k `true` in 0.08 s (release, parse included). `: > /dev/null` costs ~12 µs,
  all of it macOS's open of /dev/null.
- Score after part 1 (macOS): oils 731/2511 active, brush 720/3171 active, agent
  100/144 active. Top "not
  supported yet": functions 539, arrays 510, case 344, `[[` 219, for 182, `set -e` 181,
  `set -o` 152, `$((…))` 130, `set -u` 102, if 92, while 88.
- Docs: ARCH.md psh-engine and Host sections, TESTING.md (psh-spec in-process). Guide:
  none affected (engine.md is a follow-up).

### 2026-11-07 Host v1 contract (for 0006, psh-os)
The trait is `psh_engine::host::Host` (`crates/psh-engine/src/host.rs`, D1052). What the
engine relies on, which any host must keep:
- **Never block the engine thread on something the engine must do first.** Every
  method returns at once; waiting goes through `Poll::Pending` + the task's waker.
  `HostFuture`s may complete immediately (`std::future::ready`); the test host's latency
  mode proves the engine copes with `Pending` everywhere.
- **Wakers may be woken from any thread** (they push onto a `Mutex` queue and wake the
  embedder's waker). A host thread must wake the waker it got from the *last* poll.
- **`Drop` releases.** Dropping an `ExtProc` that still runs = abandon (TERM to its
  group, KILL after the grace, background reap); `release()` = keep running, reap in the
  background (used for `&` jobs when a script ends); dropping a `HostFile` closes it
  (a pump's writer end flushes what it buffered, then closes, so the child sees EOF).
  Cancel drops all tasks, so this is the whole cancel path: nothing else is called.
- **Paths are bytes**, absolute (the engine joins its logical cwd), `/`-separated;
  `C:/x` on Windows. `/dev/null` must open the null device (Windows: `NUL`).
  `/dev/stdin|stdout|stderr|fd/N` never reach the host (the engine emulates them).
- **`spawn(SpawnSpec)`** is synchronous. `program` is what `resolve` returned or a
  path containing `/` (made absolute against the cwd); `argv[0]` is the name as
  written; `env` is complete (do not inherit); `cwd` is the shell's. `fds` lists the
  child's descriptors 0–2 at least (the engine adds `Null` for closed ones; >2 appear
  for `exec 3>…` and are currently ignored by the std host): `File(f)` is a `HostFile`
  this host created (downcast via `Any`) to pass as is; `Pipe { input }` asks for a new
  pipe whose engine end comes back in `Spawned::pipes` as a `HostFile`; `Dup(n)` = the
  same object as fd n of the list (one pipe for `|&`). Errors: `NotFound` → 127,
  anything else → 126, message from `host::error_text` (strerror wording).
- **`ExtProc::poll_wait`** returns `Exited(code)` or `Signaled(sig)`; the engine maps
  to `$?` (`128+sig`). It may be polled after it returned Ready (must return the same).
  `signal(sig)` delivers a signal (std host: always KILL).
- **`HostFile`**: `poll_read` (0 = EOF), `poll_write` (`BrokenPipe` when no reader;
  the engine turns that into status 141 for vprocs), `is_terminal`. Shared via `Rc`
  between dup'ed descriptors: `&self` + interior mutability. Engine-side ends of pumps
  must not block the engine thread; regular files may be read/written synchronously.
- `stat(path, follow)`, `access(path, Read|Write|Execute)` (true/false; `Err` = cannot
  check), `read_dir` (no `.`/`..`), `canonicalize`, `resolve(name, PATH, cwd)` (PATH
  search incl. empty entries = cwd; PATHEXT on Windows; `None` = command not found),
  `sleep(duration)`, `pid()` (`$$`; embedders running in-process give a fake one).

What I expect 0006 to change (announce, then change it in place; the engine adapts):
- process groups / sessions per external and `signal` to the group (TERM really TERM),
  abandon = TERM → grace → KILL to the group; `kill` builtin needs a host op to signal
  arbitrary pids/groups (refusing the embedder's own pid);
- children get every open fd (`exec 3>f; cmd` passes 3): honour `fds` > 2;
- pipes between two externals passed directly (no pumps): needs a way for the engine
  to ask "can this pipe be an OS pipe" — e.g. a `Host::pipe() -> (HostFile, HostFile)`
  whose ends are passable as `ChildFd::File`, used when both neighbours are externals;
- exit + EOF with bounded drain for grandchildren holding output pipes;
- non-blocking reads of inherited stdio / FIFOs (readiness thread), FIFO opens off
  the engine thread (open may return `Pending`), real `access` (euid), umask,
  `ENOEXEC` reported distinctly (engine then runs the file as a psh script);
- temp files for large here-docs (optional).
Nothing needs freezing before 0006 starts; the engine-side users are few (`exec.rs`
`run_external`, `apply_redirect`/`open_to`, builtins `cd`/`pwd`/`test`, `expand.rs`
glob). Please keep `HostFuture`, `HostFile` (`&self`, `Any`), drop semantics and the
"never block" rule; the rest may move.

## Follow-ups
- guide: `engine.md` (stepping, vprocs, the Host trait for embedders).
- Scripts on stdin are read whole before running; bash reads as it goes (a `read` in
  the script would consume the following lines).
- Executing a file without `#!` (ENOEXEC) should run it in psh; not yet.
- std host: `access` approximates from mode bits; KILL only; no process groups (0006).
- Children get only fds 0–2 (`exec 3>f; cmd` does not pass 3): 0006.

## Result
