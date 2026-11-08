# 0014: Utilities batch C: sed

status: done
agent: parser
branch: issue/0014-utils-sed
depends-on: 0008, 0027 (psh-regex)
touches: crates/psh-utils/src/sed*, tests/utils/sed/
guide: utilities.md (not written yet), utilities reference (not written yet)
decisions: D1140–D1149

## Goal
`sed` (47k corpus commands), accepting both `-i` forms: agents write BSD `sed -i ''`
5,307 times and GNU `sed -i` 144 times in the corpus, and both must work.

## Acceptance criteria
- [x] `sed` (S), ported from uutils sed (MIT): `-n -e -f -E/-r -z -s`, GNU BRE
      extensions (`\+ \? \| \s \w`), one-line `a text`/`i`/`c`, `\U \L \E`,
      `s///Ng`, `-f` continuations.
- [x] `-i` disambiguation exactly as in D016 / ARCH.md: `''` or attached text is a
      suffix; a parsable script followed by operands is GNU; a bare `.`-prefixed
      argument without command characters followed by a script is a BSD suffix. A test
      for every branch, including `sed -i '' -e …`, `sed -i.bak`, `sed -i 's/a/b/' f`.
- [x] Each utility registered with its mode; unsupported flags fall through or fail
      clearly (tested per utility).
- [x] L4 cases in `tests/utils/<name>/` for every flag listed above, recorded against
      GNU and BSD, with the D016 rule named where they conflict.
- [x] Attribution headers and `NOTICE` entries for ported code.
- [ ] just-bash's recorded GNU fixtures (Apache) reused where they help.

## Notes
- Coordinator update: regexes go through **psh-regex** (0027, on main): POSIX BRE/ERE with
  GNU extensions, leftmost-longest, back-references in patterns, GNU error messages —
  exactly sed's semantics, so nothing needs to fall through for regex reasons and sed no
  longer depends on 0013's translator. Replacement back-references (`\1`, `&`) via its
  captures. If performance on huge inputs matters, measure against GNU sed in the container.
- oh-my-pi has a sed port (MIT) worth comparing with. GNU sed's source is off limits.

## Log

### 2026-11-07: sed
- Probed GNU sed 4.9 in the recording container and BSD sed on macOS with
  `work/scratch/0014/g.sh FILE [gnu|bsd|psh]` (blocks of `#### name` scripts, outputs
  and statuses; `p1`–`p5.txt` hold ~250 probes). psh now agrees with GNU on all of them
  but GNU's long usage text and the `e` command (which falls through).
- Implemented `crates/psh-utils/src/sed.rs` (options, `-i`), `sed/compile.rs`,
  `sed/exec.rs`, `sed/tests.rs` (D1140, D1141). Fresh, byte-oriented code instead of a
  uutils port (D1140 says why); no copied code, no NOTICE entry.
- psh-regex gained `Regex::exec_at` (search from an offset with the text before it
  visible to `^`, `\b`, `\<`), for `s///g`; and `[`/`[^` at the end is ERR 2.
- L4: `tests/utils/sed/{basic,regex,options,errors}.test.sh`, 98 cases recorded
  against GNU and BSD; every GNU/BSD conflict annotated, the `-i` branches with D016.
  All pass natively and on wasm. Unit tests: every branch of the `-i` rule.
- Ratchets: three agent and three oils/brush macOS entries pass now (they ran BSD sed).
- just-bash's fixtures: not used (the checkout is gone; GNU itself was the oracle).

### 2026-11-08: `e` in-process
- `e command`, `e` and `s///e` run through `UtilCtx::spawn_shell` (D1142), probed
  against GNU (`work/scratch/0014/p6.txt`); L4 `tests/utils/sed/exec.test.sh` (7 cases).

### 2026-11-08: numeric escapes as bytes
- spec-linux found oils/unicode/0 regressed: `sed 's/\xce//g'` must cut the first byte
  of `μ` in C.UTF-8. Regexes with escapes that leave non-UTF-8 bytes now match on bytes
  (D1143); `y` takes numeric escapes too. L4 `tests/utils/sed/escapes.test.sh` (7
  cases); `scripts/spec-linux oils/unicode/0 utils/sed` passes.

## Follow-ups
- `--debug` falls through to a real sed.
- GNU's full `--help`/usage text is not reproduced (only its first line).
- just-bash's recorded fixtures (acceptance item): not reused; GNU was probed directly.
- `--follow-symlinks` resolves through `ctx.canonicalize`; untested on Windows.

## Result
- `sed` in-process (mode S) with GNU sed 4.9's language, options and messages (D1140)
  and both `-i` forms by D016's rule (D1141). 98 L4 cases (GNU and BSD goldens) pass
  natively and on wasm; 6 macOS ratchet entries removed.
- Known gaps: no GNU `--help` text; `--debug` falls through.
