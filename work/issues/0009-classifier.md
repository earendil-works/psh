# 0009: Classifier

status: in-progress
agent: shapes
branch: issue/0009-classifier
depends-on: 0004
touches: crates/psh-classify/, crates/psh/ (the --classify flag), tests/classify/
guide: classify.md (not written yet; list it as a follow-up)
decisions: D1090–D1099

## Goal
Understanding commands with the same parser that runs them (ARCH.md "psh-classify",
D013): per-command effects and a category for UI summaries, read-only detection and
permission checks. Conservative: anything dynamic is `unknown`, never "read-only".

## Acceptance criteria
- [ ] `psh-classify` (depends only on psh-syntax; builds for wasm32): for each simple
      command its name (literal or dynamic), argv words with spans, redirections and
      their targets, nested commands (`$(…)`, `<(…)`, literal `bash -c`/`eval`
      strings, `xargs`/`find -exec`/`env`/`timeout`/`nice`/`nohup` wrapped commands).
- [ ] Categories: read, mutate, VCS, build, network, install, long-running, unknown;
      effects: paths read and written, output truncated (`| head`).
- [ ] Per-utility flag tables (e.g. `sed -i` writes) in a place both the classifier and
      the utilities can use (ARCH.md open question 2; propose and record).
- [ ] `psh-classify` is `#![no_std]` + `alloc` like the parser (D023).
- [ ] `psh --classify [-c CMD | FILE]` prints JSON via `deser-json` (D029), outside the
      no_std crate.
- [ ] `tests/classify/`: at least 300 labelled cases, including adversarial ones
      (`git commit -m "$(rm -rf /)"`, `x=rm; $x -rf /`, `;` after a safe prefix,
      aliases, functions, redirections to files).
- [ ] An accuracy run against the private corpus, done locally: compare with the
      categories of `research/corpus/analyze.py`, commit only aggregate numbers.

## Notes
- `research/corpus.md` §3 and §9 for the categories and the 52% pure-read figure;
  `research/landscape.md` §4 for what must never count as read-only.

## Log

## Follow-ups

## Result
