# timeout: 900
# inputs: crates/ Cargo.toml Cargo.lock .cargo/ js/ scripts/build-wasm tests/ docs/guide/
# The wasm modules (0011): clippy for the wasm-only code in both builds, the build
# (target/wasm/psh.wasm, psh-parser.wasm), the JS host's smoke tests under node, and the
# conformance cases through the JS host against the shared ratchets plus the wasm ones
# (tests/<layer>/expected-failures.wasm.txt, D1113). The guide's live examples print in
# psh.wasm what psh-docs recorded natively (D1361).
cargo clippy -q -p psh-wasm --target wasm32-unknown-unknown --locked -- -D warnings
cargo clippy -q -p psh-wasm --target wasm32-unknown-unknown --locked --no-default-features -- -D warnings
scripts/build-wasm --sizes
node js/test/smoke.mjs
node crates/psh-docs/tools/live-check.mjs
node --experimental-vm-modules --test crates/psh-docs/tools/live-test.mjs
cargo build -q -p psh -p psh-spec --locked
cargo run -q -p psh-spec --locked -- --wasm -j 6
