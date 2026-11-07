# timeout: 1800
# inputs: crates/ Cargo.toml Cargo.lock .cargo/ tests/
# The workspace type-checks for Linux (arm64 and x86-64, musl), tests included: the gate
# runs on macOS and Windows, and Linux-only code (cfg(target_os = "linux"), NSIG = 65)
# otherwise breaks unnoticed. A check needs no linker or C toolchain.
for target in aarch64-unknown-linux-musl x86_64-unknown-linux-musl; do
  if ! rustup target list --installed | grep -qx "$target"; then
    echo "gate: the $target target is missing: rustup target add $target" >&2
    exit 1
  fi
  cargo check --workspace --all-targets --locked --target "$target"
done
