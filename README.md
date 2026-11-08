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

See [REPORT.md](REPORT.md) for the full state of the project. In short, against recorded
bash 5.3 output on Linux (main at `3475d0aa`, 2026-11-08):

| Layer | Active | Pass | % |
| --- | ---: | ---: | ---: |
| L0 parse (`bash -n` agreement, print → reparse) | 34,501 | 34,501 | 100% |
| L1 Oils spec | 2,511 | 2,355 | 93.8% |
| L2 brush cases | 3,171 | 2,855 | 90.0% |
| L3 agent-shaped cases | 701 | 701 | 100% |
| L4 utilities | 994 | 994 | 100% |

59 utilities run in-process (`psh --utils` lists them). Not done yet: pi-env integration,
release builds and the size budget, CI off the development machine.

## Building and running

```sh
cargo build -p psh
target/debug/psh --help
target/debug/psh -c 'cd src && ls | sort | head -3'
target/debug/psh --classify -c 'git commit -m "$(rm -rf build)"'
scripts/build-wasm            # target/wasm/psh.wasm and psh-parser.wasm
```

## Where to read

| File | What |
| --- | --- |
| [REPORT.md](REPORT.md) | State of the project: what psh can do, how well, what is left |
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
