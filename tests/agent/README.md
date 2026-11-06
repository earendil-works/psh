# Agent-shaped cases (L3)

Our own cases, written to the patterns agents actually use (`research/corpus.md` §4,
§5, §9): `cd X && …` chains, pipelines into `head`/`tail`, `2>&1` and `/dev/null`,
quoted here-docs (also past a pipe buffer), `$VAR` and `${x:-y}`-style operators,
globs that match nothing, `|| true` and `; echo "exit=$?"`, `set -euo pipefail`,
`timeout`, `for`/`if`/`while read`, subshells and `&`.

The files use the Oils spec format (`#### name`, then the code; see
`tests/oils/README.md`). Each case runs as `SHELL -c CODE` with an empty stdin in a
fresh directory, like an agent's command, and is judged against the bash 5.3 goldens in
`tests/goldens/agent/`. Lines that start with `#` are dropped from the code (as in the
Oils format), so a here-doc body must not have such lines.

Keep cases deterministic (no `$$`, dates, `$RANDOM` or `/tmp` names), write files only
inside the working directory, and stay with utility flags GNU and BSD agree on: the
dialect questions belong to the utility layer (L4). After adding cases, record them with
`scripts/record-goldens agent`, check with `scripts/record-goldens --check agent`, and
add them to `expected-failures.txt` with `psh-spec --layer agent --add-failures`.
