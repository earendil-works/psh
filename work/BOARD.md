# Board

Owned by the coordinator. Status of all issues; details are in `issues/`.

## In progress

| issue | title | agent | worktree |
| --- | --- | --- | --- |

## Paused (2026-11-08 checkpoint; resume notes in the issue logs)

| issue | what is left | branch |
| --- | --- | --- |
| 0010 | pi-env integration (not started) | - |
| 0013 | upstream ripgrep/fd suites, perf re-measure, Result | issue/0013-utils-search |
| 0020 | part 3: nice, nohup | issue/0020-utils-misc |
| 0032 | engine hit list | issue/0032-engine-climb |
| 0035 | ps, pgrep, pkill, uname, df… (started) | issue/0035-utils-tier2-process |
| 0007 | Windows: strips_crlf true, nice mapping | issue/0007-windows |
| 0021 | rest of Tier 2 | - |

## Open (ready)

| issue | title | depends-on | touches |
| --- | --- | --- | --- |

## Open (blocked on dependencies)

| issue | title | depends-on |
| --- | --- | --- |
| 0007B | Windows process layer (part B of 0007) | 0006 |
| 0010 | pi-env integration behind a flag | 0006, 0007, L3 high |
| 0013 | Utilities B: rg, fd, grep + shared search layer | 0008 |
| 0021 | Utilities Tier 2 (umbrella, to be split) | Tier 1 |

Language areas (expansion, execution, shell builtins) are cut from the hit list once
0002 and 0005 have landed. 0008 + 0013–0020 are Tier 1 (62 names); once 0008's registry and context exist,
0013, 0015, 0016, 0018, 0019, 0020 run in parallel, and 0014/0017 follow 0013. Tier 3 utilities are on demand.

## Open questions and follow-ups (no owner decision pending)

| question | where |
| --- | --- |
| Where the exec-time approval hook sits (pi-env integration) | ARCH.md "Open questions"; 0010 |
| Windows PATH separator policy (busybox-style built, D1076) | owner |
| Follow-ups of 0041: `timeout` should signal then wait (nested TERM traps print after its status); parser does not mark every end-of-input case incomplete (`[[ … ( … )` newline, trailing `\`), stdin reader works around it; wasm runner/psh-wasm stdin scripts via `Script::stdin`; `psh --utils` lists no bash/sh | 0041 Follow-ups |
| `$!` of compound jobs and nested shells is a made-up id (D1900, D1411): keep, error, or make it outlive the run? (`$$`/`BASHPID` in nested shells: D1413, fall back to real bash, else error) | owner; 0041 |

## Done

| issue | title | merged |
| --- | --- | --- |
| 0001 | Workspace skeleton and the empty gate | 6ad6973 (infra) |
| 0003 | Shape corpus | e16788b (shapes) |
| 0012 | Guide check tool | 2983147 (shapes) |
| 0007A | Windows gate on the Parallels VM (part A of 0007) | bad4d63 (windows) |
| 0002 | psh-spec runner, vendored cases, recorded goldens, ratchets, status | feb2a7c (infra) |
| 0004 | Parser: psh-syntax from brush-parser, hand-written | f6bf28a (parser) |
| 0009 | Classifier | 44a7306 (shapes) |
| 0024 | Classifier follow-ups and guide page | f0b4905 (shapes) |
| 0023 | Arithmetic parser, bash-format printer, byte-exact sources | 35a1a89 (parser) |
| 0005 | Minimal interpreter | e98333b (engine) |
| 0022 | L0 leg, gate caching, hit list on the parser, deps allowlist | 342ca7a (infra) |
| 0025 | Alias expansion in the parser, syntax guide page | 64a1c69 (parser) |
| 0011 | WASM build and JS host | f5116a3 (infra) |
| 0006 | Unix process layer | 9de10bb (procs) |
| 0028 | Shell options, errexit and traps | 08e81d0 (shapes) |
| 0008 | Utility framework and first batch | d4241b1 (infra) |
| 0026 | Arrays and the rest of word expansion | a9cf1e1 (engine) |
| 0031 | Grow the L3 agent layer (144 → 701 cases) | 2cb78fe (shapes) |
| 0027 | [[ ]], test, and a POSIX regex engine | 57b282d (parser) |
| 0016 | Utilities E: text filters | 278d1dd (text) |
| 0030 | Process follow-ups | 4617d8f (procs) |
| 0033 | Per-platform ratchets and the platform tag | 4c0aee4 (shapes) |
| 0017 | Utilities F: find, xargs | f817046 (procs) |
| 0034 | psh binary command line on argument (D035) | 4e79b31 (cli) |
| 0018 | Utilities G: jq | af40922 (procs) |
| 0014 | Utilities C: sed | 888caeb (parser) |
| 0019 | Utilities H: awk | 6998430 (text) |
| 0015 | Utilities D: files and paths | afbcf3c8 (files) |
| 0029 | Core shell builtins | 5cdf21b2 (builtins) |
| 0034 | psh binary command line, --utils, merge lock | 4c364a1 (cli) |
| 0036 | The guide as a static site (psh-docs build, live examples) | 3a964cba (coordinator) |
| 0037 | Windows NUL artifact investigation and null-device aliases | 169bcb0e (coordinator) |
| 0039 | Formatted AST tab in live documentation examples | ddc417d1 (coordinator) |
| 0040 | AST outline: psh_syntax::outline, --parse --tree, interactive AST tab | a696dec2 (coordinator) |
| 0041 | Nested `bash`/`sh` run in psh: -c, FILE, -n, stdin scripts, virtual-process signals, no made-up `$$` (D1410–D1414) | 10922422 (shells) |
