# The parser: spans and printing

The parser turns command text into a syntax tree. The engine runs that tree, the
classifier reads it to decide what a command does, and `psh --parse` shows it to anyone
who asks. This page explains what the parser accepts, what the tree looks like, how it
reports errors, how it reads a script line by line with aliases, and what it costs.

> **In short.** psh's parser accepts exactly what bash 5.3 accepts, checked against bash
> on tens of thousands of real and generated scripts, and builds a tree in which every
> node knows the byte range of the text it came from. The same parser runs the command,
> classifies it and prints it back. It has no dependencies, runs without an operating
> system, and fits in about 44 KB of compressed WebAssembly.

## One parser for everything

A shell that is used to approve commands must not parse them twice: if the approval check
reads `git commit -m "$(rm -rf build)"` one way and the shell runs it another way, the
check can be bypassed. psh has a single parser. The classifier walks the tree the engine
executes, and a host that only wants to understand commands, such as a user interface in
a browser, embeds the same parser alone. That is the
[understandable commands](goals.md#understandable-commands) goal.

## What it accepts

The parser reads bash 5.3 syntax, and only that: no POSIX-sh mode and no zsh. It knows
lists and pipelines (`|`, `|&`, `!`, `time`), `&&`, `||`, `;` and `&`, assignments and every
form of redirection (`{fd}>`, `&>`, `<<<`, here-documents with `<<-` and quoted
delimiters), all compound commands (`{ }`, `( )`, `if`, both forms of `for`, `while`,
`until`, `case` with `;;`, `;&` and `;;&`, `select`, `[[ ]]`, `(( ))`), functions, `coproc`,
and words with all their parts: quotes, `$'…'`, `$"…"`, every parameter expansion
operator, `$(…)`, backquotes, `$((…))`, process substitution, brace expansion and extended
patterns. It also reads bash 5.3's `${ list; }` and `${| list; }`.

"Accepts what bash accepts" is measured, not claimed. For each input below, bash 5.3
(`bash -n`) and psh must agree on whether it parses, and every input that parses must
survive printing and parsing again with the same tree:

| Inputs | Count | Agreement |
| --- | --- | --- |
| Scripts of the shell conformance suites | 6,576 | all |
| Command skeletons from about 295,000 agent commands | 26,234 | all |
| Hand-written edge cases | 228 | all |

Randomly generated scripts, mixing shell tokens with bytes that are not UTF-8, agreed with
bash on every one of about 20,000 more. Several of the edge cases came from those runs:
bash rejects `in` and `]]` as commands, treats `a[b` at the start of a command as an
unclosed subscript, and does not pair braces inside `${…}` (`${x:-{a}b}` ends at the first
`}`).

Some things bash only checks when it runs a command, and so does psh. A backquoted command
is parsed when it runs, so `` echo `if` `` is a valid script. The text of `$((…))` is
arithmetic only after expansion, so `$((1 + ))` parses and fails when it runs. A `${…}`
with an operator bash does not know, such as `${x@Z}`, is a "bad substitution" at run
time. And an error inside the body of an unquoted here-document is reported when the
here-document is used. The tree keeps such parts as text, so the engine can report them
the way bash does.

> **Trade-off.** Matching bash's split between parse-time and run-time checks over
> catching every error early. A stricter parser would reject scripts bash runs, and an
> agent's command would fail in psh but not in bash. The cost: the tree has a few parts
> that are only text, such as backquoted commands, which the classifier parses on its
> own.

Extended patterns (`@(a|b)`) change how words are split into tokens, so they are an
option of the parser: off by default, as in `bash -n`, and turned on by the engine after
`shopt -s extglob`.

## A tree with byte spans

Every node of the tree carries a span: the start and end of its text, as byte offsets
into the input. A user interface can highlight exactly the word that writes a file, and an
error can point at its token. Line and column are computed from a span when someone
needs them, not stored in every node.

Words are not copied out of the input. A word is a list of parts, and each literal part is
only a span whose meaning comes from its kind: plain text, a backslash escape, a
single-quoted string, an ANSI-C string. Parsing therefore allocates little, and the
input stays the one copy of the text.

`psh --parse` prints the tree as JSON:

```sh
psh --parse -c 'cat "$f" | head -3'
```

```json
{
  "ok": true,
  "script": {
    "type": "list",
    "span": [0, 18],
    "items": [
      { "type": "and_or", "span": [0, 18], "pipelines": [
        { "type": "pipeline", "span": [0, 18], "pipes": ["|"], "commands": [
          { "type": "simple", "span": [0, 8], "assigns": [], "redirects": [], "words": [
            { "type": "word", "span": [0, 3], "text": "cat", "value": "cat",
              "quoted": false, "parts": [ … ] },
            { "type": "word", "span": [4, 8], "text": "\"$f\"", "value": null,
              "quoted": true, "parts": [
                { "type": "double", "span": [4, 8], "parts": [
                  { "type": "param", "span": [5, 7], "name": "f", … } ] } ] } ] },
          { "type": "simple", "span": [11, 18], … } ] } ] } ]
  }
}
```

(shortened). Each word has its text as written, its value after quote removal when it
has no expansions, and its parts; command substitutions carry their own parsed lists.
`--compact` prints one line, `-O extglob` turns on extended patterns, and a file name or
standard input can take the place of `-c`.

Inside the engine the tree is kept in arrays, and nodes refer to each other by index. The
engine pauses in the middle of a command, for example while a pipeline stage waits, and
then holds a position in the tree; an index can be kept across such a pause where a
reference into a tree could not.

## Errors

Errors carry bash's message and the span of the offending token:

```sh
psh --parse -c 'if true; then echo; fi; done'
```

```json
{
  "ok": false,
  "error": {
    "message": "syntax error near unexpected token `done'",
    "span": [24, 28],
    "line": 1,
    "column": 25,
    "incomplete": false
  }
}
```

The status is 2, as for `bash -n`. `incomplete` is true when the input ended inside a
construct (`unexpected EOF while looking for matching `)'`), which a caller can use to
ask for more input. For the scripts of the conformance suites that bash rejects, the
first line of psh's message is the same as bash's in about 95% of cases; the others
differ in the second line bash prints for some `[[ ]]` errors.

