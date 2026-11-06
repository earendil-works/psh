# Which utilities should psh run in-process?

Research for psh's builtin set, 2026-11-06. "Builtin" here means an **in-process utility**: `grep`, `sed`, `jq` and so on, which psh would run without forking. The shell's own builtins (`cd`, `export`, `trap` and the like) are part of the language, so §4.1 only lists them.

The question has two halves. **What to include** comes from demand signals. **How a builtin coexists with the real binary on PATH** is the shadowing policy, and it matters as much as the list.

## Sources

| Signal | Where | What it tells us |
|---|---|---|
| Our corpus | `research/corpus/results.json` plus a recount of `~/.cache/psh-corpus/features.jsonl` (295,401 commands, macOS only) | What agents type, and what failed with `command not found` |
| just-bash | Command directories in git history (`/tmp/just-bash`) and **194 issues + 543 PRs** (`gh … --state all`) | What users of a sandbox-only bash asked for once nothing else was available |
| oh-my-pi | `crates/pi-builtins` (about 80 utilities) and the `pi-shell` tools (`timeout`, `nohup`, `git`) | What an agent shell with a builtin-first policy chose |
| brush | `brush-builtins` (bash builtins) and `brush-coreutils-builtins` (82 uutils, feature-gated) | A Rust bash's optional coreutils set |
| Bun Shell | `src/runtime/shell/builtin/` (19 builtins) and tracking issue oven-sh/bun#9716 | The minimal cross-platform set for a JS runtime |
| deno_task_shell | `src/shell/commands/` (20 builtins) | The minimal set needed to run `package.json` scripts on Windows |
| busybox | 406 applets in total, of which **170 are enabled in busybox-w32's `mingw64_defconfig`** | The "universe", and what Windows users needed in practice. GPL, so a reference for *names* only |
| uutils | coreutils (109), findutils (`find`, `xargs`), diffutils (`diff`, `cmp`), sed, procps (18), util-linux (23) | The main permissive implementation pool |

### Caveats about the corpus

- **It is macOS-only.** GNU-only tools are suppressed because they were not installed: `tac` 1, `sha256sum` 11, `md5sum` 0. Their BSD and perl counterparts dominate instead: `shasum` 541, `md5` 73.
- **"Not found" counts reveal latent demand.** `timeout` failed 389 times, more than any other name. The next are `shuf` 3, `mapfile` 3 and `factor` 2.
- **`names` counts only the outer command name.** `xargs grep` counts as `xargs`. Declaration builtins (`export`, `local`, `declare`) are not counted at all.
- **One user, one machine.** Weight just-bash's issues and the other shells' choices as the "other users" signal.

---

## 1. just-bash

### 1.1 What it implements, in the order it was added

These dates are the first commit that added each command's directory, which is a rough demand signal. Directory names are mapped to the commands they register. The current registry (`packages/just-bash/src/commands/registry.ts`) has 89 names.

