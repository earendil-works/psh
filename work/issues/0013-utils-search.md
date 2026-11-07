# 0013: Utilities batch B: rg, fd, grep on the ripgrep crates

status: in-progress
agent: infra
branch: issue/0013-utils-search
depends-on: 0008 (crate set confirmed: D020, D021)
touches: Cargo.toml (the D020 crates), crates/psh-utils/src/search/ (shared walker seam, matcher, BRE/ERE translator), crates/psh-utils/src/{rg,fd,grep}/, tests/utils/{rg,fd,grep}/, tests/utils/walker/, NOTICE
guide: utilities.md (not written yet), utilities reference (not written yet)
decisions: D1130–D1139

## Goal
The search utilities on the ripgrep crates, exactly as ARCH.md "Search: rg, fd, grep and
find on the ripgrep crates" and D020 describe (`research/builtins.md` §5). `grep` (71k
corpus commands) and `rg` (58k) are the second and fifth most used names; `fd` is Tier 1
by owner request (D019). This issue also builds the shared search layer (walker seam,
matcher, BRE/ERE translator) that `find` (0017) and `sed` (0014) reuse.

## Acceptance criteria
Shared layer
- [ ] Only the D020 crate set, with `grep-printer` `default-features = false`; no
      `grep-cli`, no `grep-pcre2`. `cargo tree --target all` output in the Result.
- [ ] Walker seam with two implementations: native (`ignore::WalkBuilder` /
      `WalkParallel`, absolute roots, `current_dir(cwd)`, display paths in the user's
      spelling, global gitignore located from the shell's `HOME`/`XDG_CONFIG_HOME`/
      `GIT_CONFIG_*`, cancel check per visitor) and virtual-fs (single-threaded over the
      `UtilCtx` fs, `ignore`'s `gitignore`/`overrides`/`types` matchers, ignore chain
      copied from `ignore/src/dir.rs` with attribution).
- [ ] Parity test: both walkers, natively, over fixture trees covering nested
      `.gitignore`/`.ignore`/`.git/info/exclude`, negations, parent-directory rules,
      hidden files, symlinks, `-g`/`-t`/`--hidden`/`--no-ignore`; identical results.
- [ ] Ordered merge: multi-threaded searches print path-sorted by default; cost
      measured against single-threaded and against real rg on a large tree (the psh
      checkout plus `vendor/`), in the Result.
- [ ] POSIX BRE/ERE → `regex-syntax` translator (`\( \) \{ \}`, GNU `\+ \? \|`, leading
      `*` literal, `[[:class:]]`, bracket rules), unit tested, usable by `sed`.
- [ ] Regex gaps fall through (D010, D020): backreferences; leftmost-longest-sensitive
      `-o`/`--color`/`-b` (conservative check, e.g. any alternation); PCRE-only `-P`
      syntax. Each tested, including the clear error when no host program exists.

rg (S)
- [ ] ripgrep's `crates/core/flags/*` and the `search.rs`/`main.rs` glue copied at
      `91f5d22` with attribution; `lexopt` copied. Seams patched: no statics shared
      between invocations (two concurrent `rg` runs in one process are tested), cwd/env/
      stdio/TTY and `RIPGREP_CONFIG_PATH` from `UtilCtx`, `--pre`/`-z` through the command
      runner, no `process::exit`, no global logger.
- [ ] Matches real rg: no smart-case by default, `--no-heading` and line numbers only
      on a TTY, `-g` brace globs, `.gitignore` semantics including parent directories;
      corpus flags `-n --glob/-g -i -S -C -l --type -o --hidden` covered by cases;
      `--help` text identical to upstream's except for the version line.
