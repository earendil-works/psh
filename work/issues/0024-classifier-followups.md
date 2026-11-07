# 0024: Classifier follow-ups and its guide page

status: done
agent: shapes
branch: issue/0024-classifier-followups
depends-on: 0009
touches: crates/psh-classify/, tests/classify/, docs/guide/classify.md, docs/guide/SUMMARY.md (one line)
guide: classify.md
decisions: D1240–D1249

## Goal
Make the classifier less conservative where it safely can, and explain it to humans.

## Acceptance criteria
- [x] Literal variable tracking within one script: a variable assigned a literal (no
      expansion, not exported into a dynamic context, not modified in a loop/function
      that runs conditionally) earlier in the same straight-line list is substituted for
      classification, so `S=/x; sed -n 1,2p $S/f` is read-only. Conservative fallback
      to unknown on anything else. Report the corpus effect (aggregate only).
- [x] Read-only `sqlite3` invocations (`-readonly`, or a literal query that is only
      SELECT/PRAGMA reads/.schema/.tables) classified as read.
- [x] `docs/guide/classify.md`: what the classifier reports, the categories, what counts
      as read-only and why it is conservative, nested commands, how a host uses it
      (`psh --classify`, the crate API), with examples. Follows docs/guide/README.md
      rules; `psh-docs check` passes.
- [x] New labelled cases for both changes, including adversarial ones (variable
      reassigned in a branch, in a function, via `read`, `declare -n`).

## Log
- 2026-11-06: sqlite3 (`src/sqlite.rs`, D1241): literal SQL arguments and `-cmd` values
  are split into statements and dot-commands; read, mutate (the database file is
  reported as written) or unknown.
- Literal variable tracking (`src/vars.rs`, D1240): a flow-insensitive scan of every
  write per variable, plus a definite-assignment set kept by the walker along lists and
  `&&` chains; substitution in `Arg::value` (`vars::value`), so every rule benefits.
- `$HOME`/`$PWD`-prefixed words are not options (D1242), found in the corpus run.
- Cases: +70 (tracking incl. reassignment in branches, functions, `read`, `mapfile`,
  `declare -n`, `trap`, `eval`, subshells, groups, pipelines, background, `&&` after
  `cd`; sqlite3 reads, writes and the dangerous dot-commands); 645 in total.
- Corpus (aggregate, tests/classify/README.md): read-only 153,422 → 153,853 (52.1%);
  tracking ≈ +230, sqlite3 +86, `$HOME`/`$PWD` +115.
- Guide: docs/guide/classify.md written, SUMMARY.md links it; `psh-docs check` passes.
  Every example on the page was checked with `psh --classify`.

## Follow-ups
- For-loop variables over literal lists (`for f in a b; do rg x "$f"; done`) are still
  unknown where an option could matter; classifying the body once per value would cover
  them (a few hundred corpus commands).
- `sqlite3` with a literal here-document on stdin could be checked like an argument.

## Result
- `psh-classify` substitutes variables with a definitely-assigned literal value
  (D1240), classifies `sqlite3` from its SQL (D1241), and treats `$HOME`/`$PWD` paths as
  non-options (D1242). Read-only on the corpus: 52.1%.
- `docs/guide/classify.md`: what the classifier reports, the categories, read-only and
  the conservative rules, nested commands, known variables, `psh --classify` and the
  crate API, corpus numbers, source links.
- 645 labelled cases in tests/classify/cases.txt.
