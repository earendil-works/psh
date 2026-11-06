## T1 Sources
| source | commands | sessions | time range | bytes p50 | p90 | p99 |
|---|---|---|---|---|---|---|
| agent | 247561 | 5720 | 2026-03-16 → 2026-11-06 | 157 | 677 | 4000 |
| agent-codemode | 2989 | 158 | 2026-09-25 → 2026-11-05 | 112 | 421 | 789 |
| p2 | 44748 | 229 | 2026-10-12 → 2026-11-06 | 243 | 1815 | 6495 |
| p2-codemode | 103 | 5 | 2026-10-21 → 2026-11-06 | 152 | 160 | 169 |
| **total** | 295401 | 6112 |  |  |  |  |

Lines per command: 1: 83.5%, 2-5: 2.1%, 6-20: 7.6%, 21-100: 5.9%, >100: 0.9%

## T2 Top command names (first word of every simple command; % of commands containing it)
| # | name | commands | % of cmds | occurrences |
|---|---|---|---|---|
| 1 | `cd` | 140082 | 47.4% | 150610 |
| 2 | `head` | 97135 | 32.9% | 124153 |
| 3 | `grep` | 71606 | 24.2% | 108020 |
| 4 | `git` | 70242 | 23.8% | 128703 |
| 5 | `rg` | 58283 | 19.7% | 69504 |
| 6 | `sed` | 47609 | 16.1% | 65654 |
| 7 | `python3` | 31961 | 10.8% | 34565 |
| 8 | `node` | 28492 | 9.6% | 32808 |
| 9 | `tail` | 26457 | 9.0% | 32291 |
| 10 | `<local-script>` | 24521 | 8.3% | 38202 |
| 11 | `cat` | 24389 | 8.3% | 30982 |
| 12 | `echo` | 18810 | 6.4% | 25629 |
| 13 | `ls` | 18197 | 6.2% | 21481 |
| 14 | `npm` | 15871 | 5.4% | 19494 |
| 15 | `cargo` | 13333 | 4.5% | 18835 |
| 16 | `find` | 9318 | 3.2% | 10663 |
| 17 | `sleep` | 9081 | 3.1% | 12110 |
| 18 | `printf` | 8531 | 2.9% | 14559 |
| 19 | `gh` | 8128 | 2.8% | 10053 |
| 20 | `wc` | 8081 | 2.7% | 9142 |
| 21 | `sort` | 7995 | 2.7% | 9816 |
| 22 | `rm` | 7393 | 2.5% | 7886 |
| 23 | `true` | 6595 | 2.2% | 9046 |
| 24 | `npx` | 5427 | 1.8% | 6353 |
| 25 | `curl` | 4060 | 1.4% | 5112 |
| 26 | `cut` | 3874 | 1.3% | 4265 |
| 27 | `uv` | 3815 | 1.3% | 4246 |
| 28 | `cp` | 3810 | 1.3% | 5413 |
| 29 | `awk` | 3503 | 1.2% | 4315 |
| 30 | `mkdir` | 3479 | 1.2% | 3617 |
| 31 | `<dynamic>` | 2957 | 1.0% | 5738 |
| 32 | `make` | 2680 | 0.9% | 3330 |
| 33 | `set` | 2170 | 0.7% | 2326 |
| 34 | `mktemp` | 2036 | 0.7% | 2161 |
| 35 | `jq` | 2026 | 0.7% | 2467 |
| 36 | `exit` | 1886 | 0.6% | 2434 |
| 37 | `tmux` | 1789 | 0.6% | 5661 |
| 38 | `pnpm` | 1701 | 0.6% | 2239 |
| 39 | `uniq` | 1609 | 0.5% | 1736 |
| 40 | `nice` | 1533 | 0.5% | 2014 |
| 41 | `docker` | 1527 | 0.5% | 2015 |
| 42 | `nl` | 1523 | 0.5% | 2563 |
| 43 | `tr` | 1432 | 0.5% | 1679 |
| 44 | `time` | 1302 | 0.4% | 1498 |
| 45 | `seq` | 1176 | 0.4% | 1324 |
| 46 | `go` | 1147 | 0.4% | 1526 |
| 47 | `xargs` | 1009 | 0.3% | 1100 |
| 48 | `ps` | 989 | 0.3% | 1145 |
| 49 | `xcrun` | 914 | 0.3% | 1577 |
| 50 | `diff` | 913 | 0.3% | 1058 |
| 51 | `chmod` | 905 | 0.3% | 955 |
| 52 | `python` | 872 | 0.3% | 894 |
| 53 | `pwd` | 829 | 0.3% | 847 |
| 54 | `break` | 808 | 0.3% | 921 |
| 55 | `bash` | 801 | 0.3% | 859 |
| 56 | `pkill` | 760 | 0.3% | 903 |
| 57 | `date` | 754 | 0.3% | 918 |
| 58 | `which` | 744 | 0.3% | 936 |
| 59 | `perl` | 698 | 0.2% | 932 |
| 60 | `kill` | 671 | 0.2% | 819 |
| 61 | `test` | 670 | 0.2% | 817 |
| 62 | `command` | 605 | 0.2% | 1098 |
| 63 | `uptime` | 597 | 0.2% | 635 |
| 64 | `lsof` | 585 | 0.2% | 691 |
| 65 | `nohup` | 580 | 0.2% | 604 |
| 66 | `uvx` | 564 | 0.2% | 629 |
| 67 | `tar` | 552 | 0.2% | 610 |
| 68 | `du` | 545 | 0.2% | 598 |
| 69 | `shasum` | 541 | 0.2% | 609 |
| 70 | `env` | 509 | 0.2% | 532 |
| 71 | `pi` | 507 | 0.2% | 560 |
| 72 | `sips` | 482 | 0.2% | 612 |
| 73 | `mv` | 475 | 0.2% | 668 |
| 74 | `timeout` | 468 | 0.2% | 490 |
| 75 | `psql` | 465 | 0.2% | 536 |
| 76 | `file` | 426 | 0.1% | 432 |
| 77 | `pgrep` | 395 | 0.1% | 433 |
| 78 | `wait` | 382 | 0.1% | 413 |
| 79 | `tee` | 357 | 0.1% | 386 |
| 80 | `bun` | 346 | 0.1% | 390 |
| 81 | `id` | 331 | 0.1% | 869 |
| 82 | `read` | 326 | 0.1% | 336 |
| 83 | `trap` | 311 | 0.1% | 340 |
| 84 | `ln` | 306 | 0.1% | 430 |
| 85 | `gofmt` | 297 | 0.1% | 301 |
| 86 | `ffmpeg` | 273 | 0.1% | 383 |
| 87 | `basename` | 270 | 0.1% | 314 |
| 88 | `ioreg` | 270 | 0.1% | 292 |
| 89 | `rustc` | 266 | 0.1% | 297 |
| 90 | `strings` | 266 | 0.1% | 306 |