- [ ] Broken pipe ends with status 0, so `rg … | head -1` returns at once on a big tree.
- [ ] `rg --json` writes JSON through `deser-json` (D029), matching real rg's JSON lines
      (grep-printer's JSON printer needs serde, so it is not used).

fd (S)
- [ ] CLI re-implemented on psh's argument parser; `filter/size.rs`, `time.rs`,
      `owner.rs`, `exec/` and `fmt/` copied from fd `193d0df` with attribution; none of
      clap, jiff, lscolors, ctrlc, nix, etcetera, normpath, faccess, argmax,
      crossbeam-channel.
- [ ] Flags: `-e -t -H -I -g -x/-X -E -d -s -p -L -0 -l -S -o --changed-within/-before`
      and the rest of fd's documented flags; `-x`/`-X` through the command runner;
      `--base-directory` never changes the process cwd. `--changed-*` falls through
      until the shared date parser exists (0020).
- [ ] Upstream test suites (owner request): ripgrep's integration tests (`tests/`, MIT/Unlicense)
      and fd's (`tests/tests.rs`, MIT/Apache-2.0) vendored (pinned commit, attribution in
      NOTICE) and run against psh's rg/fd through a small adapter (their harnesses build
      a temp tree and run the binary: port the macros/TestEnv to run `psh -c` or the
      utility in-process). Its own ratchet of expected failures with reasons; the pass
      count in the Result. GNU grep's tests are GPL: run for reference only, never vendored.

grep, egrep, fgrep (S)
- [ ] Shares matcher, searcher, walker and exit rules (0/1/2, `-q` beats errors) with
      rg; GNU output through our own `Sink` (`file:line:text`, `file-line-text` context,
      `--` between groups, `Binary file X matches`, `-c -l -L -H/-h` defaults).
- [ ] GNU flag set `-n -E -F -v -i -r/-R -l -L -c -o -q -s -H -h -I -w -x -A/-B/-C`
      (also attached, `-A8`), `--include/--exclude/--exclude-dir` (basename fnmatch,
      `globset` with `literal_separator(false)`), multiple `-e`, `-f`, `-z`, `--color`.
- [ ] GNU recursion: `-r` without a path means `.`, hidden files searched, `.gitignore`
      ignored, `-R` follows symlinks, `-r` does not.

Windows
- [ ] `/` path separators by default for rg and fd (as `--path-separator /`; an explicit
      flag overrides); ANSI-or-no color from `UtilCtx` TTY state.
- [ ] CRLF: on Windows, `grep` and `rg` treat CRLF as a line end by default (ripgrep's
      `--crlf` behavior; D030), so `x$` matches `x\r\n`; explicit flags override; tests
      on CRLF fixtures, including `-o` and `--color` spans not containing the `\r`.

Common
- [ ] Each utility registered with its mode; unsupported flags fall through or fail
      clearly (tested per utility).
- [ ] L4 cases in `tests/utils/<name>/` for every flag listed above: rg and fd against
      real rg/fd (recording container), grep against GNU and BSD with the D016 rule named
      where they conflict.
- [ ] Attribution headers and `NOTICE` entries for all copied code.

## Notes
- Read `research/builtins.md` §5 first; the seams list in §5.4 is the patch checklist.
  The experiments of that section are in `/tmp/embed` (may be gone; reproduce from the
  section if so).
- oh-my-pi's `pi-builtins/src/{rg,grep,fd,bre}.rs` are MIT references; `pi-walker` is
  MIT but async (tokio, flume) and is not reused (D005).
- GNU grep's source is off limits (D007); its manual is fine.
- Large issue. Suggested order, merging after each: shared layer + rg; then grep; then
  fd. Tell the coordinator if it should be split into three issues.
- The backtracking backreference matcher and the leftmost-longest mode are follow-ups,
  not part of this issue.
- Back-references and leftmost-longest: psh-regex (0027) provides a POSIX engine with both;
  use it for the cases the ripgrep crates cannot do instead of falling through, once it lands.
- WASM: blocking utilities read stdin to EOF first, capped (D1080). grep/rg should feed the
  searcher incrementally in wasm if the crates allow it (e.g. an async utility wrapping
  grep-searcher over chunks), so `yes | grep x | head -1` finishes there too.

## Log

### 2026-11-07 plan
Parts, each merged on its own:
1. **Blocking utilities** (the D1080 exception, built now; D1130): `UtilCtx::run_blocking`
   (additive, no change to `UtilFn`/`UtilEntry`, so 0015/0016 are unaffected) and
   `SyncCtx` with std `Read`/`Write` stdio, cwd/env/TTY, a cancel flag and a synchronous
   fs view. Backends: a worker thread when the Host is the process's own file system
   (new additive `Host::os_fs()`, true for psh-os/StdHost), whose stdio are queues the
   engine pumps (bounded, so `rg | head -1` stops on EPIPE); inline otherwise (wasm,
   TestHost, sandbox hosts): stdin read first only if the utility asks for it (capped,
   D1080), output collected in order and written after, fs calls are Host futures
   polled once (a host that cannot answer at once gets an error, never a hang).
