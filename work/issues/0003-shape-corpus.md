# 0003: Shape corpus

status: done
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
- [x] `scripts/shape-corpus` reads the raw commands from `~/.cache/psh-corpus/` (see
      `research/corpus/extract.py` for the format) and writes `tests/shapes/shapes.txt`:
      unique skeletons with their counts, most frequent first.
- [x] A skeleton keeps operators, redirections (with fd numbers), quoting kinds,
      here-doc markers and structure (`$(…)`, `<(…)`, `${…}` operators, arithmetic
      operators, keywords). It keeps words only from a fixed **vocabulary**: shell
      keywords and builtins, utility names (`research/builtins.md`), names of
      never-builtins and their common subcommands (`git status`, `cargo test`), option
      tokens matching `^--?[A-Za-z0-9][A-Za-z0-9-]*$` (no `=value`), special parameters
      and well-known variables (`HOME`, `PATH`, `PWD`, `1`, `@`, `?`). Every other word
      becomes a placeholder (`w1`, `w2`, ...; variables `v1`, ...), numbered per
      command. Here-doc bodies become a fixed placeholder line.
- [x] A privacy check (part of the tool and of the gate leg) fails if any token of the
      output is outside the vocabulary or placeholders. The vocabulary file is
      committed and reviewed.
- [x] Skeletons must still parse where the original did: report how many originals
      that `bash -n` accepts give skeletons it rejects, and fix the tool until that is
      close to zero (run `bash -n` locally under `scripts/timeout.sh`; parsing executes
      nothing).
- [x] A short `tests/shapes/README.md`: what the file is, how it was made, the counts.
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
- 2026-11-06: Tool on tree-sitter-bash (the research venv), rewriting the original text
  in place; text the tree does not cover is sanitized word by word, so misparses cannot
  leak. Debug output of violations only ever shows token shapes (letters → `a`).
- Vocabulary: fixed lists plus corpus-frequent words. A first pass with "≥ 3 cwds"
  let project-private flags and env vars in (worktrees count as cwds); now ≥ 3 projects
  (cwd folded to 4 components, worktrees folded), subcommands not from make/just/
  pnpm/yarn/bun/npx, upper-case variables only without `_` or with public prefixes.
- Check bugs found by the check itself: `-name:w1`-style tokens (option + rest),
  `-pix_fmt` splitting into `-w1w2`, adjacent tree-sitter nodes inside one word in
  misparsed here-docs gluing into `iw1`. All fixed in the tool (option names take `_`,
  touching word segments merge) and the check (split at `./@%+:` first, option names
  pass as pieces). 0 commands dropped by the check now.
- Quoted text is always a placeholder (vocabulary words in messages/patterns were
  kept at first; harmless but pointless).
- Size: 188k unique skeletons, 18.5 MB, 172k of them seen once. Committed: all
  skeletons from ≥ 2 sessions plus 4-gram coverage from the rest (D1033): 26,234
  skeletons, 2.3 MB.
- `bash -n` (Homebrew 5.3, 10 s Python subprocess timeout): of 262,977 originals that
  parse, 7 skeletons fail (tree-sitter misparses of zsh `${(f)…}` and unbalanced
  quoting); 0 originals that fail give a parsing skeleton.
- Rebased onto main after 0001; gate leg `shapes` (self-test + check, python3 stdlib).
- Docs: TESTING.md "The shape corpus" and the gate list updated. No guide pages.

## Follow-ups
- L0 leg on the shapes (parse agreement with recorded `bash -n` verdicts, own ratchet)
  once 0002's recording and 0004's parser have landed. The verdicts should be recorded
  in the pinned container like the other goldens; the local run above used macOS bash.
- The `shapes` gate leg needs `python3`; the Windows gate (0007) must provide it or
  port the ~100-line check (e.g. into psh-spec).
- Re-run `scripts/shape-corpus vocab && build` when the corpus is re-extracted; review
  the vocabulary diff by hand.

## Result
- `scripts/shape-corpus` (local; tree-sitter-bash + private corpus): `vocab`, `build`,
  `parsecheck`, `selftest`. `scripts/shape-corpus-check` (stdlib; `--self-test`), run by
  the gate leg `shapes`.
- `tests/shapes/shapes.txt`: 26,234 skeletons (`COUNT\tESCAPED-SKELETON`, most frequent
  first) for 122,848 of 295,401 commands, covering every syntax 4-gram of the corpus.
- `tests/shapes/vocabulary.txt`: 1,362 entries in `[words]`, `[variables]`, `[options]`.
- `tests/shapes/README.md`: format, rules, privacy, counts. Decisions D1030–D1033.
- Known gaps: L0 leg not yet wired (follow-up); 7 skeletons of parsing originals do not
  parse; the 172k-skeleton single-session tail is not committed beyond coverage.
