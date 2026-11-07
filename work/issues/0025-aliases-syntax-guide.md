# 0025: Alias expansion in the parser, and the syntax guide page

status: in-progress
agent: parser
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

### 2026-11-07: bash's behavior (probed in the recording container)
- Non-interactive bash (`bash -c`, scripts) expands aliases only after
  `shopt -s expand_aliases`; every Oils alias case turns it on. Aliases defined on a
  line take effect from the next complete command line read (`alias e=echo; e one`
  does not expand), because bash expands while reading.
- Expansion happens to the first word of a command wherever a command can start: after
  `;` `|` `&&` newlines, after `!` and `time`, after assignments and redirections
  (`A=1 ll`, `>out ll`), inside `$(…)` (parsed when read), in function bodies when they
  are read. It happens *before* reserved words are recognized (`alias myif=if` works;
  `alias if=echo` makes `if x` run echo). Only an unquoted word with no `\`, quotes or
  `$` qualifies (`'q'`, `\q`, `q""` do not expand).
- The alias text is read as input: it may hold operators, several commands, newlines,
  keywords (`alias c=case`). A text ending in a blank makes the next word a candidate
  too (`alias a='echo '; a b`). An alias is not expanded again inside its own expansion
  (`alias ls='ls -l'`; `x`→`y`→`x` stops at `x`). Bash even lets a quote opened in the
  alias text continue in the source (`alias a='echo "x'; a y"`): psh does not (a known
  gap; the alias text is lexed on its own).

### 2026-11-07: proposed design (for review before the engine side)
psh-syntax:
```rust
#[derive(Clone, Default)] pub struct Aliases { /* Arc<BTreeMap<Vec<u8>, Vec<u8>>>, cheap to clone */ }
impl Aliases { fn get(&self, name: &[u8]) -> Option<&[u8]>; fn set(&mut self, name, value);
               fn remove(&mut self, name) -> bool; fn clear(&mut self);
               fn iter(&self) -> impl Iterator<Item = (&[u8], &[u8])> /* sorted by name */; }
impl Parser { pub fn set_aliases(&mut self, aliases: Option<&Aliases>); } // None = expand_aliases off
impl Ast {
    pub fn script_len(&self) -> u32;               // spans >= this are alias text
    pub fn alias_uses(&self) -> &[AliasUse];        // { name, word: Span (what was replaced), text: Span (the alias text) }
    pub fn original_span(&self, span: Span) -> Span; // alias text mapped to the word it replaced (LINENO, messages, UI)
}
```
Mechanics: when a chunk expands an alias, its AST gets its own source buffer: the script
followed by one copy of the alias text per use; the parser jumps into the copy and back
(token boundaries end it). Spans of words from an alias point into those copies (so
`ast.text(span)` works and `span.start >= script_len()` marks alias text); a span that
covers both alias text and script text (`ll x` for `ll='ls -l'`) is mapped to script
offsets. Chunks without aliases are unchanged (no copy). `parse`/`parse_with` (whole
scripts, `--parse`, the classifier) never expand aliases, like `bash -n`.

Engine side (proposal; the engine agent owns exec.rs/lib.rs):
1. Shell state holds a `psh_syntax::Aliases`; `alias`/`unalias` live in a new module
   `builtins/alias.rs` (I write it: argument handling, `alias -p` / `alias NAME`
   output with bash's quoting, `unalias -a`, errors and statuses), registered with two
   lines in `builtins::find`.
2. In the chunk loop (`run_script`, lib.rs), before each `parser.next_chunk()`:
   `parser.set_aliases(shopt.expand_aliases.then_some(&state.aliases))` (same place as
   the extglob option when that lands). The same for every place that parses text at
   run time with a `Parser`: `eval`, `source`/`.`, backquote bodies, `bash -c`-like
   re-entry.
3. LINENO and error locations from `ast.original_span(span)`.

## Follow-ups

## Result
