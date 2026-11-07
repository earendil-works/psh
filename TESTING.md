# psh: Testing

How psh is tested and how it climbs: one oracle recorded once, many fast ratchets. The
plan behind this is `research/hillclimb.md`.

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
- **The container** (`docker/Dockerfile`, D1022): Debian stable pinned by digest, with
  its GNU userland plus `gawk`, `procps` and the `en_US.UTF-8` locale; GNU bash 5.3
  built from the release tarball and the official patches through 5.3.9, each pinned by
  sha256 (building it is fine; nobody reads its source, D007). 5.3.9 is the Homebrew
  bash used for probes. Cases run as the user `spec` (uid 1000), umask 022, hostname
  `psh-oracle`, no network, `TZ=UTC`. The official `bash:5.3` image is Alpine with
  busybox utilities and is not suitable.
- `scripts/record-goldens [FILTER...]` builds the image (once; the GNU download is
  slow), builds psh-spec for Linux in the pinned Rust image, records every case (about a
  minute) and replaces `tests/goldens/`. `scripts/record-goldens --check` records again
  into `work/scratch/record/` and lists the cases that differ: the determinism check.
- **Normalization table** (`crates/psh-spec/src/normalize.rs`, D1024), for the few
  legitimate differences: run-specific paths become `<case>` (the working directory,
  also `$TMP`), `<home>`, `<repo>` (`$REPO_ROOT`), `<bin>` (the helper directory),
  `<run>` and `<sh>` (the shell binary, `$SH`); the host name becomes `<host>`; at
  comparison, the shell's name at the start of a message line (`bash:` vs `psh:`) or
  alone on a line (`$0`) becomes `<sh>`. Still to come: `uname` output and path
  separators on Windows. Every entry has a comment saying why. A normalization is never
  used to hide a real difference.
- For quick probes: `/opt/homebrew/bin/bash` (5.3, macOS), always under
  `scripts/timeout.sh`. It differs from the container in external utilities; the
  goldens win.

## Test layers

| Layer | Source | License / vendoring | What it climbs |
| --- | --- | --- | --- |
| L0 parse | every script of L1–L3, the shape corpus and `tests/l0/syntax.txt` | ours | psh-syntax agrees with `bash -n` on parse/no-parse; print → re-parse round trip (`psh-spec --l0`); spans cover the source |
| L1 Oils spec | `oils/spec/*.test.sh` at d114978 | Apache-2.0, vendored in `tests/oils/` | the language, per area; ~2.7k cases in the compat set |
| L2 brush cases | `brush-shell/tests/cases/**/*.yaml` | MIT, vendored in `tests/brush/` | the same, ~3.7k cases; the files hold no expectations, so the goldens come from our recording |
| L3 agent shapes | our own cases written to the patterns in `research/corpus.md`, in the Oils format, run as `bash -c` | ours, `tests/agent/` | what agents do: `cd X && …`, `\| head`, `2>&1`, quoted here-docs, `timeout`, globs that match nothing; 144 cases, growing toward ~500 |
| L4 utilities | per-utility cases in `tests/utils/NAME/`, recorded from GNU (container) and BSD (macOS); each case expects one or both (D1084) | ours; uutils/GNU tests may be *run* for reference, never vendored | each in-process utility, including "unsupported flag → fall through or clear error" |
| L5 process | Rust integration tests in `tests/process/` (`cargo test -p psh-os --test process`: portable cases with `psh-test-helper` as the external program, plus Unix and Windows ones; the standalone binary's signals: `cargo test -p psh --test standalone`), each under a deadline (scaled by `PSH_TEST_TIME_SCALE`) and with a leak check (D1064) | ours | cancel; `timeout` kills the whole tree; no orphans; here-docs past the pipe buffer; SIGPIPE in `\| head`; background jobs; encodings; Job Objects |
| L6 stress | L1–L3 re-run under perturbation | – | cancel at a random step (no leaked process, shell still usable); pause/resume at every k-th safepoint; a host where every op is `Pending`; tiny pipe buffers; slow consumers |

For L4, where GNU and BSD agree both goldens must match; where they conflict, the
expected output follows the tie-break rules (ARCH.md "psh-utils", D016), and the case
says which rule applies. Upstream suites back the big utilities: jaq's `jq-tests.jq`,
the onetrueawk `T.*` and goawk tests for `awk`, ripgrep's tests for `rg`, and
just-bash's recorded GNU fixtures (Apache) for `grep`/`sed`/`awk`/`jq`. Permissive ones
may be vendored (check each license); uutils' runs of the GNU test suite are run for
reference only, never vendored.

## Cases and goldens

- A case is a script plus optional stdin, arguments and files. It runs in a fresh temp
  directory (`<case>`) with its own `HOME` (`<home>`) and a fixed environment (D1020):
  nothing is inherited; `PATH` is the helper directory then the system's;
  `USER=LOGNAME=spec`, `TZ=UTC`. How each layer runs:
  - **oils**: the code on stdin of `bash` (as Oils' runner does), `LC_ALL=C.UTF-8`,
    `TMP=<case>`, `SH=<sh>`, `REPO_ROOT` a per-run copy of `tests/oils/spec/`'s tree.
  - **brush**: the case's `args` and `stdin`, files and `env` (brush's own default
    flags dropped), `LC_ALL=C`, `PS1='test$ '`.
  - **agent**: `bash -c CODE`, empty stdin, `LC_ALL=C.UTF-8`.
- Oils' Python helpers (`argv.py`, `printenv.py`, `read_from_fd.py`,
  `show_fd_table.py`, `stdout_stderr.py`) are built into psh-spec with Python 2's
  output, linked into the helper directory and the `REPO_ROOT` copy (D1023): recording
  and testing need no Python and see the same helpers everywhere.
