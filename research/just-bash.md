# just-bash (vercel-labs/just-bash): research notes

Clone: `/tmp/just-bash` (full history, HEAD `7a2a7a5`, 2026-10-22, package version **3.6.0**).
All paths below are relative to `packages/just-bash/` unless noted otherwise.

## TL;DR

just-bash is a bash interpreter written in TypeScript. It does not run real processes. It has a
virtual filesystem and roughly 80 coreutils-style commands, each reimplemented in TS. The shell
semantics are good (2255 of 2715 Oils spec cases pass), the security hardening is serious, and it
runs in Node and the browser. It is built for **analysis-style agents** that work on text, JSON and
files. It is not a shell for a coding agent: by design it cannot run `git`, `node`, `npm`, `cargo`,
compilers or real `python`. For Pi it fits two roles. It can back an optional browser/sandbox mode as
is, and its parser could be borrowed with caveats. It is not a replacement for a real-process
portable bash.

---

## 1. What it is

- **Language:** TypeScript, ESM. Node >= 20.19. pnpm monorepo with two packages: `just-bash` and the experimental `@just-bash/executor`.
- **Size:** about 147.7k lines of non-test TS under `src/` and about 152k lines of tests.
  - `src/parser` 9.5k, `src/ast` 1.1k
  - `src/interpreter` 34.4k (expansion 9.2k, builtins 8.9k)
  - `src/commands` 68.6k
  - `src/fs` 6.9k, `src/security` 10k, `src/network` 1.75k, `src/transform` 1.2k
- **Pipeline:** `Lexer → recursive-descent Parser → AST → tree-walking async Interpreter → ExecResult`.
  - Lexer: `src/parser/lexer.ts`, 2758 lines.
  - Parser: `src/parser/parser.ts` (1378), plus `word-parser.ts`, `expansion-parser.ts`, `compound-parser.ts`, `conditional-parser.ts` and `arithmetic-parser.ts` (1170).
  - AST: `src/ast/types.ts`.
  - Interpreter: `src/interpreter/interpreter.ts`, `builtin-dispatch.ts`, `pipeline-execution.ts`, `expansion/*`, `arithmetic.ts`, `control-flow.ts`, `fd-table.ts`, `process-substitution.ts`.
