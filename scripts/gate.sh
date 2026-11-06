#!/usr/bin/env bash
# The merge gate: every commit main fast-forwards to has passed it (merge-to-main.sh runs
# it in the issue worktree). TESTING.md "The gate" lists the legs.
#
#   scripts/gate.sh            every leg, in order, stopping at the first failure
#   scripts/gate.sh LEG...     only these legs (while iterating; the merge runs all)
#   scripts/gate.sh --list     the legs
#
# Each leg is a file scripts/gate.d/NN-name.sh, run in NN order with bash (`set -euo
# pipefail`, from the repository root) under scripts/timeout.sh; its first line
# `# timeout: SECONDS` bounds it, and an optional `# grace: SECONDS` line among the
# first five sets the time between TERM and KILL (default 5). To add a leg, add a file:
# no other file changes, so issues adding legs in parallel do not conflict. Pick a
# number that places it (99-windows stays last).
#
# The gate is not niced (PSH_GATE=1), so the agents' niced builds and tests yield to it;
# cargo's job and test-thread caps from .cargo/config.toml still apply, since several
# gates may run at once.
set -euo pipefail
cd "$(dirname "$0")/.."
export PSH_GATE=1

leg_name() {
  local base=${1##*/}
  base=${base%.sh}
  echo "${base#*-}"
}

if [ "${1:-}" = --leg ]; then
  # Internal: one leg file, run under timeout.sh by the loop below.
  set -euo pipefail
  # shellcheck disable=SC1090
  source "$2"
  exit 0
fi

files=$(ls scripts/gate.d/*.sh)
if [ "${1:-}" = --list ]; then
  for f in $files; do leg_name "$f"; done
  exit 0
fi

selected=
if [ $# -gt 0 ]; then
  for leg in "$@"; do
    found=
    for f in $files; do
      [ "$(leg_name "$f")" = "$leg" ] && found=$f
    done
    if [ -z "$found" ]; then
      echo "gate: unknown leg '$leg' (scripts/gate.sh --list)" >&2
      exit 2
    fi
    selected="$selected $found"
  done
else
  selected=$files
fi

total_start=$(date +%s)
for f in $selected; do
  leg=$(leg_name "$f")
  secs=$(sed -n '1s/^# timeout: *\([0-9][0-9]*\).*/\1/p' "$f")
  grace=$(sed -n '1,5s/^# grace: *\([0-9][0-9]*\).*/\1/p' "$f")
  echo "== gate: $leg"
  start=$(date +%s)
  if ! PSH_TIMEOUT_GRACE=${grace:-${PSH_TIMEOUT_GRACE:-5}} \
    scripts/timeout.sh "${secs:-1800}" scripts/gate.sh --leg "$f"; then
    echo "== gate: FAILED in leg '$leg' after $(($(date +%s) - start))s" >&2
    exit 1
  fi
  echo "== gate: $leg ok ($(($(date +%s) - start))s)"
done
echo "== gate: passed ($(($(date +%s) - total_start))s)"
