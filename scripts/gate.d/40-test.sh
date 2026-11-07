# timeout: 1800
# inputs: crates/ Cargo.toml Cargo.lock .cargo/ tests/
cargo test --workspace --locked
