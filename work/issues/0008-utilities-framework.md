# 0008: Utility framework and first batch

status: open
agent: -
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
- [ ] `psh-utils` crate: the synchronous utility interface over a small context (stdio,
      cwd, env, host fs, cancel flag that unblocks stdin reads, command runner); the
      engine runs utilities on worker threads and polls them like externals; no thread
      outlives its utility while it can still write to the command's fds.
- [ ] A registry: one entry per utility with its name(s), mode (S / F / W, D015), and
      whether it is in the destructive group. Adding a utility is one entry plus a module.
- [ ] Modes behave as specified: F checks PATH, W shadows only on Windows, explicit paths
      always run the real program; WASM/sandbox mode treats F as S.
- [ ] Off switches per session: all, destructive group, single names.
- [ ] Fall-through: an unsupported flag runs the real program from PATH if present, else
      fails with `NAME: unsupported option -X (psh builtin)` or similar; a shared helper
      makes this one line per utility. Tested.
- [ ] `command -v`, `type`, `which` (the `which` utility itself is 0020) report
      utilities truthfully; `env`, `xargs`, `find -exec`, `timeout` dispatch back into the
      shell (tested here with `env` and `timeout`), including the standalone forms of
      `printf echo test [ true false pwd kill`.
- [ ] L4 harness: cases in `tests/utils/<name>/`, goldens recorded from GNU (the 0002
      container) and from BSD (macOS, with `/usr/bin` tools); a case declares which
      dialect(s) it expects, or the D016 rule it exercises.
- [ ] Batch A, all S: `timeout` (W; GNU semantics: `s m h d` suffixes and fractions,
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

## Follow-ups

## Result
