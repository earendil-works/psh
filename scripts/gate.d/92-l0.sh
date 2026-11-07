# timeout: 600
# inputs: crates/ Cargo.toml Cargo.lock .cargo/ tests/
# L0 (TESTING.md): psh-syntax against the recorded `bash -n` verdicts for every case
# script, shape skeleton and tests/l0/syntax.txt snippet, plus print -> reparse; the
# ratchet is tests/l0/expected-failures.txt.
cargo run -q -p psh-spec --locked -- --l0 -j 6
