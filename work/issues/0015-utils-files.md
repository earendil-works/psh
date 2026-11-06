# 0015: Utilities batch D: files and paths

status: open
agent: -
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
      wait for the owner's dialect decision (ARCH.md open question 12): fall through
      until then.
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

## Follow-ups

## Result
