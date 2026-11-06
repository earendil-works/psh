# 0003: Shape corpus

status: in-progress
agent: shapes
branch: issue/0003-shape-corpus
depends-on: 0001 (the L0 leg needs 0002's bash -n recording and 0004's parser)
touches: scripts/shape-corpus*, tests/shapes/
guide: -
decisions: D1030–D1039

## Goal
Turn the private corpus of 295k agent commands into a committed, privacy-free set of
syntax skeletons that tests the parser at scale and ranks which syntax matters
(TESTING.md "The shape corpus", `research/hillclimb.md`). Nothing is ever executed.

## Acceptance criteria
- [ ] `scripts/shape-corpus` reads the raw commands from `~/.cache/psh-corpus/` (see
      `research/corpus/extract.py` for the format) and writes `tests/shapes/shapes.txt`:
      unique skeletons with their counts, most frequent first.
- [ ] A skeleton keeps operators, redirections (with fd numbers), quoting kinds,
      here-doc markers and structure (`$(…)`, `<(…)`, `${…}` operators, arithmetic
      operators, keywords). It keeps words only from a fixed **vocabulary**: shell
      keywords and builtins, utility names (`research/builtins.md`), names of
      never-builtins and their common subcommands (`git status`, `cargo test`), option
      tokens matching `^--?[A-Za-z0-9][A-Za-z0-9-]*$` (no `=value`), special parameters
      and well-known variables (`HOME`, `PATH`, `PWD`, `1`, `@`, `?`). Every other word
      becomes a placeholder (`w1`, `w2`, ...; variables `v1`, ...), numbered per
      command. Here-doc bodies become a fixed placeholder line.
- [ ] A privacy check (part of the tool and of the gate leg) fails if any token of the
      output is outside the vocabulary or placeholders. The vocabulary file is
      committed and reviewed.
- [ ] Skeletons must still parse where the original did: report how many originals
      that `bash -n` accepts give skeletons it rejects, and fix the tool until that is
      close to zero (run `bash -n` locally under `scripts/timeout.sh`; parsing executes
      nothing).
- [ ] A short `tests/shapes/README.md`: what the file is, how it was made, the counts.
- [ ] Once 0002 and 0004 have landed: the shapes are an L0 input (parse agreement with
      the recorded `bash -n` verdicts) with their own ratchet. This part may be left as
      a follow-up if 0004 is not merged yet.

## Notes
- The tokenizer must be independent of psh-syntax, or L0 would test the parser against
  itself. A small hand-written bash tokenizer in the script is enough, or tree-sitter-bash
  locally (as `research/corpus/` did; it parses 99.9% of the corpus). If you need a
  Python or npm package for this local tool, ask first; it is not part of the build.
- The coordinator reviews a sample of the output for privacy before merging.

## Log

## Follow-ups

## Result