## T2b Top subcommands
| subcommand | commands | % |
|---|---|---|
| `git status` | 26015 | 8.8% |
| `git diff` | 26007 | 8.8% |
| `git log` | 14543 | 4.9% |
| `npm run` | 10743 | 3.6% |
| `git show` | 7743 | 2.6% |
| `git commit` | 7481 | 2.5% |
| `git add` | 7160 | 2.4% |
| `git rev-parse` | 4812 | 1.6% |
| `cargo build` | 4390 | 1.5% |
| `cargo test` | 4170 | 1.4% |
| `uv run` | 3957 | 1.3% |
| `cargo fmt` | 3045 | 1.0% |
| `npm test` | 3044 | 1.0% |
| `git branch` | 2517 | 0.9% |
| `gh issue` | 2514 | 0.9% |
| `gh api` | 1834 | 0.6% |
| `gh run` | 1819 | 0.6% |
| `cargo run` | 1555 | 0.5% |
| `gh pr` | 1530 | 0.5% |
| `cargo clippy` | 1442 | 0.5% |
| `git checkout` | 1290 | 0.4% |
| `cargo check` | 1251 | 0.4% |
| `git push` | 1249 | 0.4% |
| `npx oxfmt` | 1245 | 0.4% |
| `tmux capture-pane` | 1222 | 0.4% |
| `docker run` | 1176 | 0.4% |
| `git worktree` | 1142 | 0.4% |
| `npx tsc` | 1129 | 0.4% |
| `git rebase` | 1123 | 0.4% |
| `tmux send-keys` | 1024 | 0.3% |
| `git ls-files` | 1001 | 0.3% |
| `git stash` | 949 | 0.3% |
| `npx wrangler` | 900 | 0.3% |
| `git fetch` | 804 | 0.3% |
| `tmux kill-session` | 784 | 0.3% |
| `git remote` | 770 | 0.3% |
| `npx vitest` | 757 | 0.3% |
| `npx tsx` | 729 | 0.2% |
| `npm exec` | 716 | 0.2% |
| `xcrun simctl` | 716 | 0.2% |
| `go test` | 714 | 0.2% |
| `npm view` | 705 | 0.2% |
| `npm install` | 605 | 0.2% |
| `tmux new-session` | 591 | 0.2% |
| `git tag` | 584 | 0.2% |
| `git merge-base` | 464 | 0.2% |
| `git grep` | 453 | 0.2% |
| `pnpm run` | 440 | 0.1% |
| `git blame` | 417 | 0.1% |
| `npm pack` | 414 | 0.1% |
| `cargo +nightly` | 407 | 0.1% |
| `gh search` | 406 | 0.1% |
| `pnpm exec` | 381 | 0.1% |
| `npx tsgo` | 372 | 0.1% |
| `pnpm test` | 367 | 0.1% |
| `git merge` | 346 | 0.1% |
| `cargo doc` | 315 | 0.1% |
| `npx oxlint` | 293 | 0.1% |
| `npm ls` | 286 | 0.1% |
| `npx biome` | 281 | 0.1% |

## T3 Coverage: commands made up only of the top-N utilities
| N | all | agent | p2 | agent-codemode |
|---|---|---|---|---|
| 5 | 28.9% | 31.5% | 15.6% | 15.2% |
| 10 | 52.0% | 52.4% | 50.7% | 40.5% |
| 20 | 75.6% | 75.3% | 77.2% | 75.2% |
| 30 | 84.5% | 84.5% | 84.7% | 82.3% |
| 50 | 91.7% | 91.5% | 92.2% | 94.0% |
| 100 | 97.6% | 97.6% | 97.4% | 99.1% |
| 200 | 99.4% | 99.3% | 99.5% | 99.8% |

Top-20 (by command frequency, wrappers unwrapped): `cd`, `head`, `grep`, `git`, `rg`, `sed`, `python3`, `node`, `tail`, `<local-script>`, `cat`, `echo`, `ls`, `npm`, `cargo`, `find`, `sleep`, `printf`, `wc`, `gh`

Top 21–50: `sort`, `rm`, `true`, `npx`, `uv`, `curl`, `cut`, `cp`, `awk`, `mkdir`, `<dynamic>`, `make`, `set`, `jq`, `mktemp`, `exit`, `tmux`, `pnpm`, `uniq`, `docker`, `nice`, `nl`, `tr`, `time`, `seq`, `go`, `xargs`, `ps`, `xcrun`, `diff`

