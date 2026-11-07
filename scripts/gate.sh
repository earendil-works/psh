#!/usr/bin/env bash
# The merge gate: every commit main fast-forwards to has passed it (merge-to-main.sh runs
# it in the issue worktree). TESTING.md "The gate" lists the legs.
#
#   scripts/gate.sh            every leg, in order, stopping at the first failure
#   scripts/gate.sh LEG...     only these legs (while iterating; the merge runs all)
#   scripts/gate.sh --full ... ignore the cache (still record passes)
#   scripts/gate.sh --list     the legs
#
# Each leg is a file scripts/gate.d/NN-name.sh, run in NN order with bash (`set -euo
# pipefail`, from the repository root) under scripts/timeout.sh; its first line
# `# timeout: SECONDS` bounds it, and an optional `# grace: SECONDS` line among the
# first five sets the time between TERM and KILL (default 5). To add a leg, add a file:
# no other file changes, so issues adding legs in parallel do not conflict. Pick a
# number that places it (99-windows stays last).
#
# The cache (D1221): a leg may declare its inputs on `# inputs: PATHSPEC...` lines among
# its first ten (git pathspecs; several lines add up). When those inputs, the leg file,
# gate.sh, timeout.sh and the toolchain are exactly as on an earlier pass, the leg is
# skipped with a note. A pass is recorded under the primary checkout's
# work/scratch/gate-cache/ (shared by all worktrees), keyed by a hash of all of that,
# taken from git's index. Nothing is cached when any input has uncommitted or untracked
# changes, for a leg without `# inputs:`, or when the leg touches "$PSH_GATE_NOCACHE"
# (a leg that did not really check anything, like a skipped Windows leg). Declare every
# path a leg reads: a wrong skip is worse than a slow gate.
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

full=0
if [ "${1:-}" = --full ]; then
  full=1
  shift
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

cache_dir="$(git worktree list --porcelain | awk '/^worktree /{ sub(/^worktree /, ""); print; exit }')/work/scratch/gate-cache"
# What every key includes besides the leg's inputs: the toolchains the legs build with.
toolchain=$( (rustc -Vv; cargo -V; rustup toolchain list) 2>/dev/null || true)

# cache_key LEG_FILE: prints the leg's cache key, or fails when it must not be cached.
cache_key() {
  local f=$1 inputs
  inputs=$(sed -n '1,10s/^# inputs: *//p' "$f" | tr '\n' ' ')
  [ -n "$inputs" ] || return 1
  # shellcheck disable=SC2086
  if [ -n "$(git status --porcelain --untracked-files=all -- $inputs "$f" scripts/gate.sh scripts/timeout.sh)" ]; then
    return 1
  fi
  # shellcheck disable=SC2086
  { echo "$f"; echo "$toolchain"; git ls-files -s -- $inputs "$f" scripts/gate.sh scripts/timeout.sh; } |
    git hash-object --stdin
}

total_start=$(date +%s)
for f in $selected; do
  leg=$(leg_name "$f")
  secs=$(sed -n '1s/^# timeout: *\([0-9][0-9]*\).*/\1/p' "$f")
  grace=$(sed -n '1,5s/^# grace: *\([0-9][0-9]*\).*/\1/p' "$f")
  key=$(cache_key "$f" || true)
  if [ -n "$key" ] && [ $full = 0 ] && [ -f "$cache_dir/$leg-$key" ]; then
    echo "== gate: $leg cached (inputs unchanged since a pass: $(cat "$cache_dir/$leg-$key"))"
    continue
  fi
  echo "== gate: $leg"
  start=$(date +%s)
  nocache=$(mktemp "${TMPDIR:-/tmp}/gate-nocache.XXXXXX")
  rm -f "$nocache"
  if ! PSH_GATE_NOCACHE=$nocache PSH_TIMEOUT_GRACE=${grace:-${PSH_TIMEOUT_GRACE:-5}} \
    scripts/timeout.sh "${secs:-1800}" scripts/gate.sh --leg "$f"; then
    rm -f "$nocache"
    echo "== gate: FAILED in leg '$leg' after $(($(date +%s) - start))s" >&2
    exit 1
  fi
  if [ -n "$key" ] && [ ! -e "$nocache" ]; then
    mkdir -p "$cache_dir"
    echo "$(date '+%F %T') $(git rev-parse --short HEAD) $(basename "$(pwd -P)")" >"$cache_dir/$leg-$key"
  fi
  rm -f "$nocache"
  echo "== gate: $leg ok ($(($(date +%s) - start))s)"
done
echo "== gate: passed ($(($(date +%s) - total_start))s)"
