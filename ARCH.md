# psh: Architecture

Status: the design before any code (2026-11-06). It follows the agreed constraints in
`research/README.md` §8 and the execution-core study in `research/execution-core.md`.
Where the implementation goes another way, this file is updated and the decision cited
(`Dnnn` in `docs/decisions.md`). Open questions are marked **Open** and collected at the
end.

psh is a bash-compatible shell for running the commands coding agents write. Its
interface is "command string in; stdout, stderr and exit status out", with the semantics
of GNU bash 5.3 run as `bash -c` (D002, D003). It runs on macOS, Linux and Windows
without Git Bash (D011), embeds into a host process with no async runtime (D005), and
compiles to WASM.

## Goals

The guide's `docs/guide/goals.md` is the human-facing statement of these. In short:

- **Agent-first.** Optimize for the narrow syntax agents use (lists, pipes, `cd X &&`,
  `2>&1`, quoted here-docs, `$(…)`, `for`) and their utility habits (GNU and BSD flags
  mixed), not for interactive use.
- **Portable.** The same binary semantics on macOS, Linux and Windows; Windows is not a
  port done at the end.
- **Bash's behavior.** What bash 5.3 prints and returns is the target, including error
  messages. Deviations are recorded decisions.
- **Consistent.** One shell and one utility dialect everywhere: locally, over pi-env, in
  the browser.
- **Understandable commands.** The parser is a product: hosts classify, summarize and
  approve commands with the exact parser that runs them.
- **Control of execution.** Cancel, timeouts and process-tree kill with no orphans; the
  embedder owns the thread.
- **Embeddable.** No async runtime, near-zero dependencies, a size budget.
- **Sandboxable.** Every side effect goes through a `Host`, so the same engine runs over
  a virtual file system in WASM.

Ranking when goals conflict (D033): agent-first, then portable, then bash's behavior,
then the rest, unranked among themselves. Under all of them: permissive licensing
(D007), MIT OR Apache-2.0 (D032).

## Crates

```
crates/
  psh-syntax/    lexer, parser, AST with byte spans, printer      no_std + alloc, zero deps
  psh-classify/  AST -> effects and summaries                     no_std + alloc, psh-syntax only
  psh-engine/    the stepped interpreter; Host trait; vprocs      std, psh-syntax only, wasm32
  psh-os/        OS hosts: unix and windows                       libc, windows-sys
  psh-utils/     in-process utilities (grep, sed, head, jq, ...)  ripgrep crates (D020)
  psh/           the `psh` binary: -c, scripts, --parse, --classify
  psh-spec/      the conformance runner (TESTING.md)
  psh-wasm/      later: the WASM build and its JS host
```

Dependency direction: `psh-syntax` ← `psh-classify`, `psh-engine` ← `psh-os`,
`psh-utils` ← `psh`, `psh-spec`, `psh-wasm`. pi-env depends on `psh-engine`, `psh-os`
and `psh-utils` (and `psh-classify` for a parse op); it lives in pi-mono, not here.

**std or not (D023).** `psh-syntax` (and `psh-classify`) are `#![no_std]` + `alloc`, so
the parser builds small and clean for wasm. `psh-engine` uses std; its
`wasm32-unknown-unknown` build in the gate guards against OS access. The engine must
never force std into the parser: no std-only types in the parser's API, no parser
feature that turns std on.

**Toolchain (D025).** MSRV is 1.96, pi-env's pinned toolchain; development uses current
stable. A gate leg checking 1.96 is a follow-up; until then, no features newer than 1.96.

**JSON (D029).** All JSON in psh (`psh --parse`, `psh --classify`, `rg --json`, any
later JSON input or output) goes through the `deser-json` crate (deser 0.10.x). The
parser crate does not depend on it; the AST's JSON form is produced outside
`psh-syntax` unless deser-json works under no_std.

## psh-syntax: the parser

- Copied from brush-parser (MIT, attributed) as a starting point, with peg and winnow
  replaced by a hand-written lexer and recursive-descent parser (D008). brush's other
  dependencies (bon, cached, miette, thiserror, tracing, utf8-chars) go too.
