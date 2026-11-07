# 0008: Utility framework and first batch

status: in-progress
agent: infra
branch: issue/0008-utilities
depends-on: 0005, 0006
touches: crates/psh-utils/, crates/psh-engine/ (utility dispatch, announced), crates/psh-spec/ (L4 layer), scripts/record-goldens (L4 recording), tests/utils/
guide: utilities.md (not written yet; list it as a follow-up)
decisions: D1080–D1089

## Goal
The in-process utility machinery (ARCH.md "psh-utils"; D010, D014–D018) and the first
batch, led by `timeout`, the most common "command not found" in the agent corpus. Every
later batch (0013–0021) builds on this; it fixes the interface they all use.

## Acceptance criteria
- [x] `psh-utils` crate: the synchronous utility interface over a small context (stdio,
      cwd, env, host fs, cancel flag that unblocks stdin reads, command runner); the
      engine runs utilities on worker threads and polls them like externals; no thread
      outlives its utility while it can still write to the command's fds.
- [x] A registry: one entry per utility with its name(s), mode (S / F / W, D015), and
      whether it is in the destructive group. Adding a utility is one entry plus a module.
- [x] Modes behave as specified: F checks PATH, W shadows only on Windows, explicit paths
      always run the real program; WASM/sandbox mode treats F as S.
- [x] Off switches per session: all, destructive group, single names.
- [x] Fall-through: an unsupported flag runs the real program from PATH if present, else
      fails with `NAME: unsupported option -X (psh builtin)` or similar; a shared helper
      makes this one line per utility. Tested.
- [x] `command -v`, `type`, `which` (the `which` utility itself is 0020) report
      utilities truthfully; `env`, `xargs`, `find -exec`, `timeout` dispatch back into the
      shell (tested here with `env` and `timeout`), including the standalone forms of
      `printf echo test [ true false pwd kill`.
- [x] L4 harness: cases in `tests/utils/<name>/`, goldens recorded from GNU (the 0002
      container) and from BSD (macOS, with `/usr/bin` tools); a case declares which
      dialect(s) it expects, or the D016 rule it exercises.
- [x] Batch A, all S: `timeout` (W; GNU semantics: `s m h d` suffixes and fractions,
      `-k`, `-s SIG`, `--preserve-status`, `--foreground`, statuses 124/137, kills the
      whole tree; `gtimeout` alias), `head`, `tail` (`-f`/`-F` streaming and cancellable;
      BSD `-r`), `cat` (GNU `-A -v -e -E -t -T -n -b -s`), `wc` (GNU widths), `sleep`
      (fractions, GNU suffixes, cancellable). Flags per `research/builtins.md` §4.2.

## Notes
- uutils (MIT) is the porting source for head/tail/cat/wc (D018): refactor onto the
  context, keep attribution in the file header and `NOTICE`.
- oh-my-pi's `crates/pi-builtins/src/host.rs` (MIT) is a model for the context and for
  cancellation; `tools/timeout.rs` for `timeout`'s design. Its `pi-shell/src/bash` is off
  limits (D007).
- The registry and the context are hot files for every later batch: keep them small and
  stable, and record their shape as a decision in your range.

## Log
- 2026-11-07: Coordinator correction: "worker threads" cannot be the model (no threads in
  wasm32-unknown-unknown, and wasm/sandbox mode needs utilities most). Proposal below,
  sent for review before the registry is built.

### Proposal: the utility interface (to become D1080)

**Default: async utilities on the engine scheduler, like shell builtins.** A utility is
`async fn run(ctx: &mut UtilCtx) -> i32`, called on the vproc that runs the command
(the same place a builtin runs; pipeline stages are already concurrent vprocs). The
context lives in psh-engine (`psh_engine::util`, a small public module) because it wraps
the vproc's descriptors; psh-utils implements utilities against it and the embedder
(psh binary, pi-env, psh-wasm) passes the registry in `ShellConfig`, so psh-engine never
depends on psh-utils.
- I/O: `ctx.stdin`/`stdout`/`stderr` are the command's descriptors as the engine has
  them (engine pipes, host files), read and written with `.await`. Output goes through a
  buffer that is written synchronously and flushed at a size threshold, at line ends
  for terminals, and at exit, so ported loops stay mostly synchronous: the await points
  are reads and flushes. Helpers: `read_chunk`, `lines()`, `read_to_end`, `open(path)`
  (relative to the shell's cwd, through the Host), `stat`, `read_dir`.
- Cancellation is dropping the future (as for builtins); `ctx.check().await` in long
  loops is the safepoint: it observes cancel and yields when the step budget is spent,
  so a CPU-heavy loop never starves the scheduler or the embedder.
- `ctx.run(argv)` runs a command for the wrappers (`env`, `timeout`, `xargs`,
  `find -exec`, `nice`, `nohup`): builtins in their standalone forms, utilities,
  externals, with the context's descriptors and environment, as a child vproc.
- Why: one code path natively and in wasm; no thread to stop or join, so cancel and
  `timeout` are exact; deterministic order; `cat f | head` stays in-process with no OS
  pipe or pump; `head` closing early and `tail -f` (a timer loop on `Host::sleep`) need
  nothing special. Cost: ported code (uutils uses `Read`/`Write` and `std::fs`) gets its
  I/O loops rewritten onto the context; the algorithms (parsing, formatting, counting)
  are reused as they are. All of batch A and most coreutils fit this well.

**Escape hatch: blocking utilities**, for code built on synchronous crates (0013's
ripgrep crates and `ignore` walker, jaq, a large `sort`): `fn run(ctx: &mut SyncCtx) ->
i32` with std `Read`/`Write` stdio and a synchronous fs view.
- Natively, with the OS host: on a worker thread. Its stdio are channels the engine
  pumps to and from the command's descriptors (as it pumps externals' pipes); cancel sets
  a flag and closes the channels, so reads end and writes fail with EPIPE; the command's
  status is reported only after the thread has ended or can no longer write. The fs view
  is `std::fs` resolved against the shell's cwd.
