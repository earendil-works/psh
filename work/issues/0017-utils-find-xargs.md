# 0017: Utilities batch F: find and xargs

status: done
agent: procs
branch: issue/0017-utils-find-xargs
depends-on: 0008, 0006, 0013 (shared walker, globset matching, BRE/ERE translator)
touches: crates/psh-utils/src/{find,xargs}*, tests/utils/{find,xargs}/
guide: utilities.md (not written yet), utilities reference (not written yet)
decisions: D1170–D1179

## Goal
`find` (W: Windows has an incompatible `find.exe`) and `xargs` (S), both of which run
commands and must dispatch back into the shell so utilities stay reachable.

## Acceptance criteria
- [x] `find`: default path `.`; `-maxdepth -mindepth -name -iname -path -ipath -regex
      -type -size -mtime -mmin -newer -newermt -perm -empty -prune -not/! -o -a ( )
      -print -print0 -exec {} \;/+ -execdir -delete -quit -L/-H`, GNU `-printf`, BSD
      `-E`.
- [x] `xargs`: `-0 -I{}` (also attached) `-n1 -r -P N -L -d -a -t -o`. Behavior on empty
      input is decided in this issue (D034; research recommends BSD: don't run, `-r` a
      no-op) and recorded in your range; until then, empty input without `-r` falls
      through.
- [x] `find` reuses 0013's walker seam (native `ignore` walker and the virtual-fs walker)
      with all ignore filtering off; `-name`/`-iname`/`-path`/`-ipath` via `globset` in
      fnmatch mode; `-regex` via the BRE/ERE translator (`-E`/`-regextype` choose ERE);
      `-L`/`-H` symlink handling on both walkers. `-newermt` falls through until the
      shared date parser exists (0020).
- [x] Output order: walk order like GNU find (directory order is not sorted by GNU
      either); goldens use sorted-input fixtures or `| sort` where order is not the
      point. Record the choice in your range.
- [x] `-exec` and `xargs` commands go through the shell's dispatch, cancel with the
      command, and their processes are in the command's tree.
- [x] Each utility registered with its mode; unsupported flags fall through or fail
      clearly (tested per utility).
- [x] L4 cases in `tests/utils/<name>/` for every flag listed above, recorded against
      GNU and BSD, with the D016 rule named where they conflict.
- [x] Attribution headers and `NOTICE` entries for ported code.

## Notes
- uutils findutils (MIT) for the expression language and `xargs`; the walking and
  matching come from 0013 (ARCH.md "Search", D020; `research/builtins.md` §5.5).

## Log
- xargs (`crates/psh-utils/src/xargs.rs`): own option parser (GNU getopt order, the old
  `-e[EOF] -i[R] -l[N]` forms), the three input syntaxes, `-P` with `UtilChild::poll_wait`
  over the running children. Empty input runs nothing (D1170, BSD, as research recommends). Shared engine change: `SpawnEnv::{cwd, stdin}` (util.rs,
  additive) for `-execdir` and xargs's `/dev/null` stdin.
- find (`crates/psh-utils/src/find/{mod,expr,printf}.rs`): does not reuse the walker seam
  (D1171; told the coordinator, no objection): actions must interleave with the walk and
  the seam is synchronous. Its own async walk, sorted by name. `-newermt` uses the
  shared `timefmt::parse_date` (it exists now).
- find and timeout are mode S now (D1172, coordinator decision): with W the L4 suite
  tested the real programs on Unix. ARCH's mode table and windows.md updated.
  Consequence: oils/builtin-type-bash/5 and /7 (`type -t find xargs` = file) are in the
  ratchet with that reason.
- Differential check against GNU findutils 4.10 in the Linux container: ~175 find
  command lines over a fixture tree (types, links, perms, sizes, times, printf, regex,
  exec/execdir/delete, errors), psh vs GNU byte for byte, stdout, stderr and status
  (`work/scratch/0017/{fixture.sh,cases*.txt,diff.sh}`). Remaining differences are
  only the directory order of `-exec +` batches and of unsorted `-printf`/`-depth`
  output, plus `%Z` (SELinux) and the message for `-exec … ; -o` at the very end.
- L4: tests/utils/find (47 cases) and tests/utils/xargs (25), GNU and BSD goldens; unit
  tests in psh-utils/src/tests.rs and the parser modules.
- Guide: windows.md (the colliding names); no utilities page exists yet.

## Follow-ups
- `-name`/`-path` POSIX bracket classes (`[[:digit:]]`): globset lacks them; translate
  them to ranges or match with a small fnmatch of our own.
- `find -ls`/`-fls` (GNU's `ls -dils` layout) once `ls -l` formatting is shared.
- Wrapped commands that cannot run: find/xargs should say `NAME: No such file or
  directory` (GNU) instead of the shell's `command not found`; needs a spawn result
  that distinguishes "not found" from status 127.

## Result
`find` and `xargs` are psh utilities, mode S (as `timeout` now is), on every host
including wasm.

- find: `-H -L -P` and BSD's `-E -s -x -d`; options `-maxdepth -mindepth -depth (BSD
  -depth N) -xdev/-mount -follow -regextype -noleaf`; tests `-name -iname -path -ipath
  -wholename -lname -ilname -regex -iregex -type/-xtype (lists) -size -mtime -atime -ctime
  -mmin -amin -cmin -newer -anewer -cnewer -newerXY (incl. -newermt) -perm (exact, -,
  /, BSD +) -empty -links -inum -uid -gid -user -group -nouser -nogroup -readable
  -writable -executable -samefile -true -false`; actions `-print -print0 -printf -exec
  -execdir (; and +) -delete -prune -quit`; operators `! -not -a -and -o -or , ( )`.
  GNU's messages for usage errors.
- xargs: `-0 -a -d -E/-e -I/-i -L/-l -n -P -r -s -t -x -o`, GNU's statuses.
- Fall through to the real program (else "unsupported option"): find `-ls -fls -fprint*
  -ok -okdir -daystart -used` and debug options; xargs `-p --show-limits
  --process-slot-var` and BSD `-J -R -S`.

Known gaps: `-name` brackets have no POSIX classes (`[[:alpha:]]`; globset);
`-printf %F` prints `unknown`, `%Z` nothing, `%TZ` only `UTC`/an offset or `$TZ`; a
command that cannot run in `-exec`/xargs gets the shell's "command not found" message
instead of find's/xargs's `No such file or directory`; xargs `-o` errors differ.
