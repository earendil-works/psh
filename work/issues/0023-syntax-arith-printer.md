# 0023: Arithmetic parser, bash-format printer, byte-exact sources

status: in-progress
agent: parser
branch: issue/0023-syntax-arith-printer
depends-on: 0004
touches: crates/psh-syntax/, crates/psh/ (--parse JSON for the new nodes), tests/l0/
guide: syntax.md (follow-up from 0004)
decisions: D1230–D1239

## Goal
Parser-side pieces the engine needs next (0004 follow-ups), built in psh-syntax while the
engine agent builds the interpreter (0005), so the two don't collide.

## Acceptance criteria
- [ ] **Arithmetic**: a parser for bash arithmetic expressions (`$(( ))`, `(( ))`,
      `for (( ;; ))`, array subscripts, `let`) over *expanded* text, as bash parses it
      after expansion: all operators and precedence, assignment operators, `++`/`--`,
      ternary, comma, bases (`0x`, `0`, `N#`), variable references. An AST with spans,
      exposed as a public API the engine calls at run time, plus error messages
      matching bash's (`syntax error: operand expected (error token is "…")`).
      Evaluation stays in the engine; offer a pure `eval` over an env trait only if it is
      small and the engine agent agrees.
- [ ] **Printer in bash's format**: `declare -f` / `type` output, matching bash 5.3 for
      the Oils/brush cases that print functions (indentation, `;` placement, here-docs).
- [ ] **Bytes**: sources that are not valid UTF-8 parse byte-exactly (spans stay byte
      offsets into the original input); decide and record how the AST carries non-UTF-8
      words.
- [ ] Tests: unit tests, L0 snippets for arithmetic syntax errors where bash reports
      them at parse time, and agreement with recorded goldens for the printer.
- [ ] Stays `#![no_std]` + `alloc`, zero dependencies, wasm32 build.

## Notes
- Coordinate API shapes with the engine agent (0005) through your issue log and reports
  to the coordinator; the engine agent consumes these APIs.
- Clean room: bash's manual describes arithmetic; never its source.

## Log

### 2026-11-07: proposed API (for 0005, via the coordinator)
Arithmetic, in `psh_syntax::arith` (re-exported at the crate root), over the *expanded*
text as bytes; spans are byte offsets into that text:

```rust
pub fn parse_arith(text: &[u8]) -> Result<Option<ArithExpr>, ArithError>; // None: blank text (value 0)

pub struct ArithExpr { pub span: Span, pub kind: ArithKind,
                       pub last: u32 }        // start of its last token (runtime error tokens)
pub enum ArithKind {
    Num(i64),                                  // decoded: 10, 0x1f, 017, 64#@_, wraps like bash
    Var(LValue),                               // x, a[i+1]
    Unary { op: UnaryOp, expr: Box<ArithExpr> },             // + - ! ~
    IncDec { target: LValue, inc: bool, post: bool },        // ++x x++ --x x--
    Binary { op: BinOp, lhs: Box<ArithExpr>, rhs: Box<ArithExpr> }, // ** * / % + - << >> < <= > >= == != & ^ | && || ,
    Cond { cond: Box<ArithExpr>, then: Box<ArithExpr>, otherwise: Box<ArithExpr> },
    Assign { target: LValue, op: Option<BinOp>, value: Box<ArithExpr> }, // = *= /= %= += -= <<= >>= &= ^= |=
}
pub struct LValue { pub span: Span, pub name: Span,
                    pub index: Option<Span> }  // raw subscript text: arithmetic for indexed
                                               // arrays, a key for associative ones (engine decides)
pub struct ArithError { pub kind: ArithErrorKind, pub token: u32 /* "error token is" starts here */,
                        pub prefix: Option<Span> /* None: the whole expression */ }
impl ArithError {
    pub fn message(&self) -> &'static str;     // "arithmetic syntax error: operand expected", ...
    pub fn bash_text(&self, text: &[u8]) -> String; // `1 + : arithmetic syntax error: operand expected (error token is "+ ")`
}
```

The caller adds bash's context prefix (`bash: line N: `, `let: `, `((: `). Bash evaluates while
it parses; with a separate parse, a syntax error after a runtime error (`1/0 +`) is reported
as the syntax error, where bash reports division by 0 (rare; noted).

Offered, if the engine agent wants it: a pure evaluator in psh-syntax,
`pub fn eval_arith(expr: &ArithExpr, text: &[u8], env: &mut dyn ArithEnv) -> Result<i64, ArithError>`
with `trait ArithEnv { fn get(&mut self, name: &[u8], index: Option<&[u8]>) -> Result<Option<Vec<u8>>, ArithError>; fn set(&mut self, name: &[u8], index: Option<&[u8]>, value: i64) -> Result<(), ArithError>; }`,
handling bash's rules (values of variables evaluated recursively with the recursion limit,
short-circuit `&&`/`||`/`?:`, wrapping 64-bit math, shifts mod 64, `division by 0`,
`exponent less than 0`, error tokens like bash's). About 250 lines; the engine would only
supply variables.