## T3b Candidate in-process builtin set
Builtin set (117 names): `.` `:` `[` `[[` `alias` `awk` `base64` `basename` `bc` `break` `cat` `cd` `chmod` `cksum` `cmp` `column` `comm` `command` `continue` `cp` `cut` `date` `declare` `df` `diff` `dirname` `du` `echo` `egrep` `env` `eval` `exit` `export` `expr` `false` `fgrep` `file` `find` `fold` `grep` `gtimeout` `hash` `head` `hexdump` `hostname` `id` `join` `jq` `kill` `let` `ln` `local` `ls` `md5` `md5sum` `mkdir` `mktemp` `mv` `nice` `nl` `nohup` `nproc` `od` `paste` `popd` `printenv` `printf` `pushd` `pwd` `read` `readlink` `readonly` `realpath` `return` `rev` `rg` `rm` `rmdir` `sed` `seq` `set` `sha1sum` `sha256sum` `shasum` `shift` `shopt` `sleep` `sort` `source` `split` `stat` `tac` `tail` `tee` `test` `time` `timeout` `touch` `tr` `trap` `tree` `true` `truncate` `type` `typeset` `ulimit` `umask` `uname` `uniq` `unset` `wait` `wc` `which` `whoami` `xargs` `xxd` `yes`

| metric | all | agent | p2 |
|---|---|---|---|
| only builtin-set utilities | 37.59% | 36.85% | 42.13% |
| builtin set + at most one external program | 85.22% | 87.37% | 72.95% |

Distinct external (non-builtin) programs per command: 0: 37.6%, 1: 47.6%, 2: 11.3%, 3+: 3.5%

Most common external programs: `git` 23.8%, `python3` 10.9%, `node` 9.8%, `<local-script>` 8.7%, `npm` 5.4%, `cargo` 4.8%, `gh` 2.8%, `npx` 1.9%, `uv` 1.4%, `curl` 1.4%, `<dynamic>` 1.1%, `make` 0.9%, `tmux` 0.6%, `pnpm` 0.6%, `docker` 0.5%, `go` 0.4%, `ps` 0.3%, `xcrun` 0.3%, `python` 0.3%, `bash` 0.3%, `pkill` 0.3%, `perl` 0.2%, `uptime` 0.2%, `lsof` 0.2%, `uvx` 0.2%, `tar` 0.2%, `pi` 0.2%, `sips` 0.2%, `psql` 0.2%, `pgrep` 0.1%

## T4 Categories (multi-label: % of commands touching the category)
| category | all (multi) | agent | p2 | primary (exclusive) |
|---|---|---|---|---|
| read-only | 81.1% | 78.4% | 96.1% | 51.7% |
| cd | 47.4% | 41.5% | 81.9% | 0.0% |
| script-run | 16.8% | 14.0% | 32.4% | 7.7% |
| build-test | 15.5% | 15.9% | 14.0% | 11.7% |
| interp-inline | 13.1% | 11.6% | 21.8% | 8.1% |
| file-mutate | 11.3% | 9.6% | 21.0% | 10.1% |
| echo | 10.6% | 10.5% | 10.6% | 0.1% |
| network | 5.3% | 5.9% | 1.1% | 3.5% |
| vcs-mutate | 4.9% | 4.0% | 9.6% | 4.8% |
| process | 3.9% | 4.3% | 1.8% | 1.2% |
| shell-builtin | 1.5% | 1.7% | 0.2% | 0.0% |
| dynamic | 1.1% | 1.1% | 0.9% | 0.3% |
| other | 1.0% | 0.9% | 1.1% | 0.3% |
| pkg-install | 0.5% | 0.5% | 0.1% | 0.5% |
| remote-vm | 0.1% | 0.1% | 0.0% | 0.1% |
| shell-c | 0.1% | 0.1% | 0.2% | 0.0% |
| long-running | 0.1% | 0.1% | 0.0% | 0.0% |
| windows | 0.0% | 0.0% | 0.0% | 0.0% |
| interactive-or-gui | 0.0% | 0.0% | 0.0% | 0.0% |
| empty-or-assign | 0.0% | 0.0% | 0.0% | 0.0% |

Pure read-only commands (only read-only utilities, cd, echo, builtins; no file redirect): 51.7% (agent 52.8%, p2 45.3%)

