# timeout: 600
# inputs: .
# The guide keeps its writing rules (docs/guide/README.md; 0012, D1120). Its links may
# point at any repository file, so the whole tree is its input.
cargo run -q --locked -p psh-docs -- check
