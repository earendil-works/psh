# psh: Testing

How psh is tested and how it climbs: one oracle recorded once, many fast ratchets. The
plan behind this is `research/hillclimb.md`. Sections marked *(planned)* describe tools
that issues 0001–0003 build; update them as they land.

## Oracle

- **GNU bash 5.3** is the reference (D003). The goal is what bash does, not what POSIX
  or Oils says: where Oils annotates bash as `BUG`, psh still matches bash. Deviations
  (for example where Windows needs them) are recorded decisions and declared in the
  tests.
- **Recorded once, never run in the gate.** `scripts/record-goldens` runs bash 5.3 in a
  pinned Linux container over every case and writes stdout, stderr, exit status and the
  files the case left in its directory into committed golden files. The gate and the
  inner loop compare psh against those files only. That makes the gate fast and
  deterministic, and lets macOS, Linux and Windows compare against the same goldens.
- **The container** *(planned, 0002)*: Debian stable with GNU coreutils, bash 5.3 built
  from the release tarball pinned by checksum (building it is fine; nobody reads its
  source, D007), `LC_ALL=C.UTF-8`, `TZ=UTC`, fixed `HOME`, `USER`, hostname and umask.
  The image is pinned by digest. The official `bash:5.3` image is Alpine with busybox
  utilities and is not suitable.
- **Normalization table**, per platform, for the few legitimate differences: the
  temporary case directory, the shell's name in messages (`bash:` vs `psh:`, and `$0`),
  `uname` output, path separators on Windows. Every entry has a comment saying why. A
  normalization is never used to hide a real difference.
- For quick probes: `/opt/homebrew/bin/bash` (5.3, macOS), always under
  `scripts/timeout.sh`. It differs from the container in external utilities; the
  goldens win.

## Test layers

| Layer | Source | License / vendoring | What it climbs |
| --- | --- | --- | --- |
| L0 parse | every script of L1–L3 plus the shape corpus | ours | `psh --parse` agrees with `bash -n` on parse/no-parse; print → re-parse round trip; spans cover the source |
| L1 Oils spec | `oils/spec/*.test.sh` at d114978 | Apache-2.0, vendored in `tests/oils/` | the language, per area; ~2.7k cases in the compat set |
| L2 brush cases | `brush-shell/tests/cases/**/*.yaml` | MIT, vendored in `tests/brush/` | the same, ~3.7k cases; the files hold no expectations, so the goldens come from our recording |
| L3 agent shapes | our own cases written to the patterns in `research/corpus.md` | ours, `tests/agent/` | what agents do: `cd X && …`, `\| head`, `2>&1`, quoted here-docs, `sed -i ''`, `timeout`, globs that match nothing; starts at ~500 |
| L4 utilities | per-utility cases, differential against GNU (container) and BSD (macOS) where flags differ | ours; uutils/GNU tests may be *run* for reference, never vendored | each in-process utility, including "unsupported flag → fall through or clear error" |
| L5 process | Rust integration tests | ours | cancel; `timeout` kills the whole tree; no orphans; here-docs past the pipe buffer; SIGPIPE in `\| head`; background jobs; encodings; Job Objects |
| L6 stress | L1–L3 re-run under perturbation | – | cancel at a random step (no leaked process, shell still usable); pause/resume at every k-th safepoint; a host where every op is `Pending`; tiny pipe buffers; slow consumers |

For L4, where GNU and BSD agree both goldens must match; where they conflict, the
expected output follows the tie-break rules (ARCH.md "psh-utils", D016), and the case
says which rule applies. Upstream suites back the big utilities: jaq's `jq-tests.jq`,
the onetrueawk `T.*` and goawk tests for `awk`, ripgrep's tests for `rg`, and
just-bash's recorded GNU fixtures (Apache) for `grep`/`sed`/`awk`/`jq`. Permissive ones
may be vendored (check each license); uutils' runs of the GNU test suite are run for
reference only, never vendored.

## Cases and goldens *(planned, 0002)*

- A case is a script plus optional stdin, arguments and files. It runs in a fresh temp
  directory with a fixed environment.
- Goldens live next to their layer (`tests/goldens/<layer>/…`), one file per case or one
  per source file, in a plain text format that diffs well.
- A case **passes** when stdout, exit status and the resulting files match the golden
  after normalization. stderr is compared too and counted as its own score, so error
  messages climb without blocking the language work. (Whether stderr mismatches become
  failures later is a coordinator decision.)
- Re-recording is rare and deliberate: only when the case set, the container or the
  normalization changes, in its own commit, with the diff of changed goldens reviewed.

## Groups: skip / later / active

`tests/spec.toml` *(planned)* puts every case into one group, each rule with a comment
saying why:

- **skip:** psh will never do this: interactive features, `set -m` job control,
  `select`, history, completion, zsh/mksh/POSIX-mode-only files. Left out of the
  denominator, so the headline number stays honest.
- **later:** on the roadmap but outside the current milestone. Failures are reported
  separately.
- **active:** everything else. The ratchet applies here.

Moving cases between groups is a coordinator decision.

## Ratchets

- Each layer has an `expected-failures.txt` (`case-id: reason`, sorted). The gate fails
  when a case not in the file fails (a regression). It reports cases in the file that now
  pass; `--update-ratchet` removes them. Progress is "lines deleted from the ratchet".
- Ratchets start at 0%: every active case is an expected failure, and the scoreboard is
  red on day one. That is the first milestone.
