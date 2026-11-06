# 0002: psh-spec runner, vendored cases, recorded goldens, ratchets, status

status: done
agent: infra
branch: issue/0002-spec-runner-goldens
depends-on: 0001
touches: crates/psh-spec/, scripts/record-goldens, scripts/status, scripts/fetch-*, docker/, tests/oils/, tests/brush/, tests/agent/, tests/goldens/, tests/spec.toml, tests/**/expected-failures.txt, tests/scores/, scripts/gate.sh (one leg), TESTING.md (the planned sections)
guide: testing.md (not written yet; list it as a follow-up)
decisions: D1020–D1029

## Goal
The first real milestone: **a red scoreboard**. Every case of L1 (Oils) and L2 (brush),
plus a first set of L3 agent cases, has a golden recorded from bash 5.3, the runner
compares psh against them, ratchets sit at 0%, and the status tool shows the numbers.
See TESTING.md ("Oracle", "Cases and goldens", "Groups", "Ratchets", "Status and the hit
list", "The runner") and `research/hillclimb.md`.

## Acceptance criteria
- [x] Vendored, with their license files and a README naming the upstream commit:
      Oils `spec/*.test.sh` (+ the helper files they need) at `d114978` in
      `tests/oils/`; brush's `brush-shell/tests/cases/**/*.yaml` at `943553433ae0` in
      `tests/brush/`. A `scripts/fetch-*` script reproduces each from upstream into
      `vendor/` and checks the copy.
- [x] `docker/` holds the recording image (TESTING.md "The container"): GNU userland,
      bash 5.3 built from the release tarball pinned by sha256, fixed locale, TZ, HOME,
      USER, hostname, umask. Pinned by digest once built.
- [x] `scripts/record-goldens` records stdout, stderr, status and resulting files for
      every case, plus the `bash -n` verdict of each case script (for L0), into
      `tests/goldens/`. Deterministic: a second recording produces no diff (record the
      cases that are inherently nondeterministic, e.g. `$RANDOM`, `$$`, timing, and put
      them in a group).
- [x] `tests/spec.toml`: skip / later / active rules, each with a reason (TESTING.md
      "Groups").
- [x] `crates/psh-spec` (no dependencies): Oils `.test.sh` reader, brush YAML subset
      reader, case model, normalization table, parallel runner (std threads, `-j`),
      per-case timeout, filters, `--diff`, ratchet (`--expected FILE`,
      `--update-ratchet`; stale entries reported), Markdown summary per layer and file.
      An `Engine` trait with two engines: an external shell binary (subprocess) and
      in-process psh (a stub until 0005).
- [x] Runner self-check: `/opt/homebrew/bin/bash` through the external engine against
      the goldens. Every mismatch is explained in the Log (platform utility
      differences, normalization gaps) and the runner fixed where it is wrong.
- [x] A first `tests/agent/` set (L3) of at least 100 cases written to the patterns in
      `research/corpus.md` §4 and §9, recorded like the rest.
- [x] `expected-failures.txt` per layer with every active case (0%), and a gate leg that
      runs the ratchets.
- [x] `scripts/status` (summary per layer and area, appended to
      `tests/scores/status.tsv`) and `--hitlist` (failures weighted by
      `research/corpus/results.json` `features`).
- [x] TESTING.md's *(planned)* sections updated to what was built (only those sections).

## Notes
- `research/spec-harness/` holds the earlier harness (Dockerfile, `SUITE-compat.txt`,
  results for 10 shells); reuse what helps. Oils' own expectations in the files (`##
  stdout:`, `## BUG bash`) are not our oracle; the recording is (D003).
- A case's files: record the directory listing and file contents after the run, bounded
  in size; skip what the case did not create.
- Running bash inside Docker is fine; reading its source is not (D007).
- Use a timeout for every recorded case (Oils uses 10 s).
- If the issue is too big for one pass, land it in this order and tell the coordinator:
  vendoring + runner + external engine, then recording, then ratchets + gate, then
  status/hit list, then L3.

## Log
- 2026-11-06, part 1 (runner, vendoring, recording, groups, ratchets, gate leg):
  - Vendored Oils `d114978` (SUITE-compat: 134 files, 2,745 cases) and brush `9435534`
    (133 files, 3,687 cases) unmodified, with `scripts/fetch-oils`/`fetch-brush
    [--check]` (D1025). brush cases source `tests/utils/`, so that is vendored too.
  - psh-spec: Oils reader (a faithful port of `sh_spec.py`'s rules: comment lines are
    dropped even in code, trailing blank lines belong to the code), YAML subset reader
    (block maps/seqs, `|` scalars, quoted/plain scalars, multi-line flow sequences;
    cross-checked against a count of case headers), case model, external engine (own
    process group, group killed on timeout and after exit), in-process stub, sandbox
    and file snapshot, golden format, normalization, groups, ratchets, summary and
    `--diff` (D1020, D1021, D1024, D1027).
  - Oils' Python helpers are built into psh-spec (D1023); cases also call them by path
    (`spec/bin/argv.py` in `introspect`), so the `REPO_ROOT` copy links them too.
  - Recording container (D1022): Debian 13 by digest, bash 5.3 + patches 001-009 by
    sha256 (5.3.9 = the Homebrew probe), gawk, procps, en_US.UTF-8. Downloading from
    ftp.gnu.org took 7 minutes once; Docker caches it. A whole recording takes about a
    minute.
  - Found and fixed while recording: Oils' `legacy_tmp_dir` (glob tests need `_tmp`);
    "Text file busy" when a case execs a file it was given while another thread forks
    (a lock keeps forks and written files apart); macOS `/var` vs `/private/var`.
  - Determinism: five recordings (`record-goldens --check`); 20 cases differ in
    stdout/status/files (pids, clock, `$RANDOM`, `read -t 0` and coproc races): all in
    `skip`. About 56 more differ only in stderr (pids in job messages): left active,
    they only cost stderr points.
  - Groups (tests/spec.toml, my proposal; the coordinator owns it): skip interactive,
    prompt, history, completion/bind, select/suspend, zsh files, brush's own CLI/help,
    and the nondeterministic cases; later: coproc. Active: oils 2,511, brush 3,171.
  - Ratchets: every active case the stub fails (oils 2,440, brush 3,112). 71 Oils and 59
    brush cases already "pass" with the stub: bash also prints nothing and exits 2
    (syntax errors). They stay off the ratchet.
  - `bash -n` verdicts: per case in the goldens (6,267 accept, 165 reject) and for the
    26,234 shape skeletons in `tests/goldens/shapes/bash-n.txt` (158 rejected).
  - Gate: legs moved into `scripts/gate.d/NN-name.sh` files (D1026) so parallel issues
    add legs without conflicts; 0003's `shapes`, 0012's `guide` and 0007's `windows`
    moved over unchanged (outside my `touches:`, needed for the rebase). New leg
    `spec` (about 3 s) and `deps` now also checks psh-spec has no dependencies.
  - Self-check, Homebrew bash 5.3.9 on macOS against the Linux goldens: oils 2,377/2,511,
    brush 3,066/3,171 pass; 239 failures (plus 57 stderr-only). Every one I looked at is
    a platform difference, none a runner bug:
    - BSD utilities instead of GNU (about 110): `wc`/`uniq -c` padding, `od` layout,
      `touch -d`, `date --`, `head -c 0`, `sed` on invalid UTF-8, `ls` exit 1 vs 2,
      every "cannot access/touch/remove" message format.
    - macOS libc/kernel (about 45): signal numbers and names (`kill -l`, USR1=30, stopped
      jobs 145 vs 147, "Killed: 9"), the regex engine behind `[[ =~ ]]`, `printf %06s`,
      strftime and TZ, errno texts ("File exists" vs "Is a directory", "Result too
      large"), no `/dev/full`, no `/proc`, `ulimit` values, `uname`, `$SHELL`, `exec` of
      a missing interpreter (126 vs 127).
    - Homebrew's bash expands `~` from the environment's HOME, not the shell's `HOME`
      variable (19 cases: tilde, array, brace-expansion, dbracket, word-split): a quirk
      of that build; Linux bash 5.3.9 uses the variable.
    - System layout: `/bin` vs `/usr/bin` (`type`, `command -v`, `PATH=/bin`), `/bin/sh`
      is bash 3.2, `~root` is `/var/root`, programs missing (`which` in `/bin`,
      `hexdump` vs present), the APFS case-insensitive file system (`globignore/7`),
      en_US.UTF-8 collation (`glob/31`).
    - Load: `oils/case_/10` and `brush/brush/command_substitution/15` time out at 10 s
      under `-j 12` on a busy machine and pass alone in 2 s; `oils/background/7`
      (completion order of parallel jobs) differs once.
  - Docs: TESTING.md "Oracle", "Cases and goldens", "Groups", "Ratchets", "The runner",
    "The gate"; decisions D1020-D1027.

- 2026-11-06, part 2 (status, hit list): `scripts/status [--hitlist] [-n N]
  [--no-append]` on psh-spec `--status`; first line of `tests/scores/status.tsv` (oils
  71/2,511, brush 59/3,171 with the stub). Hit list weight = corpus frequency of the
  rarest construct (D1028), detected lexically; a minimal JSON reader in psh-spec reads
  results.json (no dependency). The top areas today: assignments, word splitting,
  printf, globbing, parse errors, quoting, redirections.
- 2026-11-06, part 3 (L3): 144 agent cases in 7 files (`tests/agent/*.test.sh`: cd
  chains, pipes/output limiting, redirects, here-docs, variables/quoting, globs/status,
  control flow), recorded, deterministic over two checks, all on the ratchet (the stub
  passes none). `sed -i ''` and other dialect questions are left to L4: the GNU golden
  would contradict psh's deliberate dialect (D016). Homebrew self-check: 134/144; the 10
  differences are BSD `wc`/`uniq` padding, `ls` exit status 1 vs 2, and `timeout`
  missing on macOS (the corpus's most common "command not found").
- Docs: TESTING.md (all formerly planned sections of 0002), `tests/agent/README.md`.

## Follow-ups
- Platform-dependent cases: the goldens are Linux/GNU; on macOS and Windows, cases
  that run external programs (`wc`, `od`, `touch -d`, `/proc`, signal numbers) cannot
  pass while those programs are the host's. psh's own utilities fix most of it; the
  rest needs a per-platform group or tag in spec.toml (a coordinator decision).
- `uname` and Windows path separators in the normalization table (0007 and later).
- The external engine on Windows kills only the shell, not its tree (no process groups
  without a dependency); psh-spec does not run cases on Windows yet.
- guide: `testing.md` (not written yet).
- Hit list: detect constructs with psh-syntax once it parses (0004), instead of the
  lexical approximation.

## Result
Landed in three parts (main 21dc1ab, 7d5d158, and the L3 merge):

- **Cases**: L1 Oils (134 files, 2,745 cases; 2,511 active), L2 brush (133 files, 3,687
  cases; 3,171 active, 51 later), L3 agent (7 files, 144 active). Vendored copies are
  reproducible with `scripts/fetch-oils` / `scripts/fetch-brush [--check]`.
- **Goldens**: `scripts/record-goldens [--check] [FILTER...]` records in the pinned
  container (bash 5.3.9, Debian 13) into `tests/goldens/<layer>/<file>.golden`, with
  `bash -n` per case and `tests/goldens/shapes/bash-n.txt` for the shape corpus.
- **Runner**: `cargo run -p psh-spec -- [FILTER...] [--diff] [--layer L] [-j N]`;
  `--shell PATH` for another shell (the bash self-check); `--update-ratchet`,
  `--add-failures`, `--summary FILE`, `--list`, `--keep`.
- **Groups and ratchets**: `tests/spec.toml`; `tests/<layer>/expected-failures.txt`; the
  gate leg `spec` (seconds). Gate legs are now files in `scripts/gate.d/`.
- **Status**: `scripts/status [--hitlist] [-n N] [--no-append]`, history in
  `tests/scores/status.tsv`.

Known gaps: the in-process engine is a stub until 0005 (the gate measures the stub);
cases that call external programs depend on the platform's programs (see Follow-ups);
the hit list detects constructs lexically; Windows runs psh-spec's unit tests but not
the cases.