- **Byte spans on every node** (`u32` start and end into the source). Line and column
  are computed on demand from a line table. The classifier, the UI and error messages
  all point into the original string.
- **Arenas** (D1040): an `Ast` owns the source and holds lists, commands and here-docs
  in arenas addressed by ids; the engine keeps `(Arc<Ast>, id)` pairs. Literal word
  parts are spans, so parsing copies no text.
- What `bash -n` does not check is deferred, as in bash (D1041): backquote bodies,
  arithmetic (kept as text with expansions), invalid `${…}` ("bad substitution") and
  errors in unquoted here-doc bodies. `$(…)`, `<(…)` and `${ …; }` are parsed in place.
- Parses bash 5.3 syntax only: no zsh, no POSIX-sh mode. Aliases are not expanded at parse
  time (non-interactive bash leaves `expand_aliases` off); if `shopt -s expand_aliases`
  is used, the engine re-parses after alias definitions, as bash does line by line.
- Here-docs: bodies are attached to their redirect node with the span of the body and
  the quoting of the delimiter.
- **A printer** turns an AST back into source. `parse(print(ast)) == ast` is tested on
  every case (TESTING.md L0).
- Errors: `bash -n` agreement on whether a script parses, and bash's message text where
  a golden shows it.
- Parses incrementally, as bash does: the engine parses and runs one complete top-level
  line at a time, so `echo a` on line 1 runs before a syntax error on line 2 is
  reported (status 2). `--parse` and the classifier parse the whole input.
  `Parser::next_chunk` yields those lines; the parse options (`extglob`) may change
  between chunks, as `shopt` does in bash (D1043).

## psh-engine: the interpreter

A synchronous, sans-IO state machine (D005). The shape follows oh-my-pi's engine at the
level of architecture only (clean room, D007); the code is ours or brush's.

- `Shell::step(budget) -> Step` runs ready work and returns one of: `Done(status)`,
  `Blocked` (waiting on the host; it will wake the waker), `Yield` (budget spent), or
  `Idle`. Steps are bounded, so the embedder keeps control of its thread.
- **Virtual processes.** Subshells, pipeline stages (including compound commands and
  functions), `$(…)`, `<(…)` and `&` jobs are vprocs inside the engine, scheduled
  cooperatively. Nothing forks; this is also what makes Windows and WASM possible.
  Pipeline stages run concurrently, never one after another (pipe buffers would
  deadlock).
- **Control.** A `ControlHandle` (an atomic control word: cancel, pause, signal) is
  checked at every safepoint (each command and loop iteration at least). Cancel delivers
  a virtual TERM to the vprocs (traps run), then KILL after a grace period. A late
  cancel never leaks into the next run.
- **Wakeups.** `std::task::Waker` only. Every pending thing the engine polls (an
  external process, a host fs op, a pipe, a timer) is polled with a `Context` built from
  the engine's waker. No runtime, no threads inside the engine.
- **Pipes between vprocs** are in-memory engine objects. Pipes between two externals are
  OS pipes handed to both, with no pumping. Only engine↔external pipes need pumps.
- **Here-docs and here-strings** are engine fd objects fed through the same pollable
  pipe machinery; large or seekable bodies go to a host temp file. A body is never
  written synchronously before its consumer starts.
- **Shell builtins** (`cd`, `export`, `set`, `trap`, `read`, `printf`, `test`, `kill`,
  `wait`, ...) are part of the engine. Job control is limited to `&`, `wait`, `kill`,
  `$!`, `jobs` (D002).
- **Expansion** (parameters, arithmetic as 64-bit, globbing, brace expansion, quote
  removal, word splitting) is engine code over the AST; globbing reads directories
  through the host.

### The Host trait

Every side effect goes through one trait, with pollable operations so that any of them
may be `Pending`:

- `spawn(spec) -> ExtProc` where spec carries argv, cwd, env, the fd table (engine pipe
  ends, OS fds, files, null) and placement (own process group / job). `ExtProc` has
  `poll_wait(cx) -> Poll<ExitStatus>`, `signal(sig)`, and drop = abandon (TERM, grace,
  KILL, background reap).
- File system: open, stat, read_dir, mkdir, remove, rename, symlink, ... each may return
  `Pending`. Open files expose `poll_read` / `poll_write`.
