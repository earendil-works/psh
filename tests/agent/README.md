# Agent-shaped cases (L3)

Our own cases, written to the shapes agents actually run: the most frequent command
skeletons of the shape corpus (`tests/shapes/shapes.txt`) and the patterns of
`research/corpus.md`, instantiated with small synthetic files (a fake project tree,
logs, configs, local git repositories). Nothing comes from the raw corpus.

| File | Cases | What it covers |
| --- | ---: | --- |
| `cd-chains` | 16 | `cd X && …`, `cd X; …`, failing `cd` |
| `control` | 25 | `for`, `if`, `while read`, `case`, functions, subshells, `&` |
| `globs-status` | 20 | unmatched globs, `|| true`, `; echo "exit=$?"`, `timeout` |
| `heredocs` | 18 | quoted and unquoted here-docs, `<<-`, past a pipe buffer |
| `pipes` | 24 | `| head`/`tail`, SIGPIPE, `PIPESTATUS`, pipefail |
| `redirects` | 19 | `2>&1`, `/dev/null`, `>>`, `<`, `: >` |
| `vars-quotes` | 22 | `$VAR`, `${x:-y}` and friends, quoting |
| `git` | 84 | `git status --short`/`--branch`, `diff` (`--stat`, `--check`, `--cached`, `-- path`, `| sed -n`), `log -n --pretty`, `show`, `add && commit -m`, `rev-parse`, `-C`, the status/diff chains |
| `search` | 74 | `rg` (`-n`, `-S`, `-g`, `--glob`, `-l`, `-c`, `-C`, `| head`), `grep -rn/-nE/--include`, `find … -exec`/`-print0 | xargs -0`, `xargs` |
| `view` | 62 | `cd X && sed -n A,Bp f` and chains of them, `nl -ba f | sed -n`, `head`, `tail`, `cat`, `wc -l` |
| `tools` | 75 | `npm run X`, `npm test`, `make check`, `npx tsc --noEmit`, `node --test`, `node "$(git rev-parse --show-toplevel)/…" --run …`, `cargo test | tail`, `go test ./...`, `uv run`, `gh … --json`, and here-docs into interpreters (`python3 - <<'PY'`, `node - <<'NODE'`, `node --input-type=module`) |
| `scripts` | 54 | multi-line `set -euo pipefail` scripts, functions, traps, arrays, guards |
| `files` | 52 | `mkdir -p`, `cp`, `mv`, `rm`, `chmod +x`, `ln -s`, `tee`, `sed -i`, `diff -u`, `cmp` |
| `text` | 48 | `sort`, `uniq -c`, `cut`, `tr`, `awk`, `printf`, `jq`, `paste`, `comm`, `diff` |
| `procs` | 61 | `timeout`, background jobs and `wait`, SIGPIPE, exit statuses, command not found, missing files, `set -e`/`-u`/pipefail failures |
| `top-shapes` | 47 | the most frequent shapes the files above did not reach, in their exact form |

## Coverage of the corpus

`python3 tests/agent/coverage.py` (tree-sitter, local only; `--missing N` lists the next
shapes) compares every run of consecutive command lines of every case with the shape
corpus. With 701 cases:

| Match | Commands covered | Of the committed shapes | Of the whole corpus |
| --- | ---: | ---: | ---: |
| exact (placeholders unified) | 32,958 | 26.8% | 11.2% |
| args (also: argument lists are one argument) | 61,097 | 49.7% | 20.7% |
| syntax (also: every word is a word) | 88,278 | 71.9% | 29.9% |

The committed shapes stand for 122,848 of the 295,401 commands; the rest is a long tail
of single-session commands. "args" is the useful number: half of the commands the shapes
stand for have a case with the same command structure.

## Writing cases

The files use the Oils spec format (`#### name`, then the code; see
`tests/oils/README.md`). Each case runs as `SHELL -c CODE` with an empty stdin in a
fresh directory (HOME a fresh directory too), like an agent's command, and is judged
against the bash 5.3 goldens in `tests/goldens/agent/`. Lines that start with `#` are
dropped from the code (as in the Oils format), so a here-doc body must not have such
lines.

- Deterministic: no `$$`, dates, `$RANDOM`, `mktemp` names, `ls -l`, `du`, `ps`
  output or absolute paths in the output; files only inside the working directory.
- Tools the recording container has: bash 5.3, GNU coreutils, grep, sed, findutils,
  diffutils, gawk, procps, git, ripgrep, jq (`docker/Dockerfile`). Anything else
  (`node`, `python3`, `npm`, `make`, `cargo`, `gh`, …) is a stub script the case writes
  into `bin/` and puts on `PATH`; the agent's command keeps its exact shape.
- git: every case exports the same identity and fixed dates (and
  `GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null`) and runs `git init -q -b main`,
  so hashes and output are the same on every run. The runner leaves `.git` and bare
  `*.git` directories out of the file comparison: they are git's internals.
- Utility flags GNU and BSD agree on where possible. A case whose output depends on GNU
  behavior has a `# dialect: gnu` line after its `####` line. On macOS such cases (and
  `wc`/`uniq -c` padding, `ls` errors, `od`, `nl`, `seq`) fail until psh's own utilities
  run them; the ratchet says so per case. `rg` and `timeout` are not on the runner's
  `PATH` on macOS for the same reason.

After adding cases, record them with `scripts/record-goldens agent/FILE`, check with
`scripts/record-goldens --check agent` (a second recording must match), and add the
failing ones to `expected-failures.txt` with `psh-spec --layer agent --add-failures`,
each with its reason.
