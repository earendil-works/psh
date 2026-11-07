# timeout: 600
# inputs: crates/ Cargo.toml Cargo.lock .cargo/ docs/guide/
# The guide keeps its writing rules (docs/guide/README.md; 0012, D1120).
cargo run -q --locked -p psh-docs -- check
