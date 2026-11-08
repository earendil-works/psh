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

**Documentation:** the guide, in `docs/guide/`: what psh is, how to use and embed it,
how it works, and its status in numbers. `cargo run -p psh-docs -- build` makes it a
site in `target/docs-site/` with search and live examples that run psh.wasm in the page
(`scripts/build-wasm` first); `.github/workflows/docs.yml` publishes it to GitHub Pages.

## Status

Against recorded bash 5.3 output on Linux (main at `3475d0aa`, 2026-11-08):

| Layer | Active | Pass | % |
| --- | ---: | ---: | ---: |
| L0 parse (`bash -n` agreement, print → reparse) | 34,598 | 34,598 | 100% |
| L1 Oils spec | 2,511 | 2,355 | 93.8% |
| L2 brush cases | 3,171 | 2,855 | 90.0% |
| L3 agent-shaped cases | 701 | 701 | 100% |
| L4 utilities | 994 | 994 | 100% |

49 utilities (58 names) run in-process (`psh --utils` lists them). The guide's "Status in
numbers" has macOS and Windows, sizes, speed, what is not built yet and the known gaps.

## Building and running

```sh
cargo build -p psh
target/debug/psh --help
target/debug/psh -c 'cd src && ls | sort | head -3'
target/debug/psh --classify -c 'git commit -m "$(rm -rf build)"'
scripts/build-wasm              # target/wasm/psh.wasm and psh-parser.wasm
cargo run -p psh-docs -- build  # the guide as a site: target/docs-site/
```

## Where to read

| File | What |
| --- | --- |
| [docs/guide/](docs/guide/SUMMARY.md) | The guide: what psh is, how to use it, how it works, its status |
| [ARCH.md](ARCH.md) | The architecture: the design reference |
| [TESTING.md](TESTING.md) | The oracle, test layers, ratchets and the gate |
| [docs/decisions.md](docs/decisions.md) | Append-only decision log |
| [research/README.md](research/README.md) | Why psh exists, what agents actually run, the agreed constraints (§8) |
| [AGENTS.md](AGENTS.md) | Rules for every agent working here |
| [work/](work/README.md) | Issues and the board |

## License

MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`). Code copied from other projects
(brush, uutils, ripgrep, fd, GoAWK, ...) keeps its attribution in `NOTICE`.