## T5 Shell syntax features (% of commands using it at least once)
| feature | commands | all | agent | p2 |
|---|---|---|---|---|
| `&&` | 163914 | 55.5% | 51.7% | 78.5% |
| pipeline `|` | 139218 | 47.1% | 42.8% | 72.2% |
| starts with `cd X && …`/`cd X;` | 135549 | 45.9% | 40.0% | 80.2% |
| double-quoted string | 132794 | 45.0% | 43.2% | 57.0% |
| multiple top-level statements (`;`/newline) | 123558 | 41.8% | 35.9% | 75.2% |
| `;` separator | 108559 | 36.7% | 31.5% | 66.0% |
| single-quoted string | 86599 | 29.3% | 30.2% | 22.5% |
| fd dup `2>&1` / `>&2` | 44826 | 15.2% | 11.7% | 35.1% |
| here-doc `<<` | 39509 | 13.4% | 11.2% | 26.1% |
| here-doc with quoted delimiter `<<'EOF'` | 39405 | 13.3% | 11.2% | 26.1% |
| here-doc fed to python/node/sh | 32005 | 10.8% | 9.2% | 20.5% |
| `$VAR` | 25138 | 8.5% | 7.8% | 12.6% |
| redirect to /dev/null | 22531 | 7.6% | 7.5% | 8.5% |
| redirect stdout to file (`>`, `>>`, `&>`) | 19831 | 6.7% | 5.6% | 13.1% |
| unquoted glob word (`*`, `?`, `[..]`) | 18828 | 6.4% | 5.4% | 11.9% |
| `2>/dev/null` | 15568 | 5.3% | 5.1% | 6.2% |
| command substitution `$(…)` | 14750 | 5.0% | 5.3% | 3.6% |
| plain assignment `x=…` | 14137 | 4.8% | 4.8% | 4.8% |
| redirect into /tmp | 11607 | 3.9% | 4.0% | 3.8% |
| `for … in` | 10366 | 3.5% | 3.2% | 5.5% |
| env-assignment prefix `FOO=1 cmd` | 9312 | 3.2% | 2.2% | 8.6% |
| `cat <<EOF` (> file) | 8834 | 3.0% | 2.3% | 7.0% |
| `||` | 8383 | 2.8% | 3.3% | 0.7% |
| subshell `( … )` | 3565 | 1.2% | 1.0% | 2.4% |
| background `&` | 2116 | 0.7% | 0.7% | 1.1% |
| `${…}` | 1866 | 0.6% | 0.7% | 0.4% |
| append `>>` | 1824 | 0.6% | 0.3% | 2.2% |
| `set -e` | 1824 | 0.6% | 0.7% | 0.0% |
| `if` | 1807 | 0.6% | 0.7% | 0.2% |
| `export`/`local`/`declare` | 1666 | 0.6% | 0.5% | 1.0% |
| `[ … ]` | 1510 | 0.5% | 0.6% | 0.3% |
| `${x:-y}`/`${x#…}`/`${x/…}` ops | 1368 | 0.5% | 0.5% | 0.3% |
| backslash line continuation | 1156 | 0.4% | 0.3% | 0.7% |
| `set -u` | 947 | 0.3% | 0.4% | 0.0% |
| `set -o pipefail` | 918 | 0.3% | 0.4% | 0.0% |
| group `{ …; }` | 903 | 0.3% | 0.3% | 0.6% |
| input redirect `<` | 700 | 0.2% | 0.2% | 0.4% |
| function definition | 503 | 0.2% | 0.1% | 0.4% |
| `$(( … ))` | 473 | 0.2% | 0.2% | 0.2% |
| comment `#` | 370 | 0.1% | 0.1% | 0.0% |
| `while`/`until` | 343 | 0.1% | 0.1% | 0.1% |
| tree-sitter parse error | 321 | 0.1% | 0.1% | 0.1% |
| process substitution `<(…)` | 317 | 0.1% | 0.1% | 0.2% |
| here-doc to other command (git/gh/psql/…) | 212 | 0.1% | 0.1% | 0.1% |
| array ref `${a[@]}` | 203 | 0.1% | 0.1% | 0.0% |
| `$'…'` ANSI-C string | 190 | 0.1% | 0.1% | 0.1% |
| here-doc unquoted (expanding) | 186 | 0.1% | 0.1% | 0.0% |
| `! cmd` | 156 | 0.1% | 0.1% | 0.0% |
| `case` | 129 | 0.0% | 0.0% | 0.0% |
| brace expansion `{a,b}`/`{1..3}` | 83 | 0.0% | 0.0% | 0.0% |
| backticks | 78 | 0.0% | 0.0% | 0.0% |
| array literal `=( … )` | 60 | 0.0% | 0.0% | 0.0% |
| `[[ … ]]` | 50 | 0.0% | 0.0% | 0.0% |
| here-string `<<<` | 42 | 0.0% | 0.0% | 0.0% |
| `set -x` | 15 | 0.0% | 0.0% | 0.0% |
| `for ((…))` | 3 | 0.0% | 0.0% | 0.0% |
| here-doc `<<-` | 0 | 0.0% | 0.0% | 0.0% |
| `(( … ))` | 0 | 0.0% | 0.0% | 0.0% |

## T5b Output limiting / error suppression idioms
| idiom | commands | % |
|---|---|---|
| any output limiter (head/tail/sed -n N,Mp) | 138312 | 46.8% |
| pipes into head | 95402 | 32.3% |
| sed -n 'N,Mp' file (paged read) | 38318 | 13.0% |
| pipes into tail | 23384 | 7.9% |
| 2>&1 | head/tail | 19871 | 6.7% |
| 2>/dev/null | 15877 | 5.4% |
| || true / || : (ignore failure) | 6785 | 2.3% |
| ; echo "exit=$?" style status echo | 2175 | 0.7% |

## T5c bash-4+/zsh-specific constructs (matter for /bin/bash 3.2 and zsh)
| construct | commands |
|---|---|
| |& (pipe stderr) | 582 |
| zsh-only syntax (=(..), ${(f)..}, setopt) | 90 |
| read -d '' / -a | 14 |
| &> (redirect both) | 11 |
| coproc | 7 |
| [[ =~ ]] regex | 5 |
| $EPOCHSECONDS/$SRANDOM/BASH_REMATCH | 5 |
| declare/local -A (assoc array) | 4 |
| mapfile/readarray | 2 |

(Regex on raw text: most `|&` hits are inside quoted strings/regexes; only ~20 such commands failed.)

## T5d Parse validity
| source | checked | bash -n fail | % | zsh -n fail | % |
|---|---|---|---|---|---|
| agent | 247561 | 202 | 0.1% | 446 | 0.2% |
| p2 | 44748 | 5 | 0.0% | 40 | 0.1% |

tree-sitter-bash vs `bash -n`: ts_err=False,bash_ok=True: 295080, ts_err=True,bash_ok=True: 114, ts_err=True,bash_ok=False: 207

