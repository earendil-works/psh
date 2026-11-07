# timeout: 900
# inputs: crates/ Cargo.toml Cargo.lock .cargo/ js/ scripts/build-wasm
# The wasm modules (0011): both build (target/wasm/psh.wasm, psh-parser.wasm) and the
# JS host's smoke tests pass under node.
# The wasm-only code (the import, the stack) gets clippy too, in both builds.
cargo clippy -q -p psh-wasm --target wasm32-unknown-unknown --locked -- -D warnings
cargo clippy -q -p psh-wasm --target wasm32-unknown-unknown --locked --no-default-features -- -D warnings
scripts/build-wasm --sizes
node js/test/smoke.mjs
