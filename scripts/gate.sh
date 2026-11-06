#!/usr/bin/env bash
# The merge gate: every commit main fast-forwards to has passed it (merge-to-main.sh runs
# it in the issue worktree). TESTING.md "The gate" lists the legs.
#
#   scripts/gate.sh            every leg, in order, stopping at the first failure
#   scripts/gate.sh LEG...     only these legs (while iterating; the merge runs all)
#   scripts/gate.sh --list     the legs
#
# Each leg runs under scripts/timeout.sh. The gate is not niced (PSH_GATE=1), so the
# agents' niced builds and tests yield to it; cargo's job and test-thread caps from
# .cargo/config.toml still apply, since several gates may run at once.
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/merge-lock.sh
export PSH_GATE=1

# name:timeout-seconds, in the order they run.
LEGS="
scripts:60
fmt:300
clippy:1800
test:1800
deps:300
wasm:1800
msrv:1800
script-tests:300
shapes:120
guide:600
windows:3000
"
# Later legs go here, above `windows` (it stays last: it is the slowest, and a failure on
# macOS is cheaper to find first) and in run_leg: the spec ratchets per layer (0002
# onward).

run_leg() {
  case "$1" in
    scripts)
      # Every script parses (they run under macOS's bash 3.2 too).
      local f
      for f in scripts/*.sh; do bash -n "$f"; done
      ;;
    fmt) cargo fmt --all --check ;;
    clippy) cargo clippy --workspace --all-targets --locked -- -D warnings ;;
    test) cargo test --workspace --locked ;;
    deps)
      # psh-syntax has zero dependencies; psh-engine depends on psh-syntax only.
      check_deps psh-syntax ""
      check_deps psh-engine "psh-syntax"
      ;;
    wasm) cargo build -p psh-syntax -p psh-engine --target wasm32-unknown-unknown --locked ;;
    msrv)
      # The workspace builds with the MSRV (rust-version in Cargo.toml), not only with
      # the current stable that development uses.
      local msrv t tc=
      msrv=$(sed -n 's/^rust-version = "\(.*\)"/\1/p' Cargo.toml)
      # rust-version "1.96" is satisfied by the toolchain 1.96 or 1.96.0.
      for t in "$msrv" "$msrv.0"; do
        if [ -z "$tc" ] && rustup run "$t" rustc --version >/dev/null 2>&1; then tc=$t; fi
      done
      if [ -z "$tc" ]; then
        echo "gate: the MSRV toolchain is missing: rustup toolchain install $msrv" >&2
        return 1
      fi
      cargo "+$tc" check --workspace --all-targets --locked
      ;;
    script-tests)
      bash scripts/test-timeout.sh
      bash scripts/test-merge-to-main.sh
      ;;
    shapes)
      # The committed shape corpus holds only vocabulary words and placeholders.
      python3 scripts/shape-corpus-check --self-test
      python3 scripts/shape-corpus-check
      ;;
    guide)
      # The guide keeps its writing rules (docs/guide/README.md).
      cargo run -q --locked -p psh-docs -- check
      ;;
    windows)
      # The Rust legs on the Parallels Windows VM (D1070). PSH_GATE_WINDOWS=0 skips them
      # when the VM is broken: for the coordinator and the owner only, never to get green.
      if [ "${PSH_GATE_WINDOWS:-1}" = 0 ]; then
        echo "gate: !!! WINDOWS LEG SKIPPED (PSH_GATE_WINDOWS=0): this commit is untested on Windows !!!" >&2
        local log
        log="$(primary_checkout)/work/scratch/windows-gate/skipped.tsv"
        mkdir -p "$(dirname "$log")"
        printf '%s\t%s\t%s\n' "$(date '+%F %T')" "$(basename "$(pwd -P)")" "$(git rev-parse --short HEAD)" >>"$log"
        return 0
      fi
      scripts/windows-gate.sh
      ;;
    *)
      echo "gate: unknown leg '$1' (scripts/gate.sh --list)" >&2
      return 2
      ;;
  esac
}

# check_deps CRATE "ALLOWED...": the crate's normal, build and dev dependencies, direct and
# transitive, are exactly the allowed ones.
check_deps() {
  local got
  got=$(cargo tree -p "$1" -e normal,build,dev --prefix none --format '{p}' --locked |
    awk 'NR > 1 { print $1 }' | sort -u | tr '\n' ' ' | sed 's/ $//')
  if [ "$got" != "$2" ]; then
    echo "gate: $1 depends on '$got'; allowed: '$2' (a new dependency needs a decision, docs/decisions.md)" >&2
    return 1
  fi
}

legs=
for entry in $LEGS; do legs="$legs ${entry%%:*}"; done

case "${1:-}" in
  --list)
    for l in $legs; do echo "$l"; done
    exit 0
    ;;
  --leg)
    # Internal: one leg, run under timeout.sh by the loop below.
    run_leg "$2"
    exit
    ;;
esac

selected=${*:-$legs}
total_start=$(date +%s)
for leg in $selected; do
  secs=
  for entry in $LEGS; do
    [ "${entry%%:*}" = "$leg" ] && secs=${entry#*:}
  done
  if [ -z "$secs" ]; then
    echo "gate: unknown leg '$leg' (scripts/gate.sh --list)" >&2
    exit 2
  fi
  echo "== gate: $leg"
  start=$(date +%s)
  if ! scripts/timeout.sh "$secs" scripts/gate.sh --leg "$leg"; then
    echo "== gate: FAILED in leg '$leg' after $(($(date +%s) - start))s" >&2
    exit 1
  fi
  echo "== gate: $leg ok ($(($(date +%s) - start))s)"
done
echo "== gate: passed ($(($(date +%s) - total_start))s)"