## T6 GNU/BSD-sensitive flag usage (commands; with result; failed)
| pattern | commands | with result | failed | fail % |
|---|---|---|---|---|
| sed -n 'X,Yp' (range print) | 38664 | 38173 | 458 | 1.2% |
| grep -E | 19457 | 19393 | 602 | 3.1% |
| grep -r/-R | 9757 | 9729 | 242 | 2.5% |
| wc -l | 7380 | 7287 | 223 | 3.1% |
| find -maxdepth/-mindepth | 6277 | 6238 | 222 | 3.6% |
| sed -i (any) | 5472 | 5466 | 206 | 3.8% |
| sed -i '' (BSD form) | 5307 | 5301 | 171 | 3.2% |
| rm -rf | 3664 | 3659 | 280 | 7.7% |
| mktemp -d | 1741 | 1741 | 225 | 12.9% |
| grep -o | 1418 | 1412 | 50 | 3.5% |
| grep --include/--exclude | 1405 | 1398 | 58 | 4.1% |
| seq | 1357 | 1347 | 218 | 16.2% |
| head -c | 1265 | 1040 | 42 | 4.0% |
| ps aux / ps -ef | 530 | 527 | 27 | 5.1% |
| timeout (GNU coreutils; not on stock macOS) | 496 | 496 | 55 | 11.1% |
| lsof -i / -ti | 449 | 435 | 7 | 1.6% |
| sed -E / -r | 443 | 442 | 14 | 3.2% |
| stat -c / --format (GNU) | 398 | 197 | 5 | 2.5% |
| cp -a / -r / -R | 360 | 359 | 26 | 7.2% |
| ln -sf / -sfn | 353 | 352 | 52 | 14.8% |
| stat -f (BSD) | 327 | 325 | 11 | 3.4% |
| find -exec … \; | 319 | 318 | 13 | 4.1% |
| xargs -0 | 243 | 242 | 15 | 6.2% |
| find -print0 | 226 | 225 | 10 | 4.4% |
| date +%s | 200 | 200 | 31 | 15.5% |
| date -u / -Iseconds / --iso | 167 | 165 | 12 | 7.3% |
| find -newer/-mtime/-mmin | 148 | 146 | 1 | 0.7% |
| sed -i <script> (GNU form; breaks on BSD) | 144 | 144 | 35 | 24.3% |
| base64 -d/--decode | 142 | 136 | 9 | 6.6% |
| echo -e / -n | 141 | 141 | 5 | 3.5% |
| sed GNU-only escapes (\+ \? \| in BRE, \s) | 124 | 124 | 8 | 6.5% |
| tar --exclude / GNU long opts | 122 | 122 | 9 | 7.4% |
| tail -n +N | 113 | 112 | 6 | 5.4% |
| md5 / shasum (BSD/perl) | 88 | 88 | 5 | 5.7% |
| xargs -I | 86 | 85 | 3 | 3.5% |
| find -exec … {} + | 81 | 81 | 2 | 2.5% |
| realpath | 72 | 71 | 5 | 7.0% |
| sysctl -n hw.ncpu (macOS) | 63 | 63 | 4 | 6.3% |
| sort -V (version) | 58 | 58 | 2 | 3.4% |
| find -printf (GNU) | 48 | 48 | 5 | 10.4% |
| sort -h | 42 | 42 | 3 | 7.1% |
| readlink -f | 36 | 36 | 6 | 16.7% |
| xargs -P | 35 | 35 | 0 | 0.0% |
| xargs -r / --no-run-if-empty (GNU) | 32 | 31 | 0 | 0.0% |
| nproc (GNU) | 26 | 26 | 4 | 15.4% |
| date +%N (GNU nanoseconds) | 24 | 24 | 0 | 0.0% |
| grep -P (PCRE; not in BSD grep) | 21 | 21 | 4 | 19.0% |
| sed -i.ext (suffix attached, portable) | 21 | 21 | 1 | 4.8% |
| find -delete | 20 | 20 | 1 | 5.0% |
| gtimeout | 19 | 19 | 10 | 52.6% |
| ls --color/--sort/--time-style (GNU) | 18 | 17 | 1 | 5.9% |
| date -v / -j -f (BSD) | 14 | 14 | 3 | 21.4% |
| cp --parents / -t (GNU) | 10 | 10 | 0 | 0.0% |
| find without path (GNU default .) | 9 | 9 | 0 | 0.0% |
| pbcopy/pbpaste/open/osascript (macOS-only) | 7 | 7 | 0 | 0.0% |
| install -D (GNU) | 5 | 5 | 0 | 0.0% |
| find -regex / -regextype | 3 | 3 | 0 | 0.0% |
| awk gawk-only (gensub/strftime/asort/-i inplace) | 3 | 3 | 2 | 66.7% |
| date -d / --date (GNU) | 2 | 2 | 0 | 0.0% |
| md5sum/sha256sum (GNU) | 2 | 2 | 0 | 0.0% |
| tac (GNU) | 1 | 1 | 0 | 0.0% |
| head -n -N (GNU negative) | 1 | 1 | 1 | 100.0% |
| base64 -D (BSD) / -w (GNU) | 1 | 1 | 0 | 0.0% |

