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

## Follow-ups

## Result