2. **Crates + shared layer:** the D020 set in psh-utils, `search/` with the BRE/ERE
   translator, the matcher builder and the walker seam (native `ignore` walker; virtual
   walker with the ignore chain from `ignore/src/dir.rs`) and the parity test.
3. **rg:** ripgrep's `flags/` and `search.rs`/`main.rs` glue at `91f5d22` with the seams
   patched; lexopt copied; JSON via deser-json.
4. **grep/egrep/fgrep** on the shared layer with a GNU sink.
5. **fd.**
Checkouts: `vendor/ripgrep` (91f5d22), `vendor/fd` (193d0df) in the primary checkout.

### part 1: blocking utilities (D1130)
- `crates/psh-engine/src/util/blocking.rs`: `SyncCtx`, `BlockingOptions`, `SyncFn`,
  `UtilCtx::run_blocking`. Thread backend: `Pipe` (mutex + condvar) between the worker
  and an engine pump loop on the utility's vproc; inline backend: stdin pre-read,
  output collected, Host futures polled once (`Waker::noop`).
- `Host::native_paths` (additive, default `None`): StdHost and psh-os (unix:
  bytes as OsStr; windows: `path::native`) answer it. Touches psh-os minimally (two
  three-line impls) and std_host (`metadata` made `pub(crate)`).
- Tests: `util::blocking::tests` (inline over TestHost incl. relative paths, panics,
  the output bound; thread over StdHost: redirections, engine pipes both ways, EPIPE
  stops an endless writer, the fs view, panics).
- ARCH "Interface" updated.

### part 2: crates and the shared layer (D1131, D1132)
- The D020 set in psh-utils (`grep-printer` without default features), allowlist lines.
- `psh_regex::to_rust_syntax` (new `crates/psh-regex/src/rust.rs`, additive: one module
  and a re-export in lib.rs, outside my touches but next to the parser it reuses):
  POSIX BRE/ERE through psh-regex's parser to regex syntax, with `pattern: None` for
  back-references and `ambiguous` for spans that can differ. Tested on psh-regex's GNU
  corpus (`search::posix::tests::agrees_with_the_gnu_corpus`).
- `search::walk` (seam, native `ignore::Walk`), `search::vwalk` (virtual walk + chain
  from `ignore/src/dir.rs`), `search::gitconfig` (global excludes from the shell env).
  Findings worth knowing: walkdir reports a read error after the directory itself
  (and not at all for a skipped or max-depth directory); a dangling root link is an
  error; `ignore::Walk` with `min_depth` never loads the root's ignore files (the
  seam applies `min_depth` itself); walkdir's I/O errors embed the path in the
  message ("IO error for operation on PATH: ...", what rg prints), mapped too.
- Parity tests in `search::tests`; NOTICE entry for ripgrep/walkdir.

### part 3: rg (in progress)
- Oracle (coordinator, 2026-11-07): sha256-pinned upstream release binaries ripgrep 15.2.0
  and fd 10.5.1 (musl) into /usr/local/bin of docker/Dockerfile, so every layer's oracle
  is the version psh implements; re-record the affected 0031 agent goldens in the same
  merge (tell shapes via the coordinator); a decision superseding the rg part of D1310.
