# timeout: 1800
# inputs: crates/ Cargo.toml Cargo.lock .cargo/ tests/
cargo clippy --workspace --all-targets --locked -- -D warnings
