# 0016: Utilities batch E: text filters

status: done
agent: text
branch: issue/0016-utils-text
depends-on: 0008
touches: crates/psh-utils/src/{sort,uniq,cut,tr,paste,nl,tee,seq,comm}*, tests/utils/ (same names)
guide: utilities.md (not written yet), utilities reference (not written yet)
decisions: D1160–D1169

## Goal
The text filters of Tier 1, all S: `sort uniq cut tr paste nl tee seq comm`.

## Acceptance criteria
- [x] `sort`: `-u -n -r -k -t -V -h -s -z -o -R`. Default collation is decided in
      this issue (D034): propose it with evidence and record it in your range; until then
      sorting in a non-C locale falls through.
- [x] `uniq`: `-c` with GNU widths, `-d -u -f`.
- [x] `cut`: `-c` counts characters, not bytes, on UTF-8 (agents mean characters;
      record it as a deliberate difference from GNU); `-d -f -b`.
- [x] `tr`: classes and octal escapes. `paste -sd+`, `nl -ba`, `tee -a`, `seq -w -s
      -f`, `comm`.
- [x] Each utility registered with its mode; unsupported flags fall through or fail
      clearly (tested per utility).
- [x] L4 cases in `tests/utils/<name>/` for every flag listed above, recorded against
      GNU and BSD, with the D016 rule named where they conflict.
- [x] Attribution headers and `NOTICE` entries for ported code.

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
- With `tr`/`sort` built in, five wasm cases that used to be excused as "needs external
  programs" now run and fail on the JS host's missing mkdir/touch (D1150): wasm-ratcheted
  with the existing reason (oils/glob-bash/7, oils/globstar/0,1,3,
  utils/realpath/basic/11). Six utils wasm entries pass now and were removed.
- Gate runs started in the background inherit SIGINT ignored (non-interactive `&`) and
  fail psh-os/psh signal tests; run merge-to-main in the foreground or reset INT first.

## Follow-ups
- `nl -b pREGEX` (and `-h`/`-f` with `p`) falls through: needs the regex crate in
  psh-utils (0013 brings it); then a BRE-to-regex translation shared with `sed`/`grep`.
- Error messages quote with ASCII `'…'`; the recording container's GNU uses `‘…’`
  (stderr score only, all utilities alike). A shared decision for the quote style would
  lift the stderr score of every utility.
- `sort` reads its inputs whole on the vproc (no external merge); a multi-GB sort needs
  the blocking-utility path (D1080) or a merge on temporary files.

## Result
Landed `sort uniq cut tr paste nl tee seq comm`, all mode S, in
`crates/psh-utils/src/{sort,uniq,cut,tr,paste,nl,tee,seq,comm}.rs`, written for psh from
the GNU manuals and man pages (no ported code, so no NOTICE entries). Shared helpers in
`textio.rs`: `LineReader` (delimited lines from a `common::Input`), `char_lengths`/
`decode_char` (UTF-8 positions), `open_or_report`, and `fall_through(ctx, argv, what)` for
forms psh does not implement. Unit tests in `src/tests_text.rs`; L4: 191 cases in
`tests/utils/{sort,uniq,cut,tr,paste,nl,tee,seq,comm}/`, GNU and BSD recorded, all pass
natively and in wasm (stderr differs only in GNU's ‘’ quotes).

Decisions: D1160 (sort collation: bytes in every locale, settling D034), D1161 (`cut -c`
counts characters), D1162 (`tr` on bytes unless a set has a non-ASCII character).
Elsewhere GNU wins per D016 (`uniq -c` widths, `seq` and `paste` endings, `nl`
indentation), BSD-only flags are accepted where they do not conflict (`cut -w`, `comm
-i`, `tr -C -u`, `seq -t`). Unknown options fall through to the real program or fail
with `NAME: unsupported option` (status 2), tested for every utility.

Known gaps: see Follow-ups (`nl -bp`, quote style, huge sorts).
