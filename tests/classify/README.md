# Classifier cases

`cases.txt` holds labelled commands for `psh-classify`; `cargo test -p psh-classify`
runs them (`crates/psh-classify/tests/cases.rs`). Each line is
`CATEGORY<TAB>COMMAND[<TAB>ASSERTIONS]`; the header of `cases.txt` lists the assertions.
`read` as the expected category means read-only: the command is safe to auto-approve.
Adversarial cases (a safe prefix followed by something else, `$(…)` inside a commit
message, functions and aliases shadowing `ls`, file names spliced into `sh -c`) have
their own sections.

## Accuracy on the private corpus

`corpus_accuracy.py` compares the classifier with the categories of
`research/corpus/analyze.py` (tree-sitter, written for the corpus study) on the private
corpus. It runs locally and prints aggregates only. On the corpus of 2026-11-06:

| | |
|---|--:|
| commands | 295,401 |
| parse errors (classified unknown) | 208 (0.07%) |
| read-only, psh-classify | 153,853 (52.1%) |
| read-only, analyze.py ("pure read") | 152,595 (51.7%) |
| read-only for both | 150,866 |

Primary categories: read 52.1%, unknown 29.1%, build 8.2%, vcs 3.7%, mutate 3.5%,
network 3.0%, install 0.3%, long-running 0.1%. Unknown is large because inline
interpreters (`python3 - <<'PY'`, `node -e`) and local scripts are unknown by design and
outrank every other category in a command.

Read-only for psh-classify only (2,987): analyze.py labels `ps`, `sleep`, `lsof`,
`pgrep` "process" (about 1,580), and treats `node --check`, `--version` queries,
`command -v`, `git tag --contains` and `gzip -c` as scripts, builds or writes. All were
reviewed (as skeletons) and are read-only.

Read-only for analyze.py only (1,729), by psh-classify's reason: a dynamic word that
could be an option of a utility with writing or code-running options (1,047: mostly
`sed -n "$(…)"` scripts and `rg "$x"` with loop variables, where `--pre` runs a
program), `sips` resizing images (152, analyze.py calls all of `sips` read-only),
network (109: `git ls-remote`, `cargo search`), a dynamic word in `find`'s arguments
(95), parse errors (90), repository changes such as `git submodule update` (65),
`sqlite3` with SQL from stdin or that can run programs (57).

What the follow-ups of the first version added (read-only commands): literal variable
tracking about +230, read-only `sqlite3` +86 (of 275 `sqlite3` commands, 86 read, 9
mutate, the rest unknown, mostly SQL on stdin), `$HOME`/`$PWD` paths +115; 153,422
before, 153,853 after.
