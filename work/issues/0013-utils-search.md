# 0013: Utilities batch B: rg, fd, grep on the ripgrep crates

status: open
agent: -
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
- [ ] fd's own tests consulted or ported as L4 cases.

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

- WASM: blocking utilities read stdin to EOF first, capped (D1080). grep/rg should feed the
  searcher incrementally in wasm if the crates allow it (e.g. an async utility wrapping
  grep-searcher over chunks), so `yes | grep x | head -1` finishes there too.

## Log

## Follow-ups

## Result
