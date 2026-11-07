# 0020: Utilities batch I: diff, checksums, base64, date, env, nice, nohup, which

status: in-progress
agent: shapes
branch: issue/0020-utils-misc
depends-on: 0008, 0006
touches: crates/psh-utils/src/{diff,cmp,base64,sum,date,env,nice,nohup,which}*, tests/utils/ (same names)
guide: utilities.md (not written yet), utilities reference (not written yet)
decisions: D1200–D1209

## Goal
The rest of Tier 1, all S: `diff cmp base64 md5sum sha1sum sha256sum sha512sum shasum md5
date env printenv nice nohup which`.

## Acceptance criteria
- [ ] `diff`: default normal format, `-u -U N -c -r -q -N -x/--exclude -I -w -b -B -y
      --label --color`, exit status 0/1/2. `cmp`: `-s -l -b -n -i`.
- [ ] `base64`: `-d -D --decode`, `-w N` (GNU) / `-b N` (BSD), `-o FILE`, `-i` per D016.
- [ ] Checksums: each name keeps its native output and `-c` verify mode; `md5` with `-q
      -r -s`; `shasum -a ALG -c -b`.
- [ ] `date`: GNU `-d STRING -I -R --rfc-3339 +%N`, BSD `-v±N[ymwdHMS] -j -f`, `-u`,
      `-r` per D016, real local TZ. `-d` parsing: choose the date parser here (D028:
      `parse_datetime` or our own) and record it in your range; fall through until then. The date parser is shared:
      `touch -d` (0015), `find -newermt` (0017) and `fd --changed-*` (0013) use it.
- [ ] `env`/`printenv`: `-i -u -S -C`, `NAME=val cmd`, dispatch into the shell.
- [ ] `nice -n N cmd` (setpriority; `BELOW_NORMAL_PRIORITY_CLASS` on Windows; no-op
      wrapper in WASM). `nohup`: detached from the shell's cancellation, ignores HUP, no
      `nohup.out` when stdout is redirected; the released process is reported.
- [ ] `which` (`-a`): reports utilities and PATH hits, exit 1 when nothing is found.
- [ ] Each utility registered with its mode; unsupported flags fall through or fail
      clearly (tested per utility).
- [ ] L4 cases in `tests/utils/<name>/` for every flag listed above, recorded against
      GNU and BSD, with the D016 rule named where they conflict.
- [ ] Attribution headers and `NOTICE` entries for ported code.

## Notes
- Coordinator update: the date parser already exists (files agent, psh-utils timefmt.rs, used by
  `touch -d`): extend and share it rather than adding `parse_datetime`. `env` already landed with
  0008 (infra); `printenv` and the rest of `env`'s flags are still here. Hash functions
  (md5, sha1, sha224/256/384/512, b2sum if cheap) are implemented in-house with no dependency
  (small, well-specified algorithms; test against the NIST/RFC vectors).
- uutils, uutils diffutils (MIT/Apache), oh-my-pi's `diff.rs` and `tools/nohup.rs` (MIT).

## Log

## Follow-ups

## Result
