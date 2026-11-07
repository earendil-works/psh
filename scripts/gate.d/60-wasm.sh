# timeout: 1800
# inputs: crates/ Cargo.toml Cargo.lock .cargo/
cargo build -p psh-syntax -p psh-engine -p psh-classify --target wasm32-unknown-unknown --locked
