# 0014: Utilities batch C: sed

status: open
agent: -
branch: issue/0014-utils-sed
depends-on: 0008, 0027 (psh-regex)
touches: crates/psh-utils/src/sed*, tests/utils/sed/
guide: utilities.md (not written yet), utilities reference (not written yet)
decisions: D1140–D1149

## Goal
`sed` (47k corpus commands), accepting both `-i` forms: agents write BSD `sed -i ''`
5,307 times and GNU `sed -i` 144 times in the corpus, and both must work.

## Acceptance criteria
- [ ] `sed` (S), ported from uutils sed (MIT): `-n -e -f -E/-r -z -s`, GNU BRE
      extensions (`\+ \? \| \s \w`), one-line `a text`/`i`/`c`, `\U \L \E`,
      `s///Ng`, `-f` continuations.
- [ ] `-i` disambiguation exactly as in D016 / ARCH.md: `''` or attached text is a
      suffix; a parsable script followed by operands is GNU; a bare `.`-prefixed
      argument without command characters followed by a script is a BSD suffix. A test
      for every branch, including `sed -i '' -e …`, `sed -i.bak`, `sed -i 's/a/b/' f`.
- [ ] Each utility registered with its mode; unsupported flags fall through or fail
      clearly (tested per utility).
- [ ] L4 cases in `tests/utils/<name>/` for every flag listed above, recorded against
      GNU and BSD, with the D016 rule named where they conflict.
- [ ] Attribution headers and `NOTICE` entries for ported code.
- [ ] just-bash's recorded GNU fixtures (Apache) reused where they help.

## Notes
- Coordinator update: regexes go through **psh-regex** (0027, on main): POSIX BRE/ERE with
  GNU extensions, leftmost-longest, back-references in patterns, GNU error messages —
  exactly sed's semantics, so nothing needs to fall through for regex reasons and sed no
  longer depends on 0013's translator. Replacement back-references (`\1`, `&`) via its
  captures. If performance on huge inputs matters, measure against GNU sed in the container.
- oh-my-pi has a sed port (MIT) worth comparing with. GNU sed's source is off limits.

## Log

## Follow-ups

## Result