## T6b Top flags per utility
| utility | flags (count) |
|---|---|
| `grep` | `-n`×44964 `-E`×20362 `-v`×13800 `-rn`×9752 `-i`×4840 `--include`×4683 `-A8`×2591 `-c`×2194 `-A12`×1940 `-A6`×1895 `-A3`×1813 `-A5`×1480 `-A30`×1230 `-o`×1191 |
| `rg` | `-n`×62220 `--glob`×15942 `-g`×6369 `-i`×3420 `-S`×2937 `-C`×2234 `-l`×1578 `--type`×1431 `-o`×802 `-A`×696 `-ln`×450 `-v`×403 `--hidden`×391 `-il`×365 |
| `sed` | `-n`×56496 `-i`×6042 `-E`×505 `-e`×498 `-i.bak`×13 `-nE`×5 `-i.b2`×1 |
| `head` | `-20`×14418 `-30`×13122 `-40`×9180 `-60`×5629 `-80`×5545 `-3`×5403 `-5`×5175 `-100`×4647 `-50`×4026 `-n`×3476 `-200`×3199 `-1`×2281 `-120`×2003 `-300`×1968 |
| `tail` | `-1`×5924 `-3`×5202 `-2`×3666 `-5`×3423 `-30`×1954 `-20`×1679 `-15`×1484 `-8`×1231 `-40`×1045 `-4`×820 `-n`×757 `-12`×685 `-6`×579 `-25`×536 |
| `find` | `-type`×7794 `-maxdepth`×6895 `-name`×6517 `-o`×3936 `-print`×3443 `-path`×2276 `-iname`×1465 `-not`×748 `-exec`×418 `-prune`×344 `-print0`×241 `-c`×120 `-mindepth`×115 `-l`×65 |
| `ls` | `-la`×3815 `-l`×739 `-lh`×495 `-t`×393 `-d`×379 `-ld`×166 `-R`×144 `-1`×139 `-a`×99 `-lt`×67 `-td`×59 `-S`×27 `-lah`×19 `--time-style`×17 |
| `cat` | `-n`×96 `-v`×75 `-A`×60 `-vet`×18 `-et`×4 `-e`×4 `-`×3 `-t`×1 |
| `sort` | `-u`×986 `-rn`×771 `-n`×481 `-nr`×238 `-k2`×213 `-V`×67 `-r`×61 `-h`×44 `-k3`×38 `-t`×16 `-k4`×10 `-o`×6 `-k1`×6 `-R`×6 |
| `wc` | `-l`×8100 `-c`×1391 `-w`×87 `-lc`×33 `-rf`×1 `-L`×1 `-H`×1 |
| `cut` | `-c1-200`×742 `-f1`×562 `-c1-300`×425 `-c1-250`×270 `-c1-400`×210 `-c1-160`×185 `-c1-120`×182 `-c1-150`×180 `-c1-220`×166 `-c1-600`×127 `-d`×107 `-f2`×92 `-c1-80`×82 `-c1-140`×70 |
| `awk` | `-v`×208 `-F`×143 `-F.`×5 `-F-`×2 `-Fsize`×2 `-Fjid`×1 `-Fland.`×1 `-Fc.get`×1 `-F_`×1 `-F__private_`×1 |
| `xargs` | `-0`×262 `-n1`×77 `-r`×30 `-P`×29 `-n`×8 `-L`×6 `-n2`×3 `-P8`×3 `-P20`×2 `-P4`×1 `-P6`×1 `-n3`×1 `-L1`×1 `-P10`×1 |
| `git` | `--short`×25888 `--stat`×14904 `--oneline`×14316 `-m`×7041 `--check`×6826 `-q`×5879 `--format`×4889 `--show-toplevel`×4351 `-C`×4129 `-1`×3821 `--cached`×2916 `-A`×2606 `--branch`×2423 `--name-only`×1982 |
| `cargo` | `-p`×10197 `-q`×7385 `--all-features`×4310 `--release`×2733 `--workspace`×1924 `--all`×1772 `--all-targets`×1743 `--test`×1572 `-D`×1196 `--features`×981 `--no-default-features`×929 `--lib`×783 `--check`×625 `--target`×603 |
| `npm` | `--json`×929 `--ignore-scripts`×627 `--silent`×347 `-w`×304 `--run`×297 `--prefix`×246 `--stamp`×243 `--write`×199 `--dry-run`×196 `--workspace`×182 `--package-lock-only`×166 `--depth`×161 `--config`×155 `--no-audit`×142 |
| `curl` | `-o`×1928 `-s`×1808 `-H`×1512 `-sS`×1214 `-w`×935 `--max-time`×664 `--data-urlencode`×570 `-L`×476 `-fsSL`×439 `-fsS`×424 `-X`×402 `-sL`×339 `-D`×283 `-A`×216 |
| `python3` | `-c`×2256 `-m`×357 `--wasm`×340 `-`×233 `-n`×124 `--top`×117 `--check`×115 `--slim`×88 `--rfel`×74 `--boot`×58 `-a`×58 `--noqjs`×53 `--update`×39 `-v`×35 |
| `node` | `--run`×6957 `-e`×3914 `--test`×3650 `--check`×1793 `--input-type`×1659 `--import`×820 `-p`×520 `--purpose`×445 `--version`×378 `--timeout`×356 `--experimental-strip-types`×327 `--vmm`×263 `-t`×251 `--score`×245 |
| `rm` | `-rf`×3598 `-f`×2087 `-r`×10 |
| `cp` | `-R`×192 `-r`×135 `-n`×13 `-a`×8 `-p`×6 `-c`×3 `-Rp`×1 `-RL`×1 `-Rn`×1 `-v`×1 `-cR`×1 |
| `mkdir` | `-p`×3257 `-m`×1 |
| `date` | `-u`×123 `-r`×17 `-j`×5 `-f`×5 `-v-60M`×3 `-d`×2 `-Iseconds`×1 `-v-35S`×1 `-v-30M`×1 `-Is`×1 `-v-3H`×1 `-v-90d`×1 `-v-3d`×1 `-s`×1 |
| `tr` | `-d`×377 `-s`×36 `-`×9 `-c`×4 `-dc`×3 `-cd`×3 |
| `uniq` | `-c`×1609 `-d`×22 `-u`×5 `-f1`×1 `-name`×1 `-not`×1 `-path`×1 `-f2`×1 |
| `jq` | `-r`×990 `-c`×453 `-s`×46 `--arg`×39 `-e`×18 `-rc`×13 `-rs`×13 `--argjson`×6 `-n`×6 `-S`×4 `-sr`×3 `-cj`×3 `-er`×2 `--sort-keys`×2 |
| `echo` | `-n`×143 `-e`×5 `--stderr--`×2 `--pi.dev--`×2 `-i`×2 `-`×2 `--all`×1 `--all-targets`×1 `--target`×1 `-D`×1 `--side`×1 `-R`×1 `--json`×1 `--server`×1 |
| `printf` | `-1`×1 |
| `tee` | `-a`×34 `-c`×1 |
| `nl` | `-ba`×2559 `-v0`×1 |
| `diff` | `-u`×215 `-`×68 `-rq`×61 `--label`×52 `-q`×37 `-r`×30 `-x`×29 `--exclude`×17 `-ru`×9 `-qr`×7 `-I`×6 `-w`×3 `-U0`×3 `-y`×2 |
| `stat` | `-f`×298 `-t`×48 `-c`×3 |
| `du` | `-sh`×469 `-h`×43 `-sk`×41 `-ch`×30 `-shL`×9 `-ah`×6 `-k`×6 `--exclude`×6 `-shA`×6 `-ck`×1 `--apparent-size`×1 |
| `ps` | `-o`×561 `-p`×504 `-axo`×160 `-Ao`×106 `-r`×59 `-ax`×38 `-eo`×31 `-ef`×17 `-ww`×13 `-t`×11 `-A`×8 `-g`×7 `--sort`×5 `-M`×2 |
| `kill` | `-0`×92 `-TERM`×56 `-KILL`×21 `-9`×19 `-USR1`×8 `-USR2`×6 `-INT`×3 `-35454`×2 `-79765`×1 `-26135`×1 `-42211`×1 `-3091`×1 |
| `pkill` | `-f`×859 `-P`×14 `-x`×4 `-9`×4 `-INT`×3 `--no-session`×2 |
| `lsof` | `-nP`×254 `-i`×144 `-p`×136 `-iTCP`×111 `-P`×72 `-a`×61 `-d`×47 `-ti`×36 `-n`×33 `-Fn`×30 `-t`×22 `-U`×12 `-c`×2 |
| `tar` | `-C`×284 `-xzf`×177 `-x`×137 `-tzf`×70 `-xf`×36 `--strip-components`×31 `-`×30 `-xOf`×22 `-tf`×16 `-xOzf`×15 `-xz`×11 `-czf`×6 `-tvf`×6 `-xzOf`×5 |

