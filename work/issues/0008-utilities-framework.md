# 0008: Utility framework and first batch

status: done
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
- 2026-11-07: `exec NAME` into a utility runs it, then ends the (sub)shell with its status
  (coordinator's call; `exec -a`/`-c` do not apply to utilities).
- 2026-11-07, after rebasing onto main (0026, 0028, 0029): no regression; ratchets
  updated (oils −49, brush −13, agent −17 entries). In wasm, `oils/pipeline/22` needed
  `/dev/urandom`: MemHost has `/dev/random` and `/dev/urandom` now; `brush/.../set/20`
  joins the 0028 wasm entries (the `s` of `$-`). AGENTS.md's unsafe rule names psh-wasm
  (coordinator's request). wasm-opt -Oz (binaryen 132, measured, not adopted) on top of
  the `wasm` profile: psh.wasm 1,051,099 → 910,559 bytes, 386,813 → 381,177 gzipped
  (−1.5%); psh-parser.wasm 388,202 → 341,543, 148,282 → 146,769 gzipped (−1%).

## Follow-ups
- Signals to commands started by `UtilCtx::spawn`: stopping drops them (TERM then KILL
  for programs), so `timeout -s SIG` and `-k` only set the status; a `UtilChild::signal`
  needs the external's handle (API v2).
- Wrapper messages: `env`/`timeout` with a missing command print the shell's
  `command not found`, not GNU's `failed to run command` (stderr score only).
- Blocking utilities (D1080) are designed, not built: the first batch that needs one
  (0013, rg) adds `SyncCtx` and the worker-thread/inline runner, with the wasm input bound.
- Move `crates/psh/src/{json,parse_json,classify_json}.rs` into a crate shared by the psh
  binary and psh-wasm (now a `#[path]` include, 0011).
- guide: utilities.md.

## Result
Landed: the utility API `psh_engine::util` (UtilCtx v1, D1080–D1081), psh-utils with the
option parser (D1082), the registry and dispatch (D1083), batch A (`timeout`/`gtimeout`,
`head`, `tail`, `cat`, `wc`, `sleep`) plus `env`, and the L4 harness with GNU and BSD
goldens (D1084): 73 cases, all passing. The psh binary, psh-spec and psh-wasm (as a
sandbox) run the utilities.

### How to add a utility (for batch agents)

1. **Module** `crates/psh-utils/src/NAME.rs`:
   - `pub const SPEC: Spec` with every option the utility accepts: `both('n', "lines")
     .value()`, `flag('x')`, `long("name")`, `.optional()` for `--name[=v]`;
     `posix_order: true` when the first operand ends the options (wrappers).
   - `pub async fn run(ctx: &mut UtilCtx<'_>, argv: &[Vec<u8>]) -> i32`. Parse with
     `common::parse(ctx, &SPEC, argv)` (unknown options fall through to the real program,
     else `NAME: unsupported option 'X' (psh built-in NAME)`, status 2). Forms the parser
     cannot express (`head -5`) are rewritten on argv first (see `head::rewrite_obsolete`).
   - I/O only through the context: `common::Input::open(ctx, name)` (`-` is stdin) and
     `input.read(ctx, &mut buf)`, `ctx.write(..).await` (buffered; on error return
     `common::write_failed(ctx, &e).await`, which is 141 for a closed pipe),
     `ctx.error("msg")` for `NAME: msg` on stderr, `common::file_error` for `NAME: FILE:
     reason`, `ctx.stat/open/read_dir/access/canonicalize`, `ctx.sleep`. Never `std::fs`,
     `std::io::stdout` or threads.
   - Call `ctx.check().await` in loops that can run long (it yields when the step budget
     is spent). Cancel is dropping the future: keep no state that must be cleaned up.
   - Wrappers run commands with `ctx.spawn(argv, SpawnEnv)` and `child.wait()`.
   - Helpers in `common.rs`: `parse_duration` (GNU `1.5m`), `parse_count` (`10K`),
     `operands_or_stdin`, `usage`.
2. **Registry** in `crates/psh-utils/src/lib.rs`: `mod NAME;`, a `UtilEntry { names,
   mode, destructive, run: util_fn!(NAME::run) }` in `REGISTRY` (mode per
   `research/builtins.md` §4 / ARCH "Tiers": `Always` S, `PathFallback` F,
   `WindowsShadow` W; `destructive: true` for `rm mv cp ln dd`) and `NAME::SPEC` in
   `SPECS`.
3. **Classifier check**: `cargo test -p psh-utils specs_match_the_classifier` compares the
   spec with `psh_classify::utils` (value-taking options must be the same; every effect
   option must be accepted). If the classifier's entry is wrong or missing, fix it there
   (psh-classify) in the same change; utilities classified by rules in code go into
   `CLASSIFIED_IN_CODE` in `tests.rs`.
4. **Unit tests** in `crates/psh-utils/src/tests.rs` (engine test host: in-memory files
   `/tmp/f`, virtual time, fake programs `cat yes head wc argv env true false hang`).
5. **L4 cases** in `tests/utils/NAME/*.test.sh` (`#### name`, then the code; files only
   in the case's directory; deterministic). Annotate every case whose GNU and BSD results
   differ: `## expect: gnu (BSD NAME has no -x)`, `## expect: bsd (...)`, or a D016 rule:
   `## expect: gnu (D016: wc widths)`. Record with `scripts/record-goldens utils/NAME`
   (GNU, Docker) and `scripts/record-goldens --bsd utils/NAME` (BSD, on macOS); a
   `both` case with differing recordings fails as "goldens disagree" until annotated.
   Run `cargo run -p psh-spec -- utils/NAME --diff`; cases psh does not pass yet go into
   `tests/utils/expected-failures.txt` with `--add-failures`.
6. **Ratchets**: run the whole `psh-spec` (and `psh-spec --wasm` after
   `scripts/build-wasm`): a new utility usually makes Oils/brush/agent cases pass;
   `--update-ratchet` removes them. In wasm, cases that need programs not yet built in
   fail without a ratchet line.

Known gaps: see Follow-ups (signals to spawned commands, blocking utilities not built yet,
wrapper error messages).
