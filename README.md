# psh

A portable, bash-compatible shell in Rust, built for running the commands that coding
agents write. Unreleased and under active development.

- Runs natively on macOS, Linux and Windows, with no Git Bash, MSYS or WSL.
- Behaves like GNU bash 5.3 run as `bash -c`: no interactive features.
- Runs common utilities in-process (`head`, `sort`, `cut`, `find`, `xargs`, `rg`,
  `timeout`, `awk`, ...) and falls through to the real program for anything they don't
  support.
- Embeds into a host process through a synchronous, stepped engine with no async
  runtime (first host: pi's pi-env daemon).
- Exposes its parser and a classifier for understanding commands (read-only, mutating,
  network, ...).
- Compiles to WebAssembly for browsers and sandboxes.

## Status

Built: the parser (no_std, byte spans, `bash -n` agreement), the classifier, the stepped
engine with most of the language (expansion, arrays, functions, `[[ ]]`, options and
errexit, traps, jobs), a POSIX regex engine with GNU semantics, the Unix and Windows
process hosts, about 40 in-process utilities, the `psh` binary, and the WASM modules
with a JavaScript host. Still to come: the remaining Tier 1 utilities (`grep`, `fd`,
`sed`, `jq`, `ls`, `stat`, `date`, `diff`, ...) and pi-env integration.

Conformance against recorded bash 5.3 output (`scripts/status`, macOS, 2026-11-07,
main at `aa673bc`); a case passes on stdout, exit status and files, stderr is counted
separately:

| Layer | Active | Pass | % | Also stderr |
| --- | ---: | ---: | ---: | ---: |
| L0 parse (`bash -n` agreement, print → reparse) | 34,278 | 34,278 | 100% | – |
| L1 Oils spec | 2,511 | 2,252 | 89.7% | 2,175 |
| L2 brush cases | 3,171 | 2,747 | 86.6% | 2,640 |
| L3 agent-shaped cases | 701 | 689 | 98.3% | 687 |
| L4 utilities | 671 | 671 | 100% | 668 |

History is in `tests/scores/status.tsv`; `scripts/status --hitlist` ranks what fails by
how often agents use it.

## Building and running

```sh
cargo build -p psh
target/debug/psh -c 'cd src && ls | sort | head -3'
target/debug/psh --classify -c 'git commit -m "$(rm -rf build)"'
scripts/build-wasm            # target/wasm/psh.wasm and psh-parser.wasm
```

## Where to read

| File | What |
| --- | --- |
| [docs/guide/](docs/guide/SUMMARY.md) | The guide: what psh is, how to use it, how it works |
| [ARCH.md](ARCH.md) | The architecture: the design reference |
| [TESTING.md](TESTING.md) | The oracle, test layers, ratchets and the gate |
| [docs/decisions.md](docs/decisions.md) | Append-only decision log |
| [research/README.md](research/README.md) | Why psh exists, what agents actually run, the agreed constraints (§8) |
| [AGENTS.md](AGENTS.md) | Rules for every agent working here |
| [work/](work/README.md) | Issues and the board |

## License

MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`). Code copied from other projects
(brush, uutils, ripgrep, fd, GoAWK, ...) keeps its attribution in `NOTICE`.
