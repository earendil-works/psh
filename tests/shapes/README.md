# The shape corpus

`shapes.txt` holds the syntax skeletons of real commands that coding agents ran: what
the commands look like to a parser, with every private word removed. It tests the parser
at scale (L0: psh must agree with `bash -n` on every skeleton) and ranks which syntax
matters. Nothing in it is ever executed.

## Format

One skeleton per line, most frequent first:

```
COUNT<TAB>SKELETON
```

`COUNT` is the number of corpus commands with this skeleton. The skeleton is escaped:
`\\` is a backslash, `\n` a newline, `\t` a tab; nothing else is escaped. Examples:

```
3337	git status --short
1635	python3 - <<'PY'\nw0\nPY
925	cd w1 && node "$(git rev-parse --show-toplevel)/w2" --run w3
```

What a skeleton keeps and what it replaces:

- Kept: operators, redirections with their fd numbers, quotes of every kind,
  here-doc operators and delimiters, `$(…)`, `` `…` ``, `<(…)`, `${…}` with its
  operators, arithmetic operators, keywords, comments' `#`, and the punctuation inside
  words (`*.w1`, `{w1,w2}`, `~/w1`, `w1=w2`).
- Kept when listed in `vocabulary.txt`: command names, utility names, subcommands of
  tools like `git` and `cargo`, option tokens (`--release`, `-la`; never a `=value`),
  well-known variables (`HOME`, `PATH`), here-doc delimiters (`EOF`), single digits,
  special parameters (`$@`, `$?`, `$1`).
- Replaced: every other word becomes `w1`, `w2`, …, every other variable `v1`, `v2`,
  …, numbered per command in order of appearance; the same word gets the same
  placeholder within a command. An unlisted option keeps its dashes (`-w3`,
  `--w4=w5`). Quoted text is always a placeholder, even a listed word. A here-doc body
  becomes the single line `w0`, a comment `# w0`.

## Privacy

The raw corpus is private and never leaves the machine it was collected on. Only this
directory is committed, and `scripts/shape-corpus-check` (run by the gate) proves that
every token in `shapes.txt` is a vocabulary entry, a placeholder or a single digit.
`vocabulary.txt` is the whole list of words that can appear; review every change to it
by hand. Its corpus-derived sections take only words seen in at least 20 distinct
commands, 5 sessions and 3 projects; subcommands only from tools whose first argument
is their own subcommand (not `make`, `just`, `npm`-style script runners); upper-case
variables only without `_` or with a public tool's prefix (`CARGO_`, `NODE_`, …).

## How it was made

`scripts/shape-corpus` (Python, run locally; it needs tree-sitter-bash and the private
corpus, the committed files do not):

```
scripts/shape-corpus vocab       # count candidates, write vocabulary.txt
scripts/shape-corpus build       # write shapes.txt, then run the privacy check
scripts/shape-corpus parsecheck  # bash -n on every skeleton vs. its originals
scripts/shape-corpus selftest    # skeletons of hand-written commands
```

The tree comes from tree-sitter-bash, not from psh's parser, so L0 never tests the parser
against itself. The skeleton is the original text with each word replaced in place, so
spacing, line structure and operators are the original's. Text the tree does not
explain (tree-sitter misparses) is still replaced word by word.

Selection: every skeleton seen in at least two sessions is written. Of the skeletons
from a single session, those that add a syntax 4-gram not yet covered (tokens:
punctuation, newlines, keywords, and `W` for any other word) are written, shortest
first, up to 2 MB; skeletons over 4 KB are skipped. Every 4-gram of the corpus is
covered; the rest of the long tail is left out.

## Counts

From the corpus of 2026-11-06:

| | |
|---|--:|
| commands | 295,401 |
| unique commands | 263,184 |
| unique skeletons | 187,784 |
| skeletons seen in ≥ 2 sessions (all written) | 10,381 (106,287 commands) |
| single-session skeletons written for coverage | 15,853 |
| **written** | **26,234 skeletons, 122,848 commands, 2.3 MB** |
| syntax 4-grams covered | 42,414 of 42,414 |
| tree-sitter parse errors (rewritten anyway) | 321 |

`bash -n` (5.3) on every skeleton against the verdict on its original:

| original | skeleton | unique commands |
|---|---|--:|
| parses | parses | 262,970 |
| parses | **fails** | 7 |
| fails | fails | 207 |
| fails | parses | 0 |

The 7 are tree-sitter misparses of zsh syntax and unbalanced quoting.

Syntax in the written skeletons (skeletons / commands they stand for): here-docs
2,623 / 13,640; multi-line 4,688 / 16,110; `$(…)` 2,332 / 5,341; `for` 2,461 / 2,603;
`if` 521 / 583; functions 214 / 220; `case` 116 / 117; `[[ ]]` 132 / 136; process
substitution 168 / 171; here-strings 92 / 107; backticks 611 / 638.