- Goldens are one file per source file, `tests/goldens/<layer>/<file>.golden`, in a
  line format that diffs well (D1021, `crates/psh-spec/src/golden.rs`): per case its
  index and name, `status:` (a number, `signal N` or `timeout`), `bash-n:` (the status
  of `bash -n` on the script, for L0), then blocks for stdout, stderr, the files the case
  created or changed in its directory (contents up to 64 KiB, else size and hash;
  directories; symlinks) and `bash -n`'s stderr. Lines are `|text`, or `\text` with
  `\\` and `\xNN` escapes for control characters and invalid UTF-8.
- L4 (`tests/utils/NAME/*.test.sh`, the agent cases' format, run as `bash -c`) has two
  recordings: GNU's in `tests/goldens/utils/` (the container) and BSD's in
  `tests/goldens/utils-bsd/` (`scripts/record-goldens --bsd`, on macOS with Homebrew's
  bash 5.3 and only the system's directories on PATH). A case declares
  `## expect: gnu`, `bsd` or `both` (the default), with the reason or the D016 rule;
  `both` cases fail as "goldens disagree" until they say which (D1084).
  psh runs them with `PSH_UTILS_FORCE=all` (D1192), so a utility in F or W mode runs
  psh's implementation even though the system's program is on PATH.
- `tests/goldens/shapes/bash-n.txt` and `tests/goldens/l0/bash-n.txt` hold `bash -n`'s
  verdict for each skeleton of the shape corpus (by line in `tests/shapes/shapes.txt`)
  and each snippet of the parser's `tests/l0/syntax.txt` (by name) that bash rejects;
  every other input parses. Their header holds the hash of the source they were
  recorded from; `scripts/record-goldens --l0` re-records just these in seconds.
- **L0** (`psh-spec --l0`, D1220): every case script, skeleton and snippet is parsed
  with psh-syntax; an input passes when psh accepts exactly what bash accepts and a
  parsed tree survives print → reparse unchanged (compared by `psh_syntax::dump`). Ids
  are `cases/oils/alias/3`, `shapes/12`, `syntax/NAME`; the ratchet is
  `tests/l0/expected-failures.txt`.
- A case **passes** when stdout, exit status and the resulting files match the golden
  after normalization. stderr is compared too and counted as its own score, so error
  messages climb without blocking the language work. (Whether stderr mismatches become
  failures later is a coordinator decision.)
- Re-recording is rare and deliberate: only when the case set, the container or the
  normalization changes, in its own commit, with the diff of changed goldens reviewed.
  Cases whose stdout, status or files differ between recordings (pids, clock times,
  `$RANDOM`, races) are in the `skip` group; about 60 more differ only in stderr (pids
  in job messages), which only lowers the stderr score.

## Groups: skip / later / active

`tests/spec.toml` puts every case into one group. Rules match case ids by glob
(`oils/alias/*`, `brush/compat/**`) or case tags (`pty`, `interactive`); the first
matching rule decides, unmatched cases are active, and every rule has a reason:

- **skip:** psh will never do this: interactive features, `set -m` job control,
  `select`, history, completion, zsh/mksh/POSIX-mode-only files. Left out of the
  denominator, so the headline number stays honest.
- **later:** on the roadmap but outside the current milestone. Failures are reported
  separately.
- **active:** everything else. The ratchet applies here.

`[[serial]]` tables (same keys) are not a group: they mark cases that use fixed paths
outside their directory (`/tmp/oil-spec-test/pwd`), which race when two run at once.
Each such case holds a machine-wide lock (a file lock in the temporary directory) while
it runs, so they never overlap, in one run or across concurrent runs (D1224).

`[[flaky]]` tables (same keys, a `reason` naming the race and its owner) are not a group
either: they quarantine cases that pass or fail by timing. A flaky case runs as usual and
counts in its group, but its result never fails the run or the gate, it is in no
expected-failures file (`--update-ratchet` and `--add-failures` leave it alone), and
every run and `scripts/status` list the flaky cases with their reasons and whether they
passed. There are no retries: a retry would hide real bugs. A case goes into the
quarantine only for a documented race, and its owner takes it out by fixing it (D1225).

Moving cases between groups is a coordinator decision.

## Ratchets

- Each layer has `tests/<layer>/expected-failures.txt`: one case id per line
  (`oils/alias/3`), optionally `id: reason`, sorted by file and index. The gate (leg
  `spec`) fails when an active case not in the file fails (a regression). It reports
  cases in the file that now pass; `psh-spec --update-ratchet` removes them. Progress is
  "lines deleted from the ratchet". `psh-spec --add-failures` adds failing cases that
  are missing, for newly vendored or written cases only.
- A case's id is `layer/file/index`, its 0-based position in the source file.
- The ratchets describe psh: a run with another shell (`--shell bash`) is compared and
  summarized, but not held to them.
- Ratchets start at 0%: every active case is an expected failure, and the scoreboard is
  red on day one. That is the first milestone.
- Never add a line to hide a regression your change caused (AGENTS.md). Lines are added
  only when new cases are vendored or written, or when a case moves into `active`.
- **Platform ratchets** (D1113, D1331). The shared file lists psh's own gaps: cases
  that fail on Linux, where the goldens come from. `tests/<layer>/expected-failures.<platform>.txt`
  (`macos`, `windows`, `wasm`) lists active cases that pass on Linux but may fail on that
  platform because its programs or OS differ (BSD `od`, `ls` and `sed`, no `tac` or
  `rg`), each with its reason; never a case the shared file has. A run checks the shared
  file plus its platform's (`psh-spec` picks the OS it runs on; Linux has only the
  shared file). These files shrink as psh's own utilities replace the host's.
  - Which file a failure belongs in takes a Linux run: `scripts/spec-linux` builds psh
    for Linux and runs every case in the recording image (Docker, about two minutes),
    held to the shared ratchet; it writes each case's result to
    `work/scratch/spec-linux/results.tsv`. `scripts/spec-linux --update` then runs here
    with `psh-spec --update-ratchet --linux-results FILE`: entries move to the file they
    belong in, shared entries that pass here but fail on Linux stay, and cases that fail
    on Linux but are listed nowhere are reported as regressions. With `--linux-results`,
    `--add-failures` puts a new case that passes on Linux in the platform's file; without
    it, everything goes to the shared one. Run it after adding cases and now and then
    (the gate does not: it only type-checks Linux, leg `linux`).
  - `wasm`: `psh-spec --wasm` runs every case in `psh.wasm` under node through the JS
    host (`js/test/spec-driver.mjs`), where no external programs exist. A case that
    looked for a command and did not find one may fail there without a line (the run
    prints how many); the file lists only the other differences, which are bugs or
    missing host features.
- **Host-specific cases** (`[[platform]]` in `tests/spec.toml`, D1331) depend on the
  host itself, not on its programs: `/dev/full` and `/proc` (Linux only), where `ls` and
  `cat` live, `/tmp` being `/private/tmp`, root's home, signal numbers, the time zone
  database, file names APFS rejects. They are gated on Linux only; elsewhere they run, are
  listed apart ("host-specific") and are in neither ratchet file of that platform. Their
  Linux failures are in the shared file. `scripts/status` shows, per layer, the failing
  cases by the file that lists them and the host-specific ones.

## Status and the hit list

- `scripts/status` runs psh over every non-skip case and prints pass counts per layer
  and per area (Oils file, brush file) as Markdown, and appends one line per layer to
  `tests/scores/status.tsv` (date, commit, active, pass, pass with stderr, later, later
  passing, skip), so progress is visible over time; commit that line with the change that
  moved the numbers. It also shows L0 per source and appends an `l0` line (inputs,
  passing). `--no-append` leaves the file alone. The score is the source of
  truth, not the prose.
- `scripts/status --hitlist [-n N]` ranks the failing active cases, and the areas by
  the summed weight of their failures. A case's weight is the corpus frequency
  (`research/corpus/results.json`: `features` for syntax, `top_names` for command
  names) of the rarest construct in its script, so a failing here-doc case outranks a
  failing `select` case by orders of magnitude (D1028). Constructs the corpus did not
  measure (`select`, `coproc`, `${!x}`, namerefs, `=~`, extglob, `${ …; }`), bash
  builtins outside its top names, and shell arguments other than `-c CODE` count as
  rare. The constructs are read from psh-syntax's tree; scripts the parser rejects (on
  purpose: bash rejects them too) are scanned as text (D1222). The coordinator cuts
  issues from the top.

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