- Timers (`sleep`, `timeout`, `read -t`), clock, temp files, environment of the host
  process, user and host names.
- `resolve(name)` for command lookup (PATH, PATHEXT on Windows) and a hook where an
  embedder can veto or rewrite a command at execution time (approval at exec time).

Hosts:

| Host | Where | Notes |
| --- | --- | --- |
| `psh-os::unix` | macOS, Linux, Android | Below |
| `psh-os::windows` | Windows | Below |
| test host | `psh-engine` tests | In-memory fs and fake processes; a latency mode makes every op `Pending` to prove no engine path expects synchronous answers |
| JS host | `psh-wasm` | Virtual fs, host-provided commands, driven from the JS event loop |

## psh-os: the OS hosts

From `research/execution-core.md` §5; the pitfalls listed there are requirements.

**Unix.**
- Spawn: `posix_spawn` with its own session or process group per external, `SETSIGDEF`
  for all non-ignored signals (SIGPIPE back to default), an empty signal mask,
  `CLOEXEC_DEFAULT` on macOS, `addchdir_np` where available; `fork`+`exec` fallback for
  umask, ignored signals and child-side FIFO opens.
- Wait: a pooled waiter thread per running external in `waitid(..., WNOWAIT)`, which
  wakes the engine. No SIGCHLD handler. Exited group leaders stay unreaped while their
  group may still be signalled (no pid reuse). pidfd parking on Linux is a later
  optimization.
- Pumps (engine ↔ external): v1 is a bounded helper thread per pump doing blocking
  `read`/`write` into an engine pipe object. Later a single readiness thread (`poll`;
  `select` on macOS, where `poll` misses FIFO writer close). Never set `O_NONBLOCK` on
  inherited or shared fds.
- Kill: TERM to the group, KILL after a grace period, background reap.
- Signals: embedded, install no handlers at all. Standalone `psh`, handlers only set
  atomic bits and wake the driver; SIGPIPE is reset to default for children.

**Windows.**
- Spawn: `CreateProcessW` suspended, assigned to a Job Object per job (no breakaway),
  then resumed; only the three stdio handles inherited (`PROC_THREAD_ATTRIBUTE_HANDLE_LIST`);
  `CREATE_NEW_PROCESS_GROUP`; no console window flashes.
- argv: MSVCRT quoting rules; `.bat`/`.cmd` through `cmd.exe` with the hardened quoting
  Rust std uses (BatBadBut), refusing what cannot be quoted safely.
- Wait: a waiter thread per external on `WaitForMultipleObjects(process, wake)`.
- Kill: TERM/INT = `CTRL_BREAK_EVENT` to the group, then `TerminateJobObject`; KILL =
  `TerminateJobObject`. The whole tree dies, including grandchildren.
- Pumps: v1 helper threads; later overlapped named pipes (anonymous pipes cannot do
  overlapped I/O).

**Windows rules** (from `research/landscape.md` §3):
- Paths: forward slashes internally; accept `C:\x`, `C:/x` and `/c/x` in builtins and
  `cd`; convert to native form only at the OS boundary. No MSYS-style rewriting of argv
  to external programs. `\\?\` for long paths.
- `/dev/null` maps to `NUL`; `/dev/stdin|stdout|stderr` are emulated in redirections.
- Command lookup: PATH (`Path`, `;`-separated) with PATHEXT, case-insensitive; cwd is
  not searched for bare names. `test -x` and `command -v` decide by extension.
- Shebangs: `./script` with `#!` is run by the named interpreter (`/usr/bin/env X`
  resolves X on PATH); `#!/bin/sh` and `#!/bin/bash` run in psh itself.
- Environment names are case-insensitive for the OS environment, case-sensitive for
  shell variables.
- Output of native programs in the ANSI/OEM code page is decoded per program, with
  `encoding_rs` (in the tree anyway through `grep-searcher`, D020). **Open:** where
  decoding lives (engine output layer or host).
- CRLF: scripts with `\r\n` parse; `$(…)` and `read` strip a trailing `\r` from native
  output.
- Utilities whose names collide with System32 programs (`find`, `sort`, `timeout`,
  `tree`, `more`, `hostname`, `whoami`) are always the builtin on Windows.
