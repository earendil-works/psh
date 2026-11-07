# 0016: Utilities batch E: text filters

status: in-progress
agent: text
branch: issue/0016-utils-text
depends-on: 0008
touches: crates/psh-utils/src/{sort,uniq,cut,tr,paste,nl,tee,seq,comm}*, tests/utils/ (same names)
guide: utilities.md (not written yet), utilities reference (not written yet)
decisions: D1160–D1169

## Goal
The text filters of Tier 1, all S: `sort uniq cut tr paste nl tee seq comm`.

## Acceptance criteria
- [ ] `sort`: `-u -n -r -k -t -V -h -s -z -o -R`. Default collation is decided in
      this issue (D034): propose it with evidence and record it in your range; until then
      sorting in a non-C locale falls through.
- [ ] `uniq`: `-c` with GNU widths, `-d -u -f`.
- [ ] `cut`: `-c` counts characters, not bytes, on UTF-8 (agents mean characters;
      record it as a deliberate difference from GNU); `-d -f -b`.
- [ ] `tr`: classes and octal escapes. `paste -sd+`, `nl -ba`, `tee -a`, `seq -w -s
      -f`, `comm`.
- [ ] Each utility registered with its mode; unsupported flags fall through or fail
      clearly (tested per utility).
- [ ] L4 cases in `tests/utils/<name>/` for every flag listed above, recorded against
      GNU and BSD, with the D016 rule named where they conflict.
- [ ] Attribution headers and `NOTICE` entries for ported code.

## Notes
- uutils (MIT).

## Log

## Follow-ups

## Result