`psh-spec` is our own runner in Rust, with no dependencies:

- Reads the Oils `.test.sh` format (following Oils' `sh_spec.py` exactly, so each case
  runs the same bytes) and the subset of brush's YAML the cases use (hand parser).
- Runs psh **in-process** (a new shell per case on its own engine thread over the OS
  host: psh-os on Unix and Windows; the case's cwd and
  environment, OS pipes for stdout and stderr, a fake `$$`) wherever possible; cases
  that mention `PPID` or `kill` run in the `psh` binary, since they signal their parent
  or process group (D1056); `--shell PATH` runs a shell binary as a subprocess
  in its own process group, killed with all its children on timeout and after it exits.
  The same comparison checks the runner itself: bash 5.3 must pass its own goldens
  except for documented platform differences (on macOS: BSD utilities, libc, a
  case-insensitive file system; the 0002 log lists them).
- `psh-spec [FILTER...]`: filters are case id prefixes (`oils/alias`, `oils/alias/3`)
  or globs; `--layer`, `--diff`, `--update-ratchet`, `--add-failures`, `-j N`,
  `--timeout`, `--summary FILE` (Markdown per layer and file), `--list`, `--keep`.
  `--record`, `--record-shapes` and `--diff-goldens` serve `scripts/record-goldens`.
  All of L1 and L2 (about 5,700 active cases) take seconds.
