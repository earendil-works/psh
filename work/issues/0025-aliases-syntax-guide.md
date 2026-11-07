# 0025: Alias expansion in the parser, and the syntax guide page

status: done
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
- [x] psh-syntax: an alias-lookup hook for the chunked parser (`Parser::next_chunk` with
      an alias callback or table), applied where bash applies aliases (command
      position, after a trailing-space alias, not in quotes or after `\`), with the
      recursion guard; the AST keeps spans pointing at the original source and marks
      text that came from an alias.
- [x] Engine side kept minimal and in a new module: `alias`/`unalias` builtins and
      passing the table to the parser per chunk (coordinate the exact hook with the
      engine agent through the coordinator; the engine agent owns exec.rs).
- [x] Oils `alias.test.sh` and brush `builtins/alias.yaml` cases pass or have recorded
      reasons; ratchets updated.
- [x] `docs/guide/syntax.md`: what the parser accepts, byte spans, `--parse` JSON,
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

### 2026-11-07: part 1, the parser hook (as built; D1250)
- psh-syntax: `Aliases` (`new get set remove clear is_empty len iter`), `AliasUse { name,
  word, text }`, `Parser::set_aliases(Option<&Aliases>)`, `Ast::{script_len,
  alias_uses, original_span}`. Two passes per chunk that uses aliases; composite spans
  are now built first-start..last-end in reading order (`join`), which alias text needs.
- Bash details reproduced (probed in the container): an empty alias text leaves an
  empty simple command (no words, no assignments, no redirections: the engine runs it
  as a no-op) or exposes the next word as the command word (`alias e=''; e x` expands
  `x`); a comment in alias text runs on into the rest of the line (`alias c='echo hi #
  x'; c there` prints `hi`); a newline or `;` in alias text ends the command, the rest
  of the line is the next command (same chunk).
- Engine module `builtins/alias.rs` (declared, not registered: `#[allow(dead_code)]`):
  pure `alias(&mut Aliases, args) -> Outcome` and `unalias`, with bash's output,
  messages (`Output::Error` gets the `NAME: line N:` prefix, `Output::Usage` is printed
  raw), statuses and option handling; unit tests against the container's output. The
  module comment shows the wrapper the engine registers.

### 2026-11-07: alias fuzzing; guide page
- Toy runner `examples/aliasrun.rs` (executes only `shopt -s expand_aliases`, `alias`,
  `unalias`, `echo`, `true`/`false`, `if`, groups, `&&`/`||`) against bash in the
  recording container, on random scripts of alias definitions and uses
  (`work/scratch/0025/genalias.py`, `cmp.py`): 5,800 scripts, all agree on output and
  on syntax errors. Fixed on the way: an alias with empty or blank text is as if the word
  were not there (it never leaves an empty command; after `&&`/`||`/`|` newlines are
  skipped again; `l || q;` is an error), and a non-alias bug: a bare `!` or `time` before
  `&&`, `||`, `|` or `)` is a syntax error (bash; only `;`, `&`, newline and the end may
  follow). Snippets added to `tests/l0/syntax.txt`, L0 verdicts re-recorded.
- `docs/guide/syntax.md` written ("The parser: spans and printing" in SUMMARY.md), with
  "Not built yet" notes where the engine has not wired arithmetic, `declare -f` or the
  alias table; `psh-docs check` passes.
- Merge of part 1 was attempted once during the Windows gate rework (VM error
  `PrlJob_GetRetCode`); waiting for the merge freeze to lift.

### 2026-11-07: engine side (agreed with the engine agent via the coordinator)
- `Vp::aliases` (`psh_syntax::Aliases`, per vproc: a subshell's definitions stay in
  it) and `Vp::expand_aliases` (off; `shopt` is 0028's). `alias`/`unalias` registered
  in `builtins::find` (not special), output via `builtins::out`, messages via
  `vp.error`, usage lines raw.
- `run_script` calls `parser.set_aliases(vp.expand_aliases.then_some(&vp.aliases))`
  before each chunk; backquote bodies parse with the new one-shot
  `psh_syntax::parse_bytes_with_aliases` (eval/source in 0029 should use a `Parser` and
  set the table before each chunk instead). `Code::line` maps offsets in alias text
  through `Ast::original_span`; the backquote line table covers the script text only.
- Engine test `aliases` (expand_aliases set directly) checks expansion, `alias`
  output, subshell scope, backquotes and line numbers against bash's output.
- Spec runs with `expand_aliases` forced on (local experiment, not committed): Oils
  `alias` 40/48, brush `builtins/alias` 38/43 pass on stdout+status. Fixed on the way:
  the word after a blank-ended alias text is a candidate wherever it stands, also in a
  `for` header (`FOR2 eye2 IN onetwo 3`); `alias q2="'"` prints `alias q2=\'`.
  With the default (off), 7 alias cases newly pass (ratchet updated); no regressions.

## Follow-ups
- 0028 (`shopt`): `shopt -s expand_aliases` sets `Vp::expand_aliases`; then the Oils and
  brush alias cases that expand aliases can pass. Remaining failures with expansion on,
  none in the alias code: `wc -l` padding of the host's BSD `wc` (oils/alias/7, 28, 41,
  42), `eval` (oils/alias/31, 0029), array assignment (oils/alias/40, brush 35, 38),
  PS4/`set -x` (oils/alias/45), `declare` (brush 39), POSIX mode (brush 34), and
  brush/alias/20 only passes once `shopt -u` exists.
- `declare -f`/`type` are not built in the engine yet; `print_function_bash` is ready.

## Result
- psh-syntax: alias expansion in the chunked parser (D1250): `Aliases`,
  `Parser::set_aliases`, `parse_bytes_with_aliases`, `Ast::{script_len, alias_uses,
  original_span}`. Bash's rules, checked on 5,800 random alias scripts against the
  container's bash; spans of alias text point into a per-use copy after the script.
- psh-engine: `alias`/`unalias` builtins, the table in `Vp`, passed to the parser per
  chunk and for backquote bodies; line numbers through `original_span`. Turning
  expansion on waits for `shopt` (0028).
- Also: a bare `!`/`time` before `&&`, `||`, `|` or `)` is a syntax error (as in bash).
- `docs/guide/syntax.md` ("The parser: spans and printing").
- L0 still 100% (cases 6576, shapes 26234, syntax 237); spec ratchets: 7 more passes.
