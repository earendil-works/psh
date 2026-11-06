# 0008: Utility framework and first batch

status: open
agent: -
branch: issue/0008-utilities
depends-on: 0005, 0006
touches: crates/psh-utils/, crates/psh-engine/ (utility dispatch, announced), tests/utils/
guide: utilities.md (not written yet; list it as a follow-up)
decisions: D1080–D1089

## Goal
The in-process utility machinery (ARCH.md "psh-utils", D010) and a first batch chosen by
agent demand. Later batches are separate issues, one agent per batch, in parallel.

## Acceptance criteria
- [ ] `psh-utils` crate: the synchronous utility interface over a small context (stdio,
      cwd, env, fs, cancel flag that unblocks stdin reads, command runner); the engine
      runs utilities on worker threads and polls them like externals.
- [ ] Shadowing modes (shadow / fallback / Windows-shadow), session off switches (all,
      destructive group, per name), explicit paths always run the real program.
- [ ] Fall-through: an unsupported flag runs the real program from PATH if present, or
      fails with a clear message; tested for every utility.
- [ ] `command -v`, `type`, `which` report utilities truthfully.
- [ ] L4 harness: per-utility cases with goldens recorded from GNU (container) and BSD
      (macOS), and the tie-break rules of `research/builtins.md` §4.6.
- [ ] First batch: `timeout` (the most common "command not found"), `head`, `tail`,
      `cat`, `wc`, `true`/`false` standalone forms, `sleep` (cancellable).

## Notes
- `research/builtins.md` §4 is the source for tiers, modes (S/F/W) and the GNU/BSD
  tie-breaks. Its undecided points need a coordinator/owner decision before the batch
  that hits them: `ls -G`/`-T` meaning, `xargs` on empty input (GNU vs BSD), `sort`'s
  default collation, `grep -P` backend (pcre2 vs fancy-regex, both new dependencies).
- Port from uutils (MIT) with attribution; copying beats depending (D006).
- Proposed later batches, from the corpus ranking: (b) `grep`/`rg`/`fd` on the ripgrep
  crates; (c) `sed` (both `-i` forms); (d) `ls`/`cp`/`mv`/`rm`/`mkdir`/`touch`/`ln`/
  `mktemp`/`stat`; (e) `sort`/`uniq`/`cut`/`tr`/`tee`/`nl`/`seq`/`paste`/`comm`;
  (f) `find`/`xargs`; (g) `jq`; (h) `awk` (largest; fallback mode until conformant);
  (i) `diff`/`cmp`/checksums/`base64`/`date`/`env`/`nice`/`nohup`/`which`.

## Log

## Follow-ups

## Result