- `ln -s`: symlink, else a junction for directories, else a clear error; never a silent
  copy.

Platform bindings: `psh-os` uses `libc` and `windows-sys`, as pi-env does (D024).

**Windows CI (D027).** The gate's Windows leg runs on the owner's Parallels VM via
`prlctl exec` until the repository has a remote, then on GitHub Actions.

## psh-utils: in-process utilities

"Builtin utilities" (or just utilities) are in-process implementations of external
programs (`grep`, `sed`, `head`, `jq`, `timeout`, ...), distinct from the shell builtins
in the engine. The list, its tiers and the policies below come from
`research/builtins.md` §4 (D010, D014–D022, D028, D034).

Why have them at all, in order of weight: **portability** (Windows has none of these
tools and WASM has no processes, so the utility set *is* the toolset there),
**consistency** (one flag dialect everywhere), and **no fork** (faster, works over a
virtual fs, cancels cooperatively, and the classifier knows their effects). Their cost
is size, maintenance, and above all divergence where a builtin shadows a better real
program, which the policies below exist to contain.

### Interface

A utility is a synchronous `run(&mut UtilCtx) -> i32` over a small context:
stdin/stdout/stderr streams, cwd, env, the host fs, a cancel flag (which also makes a
blocked stdin read return EOF), and a command runner for the wrappers (`xargs`,
`find -exec`, `env`, `timeout`, `nice`, `nohup`). The engine runs utilities on worker
threads (inline in WASM) and polls them like externals; a utility never detaches a
thread that still writes to its fds. Ported code (uutils calls `std::io::stdout()` and
`std::fs` directly) is refactored onto this context, as oh-my-pi did; re-executing the
psh binary per utility, as brush does, is not an option because it fails in WASM and
breaks cancellation (D018).

### Shadowing modes (D015)

Each utility has one mode:

| Mode | Meaning | Used for |
| --- | --- | --- |
| **S, always** | Always the builtin | Where psh can be at least as good as GNU and BSD for agent use |
| **F, PATH fallback** | The builtin only if no program of that name is on PATH, or in WASM/sandbox mode | Tools whose real versions are better, or whose exact output is the point (`gzip -9c \| wc -c`, `tar`, `file`) |
| **W, Windows-only shadow** | S on Windows, F elsewhere | Names that collide with incompatible System32 programs: `find`, `sort`, `timeout`, `tree`, `expand`, `more`, `fc`, `hostname`, `whoami`. On Windows the builtin must win, or `find . -name x` runs the wrong program |

Around the modes:

- An explicit path (`/usr/bin/grep`) always runs the real program.
- **Fall-through** (D010): a utility that meets a flag or form it does not implement
  runs the real program of that name from PATH if there is one, or fails with a clear
  message naming the unsupported flag. It never silently produces different output.
- **Off switches** per session: all utilities, the destructive group (`rm mv cp ln dd`),
  or single names.
- `command -v`, `type` and `which` report utilities truthfully (`jq: shell builtin`).
- `env NAME`, `xargs NAME`, `find -exec NAME` and `timeout NAME` dispatch back into the
  shell, so utilities (and `printf`, `echo`, `test`, `true`, `false`, `pwd`, `kill`,
  which need standalone forms) stay reachable from them.

### Dialect (D016)

- Accept the **union of GNU and BSD** flags where they don't conflict.
- Where they conflict, **GNU semantics**, except where agents overwhelmingly write the
  macOS form (`sed -i ''`: 5,307 BSD vs 144 GNU in the corpus; `stat -f FORMAT`). Those
  are resolved by deterministic rules on the arguments, not by the host OS:

