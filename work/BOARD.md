# Board

Owned by the coordinator. Status of all issues; details are in `issues/`.

## In progress

| issue | title | agent | worktree |
| --- | --- | --- | --- |

## Open (ready)

| issue | title | depends-on | touches |
| --- | --- | --- | --- |
| 0001 | Workspace skeleton and the empty gate | - | Cargo.toml, crates/, scripts/, .gitattributes, NOTICE |

## Open (blocked on dependencies)

| issue | title | depends-on |
| --- | --- | --- |
| 0002 | psh-spec runner, vendored cases, recorded goldens, ratchets, status | 0001 |
| 0003 | Shape corpus | 0001 (0004 for the L0 leg) |
| 0004 | Parser: psh-syntax from brush-parser, hand-written | 0001 |
| 0005 | Minimal interpreter | 0002, 0004 |
| 0006 | Unix process layer | 0005 |
| 0007 | Windows CI and process layer | 0006 |
| 0008 | Utility framework and first batch | 0005, 0006 |
| 0009 | Classifier | 0004 |
| 0010 | pi-env integration behind a flag | 0006, 0007, L3 high |
| 0011 | WASM build and JS host | 0005 |
| 0012 | Guide check tool | 0001 |
| 0013 | Utilities B: rg, fd, grep + shared search layer | 0008; D020 confirmed |
| 0014 | Utilities C: sed | 0008, 0013 |
| 0015 | Utilities D: files and paths (17 names) | 0008 |
| 0016 | Utilities E: text filters (9 names) | 0008 |
| 0017 | Utilities F: find, xargs | 0008, 0006, 0013 |
| 0018 | Utilities G: jq | 0008; jaq decision |
| 0019 | Utilities H: awk (goawk port) | 0008 |
| 0020 | Utilities I: diff, checksums, base64, date, env, nice, nohup, which | 0008, 0006 |
| 0021 | Utilities Tier 2 (umbrella, to be split) | Tier 1 |

Language areas (expansion, execution, shell builtins) are cut from the hit list once
0002 and 0005 have landed. 0008 + 0013–0020 are Tier 1 (62 names); once 0008's registry and context exist,
0013, 0015, 0016, 0018, 0019, 0020 run in parallel, and 0014/0017 follow 0013. Tier 3 utilities are on demand.

## Owner decisions pending

| question | where |
| --- | --- |
| `no_std` or std for psh-syntax / psh-engine | ARCH.md "Crates" |
| Platform bindings for psh-os (`libc`, `windows-sys`, own FFI) | ARCH.md "psh-os"; blocks 0006/0007 |
| pi-env sessions: stateless or persistent | ARCH.md "Embedding into pi-env" |
| MSRV (pi-env pins 1.96.0) | ARCH.md "Open questions" |
| Windows CI: GitHub Actions or the Parallels VM | TESTING.md "Windows" |
| Ranking of the goals | docs/guide/goals.md |
| psh's license | README.md |
| Utility dialect points: `ls -G`/`-T`, `xargs` on empty input, `sort` collation | 0015, 0016, 0017 |
| Never-builtins beyond the owner rule: `git`, `curl`/`wget`, `ssh`/`rsync`, other proposals (D014) | ARCH.md "Never builtins" |
| D020: search crate set and approach (proposed) | 0013, 0014, 0017 |
| Utility crates: jaq, `parse_datetime`, archives, `sysinfo`, `serde_json` (`rg --json`) | 0018, 0020, 0021 |

## Done

| issue | title | merged |
| --- | --- | --- |
