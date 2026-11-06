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
- [x] `psh-classify` (depends only on psh-syntax; builds for wasm32): for each simple
      command its name (literal or dynamic), argv words with spans, redirections and
      their targets, nested commands (`$(…)`, `<(…)`, literal `bash -c`/`eval`
      strings, `xargs`/`find -exec`/`env`/`timeout`/`nice`/`nohup` wrapped commands).
- [x] Categories: read, mutate, VCS, build, network, install, long-running, unknown;
      effects: paths read and written, output truncated (`| head`).
- [x] Per-utility flag tables (e.g. `sed -i` writes) in a place both the classifier and
      the utilities can use (ARCH.md open question 2; propose and record).
- [x] `psh-classify` is `#![no_std]` + `alloc` like the parser (D023).
- [ ] `psh --classify [-c CMD | FILE]` prints JSON via `deser-json` (D029), outside the
      no_std crate.
- [x] `tests/classify/`: at least 300 labelled cases, including adversarial ones
      (`git commit -m "$(rm -rf /)"`, `x=rm; $x -rf /`, `;` after a safe prefix,
      aliases, functions, redirections to files).
- [x] An accuracy run against the private corpus, done locally: compare with the
      categories of `research/corpus/analyze.py`, commit only aggregate numbers.

## Notes
- `research/corpus.md` §3 and §9 for the categories and the 52% pure-read figure;
  `research/landscape.md` §4 for what must never count as read-only.

## Log
- 2026-11-06 (part 1): `crates/psh-classify` (no_std + alloc, psh-syntax only; gate legs
  `deps` and `wasm` extended to it, a two-line change in scripts/gate.sh).
  - `classify(src)` / `classify_ast(&Ast)` → `Classification`: every simple command
    (name, words with spans and literal values, assignments, redirections, parent,
    `via`, categories, unknown reason), nested sources, reads, writes, truncation.
  - Walk: compound commands, all word parts (`$(…)`, backquotes re-parsed with
    `backtick_body`, `<(…)`, `${…}` operands, arithmetic, braces, arrays), unquoted
    here-doc bodies, redirection targets.
  - Rules (`rules.rs`): builtins, wrappers (env, timeout, nice, nohup, command, exec,
    builtin, stdbuf, caffeinate, time, sudo/doas, xargs, find -exec, uv run), shells
    (`-c` strings and literal here-docs on stdin parsed as nested scripts), eval, trap,
    alias, git/gh/jj, npm/pnpm/yarn/bun, cargo, go, uv, pip, system package managers,
    docker, tmux, services, rsync, tar, unzip, compressors, sips, plutil, interpreters.
  - Tables (`utils.rs`, D1090) for ~250 utilities; a small sed script scanner
    (`sed.rs`: `w`, `e`, `r`, the `s///w`/`e` flags) and an awk program check
    (strings and regex literals skipped).
  - Decisions D1090–D1093; ARCH's open question 2 marked decided (one paragraph).
- Cases: tests/classify/cases.txt, 571 labelled cases incl. adversarial sections;
  unit tests for spans, nesting and the sed scanner.
- Corpus run (local, `tests/classify/corpus_accuracy.py`, aggregates in
  tests/classify/README.md): read-only 51.9% vs analyze.py's 51.7%, 150,456 in both.
  Disagreements were reviewed as privacy-safe skeletons (scripts/shape-corpus), and
  fixed where psh-classify was wrong: braces and words with a literal prefix are not
  options, awk string/regex literals, `uv run -`, gzip -c, tee /dev/null,
  `curl -o /dev/null`, sips/plutil, shells with startup files, `xargs … sh -c '{}'`.
- psh-syntax: no API changes needed.
- `psh --classify` waits for 0004 part 2 (the JSON plumbing in crates/psh).

## Follow-ups
- Guide page `classify.md` (not written yet).
- Constant propagation for variables assigned literally in the same script
  (`S=/x; sed -n 1,2p $S/f` is unknown today): ~800 corpus commands.
- `sqlite3` read-only queries (`-readonly`, `.schema`), 213 corpus commands.

## Result