| Command | Rule |
| --- | --- |
| `sed -i` | `''` next, or text attached (`-i.bak`): a suffix. Otherwise, if the next argument parses as a script and operands follow: GNU form. A bare `.`-prefixed argument without command characters followed by a script: BSD suffix |
| `stat -f` | Argument containing `%`: BSD format. `-f` alone with file operands: GNU file-system mode. `-c` and BSD `-f`/`-t` both offered |
| `date -r` | All digits and not an existing file: BSD epoch seconds; otherwise GNU reference file |
| `base64 -i` | Followed by a non-option operand: BSD input file; otherwise GNU ignore-garbage |
| `mktemp -t` | Argument containing `XXX`: GNU template; otherwise BSD prefix |
| `cp -R src/` | GNU (copies the directory), documented |
| `wc`, `uniq -c` | GNU column widths |
| `find` with no path | `.` (GNU) |
| `head -n -N`, `tail -r` | Single-dialect forms, both accepted |
| `md5`/`shasum` vs `md5sum`/`sha256sum` | Output format keyed on the name; never unified |

  Decided in their utility issues (D034), falling through until then: `ls -G`/`-T`
  (BSD color / full time vs GNU no-group / tab size; research suggests the macOS
  meaning), `xargs` on empty input (research recommends BSD: don't run, `-r` accepted as
  a no-op), and `sort`'s default collation (byte order vs locale).
- `uname`, `hostname`, `id` and friends report the real host. psh never pretends to be
  Linux.

### Tiers (D017)

- **Tier 0**, shell builtins: part of the engine (`cd`, `export`, `read`, `mapfile`,
  `printf`, `test`, `kill`, `wait`, `trap`, ...), not up for debate.
- **Tier 1**, day one, 62 names: the top corpus names that are not excluded, everything
  the agent read/edit/search loop needs, the wrappers (which must be builtins so they
  can wrap builtins and own process trees), and `fd`, which the owner moved up (D019):
  `head tail cat ls cp mv rm mkdir rmdir touch ln chmod pwd basename dirname realpath
  readlink mktemp stat du grep egrep fgrep rg fd sed awk sort uniq wc cut tr paste nl tee
  seq comm printf echo test true false sleep find xargs diff cmp jq base64 md5sum
  sha1sum sha256sum sha512sum shasum md5 date env printenv timeout nice nohup which`.
  `timeout` comes first: it is the most common "command not found" (389) in the corpus.
  `find` is W; `awk` is F until conformant; the rest are S.
- **Tier 2**, soon after (~45 names): process tools (`ps pgrep pkill`, per OS), host
  info (`uptime nproc id uname whoami hostname df`), `tree column tac rev strings od
  xxd hexdump expr yes truncate fold expand unexpand join split`, the rest of the
  checksum family, `shuf patch`, archives in F mode (`tar gzip gunzip zcat unzip`),
  `sponge ts ifne`, `dos2unix unix2dos iconv`, `install link unlink sync mkfifo logname
  tty`.
- **Tier 3**, on demand, mostly F: `file bc dc yq` (decompressors as tar codecs),
  `zip dd numfmt fmt pr csplit tsort factor uuidgen getopt envsubst chown chgrp clear
  pidwait pidof free`; `ast-grep` only as an optional feature.

### Sources (D018)

Port mature permissive code rather than write new, above all for the four big ones:

- `rg`, `fd`, `grep` and `find` on the ripgrep crates (next section), `sed` from uutils sed, `jq` on jaq, and **`awk`**,
  which has no mature permissive Rust implementation: port goawk (MIT, passes the
  onetrueawk tests), with onetrueawk (the macOS awk) and its `T.*` tests as reference.
  awk runs in F mode until it passes the onetrueawk and goawk suites.
- Most of the rest (~45 of Tier 1) from uutils coreutils, findutils and diffutils.
- Ours: the wrappers (`timeout nice nohup env xargs`, on psh's process model), `which`,
  `xxd` (written from the man page; xxd itself is under vim's license), `tree` (GNU tree
  is GPL), `column`, `strings`.
- GPL implementations (GNU coreutils, sed, grep, gawk, tree, patch, busybox, moreutils)
  are never read (D007); BSD implementations are fine as references.

**Crates (D028).** `jq` depends on jaq (copied instead only if its issue shows size
demands it). Crates for the date parser (`date -d`, `touch -d`, `find -newermt`,
`fd --changed-*`), tar/zip and process listing (`sysinfo`) are decided, and recorded, in
the utility issue that needs them. The search crates, including `encoding_rs`, are
D020/D021; there is no PCRE backend, so `grep -P` accepts what the regex crate parses
and falls through otherwise.

### Search: rg, fd, grep and find on the ripgrep crates (D020, D021)

`rg` and `fd` are embedded by owner requirement, on the upstream crates, the one
standing dependency exception (D006). Neither ripgrep nor fd has a library entry point,
and fd's `main` is unusable in-process (it changes the process cwd, installs a Ctrl-C
handler, needs threads, pulls clap and jiff). Source: `research/builtins.md` §5. All of
this is D020, confirmed by the owner (D021).

