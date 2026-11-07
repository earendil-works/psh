# 0034: The psh binary's own command line on `argument`

status: in-progress
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
- [ ] psh's own surface on `argument`: `--parse`, `--classify` (with `--compact`, `-O`),
      `--help`/`-h` with a generated help page, `--version`, and room for future modes
      (e.g. `--utils` listing, `--force-utils`). Help text explains the modes.
- [ ] bash-compatible invocation keeps working exactly as today and as bash does:
      `psh -c CMD [NAME [ARGS…]]`, `psh FILE ARGS…`, stdin scripts, combined short
      options (`-euc`, `-ec`), `+o`/`-o NAME`, `-O`/`+O shopt`, `--` and the first
      non-option ending option parsing (the shell's own rules from invocation.rs).
      Evaluate whether `argument` can express these; if not (e.g. `+o`), keep
      invocation.rs for the bash-compatible part and use `argument` for the psh-specific
      modes and the help page — record which part uses what.
- [ ] Non-UTF-8 arguments still work (args_os), on Windows too.
- [ ] Dependency recorded in scripts/deps-allowlist.txt citing D035; size effect noted.
- [ ] Tests: CLI tests for every mode and the bash invocation forms; L1/L2 invocation
      cases (oils `sh-options`, brush invocation) unchanged.

## Log

## Follow-ups

## Result
