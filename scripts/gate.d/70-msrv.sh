# timeout: 1800
# The workspace builds with the MSRV (rust-version in Cargo.toml, D025), not only with
# the current stable that development uses.
msrv=$(sed -n 's/^rust-version = "\(.*\)"/\1/p' Cargo.toml)
tc=
# rust-version "1.96" is satisfied by the toolchain 1.96 or 1.96.0.
for t in "$msrv" "$msrv.0"; do
  if [ -z "$tc" ] && rustup run "$t" rustc --version >/dev/null 2>&1; then tc=$t; fi
done
if [ -z "$tc" ]; then
  echo "gate: the MSRV toolchain is missing: rustup toolchain install $msrv" >&2
  exit 1
fi
cargo "+$tc" check --workspace --all-targets --locked