**Crates.** Exactly: `grep-matcher`, `grep-regex`, `grep-searcher`, `grep-printer`
(`default-features = false`, so no serde), `ignore`, `globset`, `regex` (plus
`regex-automata`/`regex-syntax` directly where needed), and what they pull in: about 27
crates on macOS, 30 on Windows, 25 on wasm32. Not `grep-cli` (process-centric helpers;
psh owns TTY, stdin and decompression) and not `grep-pcre2` (C code, breaks the WASM
build). `encoding_rs` arrives with `grep-searcher`. Marginal size of rg+fd+grep+find:
about 0.5–0.75 MB on top of the regex stack (1.2–1.4 MB), which `sed`, `[[ =~ ]]` and
`find -regex` need anyway. Vendoring both CLIs verbatim would cost 3.7–4.7 MB.

**rg: copy ripgrep's flag layer ("B+").** `crates/core/flags/*` (defs, lowargs, hiargs,
parse) and the glue in `search.rs`/`main.rs` are copied into `psh-utils` with
attribution (MIT OR Unlicense), so the whole flag surface and its `--help` text stay a
mechanical diff from upstream. The seams are patched: per-invocation message/error state
instead of statics, cwd/env/stdio/TTY from `UtilCtx` (also for `RIPGREP_CONFIG_PATH`),
`--pre` and `-z` through psh's command runner, no `process::exit`, no global logger.
`lexopt` is copied (about 1k lines, no dependencies). Search and printing use
`grep-searcher` and `grep-printer`; `rg --json` writes its JSON through `deser-json`
(D029), since `grep-printer`'s JSON printer needs serde. Broken pipe ends the search with
status 0, so `rg … | head` stops early.

**fd: rewrite the CLI, copy the logic.** fd's command line is re-implemented on psh's
own argument parser; its filters (`filter/size.rs`, `time.rs`, `owner.rs`), `exec/`
(`-x`/`-X` templates, run through the command runner) and `fmt/` (`{}`, `{/}`, `{//}`,
`{.}`, `{/.}`) are copied with attribution (MIT/Apache). clap, jiff, lscolors, ctrlc,
nix, etcetera, normpath, faccess, argmax and crossbeam-channel are not used.
`--changed-within/-before` use psh's shared date parser. `--base-directory` never
changes the process cwd. fd's tests and documented flags are the conformance target.

**One walker seam, two implementations.**
- *Native:* `ignore::WalkBuilder`/`WalkParallel` with absolute roots (parent-directory
  ignore discovery canonicalizes against the *process* cwd), `current_dir(cwd)`, display
  paths mapped back to the user's spelling, global gitignore located from the shell's
  `HOME`/`XDG_CONFIG_HOME`/`GIT_CONFIG_*`, not the process's, and a cancel check in each
  visitor. The parallel walker uses scoped, always-joined threads, which fits the
  utility thread rule.
- *Virtual fs (WASM, sandboxes):* psh's own single-threaded walker over the `UtilCtx`
  fs. It reuses `ignore`'s in-memory matchers (`gitignore`, `overrides`, `types`) and
  re-implements only the ignore chain (`.ignore` > `.gitignore` > `.git/info/exclude` >
  global, parent directories), about 600–800 lines copied from `ignore/src/dir.rs` with
  attribution.
- A **parity test** runs both walkers natively over the same fixture trees with the same
  flags and requires identical results.

**Sorted output.** When rg or fd search with more than one thread, output is
path-sorted by default (real rg's order is arbitrary, so sorted output is a valid rg
output). This keeps goldens and replay stable. ripgrep's `--sort path` forces a single
thread, so psh merges results in order instead; the cost is measured in 0013.

