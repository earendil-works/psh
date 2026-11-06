# 0017: Utilities batch F: find and xargs

status: open
agent: -
branch: issue/0017-utils-find-xargs
depends-on: 0008, 0006, 0013 (shared walker, globset matching, BRE/ERE translator)
touches: crates/psh-utils/src/{find,xargs}*, tests/utils/{find,xargs}/
guide: utilities.md (not written yet), utilities reference (not written yet)
decisions: D1170–D1179

## Goal
`find` (W: Windows has an incompatible `find.exe`) and `xargs` (S), both of which run
commands and must dispatch back into the shell so utilities stay reachable.

## Acceptance criteria
- [ ] `find`: default path `.`; `-maxdepth -mindepth -name -iname -path -ipath -regex
      -type -size -mtime -mmin -newer -newermt -perm -empty -prune -not/! -o -a ( )
      -print -print0 -exec {} \;/+ -execdir -delete -quit -L/-H`, GNU `-printf`, BSD
      `-E`.
- [ ] `xargs`: `-0 -I{}` (also attached) `-n1 -r -P N -L -d -a -t -o`. Behavior on empty
      input is decided in this issue (D034; research recommends BSD: don't run, `-r` a
      no-op) and recorded in your range; until then, empty input without `-r` falls
      through.
- [ ] `find` reuses 0013's walker seam (native `ignore` walker and the virtual-fs walker)
      with all ignore filtering off; `-name`/`-iname`/`-path`/`-ipath` via `globset` in
      fnmatch mode; `-regex` via the BRE/ERE translator (`-E`/`-regextype` choose ERE);
      `-L`/`-H` symlink handling on both walkers. `-newermt` falls through until the
      shared date parser exists (0020).
- [ ] Output order: walk order like GNU find (directory order is not sorted by GNU
      either); goldens use sorted-input fixtures or `| sort` where order is not the
      point. Record the choice in your range.
- [ ] `-exec` and `xargs` commands go through the shell's dispatch, cancel with the
      command, and their processes are in the command's tree.
- [ ] Each utility registered with its mode; unsupported flags fall through or fail
      clearly (tested per utility).
- [ ] L4 cases in `tests/utils/<name>/` for every flag listed above, recorded against
      GNU and BSD, with the D016 rule named where they conflict.
- [ ] Attribution headers and `NOTICE` entries for ported code.

## Notes
- uutils findutils (MIT) for the expression language and `xargs`; the walking and
  matching come from 0013 (ARCH.md "Search", D020; `research/builtins.md` §5.5).

## Log

## Follow-ups

## Result