| Date | Commands added | Notes |
|---|---|---|
| 2025-12-23 (initial commit) | `cat cp echo grep head ls mkdir mv pwd rm tail touch wc cut find sed sort tr true/false uniq basename dirname env/printenv tee xargs du stat tree alias/unalias awk chmod clear history ln readlink bash/sh printf` | The "agent explores files" core. `tree` was in it from day one. |
| 2025-12-24 | `test`, `help`, `sleep`, `base64`, `date`, `diff`, **`jq`**, **`curl`**, `html-to-markdown` | jq and curl came on day 2 |
| 2025-12-26 | `expr paste seq timeout which` | |
| 2025-12-31 | `tac comm file md5sum/sha1sum/sha256sum hostname od` | |
| 2026-01-05 | `gzip/gunzip/zcat` (#30) | |
| 2026-01-07 | `yq column expand/unexpand fold join nl rev split strings` | PR #39, "Implement missing priority tools". It ships a priority table that rates nl/rev/fold/expand as "High" |
| 2026-01-08 | `xan` (CSV) | #40 |
| 2026-01-10 | `sqlite3` (sql.js) | #43 |
| 2026-01-14 | `tar`, **`rg`** (#46) | Issue #48: "Agent surprises when using the new `rg`" |
| 2026-01-24 | `rmdir time whoami` | |
| 2026-01-26 | `python3/python` (Pyodide, later Emscripten CPython) | |
| 2026-03-10 | `js-exec` plus a `node` stub (QuickJS) | |
| 2026-07-20 | bzip2/xz/zstd codecs for `tar` | |
| 2026-09-27 | `yes` (#409), `mktemp` (#377) | Both came from issue #408 and agent transcripts |
| Open PRs | `realpath` (#410, #447), `cmp` (#621), `ls -i` (#524), `find -mmin/-newermt` (#555), `commandNotFound` fallback hook (#531), WASI "WebAssembly commands" (#462) | |

### 1.2 Requests for new commands (issues and PRs, open and closed)

The 194 issues carry these labels: `bug` 90, `commands` 55, `shell` 34, `enhancement` 25. I searched titles and bodies for add / support / implement / missing / `command not found`, then read each hit.

| Requested | Count | Links | Outcome |
|---|---|---|---|
| `git` | 1 issue, 3 comments | [#83](https://github.com/vercel-labs/just-bash/issues/83) | **Open.** The maintainer (cramforce) said "I heard [isomorphic-git] is very slow, but otherwise yes". A third party built [just-git](https://github.com/blindmansion/just-git) as a *custom command*. Never added to core. |
| `realpath`, `yes` | 1 issue, 3 PRs | [#408](https://github.com/vercel-labs/just-bash/issues/408), [#409](https://github.com/vercel-labs/just-bash/pull/409) merged, [#410](https://github.com/vercel-labs/just-bash/pull/410) and [#447](https://github.com/vercel-labs/just-bash/pull/447) open | `yes` was **accepted**. `realpath` is stuck on the `IFileSystem.realpath` API, which the maintainer says needs "a breaking change". The issue gives the rationale: "`cd "$(dirname "$x")" && pwd` is clumsy". |
| `mktemp` | 2 PRs | [#377](https://github.com/vercel-labs/just-bash/pull/377) merged, [#392](https://github.com/vercel-labs/just-bash/pull/392) closed as a duplicate | **Accepted.** It needed a new `createExclusive` FS method. The motivation came from agents: across 291 sessions `/tmp` showed up in 25 commands, and the mounts had no `/tmp`. |
| `cmp` | 1 issue, 1 PR | [#611](https://github.com/vercel-labs/just-bash/issues/611), [#621](https://github.com/vercel-labs/just-bash/pull/621) | Open. The issue says "Our LLM agent tried `cmp` … had to fall back to `diff`". |
| `shasum` (+ GNU `-c` diagnostics) | 1 PR | [#721](https://github.com/vercel-labs/just-bash/pull/721) | Closed by its author for a setup mistake, not rejected. Rationale: "the common way to verify a download on macOS". |
| `unzip` | 1 issue | [#157](https://github.com/vercel-labs/just-bash/issues/157) | Open with no response. |
| `mysql` client | 1 issue | [#205](https://github.com/vercel-labs/just-bash/issues/205) | Open with no response. A network/database client. |
| `umask` | 1 issue | [#475](https://github.com/vercel-labs/just-bash/issues/475) | Open. Needed by autoconf's `config.status`. |
| `test`/`[` | 1 issue | [#80](https://github.com/vercel-labs/just-bash/issues/80) | Closed: it already existed as a builtin. |
| `specli` (a 3rd-party CLI) | 2 PRs | [#56](https://github.com/vercel-labs/just-bash/pull/56), [#63](https://github.com/vercel-labs/just-bash/pull/63) | **Rejected.** cramforce: "I wonder if this needs to happen rather than documenting how to add it as a custom command". |
| `sqlite3` dot-commands | 2 PRs | [#215](https://github.com/vercel-labs/just-bash/pull/215), [#249](https://github.com/vercel-labs/just-bash/pull/249) | Closed. The contract didn't fit what agent harnesses depended on. This shows the cost of emulating a big client. |
| Generic fallback for missing commands (proxy them elsewhere) | 1 issue, 1 PR | [#113](https://github.com/vercel-labs/just-bash/issues/113), [#531](https://github.com/vercel-labs/just-bash/pull/531) | Open. The same need as our "fall through to PATH". |
| Configure which commands load (to save prompt context) | 1 issue | [#104](https://github.com/vercel-labs/just-bash/issues/104), [#267](https://github.com/vercel-labs/just-bash/issues/267) | Open. Embedders want per-session allow-lists. |
| WASM/WASI commands | 4 PRs | [#462](https://github.com/vercel-labs/just-bash/pull/462), #471, #472, #498 | Open. Run real tools compiled to WASI instead of reimplementing them. |

Flag-level requests and fidelity bugs outnumber new-command requests by about 5 to 1. Grouped by command:

| Command | Issues / PRs | Examples |
|---|---|---|
| `jq` | 12 | `--arg`/`--argjson`/`--slurpfile` (#175, #317, #162, #291, now merged); `-R` (#260); `-f` (#700); `input_filename` (#702); `del`/`-=` (#266, #607, #707); `group_by`/`unique`/`sort_by` (#736); date builtins (#694, #698) |
| `grep` | 8 | `-f` (#322, #184, #327); multiple `-e` OR (#297); GNU `-I -H -s -nA1 --include GLOB` (#600); `-r` with no path, symlinks, dotfiles (#581 closed *not planned*, #601); BRE `$` before `\|` (#467); RE2 performance (#625) |
| `awk` | 7 | `-f progfile`, `var=value` operands (#468); `length(arr)` (#647); `asort` (#737); `printf` padding, `> "/dev/stderr"` (#608) |
| `sed` | 7 | `-i.bak` (#609); one-line `a\text` (#393); `\U` (#609); `-f` continuations (#492); `n`/`p` order (#496); `s///Ng` (#673) |
| `ls` | 7 | `-F` (#112, #117 merged); `-i` (#464, #524); `-t`, `-L` (#465, automake); real mode bits in `-l` (#459, #559) |
| `rg` | 7 | stdin (#280); early exit under `head` (#415); `--no-heading`, `--no-messages` (#704, #709); brace globs (#602) |
| `xargs` | 3 | attached values `-n2 -I{} -d,` (#466); `-a` (#711); child environment (#530) |
| `find` | 3 | `-L/-H`, anchored `-regex`, symlinked dirs (#604); `-mmin`, `-newermt` (#555) |
| `diff` | 2 | **default normal format, not unified** (#412); `-r -q -U N -w -B` (#605) |
| `expr` | 3 | POSIX BRE for `:` (#460); `\|` chains (#500); decimals (#741) |
| Others | 1 each | `cat` GNU `-A -v -e -t` (#293); `curl -i/-I/-D` (#388, #389); `wc` padding (#289); `tr` octal escapes (#476); `cp -f` (#494); `join` (#710); `yq -I` (#696); `date -u -d` (#738); `tar czf` old-style options (#270) |

**What this tells us:**

1. Demand concentrates on the same ~10 heavy tools that dominate our corpus: grep, rg, sed, awk, jq, ls, find, xargs, diff.
2. Once a builtin exists, users expect **GNU flag fidelity**. Every gap is filed as a bug.
3. jq, grep, awk and sed are where a fresh reimplementation hurts most. Port mature code for these (jaq, ripgrep crates, uutils sed, a real awk) rather than writing new.
4. Network and database clients (git, mysql) and third-party CLIs were deferred or rejected in favor of host-provided custom commands.
5. Embedders want two things: a fallback hook for unknown commands, and per-session control over which commands exist.

---

## 2. Command sets compared

The full name × project matrix is in [Appendix A](#appendix-a-full-matrix). Summary:

| Project | Count | Set (utilities only; shell builtins omitted) |
|---|---|---|
| **just-bash** | 89 | cat cp echo grep egrep fgrep head ls mkdir mktemp mv pwd rm rmdir tail touch wc cut find sed sort tr true false uniq basename dirname env printenv tee xargs du stat tree alias unalias awk chmod clear history ln readlink bash sh printf test sleep base64 date diff jq curl html-to-markdown help expr paste seq timeout time which tac comm file md5sum sha1sum sha256sum hostname od gzip gunzip zcat yq xan column expand unexpand fold join nl rev split strings sqlite3 tar rg whoami yes python python3 js-exec node |
| **oh-my-pi** `pi-builtins` + `pi-shell` tools | 79 + 3 | ast-grep/sg b2sum base32 base64 basename cat chmod cksum cmp combine comm cp cut date dd df diff dirname du env errno fd find fold grep head hostname id ifne isutf8 jq ln ls md5sum mkdir mktemp mv nl nproc od paste pgrep pidwait pkill printenv ps readlink realpath rg rm rmdir rsync sed seq sha1sum sha224sum sha256sum sha384sum sha512sum sleep sort sponge stat tac tail tee top touch tr truncate ts uname uniq uptime wc which whoami xargs yes, plus the tools **timeout nohup git**, plus engine `kill`. Every name shadows PATH. Kill switches: `PI_DISABLE_UUTILS_BUILTINS`, `PI_DISABLE_UUTILS_DESTRUCTIVE` (rm mv cp ln dd rsync), `PI_DISABLE_RM_BUILTIN`, `PI_DISABLE_MV_BUILTIN`. **No awk, no tar/gzip, no xxd, no curl.** |
| **brush** coreutils builtins (uutils 0.12) | 82 | arch b2sum base32 base64 basename basenc cat cksum comm cp csplit cut date dd df dir dircolors dirname du echo env expand expr factor false fmt fold head hostname join link ln ls md5sum mkdir mktemp more mv nl nproc numfmt od paste pr printenv printf ptx pwd readlink realpath rm rmdir seq sha1sum sha224sum sha256sum sha384sum sha512sum shred shuf sleep sort split sum sync tac tail tee test touch tr true truncate tsort uname unexpand uniq unlink vdir wc whoami yes. This is "whatever uutils has that is cross-platform", with no grep, sed, find or awk. It **re-execs the brush binary** (`--invoke-bundled`) rather than running truly in-process. |
| **Bun Shell** | 19 | cd ls rm echo pwd bun cat touch mkdir which mv exit true false yes seq dirname basename cp (cat and cp delegate to the system binary on POSIX). Planned in #9716: timeout realpath mktemp truncate head tail cut uniq sleep tee date sha*sum md5sum sort printenv env whoami who uptime |
| **deno_task_shell** | 20 | cat cd cp echo exit export head mkdir mv pwd rm set shopt sleep test true false `:` unset xargs |
| **busybox-w32** (mingw64) | 170 | `[ [[ ar arch ascii ash awk base32 base64 basename bash bc bunzip2 bzcat bzip2 cal cat cdrop chattr chmod cksum clear cmp comm cp cpio crc32 crond crontab cut date dc dd df diff dirname dos2unix dpkg dpkg-deb drop du echo ed egrep env expand expr factor false fgrep find flock fold free fsync ftpget ftpput getopt grep groups gunzip gzip hd head hexdump httpd iconv inotifyd install ipcalc jn join less link ln logname ls lsattr lzcat lzop lzopcat make man md5sum mkdir mktemp mv nc nl nproc od paste patch pdpmake pdrop pgrep pidof ping ping6 pipe_progress pkill printenv printf pwd readlink realpath reset rev rm rmdir rpm rpm2cpio sed seq sha1sum sha256sum sha384sum sha3sum sha512sum shred shuf sleep sort split ssl_client stat strings stty su sum sync tac tail tee test time timeout touch tr true truncate ts tsort ttysize uncompress unexpand uniq unix2dos unlink unlzma unxz unzip uptime usleep uudecode uuencode uuidgen vi watch wc wget which whoami whois xargs xxd xzcat yes zcat` |
| **uutils** | 109 + 4 + 18 + 23 | coreutils (all of GNU coreutils except chcon/runcon-style SELinux bits on non-Linux), findutils (find xargs locate updatedb), diffutils (diff cmp), sed, procps (free pgrep pidof pidwait pkill pmap ps pwdx skill slabtop snice sysctl tload top vmstat w watch), util-linux (cal hexdump rev setsid uuidgen rename renice …) |

**Consensus.** Counting 7 sets (just-bash, oh-my-pi, brush-cu, Bun, deno, busybox-w32, uutils\*):

- `cat cp mkdir mv rm` are in all 7.
- `basename dirname echo false head ls pwd seq sleep touch true yes` are in 6.
- These are in 5: `base64 comm cut date du env fold ln md5sum mktemp nl od paste printenv readlink rmdir sha1sum sha256sum sort tac tail tee tr uniq wc whoami xargs`.
- These are in 4: `chmod diff find sed stat timeout which realpath expr join split truncate df dd nproc hostname`.

**Notable splits:**
- `awk` is in just-bash and busybox only.
- `tar`/`gzip` are in just-bash and busybox only.
- `jq` and `rg` are in just-bash and oh-my-pi, the two agent-focused shells.
- Process tools (`ps pgrep pkill top`) are in oh-my-pi and busybox.
- `timeout` is in just-bash, oh-my-pi, busybox and uutils.

## 3. Our corpus: the top names

From `results.json`. "cmds" is the number of commands containing the name, out of 295,401. Externals are in *italics*.

| # | name | cmds | # | name | cmds | # | name | cmds |
|--:|---|--:|--:|---|--:|--:|---|--:|
| 1 | cd | 140,082 | 21 | *curl* | 4,060 | 41 | time | 1,302 |
| 2 | head | 97,135 | 22 | cut | 3,874 | 42 | seq | 1,176 |
| 3 | grep | 71,606 | 23 | *uv* | 3,815 | 43 | *go* | 1,147 |
| 4 | *git* | 70,242 | 24 | cp | 3,810 | 44 | xargs | 1,009 |
| 5 | rg | 58,283 | 25 | awk | 3,503 | 45 | ps | 989 |
| 6 | sed | 47,609 | 26 | mkdir | 3,479 | 46 | diff | 913 |
| 7 | *python3* | 31,961 | 27 | *make* | 2,680 | 47 | chmod | 905 |
| 8 | *node* | 28,492 | 28 | set | 2,170 | 48 | pwd | 829 |
| 9 | tail | 26,457 | 29 | mktemp | 2,036 | 49 | pkill | 760 |
| 10 | cat | 24,389 | 30 | jq | 2,026 | 50 | date | 754 |
| 11 | echo | 18,810 | 31 | exit | 1,886 | 51 | which | 744 |
| 12 | ls | 18,197 | 32 | *tmux* | 1,789 | 52 | kill | 671 |
| 13 | *npm* | 15,871 | 33 | *pnpm* | 1,701 | 53 | uptime | 597 |
| 14 | *cargo* | 13,333 | 34 | uniq | 1,609 | 54 | *lsof* | 585 |
| 15 | find | 9,318 | 35 | nice | 1,533 | 55 | nohup | 580 |
| 16 | sleep | 9,081 | 36 | *docker* | 1,527 | 56 | tar | 552 |
| 17 | printf | 8,531 | 37 | nl | 1,523 | 57 | du | 545 |
| 18 | *gh* | 8,128 | 38 | tr | 1,432 | 58 | shasum | 541 |
| 19 | wc | 8,081 | 39 | `true` | 6,595 | 59 | env | 509 |
| 20 | sort | 7,995 | 40 | rm | 7,393 | 60 | timeout | 468 (+389 not found) |

Further down: `file` 426, `pgrep` 395, `tee` 357, `id` 331, `ln` 306, `basename` 270, `strings` 266, `paste` 262, `stat` 249, `cmp` 247, `gzip` 219, `sqlite3` 199, `touch` 175, `xxd` 142, `base64` 138, `readlink` 113, `dirname` 106, `uname` 78, `column` 76, `md5` 73, `comm` 71, `rsync` 71, `unzip` 67, `od` 53, `join` 37, `patch` 32, `fd` 30, `df` 24, `realpath` 23, `zstd`/`brotli` 23, `printenv` 19, `bc` 18, `fold` 16, `nproc` 13, `yes` 13, `sha256sum` 11, `dd` 10, `shuf` 9, `top` 9, `hexdump` 5, `truncate` 5, `less` 5, `tree` 2, `wget` 2, `tac` 1. Zero uses: `md5sum expr rev watch sha512sum base32 split expand dos2unix envsubst`.

`not_found` top: **`timeout` 389**, pdftotext 13, magick 12, tsc 9, vitest 6, tmux 5, python 5, `shuf` 3, `mapfile` 3 (bash 3.2), `gtimeout` 2, `factor` 2, `setsid` 1.

`results.json:builtin_set` (117 names) is the set the corpus analysis assumed. Including it would run 37.6% of commands fully in-process, or 85% with one external allowed.

**Flags that matter for builtins** (corpus `flags_by_util`):

| Command | Top flags |
|---|---|
| grep | `-n -E -v -rn -i --include -A/-B/-C -c -o -l` |
| rg | `-n --glob/-g -i -S -C -l --type -o --hidden` |
| sed | `-n` 56k, `-i` 6k (BSD `-i ''` 5.3k vs GNU 144), `-E`, `-e` |
| head / tail | `-N` (bare number) dominates, `-n`, `-c` |
| find | `-type -maxdepth -name -o -print -path -iname -not -exec -prune -print0 -mmin -mtime -size -newer -printf` |
| jq | `-r -c -s --arg -e --argjson -n -S --slurpfile` |
| awk | `-F` (many separators, including regex-ish `[][]` and multi-char) and `-v` |
| xargs | `-0 -I{} -n1 -r -P -L` |
| diff | `-u -rq --label -q -r -x --exclude -I -w -U0` |
| tar | `-C -xzf -x -tzf -xf --strip-components -xOf -czf` |
| stat | `-f` 298 (BSD) vs `-c` 3 |
| date | `-u` 123, `-r` 17 (BSD epoch), `-j -f`, `-v±N`, `-d` 2 |
| base64 | `-d` 114, `--decode`, `-i` 13 / `-o` 7 (BSD file options) |
| xxd | `-l -s -g -p -c -r` |
| ps | `-o -p -axo -Ao -r -eo -ef` |
| pkill | `-f` 859 |
| nice | `-n` 1,412 |
| id | `-u -g` |
| curl | `-o -s -H -sS -w --max-time --data-urlencode -L -fsSL -X -D` |

---

## 4. Proposal

### 4.0 Principles

**Why have a builtin at all?** In order of weight:

1. **Portability.** Windows has none of these tools, and WASM has no processes. The builtin set *is* the shell's toolset there.
2. **Consistency.** One flag dialect everywhere: the GNU ∪ BSD union, with documented tie-breaks. The corpus shows agents guessing between dialects.
3. **No fork.** It is faster, it works inside sandboxes and VFS paths (`skill://`), it cancels cooperatively, and the effects of a known builtin are known to the classifier.

**What a builtin costs:** binary size, maintenance, and above all **divergence when it shadows a better real binary**. Every flag gap becomes a bug report, as just-bash shows.

**Shadowing policy.** This is a per-builtin column in the tables below:

| Mode | Meaning | When to use it |
|---|---|---|
| **S: shadow** | Always the builtin, as oh-my-pi does | Only where we can be at least as good as GNU and BSD for agent use |
| **F: fallback** | Use the builtin only if no executable of that name is on PATH, or in WASM/sandbox mode | Tools whose real versions are better, or whose exact output (sizes, magic strings) is the point |
| **W: shadow on Windows, fallback elsewhere** | | Names that collide with incompatible System32 programs: `find.exe`, `sort.exe`, `timeout.exe`, `tree.com`, `expand.exe`, `more.com`, `fc.exe`, `hostname.exe`/`whoami.exe` (different output). On Windows the builtin **must** win, or `find . -name x` runs the wrong program. |

Other policy points:
- Like oh-my-pi, provide session-level kill switches: all builtins, destructive builtins (`rm mv cp ln dd`), or per name.
- `command -v`, `type` and `which` must report builtins honestly.
- `env NAME`, `xargs NAME` and `find -exec NAME` dispatch back into the shell, so builtins stay reachable from them.
- An explicit path (`/usr/bin/grep`) always means the real binary.

**Flag dialect policy.**
- Accept the **union of GNU and BSD** flags where they don't conflict.
- Where they conflict, use **GNU semantics**, except where macOS usage is overwhelming (`sed -i ''`, `stat -f FORMAT`). Use deterministic disambiguation rules for those (§4.6).
- `uname` and the like must report the *real* host. Don't pretend to be Linux.

**Hard exclusions (owner rule).** Interpreters, toolchains, package managers and network/database clients are never builtins (§4.5). In WASM/sandbox mode an *embedder* may still supply host-backed commands through the Proc/command-provider interface, the way just-bash supplies `curl` through fetch. Those are host commands, not part of psh's builtin set.

### 4.1 Tier 0: shell builtins (the language, not up for debate)

`: . [ [[ alias bg break builtin caller cd command compgen complete continue declare dirs disown echo enable eval exec exit export false fc fg getopts hash help history jobs kill let local logout mapfile readarray popd printf pushd pwd read readonly return set shift shopt source suspend test times trap true type typeset ulimit umask unalias unset wait`, plus the `time` keyword.

The corpus `not_found` list includes `mapfile`, which bash 3.2 lacks; we give bash-5 semantics. `kill` must handle both jobs (`%1`) and OS pids, and process groups (`kill -$pgid`, 14 uses). `printf`, `echo`, `test`, `true`, `false`, `pwd` and `kill` also need standalone forms for `env`, `xargs` and `find -exec`.

### 4.2 Tier 1: must have on day one

The criterion: top-60 corpus names that aren't hard-excluded, plus everything the agent read/edit/search loop needs, plus the command wrappers, which must be builtins so they can wrap builtins and manage process trees. That is **61 names**, counting each checksum name separately. In the table, the Ref column is the permissive implementation to port or consult.

| Builtin | Mode | Ref | GNU vs BSD notes |
|---|---|---|---|
| `head` | S | uutils (MIT) | Bare `-N`, `-n N`, `-c N`, GNU negative `-n -N`, multiple-file headers |
| `tail` | S | uutils | `-N`, `+N`, `-n`, `-c`, `-f`/`-F` (needs streaming plus cancellation), BSD `-r` (reverse) |
| `cat` | S | uutils | GNU `-A -v -e -E -t -T -n -b -s`. BSD lacks `-A`, which failed 65× in the corpus |
| `ls` | S | uutils | Conflicts: BSD `-G` (color) vs GNU no-group, BSD `-T` (full time) vs GNU tabsize. Support `-la -lh -t -d -R -1 -S -F -i -L --time-style`. Real mode bits (just-bash #459) |
| `cp` | S | uutils | **Trailing slash:** BSD `cp -R src/ dst` copies *contents*, GNU copies the directory. Pick GNU and document it. `-R -r -a -p -n -f -L`, GNU `-t --parents`, BSD `-c` (clonefile, a no-op fallback) |
| `mv`, `rm`, `mkdir`, `rmdir`, `touch`, `ln` | S | uutils | `rm -rf`/`-f` dominate. `ln -sfn`. `touch -d` (GNU) and `-t`/`-r` (both). `mkdir -p -m` |
| `chmod` | S | uutils | `+x`, octal, `-R`. On Windows: the read-only bit, and track `x` in metadata for shebang execution |
| `pwd`, `basename`, `dirname`, `realpath`, `readlink` | S | uutils | `readlink -f`/`-e`/`-m` (GNU; macOS ≥12.3 has `-f`). `realpath --relative-to` |
| `mktemp` | S | uutils | 2,036 uses, `-d` dominant. GNU vs BSD `-t` differ (BSD: prefix; GNU: template in TMPDIR). Default to `$TMPDIR`. Create with 0600/0700, exclusively |
| `stat` | S | uutils (GNU) + own BSD formatter | **Conflict:** BSD `-f FORMAT` (298 uses) vs GNU `-f` (file-system mode). Rule: `-f` followed by an argument containing `%` means BSD format. `-c`/`--format`/`--printf` are GNU; `-t` is BSD timefmt; `-L` both |
| `du` | S | uutils | `-sh -h -sk -ch -d N` (BSD) and `--max-depth` (GNU), `--exclude`, `-A`/`--apparent-size` |
| `grep` (+`egrep`, `fgrep`) | S | ripgrep `grep-regex`/`grep-searcher`/`grep-printer` (MIT/Unlicense) | GNU flag set: `-n -E -F -v -i -r/-R -l -L -c -o -q -s -H -h -I -w -x -A/-B/-C` (attached `-A8`), `--include/--exclude(-dir)`, `-e` (multiple), `-f`, `-z`, `--color`. **`-P`**: BSD lacks it and it failed 19% in the corpus; offer it via the `pcre2` crate (BSD-licensed PCRE2) or `fancy-regex`. Backreferences in BRE (`\(a\)\1`) need a backtracking fallback, unlike just-bash's RE2 |
| `rg` | S | ripgrep crates (MIT/Unlicense), `ignore`, `globset` | Must match real rg exactly: no smart-case by default (unlike just-bash), `--no-heading`, line numbers only on a TTY, `-g` brace globs, `.gitignore` semantics including parent dirs, early exit under `| head` |
| `sed` | S | uutils sed (MIT) / oh-my-pi's port | **`-i` disambiguation** (§4.6). GNU extensions in BRE (`\+ \? \| \s \w`), `-E`/`-r`, `-z`, `-s`, one-line `a text`/`i`/`c`, `\U\L\E`, `s///Ng`, `-f` files, `-n` |
| `awk` | S, F until conformant | No mature permissive Rust awk exists. Port [goawk](https://github.com/benhoyt/goawk) (MIT, POSIX and passes the onetrueawk tests) or consult [onetrueawk](https://github.com/onetrueawk/awk) (MIT-style Lucent licence, the macOS awk) and its `T.*` tests; [frawk](https://github.com/ezrosent/frawk) (MIT/Apache, JIT-heavy) as a perf reference; just-bash's TS awk (Apache) | POSIX awk plus the common gawk-isms agents use: `-F` regex and multi-char, `-v`, `-f`, `length(arr)`, `tolower`/`toupper`, `gensub`?, `strftime`/`systime`, `printf` with `%*d`, `getline` from file and command, `system()` and `"cmd" | getline` routed through the shell. **This is the largest single item.** Run it in F mode until it passes the onetrueawk and goawk suites |
| `sort` | S | uutils | `-u -n -r -k -t -V -h -s -z -o -R`. Locale: default to byte order (`LC_ALL=C` behavior) unless the locale is set? Decide once and document it |
| `uniq` | S | uutils | `-c` (GNU pads the count to 7 columns, BSD to 4; pick GNU), `-d -u -f` |
| `wc` | S | uutils | `-l -c -w -L`. Padding: GNU prints a bare number for a single count from stdin, BSD always pads to 8. Pick GNU (just-bash #289 shows people notice) |
| `cut`, `tr`, `paste`, `nl`, `tee`, `seq`, `comm` | S | uutils | `cut -c1-200` (character vs byte on UTF-8: GNU `-c` is bytes; agents mean characters, so use characters). `tr` classes and octal escapes. `nl -ba`. `paste -sd+`. `tee -a`. `seq -w -s -f` |
| `printf`, `echo`, `test`/`[`, `true`, `false`, `sleep` | S | ours (Tier 0) | `sleep` takes fractional values and GNU suffixes `1m`; it must be cancellable |
| `find` | S (W on Windows) | uutils findutils (MIT) | GNU default path `.`, `-printf` (GNU), BSD `-E`, `-maxdepth -mindepth -name -iname -path -ipath -regex -type -size -mtime -mmin -newer -newermt -perm -empty -prune -not/! -o -a ( ) -print0 -exec {} \;/+ -execdir -delete -quit -L/-H` |
| `xargs` | S | uutils findutils | `-0 -I{}` (attached), `-n1 -r -P N -L -d -a -t -o`. GNU `-r` vs BSD's default no-run-if-empty: pick GNU, but agents mostly pass `-r` |
| `diff` | S | uutils diffutils (MIT/Apache), oh-my-pi `diff.rs` | **Default normal format** (just-bash #412). `-u -U N -c -r -q -N -x/--exclude -I -w -b -B -y --label --color`. Exit status 0/1/2 |
| `cmp` | S | uutils diffutils | `-s -l -b -n -i` |
| `jq` | S | **jaq** (MIT) | `-r -c -s -n -e -S -j -a -R --arg --argjson --slurpfile --rawfile --args --jsonargs --tab --indent -f`. Fix the documented jaq↔jq gaps: `-s` over several files must produce *one* array (jaq makes one per file); `?//` is unsupported; `. |= empty` differs; check number formatting and error text |
| `base64` | S | uutils base64/basenc | Union: `-d -D --decode`, wrapping `-w N` (GNU) / `-b N` (BSD). **Conflict on `-i`:** GNU means ignore-garbage, BSD means input file (13 uses). Rule: `-i` followed by a non-option argument means BSD input file. Support `-o FILE` (BSD) |
| `md5sum sha1sum sha256sum sha512sum`, `shasum`, `md5` | S | uutils cksum machinery | Each name keeps its native output format. GNU: `hash  file`. BSD `md5`: `MD5 (file) = hash`, with `-q -r -s`. perl `shasum`: `-a ALG -c -b`. All support `-c` verify mode |
| `date` | S | uutils date plus `parse_datetime` (MIT) | Union: GNU `-d STRING`, `-I`, `-R`, `--rfc-3339`, `+%N`; BSD `-v±N[ymwdHMS]` and `-j -f INFMT`. **Conflict `-r`:** GNU reference file vs BSD epoch seconds. Rule: all digits and not an existing file means epoch. Use the real local TZ natively (just-bash forces UTC; we don't need to) |
| `env`, `printenv` | S | uutils env | Runs a command: `-i -u -S -C`, `NAME=val cmd`. Must dispatch to builtins |
| `timeout` | S (W) | uutils timeout for semantics; process-tree kill is ours (Job Objects / process groups); oh-my-pi `tools/timeout.rs` for design | **#1 not-found name (389).** GNU semantics: durations with `s m h d` suffixes and fractions, `-k`, `-s SIG`, `--preserve-status`, `--foreground`, status 124/137. Accept `gtimeout` as an alias |
| `nice` | S | uutils nice | `-n N cmd` (1,412 uses). Native: setpriority, or BELOW_NORMAL_PRIORITY_CLASS on Windows. For a builtin target, run it as a no-op wrapper |
| `nohup` | S | uutils nohup; oh-my-pi `tools/nohup.rs` for the detached-lineage design | 580 uses, almost always `nohup cmd > log 2>&1 &`. Run the command detached from the shell's cancellation, ignore HUP, and never write `nohup.out` when stdout is already redirected. Interacts with the session's background-process policy |
| `which` | S | ours | 744 uses, `-a`. Report builtins (`jq: shell builtin`) and PATH hits. Exit 1 when nothing is found |

### 4.3 Tier 2: soon after (cheap, portable value, or Windows gaps)

| Builtin | Mode | Ref | Notes |
|---|---|---|---|
| `ps`, `pgrep`, `pkill` | S | uutils procps (MIT, Linux-centric); oh-my-pi `ps.rs`/`pgrep.rs` (cross-platform); `sysinfo` crate (MIT) | 989/395/760 uses. Windows has none of them. `ps` needs both dialects: BSD `ps aux`/`-axo` and procps `-eo`/`-ef`/`-o`/`-p`. `pkill -f` dominates. Lets the shell report its own job tree |
| `uptime`, `nproc`, `id`, `uname`, `whoami`, `hostname`, `df` | S (W for hostname/whoami) | uutils | 597/13/331/78/5/–/24 uses. Real host values. `nproc` is GNU-only, so macOS agents fail on it today (`sysctl -n hw.ncpu`) |
| `tree` | S (**W**: `tree.com`) | ours. eza (MIT) for layout; **don't copy GNU `tree` (GPL)**. just-bash tree (Apache) | Only 2 corpus uses (not installed on macOS), but just-bash shipped it on day one and Windows has a conflicting `tree.com`. `-L -a -d -I -f --gitignore -J --noreport` |
| `column` | S | ours, against util-linux column (GPL, behavior only) / BSD column | 76 uses: `-t -s -c`, util-linux `-o -J -N`. BSD and util-linux differ on `-x`. Implement the util-linux superset |
| `tac`, `rev` | S | uutils tac; uutils util-linux rev | macOS lacks `tac`. `tail -r` covers it there |
| `strings` | S | ours (simple) | 266 uses: `-a -n N -t x/d/o -e l`. A binutils name, but trivial |
| `od` | S | uutils | 53 uses: `-c -An -t x1/c -v` |
| `xxd` | S | ours. **xxd ships with vim (vim licence)**, so write it from the man page | 142 uses: `-l -s -g -p -c -r -i -u`. The output format must match byte for byte |
| `hexdump` (+`hd`) | S | uutils util-linux hexdump (MIT) | 5 uses: `-C -n -s`. BSD and util-linux formats are nearly identical |
| `expr` | S | uutils | 0 corpus uses, but configure scripts need it. `:` uses POSIX BRE (just-bash #460) |
| `yes` | S | uutils | Needs pipeline streaming and backpressure (just-bash had to cap it) |
| `truncate`, `fold`, `expand`, `unexpand`, `join`, `split` | S | uutils | Cheap and in every reference set |
| `cksum`, `b2sum`, `sha224sum`, `sha384sum`, `base32`, `basenc`, `sum` | S | uutils | Comes nearly free with the checksum family |
| `shuf` | S | uutils | 9 uses plus 3 not-found (macOS lacks it). `-n -i -r -e --random-source` |
| `patch` | S | **BSD patch** (FreeBSD/OpenBSD, BSD licence) for behavior; `diffy` (MIT/Apache) for hunk application; **GNU patch is GPL** | 32 uses. `-p N -R -N --dry-run -i -o -b -F`, unified and context diffs, fuzz, `.rej`. Also lets `git apply`-free workflows run in WASM |
| `tar` | **F** (W: Windows has bsdtar `tar.exe`, which works, so F is fine there too) | `tar` crate (MIT/Apache), `flate2` | 552 uses: `-C -xzf -tzf -xf --strip-components -xOf -czf --exclude`. GNU and bsdtar long options mostly overlap. Old-style `tar czf` bundling. **Fallback** because the real tar handles sparse files, xattrs, ACLs and exotic formats |
| `gzip`, `gunzip`, `zcat` | **F** | `flate2` (MIT/Apache) with the miniz_oxide or zlib-rs backend | 219 uses, of which **`-9c` is 173: agents measure compressed size.** A different deflate gives a different number, so **never shadow** a real gzip. Decompression is deterministic either way |
| `unzip` | **F** | `zip` crate (MIT) | 67 uses: `-q -d -p -l -o -t -Z1`. Windows lacks `unzip` (`tar.exe` can read zip). just-bash #157 asked for it |
| `sponge`, `ts`, `ifne` (moreutils) | S | oh-my-pi's ports. moreutils is GPL, so these are reimplementations | Cheap. `sponge` makes `cmd < f | sponge f` safe |
| `dos2unix`, `unix2dos`, `iconv` | S | ours plus `encoding_rs` (MIT/Apache) | Windows text handling. In busybox-w32. `iconv -f -t -c` |
| `install`, `link`, `unlink`, `sync`, `mkfifo` (unix), `logname`, `tty` | S | uutils | Build scripts (`install -D -m`). Trivial |

### 4.4 Tier 3: on demand, fallback-only, or WASM/sandbox only

| Builtin | Mode | Ref | Why only Tier 3 |
|---|---|---|---|
| `file` | F | `infer` (MIT); libmagic `file` is **BSD-2**, usable as a reference | 426 uses, mostly `file X` to check a Mach-O arch or text vs binary. Fidelity means libmagic's compiled database (several MB) and its exact output strings. Fallback gives basic types in WASM and Windows |
| `bc`, `dc` | F | Gavin Howard's bc (BSD-2) as reference | 18 uses. A small language, but standard POSIX. `$(( ))` covers integers |
| `yq` | F | jaq + `jaq-fmts` (yaml/toml/xml/cbor; MIT) | 0 corpus uses (the corpus records 2 effective uses). **Two incompatible `yq`s exist** (mikefarah Go v4 vs kislyuk's Python jq wrapper). If we ship one, use mikefarah semantics on jaq |
| `fd` | F | fd (MIT/Apache) and `ignore` | 30 uses. oh-my-pi has it, but it's not standard |
| `bzip2`/`xz`/`zstd`/`brotli` decompression | F, decompress only, mainly as tar codecs | `bzip2-rs`, `lzma-rs`/`xz2`, `zstd` (MIT/BSD), `brotli` (MIT/BSD) | Compression usage (23 each for zstd and brotli) is benchmarking, where the implementation is the measurement. Never compress via a builtin when a real binary exists |
| `zip` | F | `zip` crate | 1 use |
| `dd` | F | uutils dd | 10 uses (`dd if=/dev/zero of=f bs= count=`). Destructive-capable, so it belongs in the kill-switch group |
| `numfmt`, `fmt`, `pr`, `csplit`, `tsort`, `factor`, `ptx` | S | uutils | Near-zero corpus use, but nearly free from uutils. `factor` has 2 not-found |
| `uuidgen` | S | uutils util-linux / `uuid` crate | |
| `getopt` (util-linux enhanced) | S | ours | BSD vs util-linux `getopt` differ. Scripts using it expect the util-linux form |
| `envsubst` | S | ours | gettext tool used in templating. 0 uses, but common in CI scripts |
| `chown`, `chgrp` | S (no-op on Windows/VFS) | uutils | |
| `clear` | S | ours | Prints the escape sequence. Harmless |
| `pidwait`, `pidof`, `free`, `w`/`who`/`users` | S | uutils procps | Low demand |
| `ast-grep`/`sg` | optional cargo feature, off by default | ast-grep (MIT) | oh-my-pi ships it. Tree-sitter grammars add megabytes. Pi might expose structural search as a tool instead |
| `html-to-markdown` | no (an embedder command) | | just-bash-specific. Not a standard name |

### 4.5 Never builtins

**The criteria.** If any one applies, the name is never a builtin. These rules hold even under "err on the side of too many".

1. **General-purpose interpreters and language runtimes:** `python python3 node deno bun perl ruby php lua tclsh java osascript pwsh/powershell` and similar, and just-bash's `js-exec`.
2. **Toolchains and build systems:** `cc gcc clang rustc cargo go zig swift swiftc tsc make cmake ninja bazel xcodebuild gofmt rustfmt clang-format` and similar.
3. **Package and environment managers:** `npm npx pnpm yarn pip uv uvx brew apt rustup bunx`.
4. **Network and service clients:**
   - **`curl` and `wget`.** The flag surface is huge (curl has 250+ options; the corpus uses `-o -s -H -w --max-time --data-urlencode -L -D`). TLS, proxies, the cert store and auth all matter, and each is a network-effect classification boundary. curl.exe ships with Windows 10+, macOS and every Linux.
   - `ssh scp sftp rsync nc ping dig openssl gh aws gcloud kubectl docker podman`.
   - Database CLIs: `sqlite3 psql mysql redis-cli`.
   - **`git`.**

   Network and database work stays external, so a classifier can spot it.
5. **Stateful engines and databases:** `sqlite3` (owner rule), `postgres`.
6. **Interactive, full-screen or TTY programs:** `less more vi vim nano top htop watch man tmux screen fzf stty tput`.
   - Agents can't drive them, and full-screen output gives garbage in a tool result.
   - `watch` (0 uses) is replaced by loops with `sleep`.
   - `top` is in oh-my-pi, but `ps` covers what agents need.
7. **Privilege, system administration and OS-specific system tools:** `sudo su chroot mount systemctl launchctl crontab sysctl ifconfig networksetup ioreg dmesg lsof xcrun codesign plutil sips open pbcopy say`.
   - These are platform-native by definition. `lsof` (585 uses) is deeply OS-specific.
8. **Heavy domain tools with their own ecosystem:** `ffmpeg ffprobe magick pdftotext pandoc 7z`.
9. **Measurement tools where the implementation is the result:** any compressor in compress mode, when a real one exists (`gzip -9c | wc -c`). Benchmarking tools such as `hyperfine` and `samply`.
10. **Destructive or no-op-in-our-world utilities:** `shred` (pointless on SSD/VFS and dangerous), `mkfs`, `blockdev`, `rsync` (a network protocol as well; oh-my-pi's 63k-line rsync also carries GPL provenance questions).

### 4.6 Disambiguation rules for conflicting GNU and BSD flags

| Command | Conflict | Rule |
|---|---|---|
| `sed -i` | GNU: optional *attached* suffix. BSD: *mandatory separate* suffix, where `''` means none | Next argument `''`, or `-i` attached to text (`-i.bak`), means a suffix. Otherwise, if the next argument parses as a sed script and more operands follow, it is the GNU form. A bare argument that looks like a suffix (no command characters, starts with `.`) followed by a script is BSD. The corpus has 5,307 BSD vs 144 GNU, and both must work |
| `stat -f` | BSD format string vs GNU `--file-system` | An argument containing `%` means a BSD format. `-f` alone with file operands means GNU file system. Offer both `-c` and BSD `-f`/`-t` |
| `date -r` | GNU reference file vs BSD epoch seconds | An all-digit argument that is not an existing file means epoch |
| `base64 -i` | GNU ignore-garbage vs BSD input file | A following non-option operand means the BSD input file |
| `ls -G`, `ls -T` | BSD color / full time vs GNU no-group / tabsize | BSD meaning on macOS hosts and GNU elsewhere? Or always BSD for `-G`, since agents write it for color? **Decide once.** I suggest the macOS meaning, because the GNU meanings are almost never typed |
| `cp -R src/` | BSD copies contents, GNU copies the directory | GNU, documented. The BSD behavior surprises people, and `src/.` is the portable spelling |
| `xargs` with no input | GNU runs the command once (unless `-r`), BSD doesn't | Recommend **BSD semantics** (don't run on empty input). It is the safer behavior for agents, and `-r` stays accepted as a no-op |
| `wc`, `uniq -c` padding | Widths differ | GNU widths |
| `head -n -N`, `tail -r` | Each is single-dialect | Accept both |
| `find` with no path | BSD errors | Default to `.` (GNU) |
| `mktemp -t` | BSD: prefix; GNU: template | An argument containing `XXX` means GNU template, otherwise BSD prefix |
| `md5`/`shasum` vs `md5sum`/`sha256sum` | Output formats | Keyed on argv[0]. Never unify |

### 4.7 Size and order of work

- **Tier 1 is 61 names.**
  - Cheap via uutils, findutils and diffutils: about 45 of them.
  - About 4 are the big ones: grep/rg on the ripgrep crates, sed on uutils sed, jq on jaq, and **awk, which has no good permissive Rust source**.
  - The rest are wrappers on our own process code (`timeout nice nohup env xargs`), which need the shell's process model.
- **Tier 2 is about 45 more** and mostly free once the uutils adapter exists. The exceptions are `ps`/`pgrep`/`pkill` (per-OS process listing), `patch`, `tar`/`gzip`/`unzip` (crates) and `xxd`/`strings`/`column`/`tree` (small and our own).
- **Adapter work.** uutils utilities call `std::io::stdout()` and `std::fs` directly. brush works around this by re-executing itself. oh-my-pi refactored each port onto a `Host` trait (MIT, `crates/pi-builtins/src/host.rs`). For WASM, the injected-I/O route (oh-my-pi's) is the one that works. Budget for it.
- **Conformance gates:**
  - uutils runs the GNU test suite (669/720).
  - jaq ships `jq-tests.jq`.
  - Use the onetrueawk `T.*` tests and goawk's tests for awk, and ripgrep's tests for rg.
  - just-bash's recorded GNU fixtures (Apache) can be reused for grep, sed, awk and jq.

---

## Appendix A: full matrix

Columns:

- **corpus cmds**: commands containing the name as an outer command, out of 295,401.
- **not found**: `command not found` occurrences.
- **bash builtin**: in brush-builtins or the bash language.
- **uutils\***: coreutils, findutils, diffutils, sed, procps or util-linux.
- **busybox**: all 406 applets. **bb-w32**: the 170 applets enabled in busybox-w32 mingw64.

Rows: every name in any implementation set, plus selected externals, filtered to names with at least 1 corpus use or present in at least 3 sets.

| name | corpus cmds | not found | bash builtin | just-bash | omp | brush-cu | Bun | deno | bb-w32 | busybox | uutils* |
|---|--:|--:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|
| `cd` | 140082 |  | ● |  |  |  | ● | ● |  |  |  |
| `head` | 97135 |  |  | ● | ● | ● |  | ● | ● | ● | ● |
| `grep` | 71606 |  |  | ● | ● |  |  |  | ● | ● |  |
| `git` | 70242 | 2 |  |  | ● |  |  |  |  |  |  |
| `rg` | 58283 |  |  | ● | ● |  |  |  |  |  |  |
| `sed` | 47609 | 1 |  | ● | ● |  |  |  | ● | ● | ● |
| `python3` | 31961 | 2 |  | ● |  |  |  |  |  |  |  |
| `node` | 28492 | 3 |  |  |  |  |  |  |  |  |  |
| `tail` | 26457 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `cat` | 24389 |  |  | ● | ● | ● | ● | ● | ● | ● | ● |
| `echo` | 18810 |  | ● | ● |  | ● | ● | ● | ● | ● | ● |
| `ls` | 18197 |  |  | ● | ● | ● | ● |  | ● | ● | ● |
| `npm` | 15871 |  |  |  |  |  |  |  |  |  |  |
| `find` | 9318 |  |  | ● | ● |  |  |  | ● | ● | ● |
| `sleep` | 9081 |  |  | ● | ● | ● |  | ● | ● | ● | ● |
| `printf` | 8531 |  | ● | ● |  | ● |  |  | ● | ● | ● |
| `wc` | 8081 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `sort` | 7995 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `rm` | 7393 |  |  | ● | ● | ● | ● | ● | ● | ● | ● |
| `true` | 6595 |  | ● | ● |  | ● | ● | ● | ● | ● | ● |
| `curl` | 4060 |  |  | ● |  |  |  |  |  |  |  |
| `cut` | 3874 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `cp` | 3810 |  |  | ● | ● | ● | ● | ● | ● | ● | ● |
| `awk` | 3503 |  |  | ● |  |  |  |  | ● | ● |  |
| `mkdir` | 3479 | 1 |  | ● | ● | ● | ● | ● | ● | ● | ● |
| `make` | 2680 | 1 |  |  |  |  |  |  | ● | ● |  |
| `set` | 2170 |  | ● |  |  |  |  | ● |  |  |  |
| `mktemp` | 2036 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `jq` | 2026 |  |  | ● | ● |  |  |  |  |  |  |
| `exit` | 1886 |  | ● |  |  |  | ● | ● |  |  |  |
| `tmux` | 1789 | 5 |  |  |  |  |  |  |  |  |  |
| `uniq` | 1609 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `nice` | 1533 |  |  |  |  |  |  |  |  | ● | ● |
| `docker` | 1527 | 1 |  |  |  |  |  |  |  |  |  |
| `nl` | 1523 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `tr` | 1432 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `time` | 1302 |  | ● | ● |  |  |  |  | ● | ● |  |
| `seq` | 1176 |  |  | ● | ● | ● | ● |  | ● | ● | ● |
| `xargs` | 1009 |  |  | ● | ● |  |  | ● | ● | ● | ● |
| `ps` | 989 |  |  |  | ● |  |  |  |  |  | ● |
| `diff` | 913 |  |  | ● | ● |  |  |  | ● | ● | ● |
| `chmod` | 905 |  |  | ● | ● |  |  |  | ● | ● | ● |
| `python` | 872 | 5 |  | ● |  |  |  |  |  |  |  |
| `pwd` | 829 |  | ● | ● |  | ● | ● | ● | ● | ● | ● |
| `break` | 808 |  | ● |  |  |  |  |  |  |  |  |
| `bash` | 801 |  |  | ● |  |  |  |  | ● | ● |  |
| `pkill` | 760 |  |  |  | ● |  |  |  | ● | ● | ● |
| `date` | 754 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `which` | 744 |  |  | ● | ● |  | ● |  | ● | ● |  |
| `perl` | 698 |  |  |  |  |  |  |  |  |  |  |
| `kill` | 671 | 1 | ● |  | ● |  |  |  |  |  | ● |
| `test` | 670 |  | ● |  |  | ● |  | ● | ● | ● | ● |
| `command` | 605 |  | ● |  |  |  |  |  |  |  |  |
| `uptime` | 597 |  |  |  | ● |  |  |  | ● | ● | ● |
| `lsof` | 585 |  |  |  |  |  |  |  |  | ● |  |
| `nohup` | 580 |  |  |  | ● |  |  |  |  | ● | ● |
| `tar` | 552 |  |  | ● |  |  |  |  | ● | ● |  |
| `du` | 545 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `shasum` | 541 |  |  |  |  |  |  |  |  |  |  |
| `env` | 509 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `mv` | 475 |  |  | ● | ● | ● | ● | ● | ● | ● | ● |
| `timeout` | 468 | 393 |  | ● | ● |  |  |  | ● | ● | ● |
| `file` | 426 |  |  | ● |  |  |  |  |  |  |  |
| `pgrep` | 395 |  |  |  | ● |  |  |  | ● | ● | ● |
| `wait` | 382 |  | ● |  |  |  |  |  |  |  |  |
| `tee` | 357 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `bun` | 346 | 2 |  |  |  |  | ● |  |  |  |  |
| `id` | 331 |  |  |  | ● |  |  |  |  |  | ● |
| `read` | 326 |  | ● |  |  |  |  |  |  |  |  |
| `trap` | 311 | 1 | ● |  |  |  |  |  |  |  |  |
| `ln` | 306 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `basename` | 270 |  |  | ● | ● | ● | ● |  | ● | ● | ● |
| `strings` | 266 |  |  | ● |  |  |  |  | ● | ● |  |
| `paste` | 262 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `stat` | 249 |  |  | ● | ● |  |  |  | ● | ● | ● |
| `cmp` | 247 |  |  |  | ● |  |  |  | ● | ● | ● |
| `gzip` | 219 |  |  | ● |  |  |  |  | ● | ● |  |
| `sh` | 219 |  |  | ● |  |  |  |  |  |  |  |
| `sqlite3` | 199 |  |  | ● |  |  |  |  |  |  |  |
| `touch` | 175 |  |  | ● | ● | ● | ● |  | ● | ● | ● |
| `:` | 165 |  | ● |  |  |  |  | ● |  |  |  |
| `xxd` | 142 |  |  |  |  |  |  |  | ● | ● |  |
| `base64` | 138 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `readlink` | 113 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `dirname` | 106 |  |  | ● | ● | ● | ● |  | ● | ● | ● |
| `sysctl` | 106 |  |  |  |  |  |  |  |  | ● | ● |
| `continue` | 99 |  | ● |  |  |  |  |  |  |  |  |
| `rmdir` | 98 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `.` | 97 |  | ● |  |  |  |  |  |  |  |  |
| `source` | 95 |  | ● |  |  |  |  |  |  |  |  |
| `w` | 83 |  |  |  |  |  |  |  |  |  | ● |
| `uname` | 78 |  |  |  | ● | ● |  |  |  |  | ● |
| `column` | 76 |  |  | ● |  |  |  |  |  |  |  |
| `md5` | 73 |  |  |  |  |  |  |  |  |  |  |
| `comm` | 71 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `rsync` | 71 |  |  |  | ● |  |  |  |  |  |  |
| `unzip` | 67 |  |  |  |  |  |  |  | ● | ● |  |
| `od` | 53 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `openssl` | 46 |  |  |  |  |  |  |  |  |  |  |
| `join` | 37 |  |  | ● |  | ● |  |  | ● | ● | ● |
| `egrep` | 33 |  |  | ● |  |  |  |  | ● | ● |  |
| `ping` | 33 |  |  |  |  |  |  |  | ● | ● |  |
| `nc` | 32 |  |  |  |  |  |  |  | ● | ● |  |
| `patch` | 32 |  |  |  |  |  |  |  | ● | ● |  |
| `fd` | 30 |  |  |  | ● |  |  |  |  |  |  |
| `df` | 24 |  |  |  | ● | ● |  |  | ● | ● | ● |
| `shift` | 24 |  | ● |  |  |  |  |  |  |  |  |
| `brotli` | 23 |  |  |  |  |  |  |  |  |  |  |
| `realpath` | 23 |  |  |  | ● | ● |  |  | ● | ● | ● |
| `zstd` | 23 |  |  |  |  |  |  |  |  |  |  |
| `cpio` | 22 |  |  |  |  |  |  |  | ● | ● |  |
| `exec` | 22 |  | ● |  |  |  |  |  |  |  |  |
| `printenv` | 19 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `bc` | 18 |  |  |  |  |  |  |  | ● | ● |  |
| `fold` | 16 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `type` | 16 |  | ● |  |  |  |  |  |  |  |  |
| `eval` | 15 |  | ● |  |  |  |  |  |  |  |  |
| `httpd` | 15 |  |  |  |  |  |  |  | ● | ● |  |
| `umask` | 15 |  | ● |  |  |  |  |  |  |  |  |
| `nproc` | 13 | 1 |  |  | ● | ● |  |  | ● | ● | ● |
| `yes` | 13 |  |  | ● | ● | ● | ● |  | ● | ● | ● |
| `sha256sum` | 11 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `7z` | 10 |  |  |  |  |  |  |  |  |  |  |
| `dd` | 10 |  |  |  | ● | ● |  |  | ● | ● | ● |
| `return` | 9 |  | ● |  |  |  |  |  |  |  |  |
| `shuf` | 9 | 3 |  |  |  | ● |  |  | ● | ● | ● |
| `top` | 9 |  |  |  | ● |  |  |  |  | ● | ● |
| `ssh` | 8 |  |  |  |  |  |  |  |  |  |  |
| `xz` | 8 |  |  |  |  |  |  |  |  |  |  |
| `man` | 7 |  |  |  |  |  |  |  | ● | ● |  |
| `stty` | 7 |  |  |  |  |  |  |  | ● | ● | ● |
| `disown` | 6 |  | ● |  |  |  |  |  |  |  |  |
| `whois` | 6 |  |  |  |  |  |  |  | ● | ● |  |
| `hexdump` | 5 |  |  |  |  |  |  |  | ● | ● | ● |
| `less` | 5 |  |  |  |  |  |  |  | ● | ● |  |
| `let` | 5 |  | ● |  |  |  |  |  |  |  |  |
| `truncate` | 5 |  |  |  | ● | ● |  |  | ● | ● | ● |
| `ulimit` | 5 | 1 | ● |  |  |  |  |  |  |  |  |
| `whoami` | 5 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `mkfifo` | 4 |  |  |  |  |  |  |  |  | ● | ● |
| `hash` | 3 |  | ● |  |  |  |  |  |  |  |  |
| `ar` | 2 |  |  |  |  |  |  |  | ● | ● |  |
| `arch` | 2 |  |  |  |  | ● |  |  | ● | ● | ● |
| `factor` | 2 | 2 |  |  |  | ● |  |  | ● | ● | ● |
| `gtimeout` | 2 | 2 |  |  |  |  |  |  |  |  |  |
| `jobs` | 2 |  | ● |  |  |  |  |  |  |  |  |
| `mapfile` | 2 | 3 | ● |  |  |  |  |  |  |  |  |
| `popd` | 2 |  | ● |  |  |  |  |  |  |  |  |
| `pushd` | 2 |  | ● |  |  |  |  |  |  |  |  |
| `tree` | 2 |  |  | ● |  |  |  |  |  | ● |  |
| `wget` | 2 |  |  |  |  |  |  |  | ● | ● |  |
| `alias` | 1 |  | ● | ● |  |  |  |  |  |  |  |
| `false` | 1 |  | ● | ● |  | ● | ● | ● | ● | ● | ● |
| `fmt` | 1 |  |  |  |  | ● |  |  |  |  | ● |
| `iconv` | 1 |  |  |  |  |  |  |  | ● | ● |  |
| `link` | 1 |  |  |  |  | ● |  |  | ● | ● | ● |
| `reset` | 1 |  |  |  |  |  |  |  | ● | ● |  |
| `setsid` | 1 | 1 |  |  |  |  |  |  |  | ● | ● |
| `skill` | 1 |  |  |  |  |  |  |  |  |  | ● |
| `tac` | 1 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `ts` | 1 |  |  |  | ● |  |  |  | ● | ● |  |
| `tty` | 1 |  |  |  |  |  |  |  |  | ● | ● |
| `zip` | 1 |  |  |  |  |  |  |  |  |  |  |
| `[` | 0 |  | ● |  |  |  |  |  | ● | ● |  |
| `b2sum` | 0 |  |  |  | ● | ● |  |  |  |  | ● |
| `base32` | 0 |  |  |  | ● | ● |  |  | ● | ● | ● |
| `cal` | 0 |  |  |  |  |  |  |  | ● | ● | ● |
| `cksum` | 0 |  |  |  | ● | ● |  |  | ● | ● | ● |
| `clear` | 0 |  |  | ● |  |  |  |  | ● | ● |  |
| `expand` | 0 |  |  | ● |  | ● |  |  | ● | ● | ● |
| `expr` | 0 |  |  | ● |  | ● |  |  | ● | ● | ● |
| `fgrep` | 0 |  |  | ● |  |  |  |  | ● | ● |  |
| `free` | 0 |  |  |  |  |  |  |  | ● | ● | ● |
| `groups` | 0 |  |  |  |  |  |  |  | ● | ● | ● |
| `gunzip` | 0 |  |  | ● |  |  |  |  | ● | ● |  |
| `hostname` | 0 |  |  | ● | ● | ● |  |  |  |  | ● |
| `install` | 0 |  |  |  |  |  |  |  | ● | ● | ● |
| `logname` | 0 |  |  |  |  |  |  |  | ● | ● | ● |
| `md5sum` | 0 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `more` | 0 |  |  |  |  | ● |  |  |  | ● | ● |
| `pidof` | 0 |  |  |  |  |  |  |  | ● | ● | ● |
| `rev` | 0 |  |  | ● |  |  |  |  | ● | ● | ● |
| `sha1sum` | 0 |  |  | ● | ● | ● |  |  | ● | ● | ● |
| `sha224sum` | 0 |  |  |  | ● | ● |  |  |  |  | ● |
| `sha384sum` | 0 |  |  |  | ● | ● |  |  | ● | ● | ● |
| `sha512sum` | 0 |  |  |  | ● | ● |  |  | ● | ● | ● |
| `shred` | 0 |  |  |  |  | ● |  |  | ● | ● | ● |
| `split` | 0 |  |  | ● |  | ● |  |  | ● | ● | ● |
| `sum` | 0 |  |  |  |  | ● |  |  | ● | ● | ● |
| `sync` | 0 |  |  |  |  | ● |  |  | ● | ● | ● |
| `tsort` | 0 |  |  |  |  | ● |  |  | ● | ● | ● |
| `unexpand` | 0 |  |  | ● |  | ● |  |  | ● | ● | ● |
| `unlink` | 0 |  |  |  |  | ● |  |  | ● | ● | ● |
| `uuidgen` | 0 |  |  |  |  |  |  |  | ● | ● | ● |
| `watch` | 0 |  |  |  |  |  |  |  | ● | ● | ● |
| `zcat` | 0 |  |  | ● |  |  |  |  | ● | ● |  |

## Appendix B: reproduce

- just-bash additions: `git log --diff-filter=A --name-only --reverse -- 'packages/just-bash/src/commands/*' 'src/commands/*'` in `/tmp/just-bash`, taking the first date per directory.
- Issues and PRs: `gh issue list -R vercel-labs/just-bash --state all --limit 1000 --json number,title,body,state,stateReason,labels,url,comments` (194), and `gh pr list … --state all` (543). Snapshots are in `/tmp/jb-issues.json` and `/tmp/jb-prs.json`.
- busybox-w32: `//applet:IF_X(APPLET(name…` markers joined with `configs/mingw64_defconfig` in `/tmp/ls/busybox-w32`.
- oh-my-pi: `crates/pi-builtins/Cargo.toml` `util.*` features and `crates/pi-shell/src/tools/`.
- brush: `brush-builtins/src/*.rs` and the `uu_*` dependencies in `brush-coreutils-builtins/Cargo.toml`.
- Bun: `src/runtime/shell/builtin/` and `docs/runtime/shell.mdx`. deno_task_shell: `src/shell/commands/mod.rs`.
- Corpus recount: `names`, `eff`, `flags` and `not_found` from `~/.cache/psh-corpus/features.jsonl` (private, aggregates only). The working files are in `/tmp/bi/`.