**grep shares the engine, not the rg front end.** Shared: the `grep-regex` matcher,
`grep-searcher` (context, binary detection, `-z`, encodings), the walker, and the
0/1/2 exit rules (`-q` beats errors). GNU-specific: a POSIX BRE/ERE → `regex-syntax`
translator (`\( \) \{ \}`, GNU `\+ \? \|`, leading `*` literal, `[[:class:]]`, bracket
rules; oh-my-pi's `bre.rs` is an MIT reference); GNU output through psh's own `Sink`
(`file:line:text`, `file-line-text` context, `--` separators, `Binary file X matches`,
the `-c -l -L -H/-h` defaults), since `grep-printer`'s standard printer is rg-shaped;
GNU recursion (`-r` without a path means `.`, hidden files searched, `.gitignore`
ignored, `-R` follows symlinks, `--include/--exclude/--exclude-dir` as basename fnmatch
via `globset` with `literal_separator(false)`).

**find** uses the same walker with all ignore filtering off; `-name`/`-path` go through
`globset` in fnmatch mode, `-regex` through the translator.

**Regex gaps** (shared by grep and sed):
- *Backreferences* (`\(a\)\1`): the regex crate lacks them. Plan: a small backtracking
  matcher of psh's own implementing `grep_matcher::Matcher`, used only when the
  translator sees a backreference. Until it exists, such patterns fall through to the
  host program (D010), or fail clearly where there is none (Windows, WASM).
- *Leftmost-longest:* Rust regex is leftmost-first, POSIX is leftmost-longest. Line
  selection is the same, but `grep -o`, `--color`, `-b` and `sed s///` can pick a
  different span (`echo abcd | grep -o 'ab\|abcd'`: POSIX `abcd`, Rust `ab`). Plan:
  `regex-automata` with `MatchKind::All` and a forward search to the last match end for
  those modes (needs a spike). Until then, in those modes a conservative check (for
  example, any alternation in the pattern) makes the utility fall through; the spike
  settles the exact rule.
- *PCRE-only syntax* (`grep -P` lookaround): no PCRE; what `regex-syntax` parses is
  accepted, the rest falls through.
- *Locale:* UTF-8 Unicode semantics for classes and ranges, documented; this matches GNU
  in a UTF-8 locale.

**Windows.** Default `/` path separators for rg and fd (as `--path-separator /`; an
explicit flag overrides). `psh.exe` carries a `longPathAware` manifest like ripgrep's.
Color is ANSI or none, from the `UtilCtx` TTY state, never the console API. Gitignore
matching stays case-sensitive (ripgrep's default). `grep` and `rg` treat CRLF as a line
end by default on Windows (ripgrep's `--crlf` behavior; D030); explicit flags override.

**WASM.** The `grep-*` crates, `globset`, `regex` and the `ignore` matchers build for
`wasm32-unknown-unknown` and work over `UtilCtx` readers; only the `ignore` walker does
not (every fs call is `Unsupported`), which is what the virtual-fs walker replaces.

**In pi-env** the utilities (and with them the regex stack) are a cargo feature of the
daemon build, on by default (D031).

### Never builtins and out of scope (D014, D022)

- **Never builtins:** database clients and stateful engines (`sqlite3`, `psql`, `mysql`,
  `redis-cli`, `postgres`), interpreters and language runtimes, toolchains and build
  systems, package and environment managers.
- **Out of scope for now:** `git`, `curl`/`wget`, and ssh/rsync-like network and VCS
  clients (`ssh scp sftp rsync nc gh aws gcloud kubectl docker podman` and the like).
  They are not utilities and nobody works on them, but they are not "never": psh may
  later hook them, as host-provided commands or by interception.
- **Not utilities, no work planned** (proposals from `research/builtins.md` §4.5, not
  owner rules): interactive/TTY programs (`less vim top watch tmux fzf`), privileged and
  OS-specific system tools (`sudo launchctl lsof pbcopy`), heavy domain tools (`ffmpeg
  pandoc 7z`), compressors in compress mode when a real one exists, benchmarking tools,
  `shred`, `mkfs`.
- An embedder may supply any such command through the host (in WASM, like a
  fetch-backed `curl`); those are host commands, not psh utilities.

## The `psh` binary

`psh -c CMD [ARGS]`, `psh SCRIPT [ARGS]`, script on stdin. Diagnostics:
`psh --parse` (AST as JSON with spans), `psh --classify`, `psh -n`. Standalone, it drives
the engine with a thread parker: step, and on `Blocked` park until the waker fires. It
installs minimal signal handlers (atomic bits) and resets SIGPIPE for children.

## Embedding into pi-env

pi-env (pi-mono, `packages/env/daemon`) is a synchronous, threaded Rust daemon that
runs every `exec` as `bash -c` today (`research/pi-current.md`). psh plugs in at its
`exec` op:

- Advertised as a feature in `hello.features`; `exec { command, shell: "psh" }`
  selects it. `shellPath` still selects real bash.
- One thread per exec runs the driver loop: `step`; on `Blocked`, park on a
  `Mutex`+`Condvar` woken by the engine's waker or by the request's cancel. Cancel and
  timeout go through the `ControlHandle`; TERM, grace, KILL.
- Embedded, psh installs no signal handlers; every external leads its own process
  group or job, so a cancel reaches the whole tree.
- Output stays combined stdout+stderr in arrival order, with exit status, and goes
  through the daemon's existing decoding, windowing and spill. Exit codes follow
  pi-env's contract (`128+n` for signals).
- A `parse` op returns the AST summary and classification for UI and permissions, even
  where execution still uses real bash.

**Sessions (D026).** Each `exec` gets a fresh psh shell; nothing (cwd, variables,
functions) persists across calls for now, which keeps Durable's replay model intact.
Node-side execution (`NodeExecutionEnv` and friends) uses psh through the daemon, not a
separate binary or addon, so local p2 execution moves from Node spawning to the daemon.
The utilities are a daemon cargo feature, on by default (D031).

## WASM and the browser

- `psh-syntax` and `psh-classify` build for `wasm32-unknown-unknown` with no imports; a
  host can ship the parser alone (brush's parser was about 146 KB gzipped; ours should be
  smaller without peg).
- The whole shell builds for `wasm32-unknown-unknown` with a JS `Host`: a virtual fs
  (memory, OPFS, or a remote pi-env), commands supplied by the host (tools) instead of
  processes, utilities in-process. The JS side drives `step` from its event loop; a
  pending fs op or command resolves through the waker. Because the engine is sans-IO,
  no Asyncify or JSPI is needed.
- In the browser there is no git, python or node, so this mode is for exploration and
  editing unless the host supplies such commands.

**Open:** whether the browser mode matters before pi-env integration, and whether a WASI
build is wanted at all.

## psh-classify: understanding commands

The classifier runs on the same AST the engine executes, so a command is never parsed one
way for approval and another way for execution.

- For each simple command: the command name (literal or dynamic), argv words with spans
  (literal or which expansions they contain), redirections with targets, and nested
  commands (`$(…)`, `<(…)`, `eval`/`bash -c` strings where literal, `xargs`/`find -exec`
  /`env`/`timeout` wrapped commands).
- Categories from the corpus: read, mutate, VCS, build, network, install, long-running,
  unknown; plus effects: paths read and written, whether output is truncated (`| head`).
- **Conservative.** Anything dynamic (command name from a variable, `eval` of a
  non-literal, `source`, functions or aliases defined earlier, unknown programs, inline
  interpreters) is `unknown`, never "read-only". Redirections to files, `sed -i`, `tee`
  are writes.
- Per-utility knowledge (which flags write, which options take a value, what the
  operands are) lives in `psh_classify::utils` (D1090): no_std data that the utilities
  depend on and check their own option parsing against in their tests.

## Open questions

1. Where Windows output decoding lives (engine output layer or host).
2. *(Decided, D1090: in `psh_classify::utils`.)*
3. Whether the WASM host is needed before pi-env integration, and whether a WASI build
   is wanted at all.

Decided and recorded: std vs no_std (D023), platform bindings (D024), MSRV (D025),
pi-env sessions (D026), Windows CI (D027), utility crates (D021, D028), JSON (D029),
CRLF on Windows (D030), pi-env feature (D031), license (D032), goal ranking (D033),
never-builtins and out-of-scope clients (D022), dialect points (D034).
