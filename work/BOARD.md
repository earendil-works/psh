# Board

Owned by the coordinator. Status of all issues; details are in `issues/`.

## In progress

| issue | title | agent | worktree |
| --- | --- | --- | --- |
| 0001 | Workspace skeleton and the empty gate | infra | .worktrees/infra |
| 0003 | Shape corpus (L0 leg after 0002, 0004) | shapes | .worktrees/shapes |

## Open (ready)

| issue | title | depends-on | touches |
| --- | --- | --- | --- |

## Open (blocked on dependencies)

| issue | title | depends-on |
| --- | --- | --- |
| 0002 | psh-spec runner, vendored cases, recorded goldens, ratchets, status | 0001 |
| 0004 | Parser: psh-syntax from brush-parser, hand-written | 0001 |
| 0005 | Minimal interpreter | 0002, 0004 |
| 0006 | Unix process layer | 0005 |
| 0007 | Windows CI (part A) and process layer (part B) | A: 0001; B: 0006 |
| 0008 | Utility framework and first batch | 0005, 0006 |
| 0009 | Classifier | 0004 |
| 0010 | pi-env integration behind a flag | 0006, 0007, L3 high |
| 0011 | WASM build and JS host | 0005 |
| 0012 | Guide check tool | 0001 |
| 0013 | Utilities B: rg, fd, grep + shared search layer | 0008 |
| 0014 | Utilities C: sed | 0008, 0013 |
| 0015 | Utilities D: files and paths (17 names) | 0008 |
| 0016 | Utilities E: text filters (9 names) | 0008 |
| 0017 | Utilities F: find, xargs | 0008, 0006, 0013 |
| 0018 | Utilities G: jq | 0008 |
| 0019 | Utilities H: awk (goawk port) | 0008 |
| 0020 | Utilities I: diff, checksums, base64, date, env, nice, nohup, which | 0008, 0006 |
| 0021 | Utilities Tier 2 (umbrella, to be split) | Tier 1 |

Language areas (expansion, execution, shell builtins) are cut from the hit list once
0002 and 0005 have landed. 0008 + 0013–0020 are Tier 1 (62 names); once 0008's registry and context exist,
0013, 0015, 0016, 0018, 0019, 0020 run in parallel, and 0014/0017 follow 0013. Tier 3 utilities are on demand.

## Open questions and follow-ups (no owner decision pending)

| question | where |
| --- | --- |
| Gate leg building with Rust 1.96, the MSRV (D025) | follow-up issue, after 0001 |
| Where Windows output decoding lives | ARCH.md "Open questions" 1; 0007 |
| Where the per-utility effect tables live | ARCH.md "Open questions" 2; 0009 |
| Whether the WASM host is needed before pi-env integration | ARCH.md "Open questions" 3; 0011 |

## Done

| issue | title | merged |
| --- | --- | --- |