- The psh binary next to psh-spec (some cases run in it) and, for `--wasm`,
  `target/wasm/psh.wasm` must be newer than their sources: psh-spec refuses to run with
  a stale one and prints the command that rebuilds it (`cargo build -p psh`,
  `scripts/build-wasm`).

## The gate

`scripts/gate.sh` runs inside `scripts/merge-to-main.sh` and must pass before anything
reaches `main` (AGENTS.md). Legs, as they come into existence:

1. `cargo fmt --check`, clippy `-D warnings`, `cargo test --workspace` (0001).
2. `psh-syntax` and `psh-engine` build for `wasm32-unknown-unknown` (0001). `deps`:
   every dependency of every workspace crate (direct or transitive, any target) is
   listed in `scripts/deps-allowlist.txt` with the decision that allows it, and nothing
   else is (0022, D1223).
3. `cargo check` with Rust 1.96, the MSRV (D025, 0001); `linux`: `cargo check` of the
   workspace, tests included, for `aarch64-` and `x86_64-unknown-linux-musl` (0033,
   D1330), since no other leg compiles Linux-only code; the scripts parse and the
   tests of `timeout.sh` and `merge-to-main.sh` pass (0001, D1013).
   `shapes`: the shape corpus passes its privacy check (0003, D1030).
   `guide`: `psh-docs check` holds the guide to its writing rules (0012, D1120).
