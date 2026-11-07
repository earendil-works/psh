# 0034: The psh binary's own command line on `argument`

status: done
agent: cli
branch: issue/0034-psh-cli
depends-on: -
touches: crates/psh/ (main.rs, CLI), crates/psh-engine/src/invocation.rs (only if needed), Cargo.toml, scripts/deps-allowlist.txt, docs/guide (CLI page if one exists)
guide: cli.md (not written yet)
decisions: D1340–D1349

## Goal
Owner decision (D035): the psh binary's own argument parsing (its modes and flags, not
the shell language) uses the `argument` crate (crates.io, Apache-2.0, by the owner; it
depends only on `argument-parser`). It gives a static spec, a plain parse loop and
generated help pages, usage lines and errors.

## Acceptance criteria
- [x] psh's own surface on `argument`: `--parse`, `--classify` (with `--compact`, `-O`),
      `--help`/`-h` with a generated help page, `--version`, and room for future modes
      (e.g. `--utils` listing, `--force-utils`). Help text explains the modes.
- [x] bash-compatible invocation keeps working exactly as today and as bash does:
      `psh -c CMD [NAME [ARGS…]]`, `psh FILE ARGS…`, stdin scripts, combined short
      options (`-euc`, `-ec`), `+o`/`-o NAME`, `-O`/`+O shopt`, `--` and the first
      non-option ending option parsing (the shell's own rules from invocation.rs).
      Evaluate whether `argument` can express these; if not (e.g. `+o`), keep
      invocation.rs for the bash-compatible part and use `argument` for the psh-specific
      modes and the help page — record which part uses what.
- [x] Non-UTF-8 arguments still work (args_os), on Windows too.
- [x] Dependency recorded in scripts/deps-allowlist.txt citing D035; size effect noted.
- [x] Tests: CLI tests for every mode and the bash invocation forms; L1/L2 invocation
      cases (oils `sh-options`, brush invocation) unchanged.

## Log
- Evaluated `argument` (0.1.0) for bash's invocation: it cannot express it. `+e`/`+o NAME`
  are positionals to argument-parser; `-o`/`-O` must take the *next* argument even inside
  a cluster (`-eo pipefail`; argument-parser would take `pipefail` only as `-opipefail`);
  `-c` is a flag whose command is the first operand (`-c -- CMD`); `-h` is `set -h`;
  bash's errors (`psh: -Z: invalid option`) are worded differently.
- Split by position (bash's own rule: GNU long options first): `crates/psh/src/cli.rs`
  runs `argument` over the leading `--name` arguments; bash's long options
  (`invocation::LONG_OPTIONS`, new const) pass through, the first other argument and
  everything after it go to `invocation::parse_args` unchanged. `--parse`/`--classify`
  are `argument` sub-specs (`p.subcommand`), so `psh --parse --help` / `-h` work and
  usage reads `psh --parse [OPTIONS] [FILE]`. D1340.
- `--classify` gained `-O extglob` (parse_with + classify_ast in main.rs).
- `--help` now prints the generated page to stdout with 0 (bash does the same), not the
  old usage on stderr with 2; `psh -e --help` (Help from invocation.rs) prints it too.
- Non-UTF-8: `OsString` throughout (`raw_value`, `raw_arg`); a unix CLI test passes
  `caf\xe9` to `-c`'s `$1` and to `--parse -c`. On Windows args stay lossy UTF-8 as before.
  clippy clean for x86_64-pc-windows-msvc.
- Tests: unit tests in cli.rs (pass-through of bash forms, modes, error kinds, help page
  without `-h`/`-V`); CLI tests for help/version/mode help, errors on both sides, bash
  forms (`-euc`, `+o`, `-eo`, `-O`/`+O`, `-c --`, `--norc`, `--posix`), stdin scripts
  (`-s`, none, `-e`), non-UTF-8, classify with extglob. The helper now pipes stdin.
- L1/L2 unchanged: oils sh-options* 7/9, brush compat/arguments 77/92, no ratchet change.
- Size: release psh 8,874,176 → 8,985,456 bytes (+109 KiB, macOS arm64).
- Docs: ARCH.md "The `psh` binary" updated; no guide page affected (cli.md does not exist).

- 0034b (branch issue/0034-cli-followups): `psh --utils [--json]` (crates/psh/src/utils.rs,
  D1341): every registry name with mode, destructive, what runs here and the PATH
  program, answered by a shell configured as `psh -c` (`type -a`), not a re-implemented
  lookup; `shell_config()` is shared with the normal run. An unreadable script operand
  is now 126 (127 only when missing), as bash. Guide page docs/guide/cli.md written and
  in SUMMARY.md; utilities.md points at `--utils`; ARCH.md mentions it. Tests: unit
  (row parsing, names need no quoting), CLI (`--utils` table and JSON with an empty
  PATH; on Unix an `awk` on PATH runs the program, `PSH_UTILS_FORCE=awk` psh; 126/127).

## Follow-ups
- `--force-utils` (a flag for what PSH_UTILS_FORCE does) fits as another top-level `Opt`.
- psh-spec's in-process runner still reports every unreadable script as 127.
- bash rejects GNU long options after single-letter ones (`bash -e --norc` is an error);
  invocation.rs still accepts them anywhere among the options (unchanged here).

## Result
psh's own command line is on `argument` (`crates/psh/src/cli.rs`): `--parse` and
`--classify` (each with `-c COMMAND`, `--compact`, `-O extglob`, `FILE`, own `--help`/`-h`),
`--help` and `--version`, with generated help pages, usage and suggestions on typos.
bash's invocation stays in `psh_engine::invocation` (shared with psh-spec) and behaves as
before. Known gaps: see Follow-ups.