- Engine (committed): `SyncWriter` (`SyncCtx::writer(fd)`: Clone + Send writer to fd 1/2,
  in order with the ctx's own output; inline output now in `Arc<Mutex<InlineOut>>`),
  `SyncCtx::stdin_readable()` / `UtilCtx::stdin_readable()` (engine pipe, or host file
  of kind File/Fifo/Socket), additive `HostFile::kind()` (psh-os unix/windows, StdHost,
  PumpFile, TestHost files).
- Copied verbatim (own commit): ripgrep crates/core flags/, main.rs, search.rs,
  haystack.rs, messages.rs, and lexopt 0.3.2 → crates/psh-utils/src/rg/. Shims written:
  rg/anyhow.rs (chain error, Context, anyhow!/bail!/ensure!; `io_kind()`), rg/textwrap.rs
  (refill/indent, textwrap 0.16.2 first fit, for `--help`).
- Plan for the patches: rg/cli.rs (grep-cli's parse_human_readable_size,
  patterns_from_reader, InvalidPatternError; stdout stream = termcolor Ansi/NoColor over a
  line/block-buffered SyncWriter; color Auto decided from ctx TTY + TERM/NO_COLOR of the
  shell env); `grep::x` → `grep_x`; `log::*!` → no-op macros (`--debug/--trace` fall
  through); pcre2 cfg blocks removed (`-P`/`--engine pcre2|auto` fall through); messages
  per invocation (thread-local scoped state, written to a stderr SyncWriter);
  config.rs/parse.rs from ctx (argv, RIPGREP_CONFIG_PATH, file via ctx); HiArgs gets a
  State from SyncCtx (cwd, TTY, stdin_readable, fs checks via ctx); walk_builder →
  search::walk::Walker + WalkOptions (globs as strings, .rgignore custom name);
  haystack on walk::Entry; search_path natively (entry.native), else search_reader over
  ctx.open; stdin via ctx.stdin(). Parallel: sequential sorted walk on the rg thread,
  N search threads (native only) with per-haystack termcolor buffers, output in walk
  order (reorder buffer, bounded in flight). `--json` first falls through, then our own
  JSON sink over deser-json. `--pre`, `-z` fall through first. `--help` identical but
  the version line. Windows: default path separator `/`, CRLF default (D030).

- Landed (part 3a, D1133, D1134): `crates/psh-utils/src/rg/` built and registered (S
  mode). Front end in `rg/mod.rs` (parse, help/version, fall-through for `--pre`, `-z`,
  `-P`, `--engine auto`, `--json`, `--debug/--trace`), body `main::run_sync` on the
  blocking layer. Thread-local per-run message state (`messages::scoped`). The module is
  `#[rustfmt::skip]` and formatted with `rustfmt --edition 2024 --config-path
  crates/psh-utils/src/rg/rustfmt.toml crates/psh-utils/src/rg/mod.rs` (ripgrep's style;
  lexopt.rs is skipped too) so the copy stays a small diff from upstream.
- Found: real rg's parallel walker words I/O errors without walkdir's "IO error for
  operation on PATH:" prefix, so the wording depends on rg's thread count;
  `WalkOptions::parallel_errors` (both walkers) reproduces that.
- Found: grep-printer calls `Instant::now()` for every sink; on wasm32-unknown-unknown
  that panics (trap). rg reports itself unavailable there for now (wasm ratchet lines in
  tests/{agent,utils}/expected-failures.wasm.txt); next part: an in-tree patched
  grep-printer (coordinator: option A).
- The classifier's rg table lacked value flags (`--pre`, `--engine`, `--type-clear`,
  `--dfa-size-limit`, `--regex-size-limit`, `--hostname-bin`, `--hyperlink-format`,
  `--generate`, `--maxdepth`, `--field-context-separator`): added in psh-classify (outside
  my touches, data only); `tests::rg_flags_match_the_classifier` checks it against
  ripgrep's flag table.
- Oracle: docker/Dockerfile has the rg 15.2.0 / fd 10.5.1 release binaries; `record-goldens
  --check agent` with it: no agent golden changed. tests/utils/rg/basic.test.sh: 34 cases,
  all pass natively incl. stderr; 58 agent ratchet entries pass now (removed).
- Engine tests added for `SyncWriter` and `stdin_readable` (inline and thread).
- Measurement (machine at load ~100, so rough): on ~112k files (the cargo registry
  sources) single-threaded psh ≈ rg (3.8 s vs 3.4–4.8 s); with 8 threads psh 4.2–5.9 s vs
  rg 1.9 s, and twice rg's system time; the walk alone (`--files`) 1.1 s vs 0.45 s. To
  redo on a quiet machine and tune the pool (shared `Mutex<Receiver>`) next part.
- Docs: ARCH "Search" (walk sequential, search threads, in-order merge). No guide page
  affected (utilities.md not written yet).