4. `spec`: the ratchets of every layer that exists (0002 onward); `l0`: the L0 ratchet
   (0022).
   `wasm-host` (0011): `psh-wasm` passes clippy for wasm32, both wasm modules build, the
   JS host's smoke tests pass under node, and `psh-spec --wasm` holds the shared and the
   wasm ratchets.
5. `windows`, always last: `fmt`, clippy, `cargo test` (arm64 and x64) and the MSRV
   check on the owner's Windows VM, through `scripts/windows-gate.sh` (0007; D027,
   D1070, D1071). See "Windows" below.

Each leg is a file `scripts/gate.d/NN-name.sh`, run in order under
`scripts/timeout.sh` with the timeout on its first line (D1026); adding a leg is adding
a file. `scripts/gate.sh --list` names the legs; `scripts/gate.sh LEG...` runs a few
while iterating.

The gate caches passes (D1221): a leg file lists the paths it reads on `# inputs:`
lines (git pathspecs), and a leg whose inputs, leg file, `gate.sh`, `timeout.sh` and
toolchains are exactly as on an earlier pass, anywhere in the repository's worktrees, is
skipped with a note. Only committed, clean inputs are cached; a leg without `# inputs:`
always runs; `scripts/gate.sh --full` ignores the cache. A docs-only change gates in
seconds, and the Windows leg is skipped when no Rust input or Windows script changed.
When a leg starts reading a new path, add it to its `# inputs:`: a missing input is a
wrong skip. Never weaken a leg to get green.

## Inner-loop rules

- Run only what your change can break: one crate's unit tests, one spec file, one
  utility. The full gate runs once, at merge time.
- Everything that runs a shell goes under `scripts/timeout.sh` (macOS has no
  `timeout`).
- Every process test checks afterwards that no process of the case survived (leak
  check by process group or job). psh-spec does the same for every case (D1305): an
  in-process case's host kills what the run left, `nohup` jobs included, and a group
  still alive after the grace period fails the case as `leak`; a binary-run case's
  process group must empty once the runner killed it. Processes that move into a
  session of their own (`setsid` inside a script) are out of its reach.
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
- Runs take turns: one at a time, the next waiting ("waiting for the Windows VM") for
  at most an hour, which does not count against the legs' timeouts. The VM fetches a
  snapshot commit into its one checkout, `C:\psh-gate\shared`, whose warm `target\`
  every run reuses, and builds with the host's exact `rustc` version (D1070, D1072).
- Each leg has a timeout inside the VM. After each leg the runner kills and waits for
  every process the leg started; a test process still running after `cargo test` ended
  fails the leg (a test leaked it). Nothing a run starts outlives it.
- On arm64 Windows, the XtaCache service holds x64 executables that ran under emulation
  for a while; the runner waits until they are released before a test leg, and runs a
  leg that failed to replace a held file (LNK1104) once more (D1072).
- The gate runs it as its last leg, always. It wakes the VM if it sleeps and lets it
  sleep again once no run used it for ten minutes (`PSH_WINDOWS_IDLE`); a VM that was
  already running stays running. Each run's output is kept in
  `work/scratch/windows-gate/logs/` (D1916). An unreachable VM fails the leg in bounded time. `PSH_GATE_WINDOWS=0` skips
  the leg when the VM is broken, for the coordinator and the owner only; the skip is
  logged (D1071).
- `spec`: psh-spec's L4 (utilities) in the VM against the shared goldens, with the
  `windows` platform ratchet (`tests/<layer>/expected-failures.windows.txt`): a case
  may fail only if it needs a program Windows lacks or the file lists it (D1914).
- Not on Windows: the bash/perl script legs and the `deps` and `wasm` legs, which are
  the same on every host.
