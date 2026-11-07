# 0015: Utilities batch D: files and paths

status: done
agent: files
branch: issue/0015-utils-files
depends-on: 0008
touches: crates/psh-utils/src/{ls,cp,mv,rm,mkdir,rmdir,touch,ln,chmod,pwd,basename,dirname,realpath,readlink,mktemp,stat,du}*, tests/utils/ (same names)
guide: utilities.md (not written yet), utilities reference (not written yet)
decisions: D1150–D1159

## Goal
The file and path utilities of Tier 1, all S: `ls cp mv rm mkdir rmdir touch ln chmod pwd
basename dirname realpath readlink mktemp stat du`. `rm mv cp ln` form the destructive
group (off switch).

## Acceptance criteria
- [ ] `ls`: `-la -lh -t -d -R -1 -S -F -i -L --time-style`, real mode bits. `-G`/`-T`
      are decided in this issue (D034): propose the meaning with corpus evidence and
      record it in your range; they fall through until then.
- [ ] `cp`: `-R -r -a -p -n -f -L`, GNU `-t --parents`, BSD `-c` as a no-op; `cp -R
      src/` copies the directory (GNU, D016). `mv rm mkdir rmdir touch ln`: `rm
      -rf/-f`, `ln -sfn`, `touch -d` (GNU) and `-t/-r`, `mkdir -p -m`.
- [ ] `chmod`: `+x`, octal, `-R`; on Windows the read-only bit and `x` tracked for
      shebang execution.
- [ ] `realpath` (`--relative-to`), `readlink -f/-e/-m`.
- [ ] `mktemp`: `-d`, `-t` per D016 (`XXX` = GNU template, else BSD prefix), default
      `$TMPDIR`, exclusive create with 0600/0700.
- [ ] `stat`: GNU `-c/--format/--printf` and BSD `-f FORMAT`/`-t`, `-L`; `-f`
      disambiguation per D016.
- [ ] `du`: `-sh -h -sk -ch -d N`, `--max-depth`, `--exclude`, `-A/--apparent-size`.
- [ ] Windows: paths in all three forms, symlink → junction → clear error for `ln -s`,
      `rm -r` never follows junctions.
- [ ] Each utility registered with its mode; unsupported flags fall through or fail
      clearly (tested per utility).
- [ ] L4 cases in `tests/utils/<name>/` for every flag listed above, recorded against
      GNU and BSD, with the D016 rule named where they conflict.
- [ ] Attribution headers and `NOTICE` entries for ported code.

## Notes
- Mostly uutils (MIT). The BSD `stat` formatter is ours.

## Log
- Host and UtilCtx lacked every file system change. Added them (D1150): `Host::{stat_extra,
  mkdir, remove_file, remove_dir, rename, symlink, hard_link, read_link, set_mode,
  set_times, user_name, group_name, utc_offset}` with Unsupported/None defaults,
  `ExtraMetadata`, UtilCtx v2 (same plus `now`, `umask`, `utc_offset`). Implemented in
  psh-os unix (`unix/fs.rs`), std_host and the test host (MemFs now has symlinks, inode
  numbers, dir modes, link counts). Windows and JS hosts: by their owners, on the
  coordinator's split; until then they answer Unsupported. Outside `touches:` by
  necessity: psh-engine host.rs/util.rs/std_host.rs/testhost.rs, psh-os unix.
- Part 1: `basename dirname readlink realpath mkdir rmdir touch` (written for psh after
  the manuals; no ported code). Shared helpers in new modules, for any utility:
  `fsutil.rs` (GNU quoting `q`, `resolve` for realpath/readlink -f/-e/-m, `lexical`,
  `relative_to`, `base_name`/`dir_name`), `modes.rs` (chmod modes, octal and symbolic,
  `mode_string`), `timefmt.rs` (civil calendar, local time via `UtilCtx::utc_offset`,
  `parse_date` for a subset of GNU date strings, `parse_stamp` for `-t`). Unit tests in
  `src/tests/files.rs` (a child of tests.rs, so batch tests don't collide).
- `pwd` stays the shell builtin (found first; wrappers run it as a standalone builtin):
  no utility.
- Dialect choices in part 1: `realpath` with no operand resolves `.` (BSD; GNU errors);
  `realpath f/` and a missing last component follow GNU (D016 default); `touch -d`
  parses ISO dates and times, `@N`, month names, zones, relative items (`2 days ago`,
  `-1 hour`, `yesterday`, `last week`), `am/pm`; anything else falls through to the real
  `touch` (or `invalid date format`). Messages are GNU's, with its quoting (‘x’ in UTF-8
  locales).
- Classifier: `basename -s/--suffix`, `realpath --relative-to/--relative-base` and
  `touch --time` take values (psh-classify utils.rs).
- Wasm: the JS host has no D1150 operations yet, so `mkdir`/`touch`/`rmdir` fail there
  with "Operation not supported" instead of "command not found"; the cases that used
  them got wasm ratchet lines naming that (remove with --update-ratchet once the JS host
  has them).