## T6c Option errors seen in output (`X: illegal/invalid/unrecognized option`)
| utility | occurrences |
|---|---|
| `cat` | 65 |
| `grep` | 28 |
| `ls` | 10 |
| `script` | 7 |
| `sandbox.wasm` | 6 |
| `head` | 5 |
| `error` | 3 |
| `du` | 3 |
| `screencapture` | 2 |
| `jot` | 2 |
| `wc` | 2 |
| `diff` | 2 |
| `sort` | 2 |
| `ld` | 2 |
| `date` | 2 |
| `time` | 1 |
| `dyld_info` | 1 |
| `mktemp` | 1 |
| `sed` | 1 |
| `chmod` | 1 |

## T7 Failures
| source | with result | failed | rate |
|---|---|---|---|
| agent | 247414 | 13867 | 5.6% |
| p2 | 44741 | 1098 | 2.5% |

| failure kind | count | % of failures | agent | p2 |
|---|---|---|---|---|
| other non-zero (tool-specific) | 5311 | 35.5% | 4638 | 673 |
| js/ts build/test failure | 3397 | 22.7% | 3382 | 15 |
| benign: no-match/false (exit 1, no output) | 1783 | 11.9% | 1783 | 0 |
| node runtime error | 837 | 5.6% | 759 | 78 |
| python error | 592 | 4.0% | 525 | 67 |
| command not found | 573 | 3.8% | 562 | 11 |
| timeout | 519 | 3.5% | 502 | 17 |
| rust build/test failure | 486 | 3.2% | 471 | 15 |
| missing file | 451 | 3.0% | 358 | 93 |
| git error | 259 | 1.7% | 237 | 22 |
| aborted | 218 | 1.5% | 218 | 0 |
| shell syntax/quoting error | 210 | 1.4% | 185 | 25 |
| usage message | 90 | 0.6% | 80 | 10 |
| network error | 77 | 0.5% | 76 | 1 |
| zsh: no matches found (glob) | 65 | 0.4% | 0 | 65 |
| BSD/GNU option mismatch | 52 | 0.3% | 46 | 6 |
| permission denied | 42 | 0.3% | 42 | 0 |
| arg list too long | 2 | 0.0% | 2 | 0 |
| SIGPIPE (141) | 1 | 0.0% | 1 | 0 |

Exit codes of failed commands: 1: 10818, 2: 2163, timeout: 519, 101: 487, aborted: 218, 128: 172, 127: 160, None: 124, 125: 60, 254: 38, 5: 36, 3: 21, 255: 16, 8: 13, 126: 13

### Errors hidden in successful output (message present but overall exit 0)
| message | commands with message | of which exit 0 |
|---|---|---|
| command not found | 606 | 452 (74.6%) |
| No such file or directory | 2291 | 1368 (59.7%) |
| illegal/invalid/unrecognized option | 152 | 123 (80.9%) |
| shell syntax error | 184 | 11 (6.0%) |
| zsh: no matches found | 815 | 750 (92.0%) |

### `command not found` (per command, all outputs)
| missing | commands | failed |
|---|---|---|
| `timeout` | 389 | 51 |
| `pdftotext` | 13 | 6 |
| `magick` | 12 | 12 |
| `tsc` | 9 | 6 |
| `pdfinfo` | 6 | 1 |
| `vitest` | 6 | 3 |
| `tmux` | 5 | 4 |
| `python` | 5 | 5 |
| `yt-dlp` | 4 | 1 |
| `email` | 4 | 1 |
| `pi` | 4 | 1 |
| `c` | 4 | 1 |
| `llvm-objdump` | 4 | 3 |
| `mapfile` | 3 | 2 |
| `shuf` | 3 | 3 |
| `biome` | 3 | 2 |
| `shellcheck` | 3 | 1 |
| `crucible` | 3 | 2 |
| `readelf` | 3 | 2 |
| `pandoc` | 2 | 1 |
| `bun` | 2 | 1 |
| `run` | 2 | 1 |
| `omp` | 2 | 1 |
| `emcmake` | 2 | 2 |
| `inkling` | 2 | 2 |

