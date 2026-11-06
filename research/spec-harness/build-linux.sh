set -e
cd /work/brush && CARGO_TARGET_DIR=/work/tl/brush cargo build -q --release -p brush-shell
cd /work/yash-rs && CARGO_TARGET_DIR=/work/tl/yash cargo build -q --release -p yash-cli
cd /work/dts-cli && CARGO_TARGET_DIR=/work/tl/dts cargo build -q --release
cd /work/sh && GOFLAGS=-buildvcs=false go build -o /work/tl/gosh ./cmd/gosh
ls -la /work/tl/brush/release/brush /work/tl/yash/release/yash3 /work/tl/dts/release/dts-cli /work/tl/gosh
bun --version