- Part 2: `rm mv cp ln chmod` (destructive group but chmod), written for psh after the
  manuals. GNU semantics and messages; no prompting (`-i`/`-I` fall through). `rm`:
  refuses `.`/`..` and `/` (`--no-preserve-root`), never follows symlinks, and through
  `link/` removes the target's contents but not the target (as GNU; macOS's rmdir would
  remove it). `cp`: `-R src/` copies the directory itself (GNU, D016), `-c`/`-X` (BSD)
  plain copies, `-p`/`-a`/`--preserve` keep mode and times (owners, xattrs not kept),
  simple backups (`-b`, `-S`; numbered fall through), copies `/dev/fd/N` as streams
  (`cp <(..) >(..)`), into-itself makes the directory then refuses (as GNU). `mv`:
  rename, across file systems copy + remove. `ln`: `-sfn` (and BSD `-h`), `-r`, `-t/-T`,
  hard links to directories refused. `chmod`: `-x`-style modes as operands, `-R` skips
  symlinks inside, `-v/-c`, `--reference`.
- D1151 (coordinator's call): `type`/`type -p`/`-f`/`command -v` report the utility that
  runs; `type -P` prints the PATH file. Two Oils type cases ratcheted with that reason.
- The JS host (0011, D1115) and the Windows host gained D1150 meanwhile; the wasm lines
  for it went away with them.
- Windows L4 leg (D1914): `tests/utils/expected-failures.windows.txt` lists what the
  Windows host cannot show: no file symlinks without Developer Mode, junctions with
  absolute targets for directory links, only the read-only bit of a mode, no allocated
  block counts, no stat of /dev/null, read-only directories that do not protect their
  entries.
- Part 3: `ls stat du mktemp`. `ls` (D1152: `-G`/`-T` are BSD's, corpus: 23 `-T`, all
  `-lT`-style, one `-laG`): GNU long format, `total` in KiB, `-h`/`--si` rounding up,
  `-C`/`-x` with GNU's grid and tab stops (strictly narrower than the width), `-m`,
  `-R`, `-F -p -i -s -n -g -o -L -H -d -a -A -t -S -U -v -X -c -u -r -I --hide
  --time-style --full-time --group-directories-first --sort --time -w`; command-line
  symlinks to directories listed as directories unless `-l/-d/-F`; byte-order names;
  `--color=always` and birth times fall through. `stat` (D1153): GNU `-c/--printf/-t`
  and default output, BSD `-f` formatter of our own, `-t` disambiguated; GNU `-f`
  (statfs) falls through. `du`: GNU units, `-A`/`-g` from BSD, hard links and repeated
  directories once, apparent sizes count directories as 0 (as GNU here). `mktemp`
  (D1154). Shared: `fsutil::human`, `timefmt::strftime` (GNU flags `-_0^`, `%N`, `%:z`),
  `ls::glob` (fnmatch subset, used by du). Classifier: ls value options, du's long
  values, mktemp's `--tmpdir` takes an optional value only.
- `ls` in-process ended 23 agent/oils/brush ratchet lines (BSD `ls` differences on
  macOS); two more D1151 cases (`command -v ls`, `type -t ls`). The env L4 case that used
  `ls` as an example external now uses `sh` (env's case, minimal edit).
- Guide pages: utilities.md and the utilities reference are not written yet; nothing to
  update (follow-up below).

## Follow-ups
- Windows: `chmod +x` should be tracked for shebang execution (the read-only bit is
  mapped by set_mode); `rm -r` relies on the host's lstat reporting junctions as
  symlinks and remove_file removing them.
- Windows host: file symlinks need Developer Mode (else "Permission denied" today; ARCH
  wants symlink → junction → clear error), x bits and group/other bits are not kept,
  `stat /dev/null` fails with "Incorrect function", allocated blocks are unknown (du
  shows sizes in 512-byte units).
- Guide: a utilities page for the file utilities (dialect choices: D016 cp/stat/mktemp
  rules, D1151, D1152, D1153, D1154; what falls through).
- Not done: `ls` colors and terminal quoting of names, `stat -f` (statfs) and BSD
  `stat -l/-s/-x/-r`, numbered backups (`--backup=numbered`), `touch -d` beyond the
  common subset, `rm -i`/`-I` prompting (all fall through or fail clearly).
- Flaky, not caused here: oils/builtin-trap/31 (USR1 to a child shell 0.1 s after start)
  failed once in four runs under load.

## Result
Landed (D1150–D1154): the Host's file system changes and `UtilCtx` v2 (stat_extra, mkdir,
remove_file, remove_dir, rename, symlink, hard_link, read_link, set_mode, set_times,
user/group names, utc_offset, now, umask) with implementations in psh-os unix, std_host
and the test host; and 16 S-mode utilities, all written for psh after the manuals (no
ported code, NOTICE unchanged): `basename dirname realpath readlink mkdir rmdir touch`
(part 1), `rm mv cp ln chmod` (part 2; `rm mv cp ln` in the destructive group), `ls stat
du mktemp` (part 3). `pwd` stays the shell builtin. Shared helpers for later batches:
`fsutil.rs` (GNU quoting, path resolution, relative paths, `human` sizes), `modes.rs`
(chmod modes, `mode_string`), `timefmt.rs` (calendar, local time, GNU date strings for
`touch -d`, `strftime`), `ls::glob`.

L4: 229 cases in tests/utils/{basename,dirname,readlink,realpath,mkdir,rmdir,touch,rm,
mv,cp,ln,chmod,ls,stat,du,mktemp}, recorded against GNU and BSD with the dialect or D016
rule named where they differ; all pass natively (utils 493/493 with the other batches),
wasm and Windows platform ratchets list the host limits. Unit tests in
psh-utils/src/tests/files.rs (latency mode, a host without D1150, unsupported options).

How to use: nothing to configure; the registry entries are in psh-utils' lib.rs. Known
gaps: see Follow-ups (Windows host limits, colors, statfs, prompting, numbered backups).
