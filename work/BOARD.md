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
| 0008 | Utility framework and first batch | 0005, 0006; builtins.md |
| 0009 | Classifier | 0004 |
| 0010 | pi-env integration behind a flag | 0006, 0007, L3 high |
| 0011 | WASM build and JS host | 0005 |
| 0012 | Guide check tool | 0001 |

Language areas (expansion, execution, shell builtins) are cut from the hit list once
0002 and 0005 have landed.

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

## Done

| issue | title | merged |
| --- | --- | --- |