## Reading a script line by line

Bash does not parse a whole script before running it. It reads one complete command line,
runs it, and reads the next. So `echo a` on line 1 runs even if line 2 has a syntax
error, and `shopt -s extglob` on one line changes how the next line is read. psh's engine
works the same way: the parser hands it one chunk at a time, and the engine may change the
parser's options between chunks. The classifier and `psh --parse` read the whole input at
once.

### Aliases

> **Not built yet.** The parser expands aliases from a table it is given, but the shell
> does not yet keep that table, pass it on or offer the `alias` builtin, so scripts that
> psh runs see no aliases yet.

Bash expands aliases while it reads, so in psh alias expansion is part of the parser. A
script turns it on with `shopt -s expand_aliases` (bash leaves it off in scripts and
`bash -c`), and the engine gives the parser the current alias table before each chunk.
An alias therefore takes effect from the next line: `alias e=echo; e hi` on one line does
not expand `e`.

The parser follows bash's rules. The first word of a command is checked, wherever a
command can start: after `;`, `|`, `&&` or a newline, after `!` and `time`, after
assignments and redirections, inside `$(…)`, and even where a reserved word could stand,
so `alias myif=if` works. Only an unquoted word qualifies: `'ll'`, `\ll` and `ll""` do not
expand. The alias text is read as if it were input, so it may hold several commands,
keywords and operators. A text that ends in a blank makes the next word a candidate too
(`alias sudo='sudo '`), and an alias is not expanded again inside its own expansion
(`alias ls='ls -l'`).

Words that came from an alias still have spans, pointing at a copy of the alias text that
follows the script in that chunk's input. A command whose text runs from the alias into
the script, like `ll x` with `alias ll='ls -l'`, gets the span of `ll x`, so line numbers
and error positions name the line where the alias was used.