- Never add a line to hide a regression your change caused (AGENTS.md). Lines are added
  only when new cases are vendored or written, or when a case moves into `active`.

## Status and the hit list *(planned, 0002)*

- `scripts/status` prints pass counts per layer and per area (Oils file, brush file,
  utility) and appends them to `tests/scores/status.tsv`, so progress is visible over
  time. The score is the source of truth, not the prose.
- `scripts/status --hitlist` joins the failing cases with the feature frequencies in
  `research/corpus/results.json`, so a failing here-doc case outranks a failing `select`
  case by orders of magnitude. The coordinator cuts issues from the top.

## The shape corpus

The private corpus (295k real agent commands, `~/.cache/psh-corpus/`) is never executed
and never committed. `scripts/shape-corpus` (local Python on tree-sitter-bash, so the
parser is not its own oracle) turns each command into its syntax skeleton: words outside
a reviewed vocabulary (`tests/shapes/vocabulary.txt`: keywords, builtins, utility and
tool names, subcommands, frequent options, well-known variables) become placeholders
(`w1`, `w2`, ..., variables `v1`, ...), quoted text and here-doc bodies too, while
operators, redirections, quoting kinds, here-doc markers and expansions are kept.
`tests/shapes/shapes.txt` holds 26k skeletons standing for 123k commands, all those seen
in two or more sessions plus enough single-session ones to cover every syntax 4-gram;
`tests/shapes/README.md` has the format and the counts. The gate leg `shapes` checks
that every token is a vocabulary word or a placeholder. The skeletons feed L0 (parser
at scale) and syntax ranking. Running anything is never involved.

## The runner *(planned, 0002)*

`psh-spec` is our own runner in Rust, with no dependencies:

- Reads the Oils `.test.sh` format and the subset of brush's YAML we need (hand parser).
- Runs psh **in-process** (new shell, real-fs host, temp cwd) wherever possible; as a
  subprocess only for L5 and invocation tests. It can also run an external shell binary
  through the same comparison, which is how the runner itself is checked: bash 5.3 must
  pass its own goldens except for documented platform differences.
- Filters by file or case name; `--diff`, `--update-ratchet`, `-j N`, `--timeout`.
  Cases run in parallel; L1+L2 should take seconds, not minutes.

## The gate

`scripts/gate.sh` runs inside `scripts/merge-to-main.sh` and must pass before anything
reaches `main` (AGENTS.md). Legs, as they come into existence:

1. `cargo fmt --check`, clippy `-D warnings`, `cargo test --workspace` (0001).
2. `psh-syntax` and `psh-engine` build for `wasm32-unknown-unknown`; their dependency
   sets are checked (`deps`: none, and `psh-syntax` only) (0001).
3. `cargo check` with Rust 1.96, the MSRV (D025, 0001); the scripts parse and the
   tests of `timeout.sh` and `merge-to-main.sh` pass (0001, D1013).
   `shapes`: the shape corpus passes its privacy check (0003, D1030).
   `guide`: `psh-docs check` holds the guide to its writing rules (0012, D1120).
4. The spec ratchets for every layer that exists (0002 onward).
5. `windows`, always last: `fmt`, clippy, `cargo test` (arm64 and x64) and the MSRV
   check on the owner's Windows VM, through `scripts/windows-gate.sh` (0007; D027,
   D1070, D1071). See "Windows" below.

`scripts/gate.sh --list` names the legs; `scripts/gate.sh LEG...` runs a few while
iterating. Each leg runs under `scripts/timeout.sh`.

Later the gate caches each leg by its inputs, so a docs-only change gates in seconds.
Never weaken a leg to get green.

## Inner-loop rules

- Run only what your change can break: one crate's unit tests, one spec file, one
  utility. The full gate runs once, at merge time.
- Everything that runs a shell goes under `scripts/timeout.sh` (macOS has no
  `timeout`).
- Every process test checks afterwards that no process of the case survived (leak
  check by process group or job).
- Read goldens instead of running bash; if you think a golden is wrong, say so in your
  issue log and tell the coordinator.
- Cap parallelism (`-j 6`) to leave cores for other agents.

## Windows

Windows compares against the same goldens through the normalization table. The gate's
Rust legs run on Windows on the owner's Parallels VM (Windows 11 on arm64) via `prlctl
exec` until the repository has a remote, then on GitHub Actions (D027):

- `scripts/windows-gate.sh [LEG...]` runs them from the Mac: `fmt`, `clippy`, `test`
  (aarch64-pc-windows-msvc), `test-x64` (x86_64-pc-windows-msvc under emulation),
  `msrv`. It tests the worktree as it is, uncommitted changes included, so it is also
  the inner loop for Windows work: `scripts/windows-gate.sh test`.
- The VM fetches a snapshot commit into `C:\psh-gate\<worktree>` and builds there with
  the host's exact `rustc` version. Each leg has a timeout inside the VM, and nothing the
  run starts outlives it (D1070).
- The gate runs it as its last leg, always. It wakes the VM if it sleeps and lets it
  sleep again when the last concurrent run ends; a VM that was already running stays
  running. An unreachable VM fails the leg in bounded time. `PSH_GATE_WINDOWS=0` skips
  the leg when the VM is broken, for the coordinator and the owner only; the skip is
  logged (D1071).
- Not on Windows: the bash/perl script legs and the `deps` and `wasm` legs, which are
  the same on every host.
