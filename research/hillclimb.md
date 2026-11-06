# How to hill-climb psh

The plan for getting psh from zero to "bash for agents" quickly, with many agents working in
parallel, modeled on how djs climbs test262. The constraints it builds on are in
[README.md](README.md) §8.

## Principle: one oracle, recorded once, many fast ratchets

- **The oracle is GNU bash 5.3.** psh's goal is "what bash does", not "what POSIX or Oils
  says". Where Oils annotates bash as `BUG`, we still match bash. We deviate only by recorded
  decision, for example where Windows needs it.
- **Record the oracle once, never run it in the gate.** `scripts/record-goldens` runs bash
  5.3 in a pinned Linux container (Docker is available locally) over every case and writes
  the stdout, stderr, status and touched files into committed golden files. The gate and
  the inner loop compare psh against those files only. This makes the gate fast and
  deterministic, and lets Windows and macOS CI compare against the same goldens. A
  per-platform normalization table handles the few legitimate differences: paths,
  `uname`, error-message prefixes.
- **Ratchets, not targets.** Each layer has an `expected-failures.txt`. The gate fails when
  a case that used to pass now fails. It reports cases that newly pass, and
  `--update-ratchet` removes them from the expected failures. An agent's progress is "lines
  deleted from the ratchet".
- **The source of truth is the score, not the prose.** A `scripts/status` tool prints pass
  counts per layer and per area and appends them to `tests/scores/*.tsv`, so progress is
  visible over time.

## Test layers

| Layer | Source | License / vendoring | Size | What it climbs |
|---|---|---|---|---|
| L0 parse | all cases of L1–L3 + a *shape corpus* | ours | ~10k scripts | parser: `psh --parse` vs `bash -n` agreement; print → re-parse round-trip; byte spans |
| L1 Oils spec | `oils/spec/*.test.sh`, pinned to d114978 | Apache-2.0, vendored | 223 files, ~2.7k cases in the compat set | the shell language: bash behavior per file |
| L2 brush cases | `brush-shell/tests/cases/**/*.yaml` | MIT, vendored | ~3.7k cases, oracle-only (no expectations in the files) | same, with goldens from our bash recording |
| L3 agent shapes | our own cases, written to the patterns in corpus.md | ours | grows; start ~500 | what agents actually do: `cd X && …`, `| head`, `2>&1`, here-docs, `sed -i ''`, `timeout`, globs that match nothing |
| L4 builtins | per-utility cases, differential against GNU coreutils (container) and BSD (macOS) where flags differ | ours; uutils/GNU tests may be *run* for reference, never vendored | per utility | each in-process utility; "unsupported flag → fall through or clear error" is tested too |
| L5 process | Rust integration tests | ours | ~100 | cancel, timeout kills the whole tree with no orphans, big here-docs past the pipe buffer, SIGPIPE in `| head`, background jobs, encodings, Windows Job Objects |
| L6 stress | re-run L1–L3 under perturbation | – | – | cancel at a random step and check that no process leaks and the shell is still usable; tiny pipe buffers; slow consumers |

**The shape corpus** solves "we can't run the corpus safely": `scripts/shape-corpus` turns
each private corpus command into its syntax skeleton. Every word becomes a placeholder
(`w1`, `w2`), while operators, redirections, quoting kinds, here-doc markers and command
names from the builtin list are kept. The output contains no private data, can be
committed, and is enough to test the parser at scale and to rank which syntax matters.
Running anything is never involved.

## The inner loop must be fast

- `psh-spec` is our own runner in Rust (no Python, no dependencies). It reads the Oils
  `.test.sh` format and the brush YAML (a small hand parser, since we only need their
  subset), runs each case in a fresh temp directory, compares against the golden, and runs
  cases in parallel. Its CLI works like djs's test262 runner: filter by file or case name,
  `--diff`, `--update-ratchet`, `-j N`.
- Cases run psh **in-process** (the engine as a library: new shell, real-FS host, temp cwd)
  wherever possible, and as a subprocess only for L5 and invocation tests. Expect the whole
  of L1+L2 in seconds, not minutes.
- The gate caches each leg by its inputs, as djs does: a docs-only change gates in seconds.

## Prioritization: weight failures by what agents use

`scripts/status --hitlist` joins the failing cases with the feature frequencies from
`research/corpus/results.json`. A failing here-doc case outranks a failing `select` case by
three orders of magnitude. The coordinator cuts issues from the top of the hit list.
Spec files we never intend to support (interactive, `set -m` job control, `select`, zsh/mksh
only) go into a `skip` group with a reason and leave the denominator, so the headline
number stays honest. As in djs, a `later` group holds things on the roadmap but outside
the current milestone.

## Parallelism: areas with disjoint `touches:`

The work splits naturally into areas that rarely touch the same files:

1. **Infra:** the runner, golden recording, gate, status/hit list, shape corpus.
2. **Parser:** copied from brush-parser with peg and winnow removed (hand-written
   recursive descent), byte spans, printer. Climbs L0.
3. **Expansion:** words, parameters, arithmetic, globbing, brace expansion, quoting.
4. **Execution:** control flow, functions, traps, `set -e`/pipefail, subshells, the
   builtins of the shell itself.
5. **Process layer:** spawn, pipes, redirections, waiting, cancellation; Unix first,
   Windows second. Climbs L5.
6. **Utilities:** each in-process utility is independent, which makes this the most
   parallel area (one agent per batch of utilities). `rg` and `fd` sit on the ripgrep
   crates (see builtins.md).
7. **Windows:** path rules, PATHEXT, argv quoting, encodings; same goldens plus
   normalization; Windows CI (GitHub runners, or the owner's Parallels VM through
   `prlctl exec` for local runs).

## Bootstrap order

1. Repo skeleton, AGENTS/ARCH/TESTING/decisions/work board in the djs style, and the gate
   (empty but enforced).
2. Infra: `psh-spec` runner, vendored Oils + brush cases, recorded bash 5.3 goldens, ratchets
   at 0%, and the status tool. This is the first real milestone: **a red scoreboard**.
3. Parser (L0 to ~100%), then a minimal interpreter (simple commands, pipes, `&&`/`||`,
   redirects, variables) so L1/L2 start moving.
4. Then climb in parallel: expansion, execution, process layer, utilities. The coordinator
   feeds issues from the hit list; agents delete ratchet lines.
5. Windows as soon as the process layer exists, not at the end.
6. Integration into pi-env (`exec` through psh behind a flag) once L3 is high. Agents' real
   commands are the ones that matter.
