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
- Part 1: `tee seq tr cut paste`, written for psh from GNU's manual and man pages (no
  uutils code copied, so no NOTICE entries). Shared helpers for the text filters go in
  `crates/psh-utils/src/textio.rs` (line reader over `common::Input`, UTF-8 character
  lengths, open-or-report); unit tests in `src/tests_text.rs` (a submodule of
  `tests.rs`, so other batches' edits there do not conflict).
- Classifier: `seq` now declares `-f -s -t`/`--format --separator` as value options.
- Decisions: D1161 (`cut -c` counts characters everywhere), D1162 (`tr` on bytes unless a
  set has a non-ASCII character). `seq`, `paste` follow GNU where BSD differs (D016):
  `seq 0` prints nothing, `-s` separates (no trailing separator), decimals follow the
  operands; `paste -s` always ends lines, `-d ''` is no delimiter.
- `oils/builtin-meta/10` puts a `tr` script on PATH and expects it to win; `tr` is mode S,
  so it is ratcheted with that reason.
- Error messages use ASCII quotes like the other utilities; the container's GNU prints
  ‘’ (stderr score only).
- Guide: none affected (utilities.md not written yet).
- Part 2: `uniq comm nl`, also written from the manuals. `textio::fall_through(ctx,
  argv, what)` runs the real program for a form psh lacks (`nl -b pREGEX`: no regex
  engine in psh-utils yet), else `NAME: unsupported WHAT (psh built-in NAME)`, status 2.
  Classifier: `nl`'s long value options added. GNU over BSD (D016): `uniq -c` widths,
  `nl` indents unnumbered lines with spaces and prints section delimiters as empty
  lines; `comm` checks the order as GNU does; BSD's `comm -i` and `cut -w` accepted.
- Part 3: `sort`, written from GNU's manual (keys, last resort, `-V` as documented for
  filevercmp: Debian version order plus the suffix rule). D1160: byte collation in every
  locale (evidence in the decision). Inputs are read whole, sorted with a stable sort
  (`-u` keeps the first of equal keys, `-o` may name an input), on the vproc; 1M lines
  sort in well under a second natively. `-S -T --parallel --batch-size` are accepted and
  ignored; `--compress-program --files0-from --random-source --debug` fall through.
  Classifier: `sort`'s long value options completed.

## Follow-ups

## Result
