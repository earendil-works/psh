# 0017: Utilities batch F: find and xargs

status: open
agent: -
branch: issue/0017-utils-find-xargs
depends-on: 0008, 0006
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
- [ ] `xargs`: `-0 -I{}` (also attached) `-n1 -r -P N -L -d -a -t -o`. Empty input
      waits for the owner's decision (ARCH.md open question 12; research recommends BSD).
- [ ] `-exec` and `xargs` commands go through the shell's dispatch, cancel with the
      command, and their processes are in the command's tree.
- [ ] Each utility registered with its mode; unsupported flags fall through or fail
      clearly (tested per utility).
- [ ] L4 cases in `tests/utils/<name>/` for every flag listed below, recorded against
      GNU and BSD, with the D016 rule named where they conflict.
- [ ] Attribution headers and `NOTICE` entries for ported code.

## Notes
- uutils findutils (MIT). Whether `find` reuses the `ignore`/`globset` crates is ARCH.md
  open question 6.

## Log

## Follow-ups

## Result