> **Not like bash.** A quote opened in an alias text does not continue into the script:
> with `alias a='echo "x'`, bash reads `a y"` as `echo "x y"`; psh reports an unclosed
> quote. Such aliases are not something agents write.

## Arithmetic

> **Not built yet.** The arithmetic parser and evaluator exist, but the shell does not
> evaluate `$((…))`, `((…))` or `for ((…))` with them yet.

The text inside `$((…))`, `((…))` and `for ((…))` is arithmetic only after expansion: in
`$(( $x * 2 ))` the value of `x` becomes part of the expression. So the tree keeps that
text as a word, and a separate arithmetic parser reads the expanded text when the
command runs. It knows every operator of bash with its precedence, `++` and `--`,
assignments, the conditional operator, and numbers in any base (`0x1f`, `017`, `64#@_`).
Its error messages are bash's, down to the "error token":

```text
1 + : arithmetic syntax error: operand expected (error token is "+ ")
```

The same module evaluates expressions over the shell's variables, with 64-bit wrapping
arithmetic, short-circuit `&&`, `||` and `?:`, and variable values that are themselves
expressions. Over 9,000 random expressions, psh and bash agree on every value, every
side effect and every error message.

## Printing

There are two printers. The first turns a tree back into a script: commands one per line,
words exactly as written. Every input above passes through it and parses back into the
same tree, which is how the tests check that the tree is complete.

The second prints a function the way bash's `declare -f` and `type` do (the shell does
not offer those two builtins yet). Bash reformats
function bodies with its own layout, four spaces per level, `elif` written as a nested
`if`, `>&2` written as `1>&2`, and scripts sometimes compare that output. psh's printer
reproduces it byte for byte for all 1,027 functions defined in the conformance suites and
for all but a handful of about 2,000 random ones.

## Scripts that are not UTF-8

A script may contain bytes that are not valid UTF-8, such as a file name in Latin-1. The
parser works on bytes, so such a script parses exactly as bash parses it, and spans stay
byte offsets into the input as it was given. Where an interface wants text, each byte that
is not part of valid UTF-8 shows up as a character from a private-use range (U+F780 to
U+F7FF), which turns back into the original byte when the text is used. `psh --parse`
adds `"utf8": false` to its output for such input.

## Small and portable

The parser is a Rust library with no dependencies that needs neither an operating system
nor the standard library, only an allocator. It builds for WebAssembly, where the parser
with both printers is about 131 KB, or 44 KB compressed (85 KB and 34 KB built for size).
It parses the 26,234 command skeletons, 2.3 MB of text, in about 50 milliseconds on a
laptop. Deeply nested input stops with an error ("nesting too deep") rather than
exhausting the stack, which matters on WebAssembly's small default stack. This serves
the [embeddable](goals.md#embeddable) and [portable](goals.md#portable) goals.

## Where to look

- [crates/psh-syntax/src/parser.rs](../../crates/psh-syntax/src/parser.rs): the parser,
  including alias expansion
- [crates/psh-syntax/src/ast.rs](../../crates/psh-syntax/src/ast.rs): the tree
- [crates/psh-syntax/src/arith.rs](../../crates/psh-syntax/src/arith.rs) and
  [arith_eval.rs](../../crates/psh-syntax/src/arith_eval.rs): arithmetic
- [crates/psh-syntax/src/printer.rs](../../crates/psh-syntax/src/printer.rs) and
  [bash_print.rs](../../crates/psh-syntax/src/bash_print.rs): the two printers
- [crates/psh-syntax/src/source.rs](../../crates/psh-syntax/src/source.rs): bytes and
  their text view
- [crates/psh/src/parse_json.rs](../../crates/psh/src/parse_json.rs): the JSON of
  `psh --parse`
- [tests/l0/syntax.txt](../../tests/l0/syntax.txt): the hand-written edge cases with
  bash's verdicts
