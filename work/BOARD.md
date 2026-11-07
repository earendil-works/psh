# Board

Owned by the coordinator. Status of all issues; details are in `issues/`.

## In progress

| issue | title | agent | worktree |
| --- | --- | --- | --- |
| 0034b | cli.md guide page, psh --utils | cli | .worktrees/cli |
| 0020 | Utilities I: diff, checksums, base64, date, env, nice, nohup, which | shapes | .worktrees/shapes |
| 0018 | Utilities G: jq (jaq) | procs | .worktrees/procs |
| 0007 | Windows: part A merged (bad4d63, rework 6ca3d08); part B in progress | windows | .worktrees/windows |
| 0008-wasm | JS host: Host::now/mode (D1069), then 0013 | infra | .worktrees/infra |
| 0015 | Utilities D: files and paths | files | .worktrees/files |
| 0019 | Utilities H: awk (goawk port) | text | .worktrees/text |
| 0014 | Utilities C: sed (on psh-regex) | parser | .worktrees/parser |
| 0032 | Engine follow-ups and the hit list | engine | .worktrees/engine |
| 0029 | Core shell builtins | builtins | .worktrees/builtins |

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
