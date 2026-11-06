# 0013: Utilities batch B: grep, egrep, fgrep, rg (and fd)

status: open
agent: -
branch: issue/0013-utils-search
depends-on: 0008; the rg/fd embedding section of research/builtins.md (in progress)
touches: crates/psh-utils/src/{grep,rg,fd}*, tests/utils/{grep,rg,fd}/, Cargo.toml (the ripgrep crates)
guide: utilities.md (not written yet), utilities reference (not written yet)
decisions: D1130–D1139

## Goal
Search, the second and fifth most used names in the corpus (`grep` 71k, `rg` 58k
commands). Built on the ripgrep crates, the standing dependency exception (D006).

## Acceptance criteria
- [ ] `rg` (S): matches real rg exactly: no smart-case by default, `--no-heading` and
      line numbers only on a TTY, `-n --glob/-g -i -S -C -l --type -o --hidden`, `-g`
      brace globs, `.gitignore` semantics including parent directories, early exit
      under `| head`.
- [ ] `grep`, `egrep`, `fgrep` (S): GNU flag set `-n -E -F -v -i -r/-R -l -L -c -o -q
      -s -H -h -I -w -x -A/-B/-C` (also attached, `-A8`), `--include/--exclude(-dir)`,
      multiple `-e`, `-f`, `-z`, `--color`; BRE with backreferences
      (`\(a\)\1`, needs a backtracking fallback).
- [ ] `grep -P`: only once the backend crate is decided (`pcre2` or `fancy-regex`,
      ARCH.md open question 6); until then it falls through.
- [ ] `fd` (Tier 3, F) if the embedding section asks for it.
- [ ] Each utility registered with its mode; unsupported flags fall through or fail
      clearly (tested per utility).
- [ ] L4 cases in `tests/utils/<name>/` for every flag listed below, recorded against
      GNU and BSD, with the D016 rule named where they conflict.
- [ ] Attribution headers and `NOTICE` entries for ported code.
- [ ] ripgrep's own tests consulted or run for `rg`.

## Notes
- **Placeholder:** `research/builtins.md` is getting a section on embedding rg/fd via
  the ripgrep/fd crates. Read it before starting and refine these criteria with the
  coordinator; ARCH.md "rg and fd on the ripgrep/fd crates" will carry the outcome.
- GNU grep's source is off limits (D007); its manual is fine.

## Log

## Follow-ups

## Result
