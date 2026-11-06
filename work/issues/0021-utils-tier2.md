# 0021: Utilities Tier 2 (umbrella)

status: open (later; the coordinator splits it into batches)
agent: -
branch: -
depends-on: 0008 and the Tier 1 batches it shares code with
touches: crates/psh-utils/, tests/utils/
guide: utilities reference (not written yet)
decisions: D1210–D1219

## Goal
Tier 2 of `research/builtins.md` §4.3 (~45 names, D017), once Tier 1 is in. Mostly cheap
once the uutils adapter exists. Proposed batches:

- Process and host: `ps pgrep pkill` (per-OS listing; BSD `ps aux/-axo` and procps
  `-eo/-ef/-o/-p`; `pkill -f`), `uptime nproc id uname whoami hostname df` (W for
  `hostname`/`whoami`). `sysinfo` or our own per-OS code is decided in that batch (D028).
- Text and binary: `tree` (W; ours), `column` (util-linux superset; ours), `tac rev
  strings od xxd hexdump expr yes truncate fold expand unexpand join split shuf`.
- Checksums and encodings: `cksum b2sum sha224sum sha384sum base32 basenc sum`,
  `dos2unix unix2dos iconv` (needs `encoding_rs`).
- Archives, F mode: `tar gzip gunzip zcat unzip` (crates decided in that batch, D028);
  never shadow a real compressor.
- Small ones: `patch` (BSD patch as reference; GNU patch is GPL), `sponge ts ifne`,
  `install link unlink sync mkfifo logname tty`.

Tier 3 (§4.4) stays on demand and is tracked on the board, not here.

## Log

## Follow-ups

## Result
