# 0025: Alias expansion in the parser, and the syntax guide page

status: open
agent: -
branch: issue/0025-aliases-syntax-guide
depends-on: 0023
touches: crates/psh-syntax/, crates/psh-engine/src/builtins/alias* (new module only), tests/l0/, docs/guide/syntax.md, docs/guide/SUMMARY.md (one line)
guide: syntax.md
decisions: D1250–D1259

## Goal
bash expands aliases while it reads commands, so alias expansion is a parser feature:
the chunked parser must consult the alias table as it parses each complete command,
including bash's rules (trailing space chains to the next word, recursion guard,
`expand_aliases` on by default in psh's non-interactive mode? — check what bash 5.3
does in `bash -c` and scripts and follow the goldens). Also the human-facing page on
the parser.

## Acceptance criteria
- [ ] psh-syntax: an alias-lookup hook for the chunked parser (`Parser::next_chunk` with
      an alias callback or table), applied where bash applies aliases (command
      position, after a trailing-space alias, not in quotes or after `\`), with the
      recursion guard; the AST keeps spans pointing at the original source and marks
      text that came from an alias.
- [ ] Engine side kept minimal and in a new module: `alias`/`unalias` builtins and
      passing the table to the parser per chunk (coordinate the exact hook with the
      engine agent through the coordinator; the engine agent owns exec.rs).
- [ ] Oils `alias.test.sh` and brush `builtins/alias.yaml` cases pass or have recorded
      reasons; ratchets updated.
- [ ] `docs/guide/syntax.md`: what the parser accepts, byte spans, `--parse` JSON,
      round-trip printing, error messages, aliases, non-UTF-8 input, no_std/wasm.
      `psh-docs check` passes.

## Log

## Follow-ups

## Result