### 2026-11-07: part 1, arithmetic (as landed; supersedes the proposal above)
The engine agent agreed to both (via the coordinator) and asked for `set -u` and
subscript handling in the environment. API (`psh_syntax::*`, D1230, D1231):

```rust
pub fn parse_arith(text: &[u8]) -> Result<Option<ArithExpr>, ArithError>; // None: blank (0)
pub fn eval_arith<E: ArithEnv>(text: &[u8], env: &mut E) -> Result<i64, EvalError<E::Error>>;
pub fn eval_arith_expr<E: ArithEnv>(expr: &ArithExpr, text: &[u8], env: &mut E)
    -> Result<i64, EvalError<E::Error>>;                       // reuse a parse (for loops)
pub trait ArithEnv {
    type Error;                                                // the engine's own errors
    fn is_assoc(&mut self, name: &[u8]) -> bool;               // subscripts are keys then
    fn get(&mut self, name: &[u8], index: Option<ArithIndex<'_>>)
        -> Result<Option<Vec<u8>>, Self::Error>;              // None = unset (0); set -u: Err
    fn set(&mut self, name: &[u8], index: Option<ArithIndex<'_>>, value: i64)
        -> Result<(), Self::Error>;                            // readonly: Err
}
pub enum ArithIndex<'a> { Num(i64) /* evaluated, may be negative */, Key(&'a [u8]) /* raw text */ }
pub enum EvalError<E> { Arith { error: ArithError, text: Vec<u8> }, Env(E) }
// EvalError::bash_text() -> Option<String>: `EXPR: message (error token is "…")`
// ArithError { kind: ArithErrorKind, token: u32, prefix: Option<Span> },
//   .message(), .bash_text(text); ARITH_RECURSION_LIMIT = 1024
```

`text` in `EvalError::Arith` is the text the error is in: the expression, a variable's
value or a subscript, as bash names it. A plain `a` for an array is `get("a", None)`:
the environment maps it to element 0.

Tests: unit tests (precedence, increments, numbers, 43 error texts, spans, values,
assignments, runtime errors, recursion limit, a 1 MiB stack in release). Differential
fuzzing against bash 5.3 (`work/scratch/0023/fuzz.py`, `fuzzeval.py`, through
`examples/arithcheck`): 30k random token strings agree on parse errors (the only
difference is `let --`, which is `let`'s option end), and 9k random expressions agree on
value, side effects and runtime error text.

### 2026-11-07: part 2, the bash-format printer
- `print_function_bash(&ast, cmd_id) -> String`: what `declare -f NAME` prints (no
  trailing newline); `type NAME` prints `NAME is a function` and then the same. Also new:
  `decode_ansi_c(&[u8]) -> Vec<u8>` (bash's `$'…'` escapes, for the engine too),
  `Ast::cmd_ids()`, and `AndOr::newline` (the item ended at a newline; bash keeps
  that inside substitutions).
- Rules derived from bash's output only (D1232), with a differential harness:
  `examples/fncheck.rs` (`extract` function definitions from scripts, `print` them) and
  `work/scratch/0023/fncompare.sh DIR` (runs `bash -c 'source F; declare -f NAME'` per
  function and diffs), plus `work/scratch/0023/genfn.py DIR SEED N` (random function
  bodies: nested compounds, here-docs, redirections, substitutions).
- Results: all 1027 functions defined in the Oils and brush case scripts print
  identically; random functions: 2,0xx valid ones over seeds 1–8, all identical except
  10 that put here-docs and further substitutions inside a `$((…) )` that is not
  arithmetic (bash keeps that text but re-lexes parts of it in ways I did not pin down).
- Bash quirks reproduced (all in tests): `elif` printed as `else if … fi`; `;` after every
  command of loop and `if` bodies but not brace groups; `for` puts `do` on its own line;
  `for ((;;))` prints `((1; 1; 1))`; redirections normalized (`>&2` → `1>&2`, `3<&-` →
  `3>&-`, `0<f` → `< f`); `|&` → `2>&1 |`; `[[ a ]]` → `[[ -n a ]]`; coproc names
  `COPROC`; nested functions get `function`; `$'…'` → `'…'`; comments and line
  continuations dropped; here-doc bodies after the next operator, after which bash
  leaves out the next `;` (until a redirection or brace group) and writes
  `while a;    do`; inside `$(…)` the separators stay as written and brace groups stay
  on one line; `$((…) )` that is not arithmetic is kept as written.
- `tests/l0/declare-f.txt` (206 cases: probes + random functions, bash 5.3 output) is
  checked by `crates/psh-syntax/tests/declare_f.rs`. Format: `#### N`, source,
  `## declare -f:`, bash's output, `## end`.

## Follow-ups

## Result
