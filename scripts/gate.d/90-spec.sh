# timeout: 600
# inputs: crates/ Cargo.toml Cargo.lock .cargo/ tests/
# The spec ratchets (TESTING.md "Ratchets"): every active case of every layer either
# passes or is listed in its layer's expected-failures.txt. Stale entries are reported,
# not fatal; `psh-spec --update-ratchet` removes them.
cargo build -q -p psh -p psh-spec --locked
cargo run -q -p psh-spec --locked -- -j 6
