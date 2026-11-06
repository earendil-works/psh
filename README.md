# psh

A portable, bash-compatible shell in Rust, built for running the commands that coding
agents write. Status: design only; nothing is implemented yet.

- Runs natively on macOS, Linux and Windows, with no Git Bash, MSYS or WSL.
- Behaves like GNU bash 5.3 run as `bash -c`: no interactive features.
- Ships common utilities (`grep`, `sed`, `head`, `jq`, `timeout`, ...) in-process, and
  falls through to the real program for anything they don't support.
- Embeds into a host process (first: Pi's pi-env daemon) through a synchronous, stepped
  engine with no async runtime.
- Exposes its parser for classifying commands (read-only, mutating, network, ...).
- Compiles to WebAssembly for browsers and sandboxes.

## Where to read

| File | What |
| --- | --- |
| [research/README.md](research/README.md) | Why psh exists, what agents actually run, the agreed constraints (§8) |
| [ARCH.md](ARCH.md) | The architecture as designed, with open questions |
| [TESTING.md](TESTING.md) | The oracle, test layers, ratchets and the gate |
| [docs/decisions.md](docs/decisions.md) | Append-only decision log |
| [docs/guide/](docs/guide/SUMMARY.md) | The human-facing guide |
| [AGENTS.md](AGENTS.md) | Rules for every agent working here |
| [work/](work/README.md) | Issues and the board |

## License

MIT or Apache-2.0 (to be confirmed). Code copied from brush (MIT) keeps its attribution.
