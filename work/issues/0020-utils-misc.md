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
- Part 1 (shapes): `base64`, `md5sum sha1sum sha224sum sha256sum sha384sum sha512sum`,
  `md5`, `shasum`, `cmp`, `which`, `printenv`, `env -S -C` (D1200–D1203). Digests are
  ours (`sum_hash.rs`, RFC 1321 / FIPS 180-4 vectors in its tests). Probed GNU in the
  recording image and BSD on macOS 26 (scratch scripts in work/scratch/0020/). Findings:
  macOS 26 `md5 -s` prints the bare hash (not FreeBSD's `MD5 ("s") = h`); macOS
  md5sum's `-c` does not read stdin and warns about blank/comment lines (GNU skips them);
  GNU quotes names with spaces in sum messages (`'standard input'`); GNU `cmp -l` pads
  offsets to the largest printable offset (19 digits when no size is known).
- `which` first printed `NAME: shell builtin` for every utility (ARCH's example); that
  broke `$(which env)` (brush special_parameters/8), so the program's path wins when
  there is one (D1200). Coordinator: ARCH's "which reports utilities truthfully" line may
  want this nuance.
- Outside touches (minimal): psh-engine util.rs (`SpawnEnv::cwd`,
  `UtilCtx::is_utility`, `UtilCtx::search_path`); psh-classify utils.rs (base64's long
  value options and `--output` effect, md5 `-c`, sha224sum/sha384sum).
- Ratchets from `scripts/spec-linux --update` plus the macOS run: agent passes 701/701 on
  Linux (its 71 shared entries were stale), oils/brush shared entries removed; two cases
  the shared file had covered for wasm got wasm lines (case-insensitive APFS, `~root`).
- L4: tests/utils/{base64,cmp,sum,which,printenv}/ and env/flags (the case dirs are
  recorded, so a file name with a newline must be removed before a case ends).

- Part 2 (shapes): `date` (D1204) and `diff` (D1205). The engine's TZif reader now
  backs `UtilCtx::load_zone`/`zone_abbr`/`use_utc` (outside touch: psh-engine util.rs,
  builtins/strftime.rs `load`), so a command's `TZ` works in utilities; touch and find do
  not call `load_zone` yet (follow-up). timefmt (shared with touch/find): days of the
  week, `YYYYMMDD`, zone names, `+ 2 weeks`, and a GNU `date` strftime. diff was tuned
  against GNU as an oracle on generated and real pairs (scratch: rnd/, real/, proto*.py);
  probes showed GNU's `-y` geometry (formula in D1205). macOS's diff agrees with GNU on
  nearly every case except times in headers, `--left-column`, `--color`, quoting.
- oils/array-assign/6 failed once in a loaded spec-linux run and passed alone; quarantine
  it if it recurs.
- ARCH.md: the coordinator rewrote the `which` reporting line (D1200) on main.

## Follow-ups
- touch -d / find -newermt: call `ctx.load_zone()` so `TZ=...` applies (one line each).
- diff: `-e`/`-f`/`-n` (ed/RCS output), `-t`/`-T`, `--tabsize`, `-D`, `--no-dereference`,
  `-y` with `\ No newline` lines; the remaining GNU alignment differences (D1205).
- date: `-z` (BSD output zone), `--debug`, GNU's `TZ="..."` inside `-d` strings.
- cksum and b2sum (BLAKE2b) for the checksum family.

## Result