- **Main authors:** Malte Ubl (Vercel CTO, 328 of about 470 commits; the package's author field is "Malte and Claude"), then Hiroki Osame and Lars Trieloff.

### Bash semantics: what works

I checked these with my own probes against `src/Bash.ts`, in addition to reading the tests:

| Feature | Status |
|---|---|
| Functions, `local`, recursion | Works (`maxCallDepth`) |
| Indexed and associative arrays, `+=`, slices, `${!m[@]}`, `mapfile`/`readarray`, `read -a` | Works |
| `$(( ))`, `(( ))`, `let`, C-style `for` | Works, but numbers are JS numbers: `$((1<<40))` → **256** (bitwise ops are 32-bit), `$((9223372036854775807))` → 9007199254740991 (clamped to MAX_SAFE_INTEGER). No 64-bit integers. |
| Here-docs (`<<`, `<<-`, quoted delimiters), here-strings | Works |
| `[[ ]]` with `=~` and `BASH_REMATCH` | Works. The regex engine is **RE2** (`re2js`), so there are no backreferences and no lookaround (`src/regex/user-regex.ts`). |
| Process substitution `<( )` / `>( )` | Works (added 2026-08, #325). It is emulated with buffers. |
| `set -e`, `set -o pipefail`, `PIPESTATUS`, `set -u`, `set -f`, `set -x` | Present. `set -x` trace output differs from bash (15 spec cases skipped). |
| `extglob`, `globstar`, `nullglob`, `failglob`, `dotglob`, `nocasematch` | Works |
| Namerefs (`declare -n`), `${!var}` indirection | Works |
| `getopts`, `printf` incl. `%q` and `%(fmt)T` | Works |
| Subshells, `$BASHPID` | Works. PIDs are virtualized. |
| `exec 3>file`, `>&3`, `{fd}>`, `>&-` | Works (`fd-table.ts`) |
| `source` / `.`, `eval`, `alias` | Works |
| Builtins list (`builtin-dispatch.ts`) | `. [ break builtin cd command compgen complete compopt continue dirs eval exec exit export false getopts hash help let local popd pushd read readarray readonly return set shift shopt test true type typeset unset wait` |

### Explicitly unsupported or missing

- **No `trap` builtin at all.** `trap "…" EXIT` gives `trap: command not found`. All the `builtin-trap*.test.sh` files are skipped.
- **No job control or real background execution.** `cmd &` is parsed (`StatementNode.background`), but the commands run sequentially. `wait` is a no-op stub and `$!` is 0. There is no `jobs`, `fg`, `bg`, `kill`, `disown`, `ulimit`, `umask` or `times`.
- `coproc` and `select` are deliberately rejected by the parser ("fail closed", #348).
- No `history` or `fc`, and no interactive or TTY features.
- No `$SH -c` style re-invocation of the real shell. `bash`/`sh` exist only as internal commands.
- **Pipelines are sequential and fully buffered.** Each stage runs to completion and its output becomes the next stage's stdin string (`src/interpreter/pipeline-execution.ts`). Output is never streamed back to the caller. `ExecResult` holds only `stdout`, `stderr`, `exitCode` and `env` strings.
- NUL-byte handling, locale settings and Unicode edge cases are only partly supported.
- **Each `exec()` call starts with fresh shell state:** env, functions and cwd are reset. Only the FS persists between calls.

Note: `spec-tests/bash/KNOWN_LIMITATIONS.md` is **stale**. It still lists getopts, nameref, extglob and assoc arrays as missing, but they work now. The `## SKIP` markers in the case files are the authoritative list.

## 2. Filesystem, network, commands

### Filesystem

The FS interface is async and pluggable: `IFileSystem` in `src/fs/interface.ts`, with `readFile`, `writeFile`, `stat`, `lstat`, `readdir`, `symlink`, `link`, `chmod`, `utimes`, `realpath` and more. Four implementations:

- `InMemoryFs` is the default. It supports lazy or async file providers and is capped by `maxFileSystemBytes`.
- `OverlayFs` is copy-on-write over a real directory. Reads come from disk and writes stay in memory. It is mounted at `/home/user/project` by default and the CLI uses it.
- `ReadWriteFs` gives real read/write access to a root. It has extensive hard-link and symlink TOCTOU defenses (`O_NOFOLLOW`, copy-on-write for multiply-linked files).
- `MountableFs` mounts several of the above into one namespace.

The default layout has `/home/user`, `/bin` (stubs for every command), `/usr/bin` and `/tmp`.

### Network

Off by default. `curl` does not exist unless `network` is configured. With network enabled:

- Requests are checked against an allow-list of URL prefixes (origin plus path prefix).
- Only GET and HEAD are allowed by default.
- Redirects are re-checked.
- Header transforms can inject credentials, so secrets never enter the sandbox.
- It is built on `guarded-fetch`.

### Commands

Every command is reimplemented in TS, one directory per command in `src/commands/*`, registered in `src/commands/registry.ts`. The groups are:

- **Files:** cat cp file ln ls mkdir mktemp mv readlink rm rmdir split stat touch tree
- **Text:** awk base64 column comm cut diff expand fold grep/egrep/fgrep head join md5sum/sha1sum/sha256sum nl od paste printf rev rg sed sort strings tac tail tr unexpand uniq wc xargs
- **Data:** jq (own engine, `commands/query-engine/`, 2.3k-line evaluator), yq (YAML/XML/TOML/CSV/INI), xan (CSV), sqlite3 (sql.js WASM in a worker)
- **Archives:** gzip/gunzip/zcat, tar (modern-tar, optional zstd/xz/bzip2)
- **Misc:** basename cd dirname du echo env find hostname date expr seq sleep time timeout which whoami yes curl html-to-markdown
- **Opt-in runtimes:**
  - `js-exec`: QuickJS via the `run` package. It has a Node-ish shim for `fs`, `path`, `child_process` (which goes back into just-bash) and others. Memory is capped at 64 MB.
  - `python3`: CPython 3.13 compiled with Emscripten, in a worker. The files live in `vendor/cpython-emscripten`, which is 9.9 MB.

How faithful the commands are:

- sed, awk, grep and jq are tested against imported spec suites, all passing: grep 332, sed 237, awk 153, jq 768.
- There are 57 recorded-fixture comparison files against real bash or GNU tools, and `rg` is compared against locked ripgrep 15.2.0 fixtures.
- Real gaps exist:
  - `xargs -I{}` (no space) fails with "invalid option -- 'I'".
  - `ls -la` shows a symlink as `d…` rather than `l… -> target`.
  - `rg` defaults to smart-case, unlike real rg.
  - All regex is RE2, so there are no backrefs in grep or sed.
- **Performance:** OverlayFs over `pi-mono/packages` (about 3.9k `.ts` files):
  - `find … | wc -l` took 384 ms (real find: 150 ms).
  - `grep -rl --include … --exclude-dir=node_modules` took 718 ms.
  - `rg -l export --type ts` took **10.9 s** (real rg: 75 ms).
  - A `while` loop with `i=$((i+1))` runs about 75k commands/s and hits `maxCommandCount=100000` after 1.3 s.

### Real external binaries

Not possible on purpose. The THREAT_MODEL says "No spawn()", and `AGENTS.npm.md` says "No binaries/WASM: Only built-in commands work".

The only escape hatch is host-supplied **trusted custom commands** (`defineCommand`). I confirmed that a custom `git` command calling `child_process.spawnSync` works and composes with pipes: `git log --oneline -3 | wc -l` → 3. The catches:

- That binary sees the real disk, not the virtual FS, so paths must be mapped by hand.
- Its output is buffered.
- It gets no TTY and no streaming stdin.

## 3. Security and sandbox model

`THREAT_MODEL.md` is 400 lines. The primary adversary is an untrusted script author. There is **no VM or process isolation**: it all runs in the host JS realm. The defenses are:

- Null-prototype objects and Maps everywhere, plus a prototype-pollution test suite.
- **Defense-in-depth box**, on by default (`src/security/defense-in-depth-box.ts`). It monkey-patches `Function`, `eval`, `setTimeout`, `process.*`, module resolution and more, scoped through AsyncLocalStorage to the untrusted async context, and uses reversible proxies on `Reflect`, `JSON` and `Math`. This is intrusive for an embedding host. The changelog mentions it breaking Next.js routes (#373).
- Parser limits: MAX_TOKENS 100k, parser depth 200, and `maxSourceBytes`.
- **Execution limits** (`src/limits.ts`, about 50 knobs, with `normal` and `hardened` profiles):
  - `maxCommandCount` (100k), `maxLoopIterations` (100k), `maxCallDepth` (100), `maxSubstitutionDepth` (50)
  - `maxStringLength` (64 MiB), `maxArrayElements`, `maxBraceExpansionResults`, `maxGlobOperations`
  - `maxOutputSize` (256 MiB normal, 32 MiB in the README example), `maxFileSystemBytes`, `maxLiveBytes`, `maxInputBytes`
  - Separate iteration and parser caps for awk, sed and jq
  - Archive, CSV and SQLite caps
  - `maxExecutionTimeMs`: a wall-clock deadline (1 h normal, 30 s in the hardened example)
  - Per-runtime timeouts for Python, JS and SQLite (30 s)
  - An exceeded limit gives exit code 126 and names the resource.
- Cancellation is cooperative: an `AbortSignal` stops execution at the next statement boundary.
- Fuzzing uses fast-check under `src/security/fuzzing/`. Attack and exploit regression fixtures live in `src/security/attacks/`.
- The `date` command defaults to UTC so the host timezone does not leak, and hostname and PIDs are virtualized.

## 4. Runtime targets, bundle size, dependencies

- **Node** builds: ESM `dist/bundle/index.js` (517 KB plus 2 MB of chunks) and CJS `index.cjs` (1.8 MB).
- **Browser**: `dist/bundle/browser.js` is **1.26 MB minified, 355 KB gzipped**, not counting the externals `diff`, `sprintf-js`, `turndown` and `guarded-fetch`.
  - The browser build has no python3, sqlite3, js-exec, OverlayFs or ReadWriteFs.
  - The core shell, InMemoryFs, MountableFs and all TS commands do work.
- **Edge:** it should work wherever the browser build works. There is no specific edge build.
- **WASM:** the shell itself is not WASM. Only the optional CPython and sql.js are.
- **Parser alone** (`parse` + `serialize`, bundled with esbuild for a neutral platform): **88 KB minified, 24 KB gzipped**. Its only dependencies are tiny internal modules (`source-limit`, `limits`, `interpreter/errors`).
- **npm package:** unpacked size about 21.6 MB, mostly the CPython vendor files.
- **Runtime dependencies:** `diff`, `fast-xml-parser`, `file-type`, `guarded-fetch`, `ini`, `modern-tar`, `papaparse`, `re2js`, `run` (QuickJS), `seek-bzip`, `smol-toml`, `sprintf-js`, `sql.js`, `turndown`, `typescript` (for js-exec TS stripping), `yaml`. Optional: `@mongodb-js/zstd`, `node-liblzma`.
- **CI** is ubuntu-latest only (`.github/workflows/*.yml`). **No Windows CI.** Windows path handling appears only in a few spots, e.g. `isPathWithinRoot` in `src/fs/real-fs-utils.ts`. Because the shell never touches the OS for commands, it is portable to Windows by construction, except for the real-FS adapters, which are untested there.

## 5. API for agents

The core API:

```ts
const bash = new Bash({ files, env, cwd, fs, network, python, javascript, customCommands, executionLimits });
const { stdout, stderr, exitCode, env } = await bash.exec(script, { env, cwd, stdin, args, signal, replaceEnv, rawScript });
```

Output comes back as complete strings only, with no streaming callbacks. `stdoutKind: "text" | "bytes"` marks binary output.

Other APIs:

- `defineCommand(name, async (args, ctx) => ExecResult, { trusted })` adds custom commands. `ctx` has `fs`, `cwd`, `env`, `stdin` (a ByteString), `exec` (for subcommands), `limits` and `executionScope`. Lazy-loaded commands are supported.
- The **`Sandbox`** class (`src/sandbox/`) mimics `@vercel/sandbox`: `runCommand`, `writeFiles`, `readFile`, `mkDir`, `stop`.
- **`bash-tool`** (a separate repo, vercel-labs/bash-tool) is the AI SDK integration. It returns `{bash, readFile, writeFile}` tools, where `bash` returns `{stdout, stderr, exitCode}`, and it can swap in a real `@vercel/sandbox`.
- The `javascript.invokeTool` hook plus `@just-bash/executor` exposes MCP, OpenAPI or GraphQL tools both as `tools.x.y()` inside js-exec and as bash commands.
- CLI: `just-bash -c '…' --root DIR [--allow-write] [--json]`. It uses OverlayFs, so writes are discarded.
- `AGENTS.npm.md` ships as `dist/AGENTS.md` with usage tips for models.
- **Adoption:** about 4.5k GitHub stars and 296 forks, and **about 3.6M npm downloads per week** (2026-10-29 to 11-04). Much of that is probably transitive via the AI SDK or Vercel tooling.

## 6. Parser and AST for command classification

- `parse(script)` (exported from the root) is synchronous and pure, and returns a `ScriptNode`. `serialize(ast)` goes back to bash, aiming for functional equivalence: `parse(serialize(parse(x))) == parse(x)`. `BashTransformPipeline` adds plugins. Built-in plugins: `CommandCollectorPlugin` (the list of command names, including those nested in `$( )`) and `TeePlugin`.
- **The AST is good semantically.**
  - Script, Statement (pipelines plus `&&`/`||`/`;`, `background`), Pipeline (`negated`, `timed`, `pipeStderr` for `|&`).
  - SimpleCommand (`assignments`, `name: WordNode`, `args`, `redirections`), compound nodes (If, For, CStyleFor, While, Until, Case, Subshell, Group, ArithmeticCommand, ConditionalCommand) and FunctionDef.
  - Words are parts lists: Literal, SingleQuoted, DoubleQuoted, Escaped, ParameterExpansion with a typed op (DefaultValue, PatternRemoval, Substring, Indirection and so on), CommandSubstitution (with a nested `body: ScriptNode`), ArithmeticExpansion, ProcessSubstitution, brace and tilde parts. Arithmetic has its own expression AST.
  - Here-docs and redirections are typed nodes.
  - So `git commit -m "fix $(rm -rf /)"` correctly shows a nested `rm -rf /`. That is exactly what a classifier for read-only vs mutating commands needs: walk every SimpleCommand, including those inside substitutions, and look at redirections for `>` writes.
- **Weaknesses for classification and highlighting:**
  - **No source spans.** Nodes carry only `line?: number`. A `Span` type is defined but never used, so you cannot map a node back to byte offsets for UI highlighting or precise rewriting.
  - Some inputs parse leniently and lossily: `echo $((1+` parses OK and serializes as `echo $((1))`.
  - Serializer bugs:
    - Nested backticks: `` echo `echo \`date\`` `` serializes as `` echo `echo `date`` ``, which is broken.
    - `[[ $y =~ ^a(b)$ ]]` serializes as `^a\(b\)$`, which changes the regex.
  - There is no "incomplete input" signal distinct from a syntax error, e.g. for continuation prompts. You get `LexerError` or `ParseException` messages to parse.
  - Bash-only grammar: `coproc` and `select` are rejected. There is no POSIX-sh or zsh mode.
  - Errors are thrown as exceptions, with no error recovery or partial AST.
- **Verdict:** it is usable standalone as a classification front-end, at 24 KB gzipped with zero external dependencies. But anything that needs offsets, such as UI highlighting or exact rewriting, would need span tracking added. Compare tree-sitter-bash or our own parser.

## 7. Tests, maturity, license

- **Test suite:** 649 test files. `pnpm test:run` reports **16,327 passed and 98 skipped** in about 13 s. About 12k `it(` calls. Separate configs cover comparison, wasm, fuzz and dist tests.
- **Oils spec tests** are imported from oils-for-unix/oils `spec/`, Apache-2.0, in December 2025: `src/spec-tests/bash/cases/`, 137 files and 2715 cases.
  - `spec.test.ts` skips 27 whole files: interactive, background, trap, kill, process, history, toysh, blog and others.
  - It uses inline `## SKIP` markers and reports a skipped test that unexpectedly passes as a failure.
  - My own count over **all** cases: **2255 pass, 283 skipped (marked N-I or SKIP), 177 fail**. The fails are all in the skipped files. That is about 83% of the full Oils bash corpus passing, and 100% of what is enabled.
- **Comparison tests** (`src/comparison-tests/`, 57 fixture files) replay outputs recorded from real bash, GNU tools and ripgrep.
- **Spec suites for tools:** grep, sed, awk and jq (`src/spec-tests/{grep,sed,awk,jq}`).
- **Activity:**
  - Initial commit 2025-12-23. About 470 commits. Monthly commit counts: 91 / 67 / 96 / 60 from Dec to Mar, then 12–37 per month through Oct 2026.
  - Versions run up to 3.6.0, releases use changesets, and there are 350 open issues.
  - Labelled "beta software" and a "Vercel Labs product".
  - Much of the code is clearly AI-assisted (the author field is "Malte and Claude"). The code and tests are verbose, with a heavy security-review flavor (deepsec, banned-pattern lint).
- **License:** Apache-2.0, for both the package and the imported Oils cases.

## 8. Assessment for Pi

**As-is for a browser or sandbox mode: yes, a good fit.**

- With the browser build (355 KB gzipped) and `InMemoryFs` or a custom `IFileSystem`, Pi in a browser could offer a "bash" tool that edits and greps a virtual workspace.
- It could also pull files lazily from a remote via an async FS adapter, for example one backed by pi-env.
- The limits and abort support are the right shape for an agent tool.
- `customCommands` lets Pi bridge selected host capabilities, such as `git` over an API or Pi's own tools.

**As a parser for command classification: plausible, with caveats.**

- The AST is semantically rich: nested substitutions, typed redirections, assignments.
- It is small and has no dependencies.
- But there are no source spans, a few serializer bugs, lenient parsing in places, and it is bash-only. For read-only vs mutating classification (a command-name allowlist plus redirection and substitution analysis) it would work today. For anything that rewrites or highlights commands we would need spans, which means patching the lexer and parser or forking.

**As a portable real shell (the Windows, remote-host and "consistent everywhere" goal): no.**

The design center is the opposite of what a coding agent needs:

- **No process execution.** A coding agent's main workload is `git`, `npm`/`pnpm`, `node`, `cargo`, `go`, `python`/`pytest`, `make`, compilers and linters. None of these can run except through hand-written custom-command shims that spawn real processes. Those shims then see a different FS than the shell, unless you use `ReadWriteFs` with a path mapping.
- **No streaming and no concurrency.** Pipelines are buffered stage by stage. `cmd &` runs in the foreground. There is no `trap`, so EXIT-trap cleanup idioms are ignored. A long build or test run gives no incremental output, and `tail -f` or watch-mode servers cannot work.
- **Numerics and regex differ from bash.** Bitwise arithmetic is 32-bit and there are no 64-bit integers. RE2 means no backrefs in grep or sed, `rg` defaults to smart-case, and some flags are missing (e.g. `xargs -I{}`).
- **Performance** is fine for small scripts but 10–150× slower than native for tree-wide search (rg: 11 s vs 75 ms on pi-mono/packages).
- **The defense-in-depth box** monkey-patches host globals during execution. That is risky inside a long-lived agent host process like Pi. It would need to be disabled (`defenseInDepth: false`) or run in a worker.
- **Windows:** the pure-TS core is OS-agnostic, which is good. But the real-FS adapters are not tested on Windows, and the shell still could not launch Windows executables.

**As inspiration: yes, strongly.**

- The Oils spec harness, with SKIP markers and unexpected-pass detection (`src/spec-tests/runner.ts`, `parser.ts`), is directly reusable to score any shell we build or embed.
- So are the recorded-fixture comparison approach, the limits taxonomy (`src/limits.ts`), the FS abstraction (`IFileSystem` with InMemory, Overlay, ReadWrite and Mountable) and the network allow-list with credential-injecting header transforms.
- The AST shape (`src/ast/types.ts`) is a reasonable reference for our own parser. We should add spans.

**Suggested direction:**

- If Pi ships its own bash for real-process portability, it should be a real shell that spawns processes, with pipes, signals and streaming. Candidates: a Rust or Go implementation such as brush or mvdan/sh distributed via pi-env, or busybox-w32 on Windows.
- Use just-bash only for (a) the browser or WASM-less "virtual workspace" mode and (b) possibly the classifier front-end or the test harness.
- A hybrid is also possible: just-bash as the interpreter, with a "spawn real binary" fallback through `customCommands` mapped onto a `ReadWriteFs` root. That gives read/grep/edit portability, but builds and tests would still run with buffered output and no job control. That is a weak substitute for a real shell.