- Landed next (part 3b, D1135): `third_party/grep-printer/` (0.3.1 patched: `Instant`
  from `src/instant.rs`, zero clock on wasm32-unknown-unknown; PATCHES.md), through
  `[patch.crates-io]`, excluded from the workspace. rg runs in psh.wasm now (the rg
  `--stats` clock is zero there too; paths get Unix's color on wasm); all rg wasm ratchet
  lines removed (49 agent, 28 utils).
- Upstream issue text for the owner to file (BurntSushi/ripgrep):

  > **grep-printer: sinks panic on wasm32-unknown-unknown (`Instant::now`)**
  >
  > `StandardSink`, `SummarySink` and `JSONSink` call `std::time::Instant::now()` when
  > they are created (`sink_with_path`/`sink`) and in `begin`, to record the elapsed time
  > for `Stats`. On `wasm32-unknown-unknown` std has no clock and `Instant::now()` panics
  > ("time not implemented on this platform"), so no printer can be used there, even
  > with stats off, although grep-searcher and grep-regex work fine on that target.
  > Proposal: only read the clock when stats are enabled (`config.stats`), i.e. make
  > `start_time` an `Option<Instant>` set in `begin` when stats are requested, which also
  > saves two clock reads per searched file in the common case. We patch it locally with
  > an `Instant` shim (zero elapsed time on that target); happy to send a PR for the
  > `Option` variant. (Separately, `HyperlinkPath::from_path` and `encode` warn as unused
  > on targets that are neither `unix` nor `windows`.)

- Part 3c (D1136): `rg --json` in-process: `rg/json.rs` is grep-printer's JSON printer with
  the messages built as a small JSON tree that deser-json writes (same fields, order and
  escapes as serde_json; the summary's keys sorted as `json!`'s map sorts them). 3 L4
  cases (times removed with sed). Only `--pre`, `-z`, PCRE2 and `--debug/--trace` still
  run the real rg.

### part 4: grep, egrep, fgrep (D1137)
- `crates/psh-utils/src/grep/`: `mod.rs` (options, patterns, `-NUM`, colors), `pattern.rs`
  (GNU pre-pass, line matcher, span matcher `Pats` with `psh_regex` for back-references
  and ambiguous spans), `sink.rs` (GNU sink and output buffer), `search.rs` (operands,
  recursion on the walker seam, binary rules, exit status). Registered as mode S for
  `grep egrep fgrep`; SPEC/EGREP_SPEC/FGREP_SPEC checked against the classifier.
- Oracle loop: `work/scratch/0013/gb.sh` builds a Linux psh into target/record,
  `gc.sh SETUP < cmds` runs each command with GNU grep and psh in the recording image;
  ~180 probe commands agree (agent-style `-rn --include`, contexts, colors, -o, -w,
  binary, UTF-8, CRLF, -z).
- Found (GNU 3.11): the binary message is `grep: F: binary file matches` on stderr;
  lines that are invalid UTF-8 are left out (with the message) in C.UTF-8; `-A0` still
  separates groups; `-f /dev/null` reads no file at all (`-c` prints nothing, `-L` lists);
  ERE leading `*` is dropped with a warning, `x{1` is literal, `^*a` = `a`; `-m -1` is no
  limit; `--include` braces are literal; output to /dev/null makes GNU stop at the first
  match (measure with a file). grep-searcher's decoder returns the first 3 bytes on their
  own (BOM peek), so its binary detection came after the first line: grep reads the
  first 96 KiB itself.
- psh-classify: grep/egrep/fgrep tables now list every GNU value option (`--include`
  etc. were missing) and `--exclude-from` as a read (data only, outside my touches).
- L4: tests/utils/grep/basic.test.sh, 35 cases, GNU and BSD recorded; 17 `## expect:
  gnu` with the D016 reason. Pass natively and in wasm.
- Ratchets: oils/builtin-meta/1 (`command -v grep` prints the name, D1151);
  brush well_known_vars/4 on wasm (`env -v` now reached; the JS host has no env);
  utils/grep/basic/29 on Windows (file symlinks).
- Speed (Linux, 1M lines, 76 MB): on par with GNU grep (-c 0.06 s vs 0.04 s, -o 0.2 s vs
  1.5 s, back-references 2.3 s vs 5.1 s, `-rn` over vendor/ripgrep 0.58 s vs 0.47 s).
- Docs: ARCH "Search" grep paragraph. Guide: none of the written pages describes grep;
  processes.md's `seq 1 1000000 | grep 7 | wc -l` "real kernel pipes" example names three
  psh utilities now (follow-up for the docs agent).

### part 5: fd (D1138)
- `crates/psh-utils/src/fd/`: `cli.rs` (fd 10.5.1's clap definition on psh's option
  parser: flags, hidden flags, aliases, overrides, conflicts and value errors in clap's
  words; `-x`/`-X` taken out up to `;` first), `mod.rs` (front end: help/version, the
  checks of fd's `run` (base directory, search paths, path-like patterns, leading-dot
  patterns), the configuration, fall-through, then the commands), `walk.rs` (the
  blocking body on the walker seam: fd's filters and receiver rules), copied from fd
  193d0df: `filter/` (size; owner numeric only; time `@timestamp` only), `fmt/` (byte
  paths, no aho-corasick), `exec.rs` (templates and batching; execution through
  `UtilCtx::spawn`), `output.rs`, `patterns.rs` (+ regex_helper), `sanitize.rs`;
  `colors.rs` (what fd uses of lscolors 0.21.0 and nu-ansi-term 0.50.3, fd's default
  `LS_COLORS` string verbatim, newlines included). `help-long.txt`/`help-short.txt`:
  the release binary's `--help`/`-h`. NOTICE entries for fd, lscolors, nu-ansi-term.
- Design for `-x`/`-X`/`-l`: the blocking body collects the stripped results into an
  `Arc<Mutex<Vec>>` passed in its argument; the async front end runs the commands after
  the walk, one after another in path order (`/dev/null` stdin unless `--threads=1`,
  as fd's captured `Command::output`). A missing command gets fd's message without a
  spawn (the shell's "command not found" would otherwise appear).
- Found (fd 10.5.1): directories print with a trailing `/` (also colored); a leading
  `-` keeps `./`; `-0`/commands keep `./`; output is sorted by `Path` components only
  because the receiver buffers 100 ms/1000 results (the container sometimes streamed
  unsorted under load: tests sort `-x` output); `-1`/`--max-results` pick arbitrary
  results; `NO_COLOR` does not affect `--color=always`; with `-L` broken links are
  results, not errors; `--ignore-file` missing prints "Malformed pattern in custom
  ignore file" and goes on; clap's conflict usage line names the first option and
  `<pattern>` when one was given. fd's `DEFAULT_LS_COLORS` has two newlines in it, so
  `*.xml` is unstyled.
- Oracle loop: `work/scratch/0013/fd/{setup,setup2}.sh` and `cmds{1,2,3}.txt` through
  `gc.sh`: ~190 commands, all equal except fd's nondeterministic orders (`-X`, `-x`,
  `-1`) and `pwd` with arguments (psh's standalone builtin is quiet where GNU's warns).
- L4: tests/utils/fd/basic.test.sh, 30 cases (`## expect: gnu (fd is not a BSD tool)`),
  recorded GNU and BSD, `--check` deterministic; pass natively and in wasm (but the
  `cksum` one, which needs the program). Not in L4: `-l` (ls output differs by OS and
  has dates), `--owner` (ids differ).
- psh-classify: fd's table lists all value options (`-C`, `--and`, `--size`,
  `--format`, `--search-path`, `--base-directory`, ... were missing) and `--ignore-file`
  as a read (data only).
- Docs: ARCH "Search" fd paragraph. No guide page describes fd yet.

## Follow-ups
- fd: `--changed-within`/`--changed-before` durations and dates once 0020's date parser
  exists; `--owner` names (needs a name lookup in the utility context); `--hyperlink`
  (needs the host name); `-x` in parallel (fd's default) if it ever matters.
- grep: `-T/--initial-tab` falls through; GNU's "input file is also the output" check
  (`grep -r x . > out`) is not done (same need as rg's skip_stdout).
- docs: guide/processes.md's pipe example (`seq | grep | wc`) now names psh utilities, not
  programs.
- rg: `skip_stdout` (real rg skips the file stdout is redirected to, `rg x . > out`) is not
  done; needs the identity of fd 1 from the utility context.

## Result