### Failure rate by category
| category | with result | failed | rate |
|---|---|---|---|
| read-only | 237101 | 8054 | 3.4% |
| cd | 139437 | 5194 | 3.7% |
| script-run | 49222 | 3722 | 7.6% |
| build-test | 45519 | 4668 | 10.3% |
| interp-inline | 38454 | 1840 | 4.8% |
| file-mutate | 33110 | 2026 | 6.1% |
| echo | 30784 | 1368 | 4.4% |
| network | 15109 | 883 | 5.8% |
| vcs-mutate | 14310 | 563 | 3.9% |
| process | 11432 | 689 | 6.0% |
| shell-builtin | 4309 | 780 | 18.1% |
| dynamic | 3172 | 462 | 14.6% |
| other | 2781 | 327 | 11.8% |
| pkg-install | 1318 | 97 | 7.4% |
| remote-vm | 258 | 38 | 14.7% |
| shell-c | 239 | 12 | 5.0% |
| long-running | 152 | 21 | 13.8% |

### Failure rate by first command
| first | with result | failed | rate |
|---|---|---|---|
| `cd` | 134931 | 4790 | 3.5% |
| `git` | 37517 | 654 | 1.7% |
| `rg` | 33217 | 1844 | 5.6% |
| `npm` | 9202 | 1281 | 13.9% |
| `node` | 8085 | 1163 | 14.4% |
| `gh` | 5707 | 296 | 5.2% |
| `find` | 5448 | 219 | 4.0% |
| `ls` | 4425 | 213 | 4.8% |
| `sed` | 4341 | 80 | 1.8% |
| `python3` | 4257 | 236 | 5.5% |
| `cat` | 4029 | 181 | 4.5% |
| `grep` | 3109 | 100 | 3.2% |
| `cargo` | 2490 | 426 | 17.1% |
| `npx` | 2392 | 395 | 16.5% |
| `set` | 1985 | 317 | 16.0% |
| `printf` | 1755 | 87 | 5.0% |
| `curl` | 1634 | 70 | 4.3% |
| `rm` | 1513 | 100 | 6.6% |
| `make` | 1422 | 308 | 21.7% |
| `mktemp` | 1327 | 164 | 12.4% |
| `uv` | 1245 | 135 | 10.8% |
| `sleep` | 1186 | 83 | 7.0% |
| `<dynamic>` | 1158 | 230 | 19.9% |
| `nl` | 1148 | 2 | 0.2% |
| `echo` | 1120 | 47 | 4.2% |
| `wc` | 1114 | 39 | 3.5% |
| `mkdir` | 1112 | 71 | 6.4% |
| `tmux` | 897 | 14 | 1.6% |
| `docker` | 865 | 164 | 19.0% |
| `python` | 689 | 65 | 9.4% |

Timeouts by first command: `cd` 157, `node` 63, `find` 31, `set` 26, `sleep` 23, `git` 16, `seq` 15, `docker` 14, `python3` 12, `gh` 11, `cargo` 9, `npm` 8, `rg` 8, `printf` 8, `uv` 7

codemode nested bash status: ok: 2956, error: 135

## T8 Long-running / background / interactive
| pattern | commands | % | agent | p2 |
|---|---|---|---|---|
| sleep | 9467 | 3.2% | 8678 | 748 |
| timeout wrapper (timeout/gtimeout/timeout.sh) | 8954 | 3.0% | 939 | 8015 |
| polling loop (loop containing sleep) | 3048 | 1.0% | 2688 | 346 |
| background & | 2116 | 0.7% | 1620 | 478 |
| tmux (send-keys/capture-pane, etc.) | 1941 | 0.7% | 1920 | 13 |
| kill/pkill/lsof (process cleanup) | 1696 | 0.6% | 1479 | 211 |
| server/dev-server/watch-mode hint | 587 | 0.2% | 547 | 37 |
| nohup | 585 | 0.2% | 449 | 136 |
| interactive-risk (vim/less/top/git rebase -i/python REPL) | 105 | 0.0% | 90 | 15 |
| disown/setsid | 19 | 0.0% | 11 | 8 |
| watch | 4 | 0.0% | 3 | 1 |
| tail -f/-F | 2 | 0.0% | 2 | 0 |

sleep durations (max per command): <=1s: 1746, <=5s: 3559, <=30s: 3367, <=120s: 455, >120s: 340

Tool `timeout` argument set by the model: none: 43.2%, >600s: 6.2%, <=120s: 16.6%, <=30s: 25.5%, <=600s: 8.5%, invalid: 0.0%

| source | n with duration | p50 ms | p90 ms | p99 ms | >10s | >60s |
|---|---|---|---|---|---|---|
| agent | 55202 | 71 | 15426 | 146713 | 13.43% | 3.44% |
| p2 | 44710 | 72 | 22865 | 261550 | 17.04% | 4.43% |

## T9 Windows / remote
| signal | commands |
|---|---|
| prlctl (Parallels VM) commands | 254 |
| mentions powershell/pwsh | 161 |
| prlctl exec into Windows VM | 112 |
| contains drive path C:\ | 67 |
| cmd /c | 33 |
| wsl | 14 |
| output: 'is not recognized as an internal or external command' | 3 |
| prlctl Windows with result | 112 |
| prlctl Windows failed | 17 |
| scp/rsync | 47 |
| docker exec/run | 1217 |
| ssh | 7 |
| ssh with result | 7 |
| ssh failed | 3 |
