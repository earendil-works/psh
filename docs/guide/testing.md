# How psh is tested

psh promises to do what bash does with the commands agents write. This page explains how
that promise is checked: what psh is compared against, which sets of cases it runs, how a
failing case is allowed to stay failing without letting a new failure through, and what
has to pass before a change lands.

> **In short.** psh's output is compared with GNU bash 5.3's, recorded once in a pinned
> Linux container and committed as golden files, so no test run ever starts bash. The
> cases come in layers, from parsing alone through two upstream shell test suites, cases
> shaped like real agent commands and per-utility cases to process tests that check
> nothing leaks. Each layer keeps a list of the cases psh is known to fail, which may only
> shrink, and a gate holds every change to these lists, the lints and the builds on
> every platform.

## The oracle: bash, recorded once

The reference is GNU bash 5.3, not POSIX and not another shell's opinion of bash: where
a test suite marks a bash behavior as a bug, psh still matches bash. That is the
[bash's behavior](goals.md#bashs-behavior) goal made testable.

Bash runs only when the expected results are recorded. A pinned Linux container (Debian
stable, fixed by digest) holds bash 5.3 built from the release tarball with its official
patches, the GNU userland, git, jq and the upstream release binaries of ripgrep and fd,
the versions psh's own `rg` and `fd` follow, each pinned by checksum. Every case runs
there once, as an unprivileged user with no network, and its stdout, stderr, exit status
and the files it left in its directory go into a committed golden file, one per source
file. A golden reads like this:

```text
=== 3: failing cd stops the && chain
status: 0
bash-n: 0
--- stdout
|status=1
--- stderr
|bash: line 1: cd: does-not-exist: No such file or directory

=== 4: failing cd does not stop a ; chain
```

`bash-n` is bash's verdict on parsing the script alone, which the parser tests use. Lines
start with `|`, so a diff of two recordings stays readable; control characters and
invalid UTF-8 get escapes. Recording again is rare and deliberate, and a second recording
must produce the same files: a case whose output differs between runs (process ids,
clock times, `$RANDOM`) cannot be compared and is skipped.

> **Trade-off.** Recorded goldens over running bash next to psh. Every platform, Windows
> and the browser build included, compares against the same files, and a test run needs
> neither bash nor Docker and finishes in seconds. The cost: a golden is only as current
> as its recording, and a probe with a local bash can disagree with it, for example
> through macOS's BSD utilities; the goldens win.

Before comparing, both sides go through a short normalization table. It replaces only
what differs from run to run: paths such as the case's directory, home, helper directory
and shell binary become `<case>`, `<home>`, `<bin>` and `<sh>`, the host name becomes
`<host>`, and the shell's own name at the start of an error message (`bash:` against `psh:`)
becomes `<sh>`. Nothing else is rewritten; a difference that cannot be made
deterministic is not normalized away.

A case passes when stdout, the exit status and the resulting files match. stderr is
compared too but counted as a separate score, so error messages can catch up without
holding back the language.

## The layers

| Layer | What it is | What it checks |
| --- | --- | --- |
| L0 parse | every case script below, the shape corpus and the parser's own snippets | the parser accepts exactly what `bash -n` accepts, and every tree survives print and reparse unchanged |
| L1 Oils | the Oils project's spec tests, vendored | the language, area by area |
| L2 brush | the brush shell's test cases, vendored | the language again, from another project's angle |
| L3 agent | cases written to the shapes of real agent commands | `cd X && …`, `\| head`, `2>&1`, quoted here-documents, globs that match nothing, git, rg and jq pipelines |
| L4 utilities | cases per in-process utility | each utility's output, and a clear error or a fall-through for flags it does not support |
| L5 process | Rust integration tests | cancel, `timeout` killing a whole tree, no orphans, `SIGPIPE` in `\| head`, background jobs |

Each layer runs its cases the way its authors meant them: Oils cases get their code on
bash's standard input, brush cases their arguments and input, agent and utility cases run
as `bash -c CODE`, the way an agent's command runs. Every case starts in a fresh
directory with its own home and an environment built from scratch, nothing inherited.

**L0** runs the parser over 7,807 case scripts, 26,234 skeletons of the shape corpus and
237 snippets written for the parser; all of them agree with `bash -n` and round-trip.
[The parser](syntax.md) explains the printing that makes the round trip possible.

**L1 and L2** are the two largest suites of bash behavior with permissive licenses. They
hold no bash expectations psh relies on: the goldens come from the recording, so a case
means what bash does.

**L3** is the layer closest to [agent-first](goals.md#agent-first): 701 cases, each
written to the pattern of commands agents actually run, with stub scripts standing in for
tools the container lacks (`node`, `npm`, `cargo`) so the command keeps its shape.
Measured against the shape corpus, about half of the agent commands it stands for have a
case with the same command structure.

**L4** records every utility case twice: from GNU in the container and from BSD on macOS.
Where the two agree, psh must match both. Where they disagree, the case declares which one
psh follows and why, and a case that declares nothing fails as "goldens disagree" until
it does. That keeps psh's utilities [consistent](goals.md#consistent) across platforms
instead of copying whichever system the recording ran on.

**L5** starts real programs, with a small helper binary as the external program, on Unix
and Windows. Every test runs under a deadline and ends by checking that no process it
started survived.

### Stress runs

> **Not built yet.** L6 re-runs the other layers under perturbation: cancel at a random
> step (nothing may leak, the shell must stay usable), pause and resume every few steps,
> a host on which every operation first reports "not ready", tiny pipe buffers and slow
> readers.

## The shape corpus

What agents run comes from a private corpus of about 295,000 real agent commands. It
holds paths, project names, file contents and sometimes secrets, so it is never committed
and never executed: running it would mean running other people's commands. A local tool
turns each command into its syntax skeleton. Operators, redirections, quoting, here-doc
markers and expansions stay; every word outside a reviewed vocabulary of keywords,
builtins, tool names, subcommands and common options becomes a placeholder:

```text
3387	cd /w1 && sed -n w2,w3 w4
3337	git status --short
1635	python3 - <<'PY'\nw0\nPY
```

The committed file holds 26,234 skeletons standing for about 123,000 commands, each with
its count. A gate check proves that every token in it is a vocabulary word, a placeholder
or a single digit. The skeletons feed L0, the coverage measure of L3 and the ranking of
failures below.

## Groups, and cases that need care

Every case is in one group, chosen by rules that match case ids or tags, each with its
reason:

- **skip**: psh will never do this: interactive features, `set -m` job control, `select`,
  history, completion, zsh-only files, and cases whose output differs between recordings.
  Skipped cases leave the denominator, so a score never counts features psh does not aim
  at, and it never looks better because of them either.
- **later**: on the roadmap but not now (`coproc`, for one). Reported separately.
- **active**: everything else. Only active cases are held to the ratchet.

Two other marks sit on top of the groups. A **serial** case uses a fixed path outside its
directory, such as a directory under `/tmp`, and holds a machine-wide lock while it runs,
so no two such cases overlap, not even across test runs of different checkouts. A
**flaky** case passes or fails by timing; it runs and counts as usual, but its result
never fails a run, and every report lists it with the race and who fixes it. Flaky cases
are never retried: a retry turns a real intermittent bug into a green run.

## Ratchets

Each layer has an expected-failures file: the active cases psh may fail, one id per line
with an optional reason. A case id is the layer, the file and the case's position in it:

```text
oils/arg-parse/1
oils/bugs/13: ulimit is not supported yet (fails on Linux; macOS lacks /dev/full, so it is host-specific there)
```

A run fails when an active case fails that is not listed: a regression. It reports
listed cases that now pass, and updating the ratchet deletes their lines. Lines are added
only for newly written or vendored cases, never to hide a regression, so progress is
counted in lines deleted.

The goldens come from Linux, so the shared file lists what psh fails on Linux. Other
platforms add their own file for cases that pass on Linux but fail there because the
system's programs differ: `macos` (BSD `ls`, `sed`, `od`), `windows` (programs Windows
lacks) and `wasm` (no external programs at all). A run checks the shared file plus its
platform's, and the platform files shrink as psh's own utilities replace the system's. A
few cases depend on the host itself rather than its programs: `/dev/full`, `/proc`, where
`ls` lives, `/tmp` being a symlink on macOS, signal numbers. They are held to the ratchet
on Linux only and reported apart elsewhere.

## Leaks

Every case of every layer is also checked for leftover processes, not only the process
tests. An in-process case's host kills whatever the run left, `nohup` jobs included, and a
process group (on Windows, a job) still alive after the grace period fails the case as
`leak`. That is the
[control of execution](goals.md#control-of-execution) goal checked on thousands of
scripts instead of a handful of tests. A process that starts a session of its own with
`setsid` is out of the check's reach.

## Where psh stands

Active cases passing (stdout, status and files), and how many also match on stderr:

| Layer | Active | Pass | % | Also stderr | Skip |
| --- | ---: | ---: | ---: | ---: | ---: |
| L1 Oils | 2,511 | 2,252 | 89.7% | 2,175 | 234 |
| L2 brush | 3,171 | 2,747 | 86.6% | 2,640 | 465 |
| L3 agent | 701 | 689 | 98.3% | 687 | 0 |
| L4 utilities | 671 | 671 | 100% | 668 | 0 |
| L0 parse | 34,278 | 34,278 | 100% | – | – |

These are macOS numbers. The twelve agent cases that fail all fail only there, on BSD
utilities' output; the shared agent ratchet is empty.

## The hit list

A failing here-document case matters more than a failing `select` case, and a plain count
treats them alike. The hit list ranks failing active cases by weight: the corpus frequency
of the rarest construct in the case's script, read from the parser's tree, since a test
case usually exists for its most unusual part. Constructs the corpus never measured
(`coproc`, namerefs, `=~`, extended globs) count as rare. Areas rank by the summed weight
of their failures, and work starts from the top.

## The gate

Nothing reaches the main branch without passing the gate. Its legs run in order, each
under a timeout:

| Leg | What it checks |
| --- | --- |
| `scripts` | every repository script parses |
| `fmt`, `clippy`, `test` | formatting, lints with warnings as errors, the workspace's tests |
| `deps` | every dependency, direct or transitive, is on an allowlist with the reason it is allowed, and nothing else is |
| `wasm`, `wasm-host` | the parser, engine and classifier build for WebAssembly; the browser build passes its tests and its ratchets |
| `msrv` | the workspace builds with the oldest supported Rust |
| `linux` | a type-check for Linux, since the gate runs on macOS |
| `script-tests` | tests of the gate's own scripts |
| `shapes` | the shape corpus passes its privacy check |
| `guide` | this guide follows its writing rules |
| `spec`, `l0` | the ratchets of every layer, shared and this platform's, and of L0 |
| `windows` | always last: formatting, lints, tests on arm64 and x64, the oldest Rust and L4, in a Windows virtual machine |

Each leg declares the files it reads. A leg whose inputs, script and toolchain are exactly
as on an earlier pass is skipped, so a change to the guide alone gates in seconds, and
the Windows leg runs only when Rust code or a Windows script changed. A gate never gets
weaker to get green: no deleted tests, no skipped legs, no ratchet lines for a regression.
That Windows runs in every gate is what keeps psh [portable](goals.md#portable) rather
than portable in principle; [psh on Windows](windows.md) explains what it covers there.

## Where to look

- [crates/psh-spec/src/runner.rs](../../crates/psh-spec/src/runner.rs): runs a case and
  compares it; [crates/psh-spec/src/golden.rs](../../crates/psh-spec/src/golden.rs): the
  golden format; [crates/psh-spec/src/normalize.rs](../../crates/psh-spec/src/normalize.rs):
  the normalization table.
- [crates/psh-spec/src/l0.rs](../../crates/psh-spec/src/l0.rs): L0;
  [crates/psh-spec/src/ratchet.rs](../../crates/psh-spec/src/ratchet.rs): the ratchets;
  [crates/psh-spec/src/groups.rs](../../crates/psh-spec/src/groups.rs) and
  [tests/spec.toml](../../tests/spec.toml): groups, serial, flaky and host-specific cases.
- [crates/psh-spec/src/status.rs](../../crates/psh-spec/src/status.rs) and
  [crates/psh-spec/src/ast_features.rs](../../crates/psh-spec/src/ast_features.rs): the
  scores and the hit list.
- [docker/Dockerfile](../../docker/Dockerfile) and
  [scripts/record-goldens](../../scripts/record-goldens): the recording;
  [tests/goldens/agent/cd-chains.golden](../../tests/goldens/agent/cd-chains.golden): a
  golden file.
- [tests/agent/](../../tests/agent/), [tests/utils/](../../tests/utils/) and
  [tests/process/main.rs](../../tests/process/main.rs): L3, L4 and L5;
  [tests/shapes/shapes.txt](../../tests/shapes/shapes.txt): the shape corpus.
- [scripts/gate.sh](../../scripts/gate.sh) and
  [scripts/gate.d/](../../scripts/gate.d/): the gate and its legs.