- In wasm or with a non-OS host (sandbox): inline on the engine thread. stdin is read to
  EOF first, stdout is collected and written after; the fs view makes synchronous host
  calls, which a host must answer at once (the JS MemHost does; others make such a
  utility fail clearly). Streaming is lost there (`tail -f | rg` does not stream in the
  browser), nothing else.
- Few utilities need it; batch A needs none.

**Option parsing and the classifier (D1090).** Utilities declare their options as data
(`Spec`: short and long options, which take a value, GNU-style permutation, `--`,
`-n5`/`-n 5`, `--x=y`, unambiguous long-option prefixes, plus hooks for forms like
`head -5`). One shared parser; `fallthrough_or_fail` for unknown options is one line per
utility. A test compares every utility's `Spec` with its `psh_classify::utils` entry:
the value-taking short and long options must be the same, and every effect option the
classifier lists must be one the utility accepts (or falls through on).

**Registry and dispatch.** A static table of entries `{ names, mode: S|F|W,
destructive, run: Async(fn) | Blocking(fn), spec }` in psh-utils. The engine looks a
name up after functions and builtins and before PATH: S runs the utility, F only when
PATH has no such program (or the host is a sandbox, which treats F as S), W is S on
Windows and F elsewhere; a name with a slash always runs the program; off switches
(all, destructive group, names) are per shell. `command -v`/`type` say what runs.

Alternatives weighed: synchronous utilities on threads natively plus "something else"
in wasm. Inline synchronous code in wasm deadlocks as soon as it waits on a pipe whose
writer is another vproc on the same thread (`yes | head`), so wasm would need either a
second implementation or buffering every input first; and natively, threads make cancel
and `timeout` approximate and output order racy. The porting saving (keeping uutils'
`Read`/`Write` loops) is real but small next to that.
- 2026-11-07, implementation (decisions D1080–D1084, approved; engine part waiting for
  the engine agent's OK):
  - `psh_engine::util` (UtilCtx v1): descriptors, buffered stdout (flushed before reads,
    before stderr writes, when full, at line ends on a terminal; a failed flush makes
    every later write fail, so `yes | cat | head` ends), Host file operations, `sleep`,
    `check`, `spawn` (wrappers) and `run_program` (fall-through). Dispatch after
    functions and builtins; `command`, `type`, `command -v`; hashed names and
    `command -p` run the program (two brush cases showed why).
  - psh-utils: the option parser (GNU conventions, `Spec` data), the classifier check
    (`timeout` is classified in code, so it is exempt), `REGISTRY`, `policy()`, and batch
    A: `timeout`/`gtimeout` (W), `head`, `tail` (`-f`/`-F` as a host-timer loop, BSD
    `-r`), `cat`, `wc` (GNU widths, `--total`), `sleep`. Written for psh from GNU's and
    BSD's documented behavior (no uutils code copied, so no attribution needed).
    Tests on the engine's test host (virtual time: timeout cases use `hang`).
  - Wired into the psh binary, psh-spec (in-process) and psh-wasm (sandbox policy).
    Native spec run: oils +19, brush +13, agent +7 cases pass now, no regression.
    In wasm the utilities are the toolset: oils 1,150 pass there now (was 867).
  - L4: `tests/utils/` with 65 cases for batch A, GNU and BSD goldens (both
    deterministic over two recordings), `## expect:` per case; psh passes all 65.
  - Docs: ARCH "Interface", TESTING (L4 row, goldens), decisions D1083–D1084.

## Follow-ups

## Result
