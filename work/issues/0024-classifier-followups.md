# 0024: Classifier follow-ups and its guide page

status: open
agent: -
branch: issue/0024-classifier-followups
depends-on: 0009
touches: crates/psh-classify/, tests/classify/, docs/guide/classify.md, docs/guide/SUMMARY.md (one line)
guide: classify.md
decisions: D1240–D1249

## Goal
Make the classifier less conservative where it safely can, and explain it to humans.

## Acceptance criteria
- [ ] Literal variable tracking within one script: a variable assigned a literal (no
      expansion, not exported into a dynamic context, not modified in a loop/function
      that runs conditionally) earlier in the same straight-line list is substituted for
      classification, so `S=/x; sed -n 1,2p $S/f` is read-only. Conservative fallback
      to unknown on anything else. Report the corpus effect (aggregate only).
- [ ] Read-only `sqlite3` invocations (`-readonly`, or a literal query that is only
      SELECT/PRAGMA reads/.schema/.tables) classified as read.
- [ ] `docs/guide/classify.md`: what the classifier reports, the categories, what counts
      as read-only and why it is conservative, nested commands, how a host uses it
      (`psh --classify`, the crate API), with examples. Follows docs/guide/README.md
      rules; `psh-docs check` passes.
- [ ] New labelled cases for both changes, including adversarial ones (variable
      reassigned in a branch, in a function, via `read`, `declare -n`).

## Log

## Follow-ups

## Result
