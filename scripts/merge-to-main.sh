#!/usr/bin/env bash
# Land the current issue branch on main. Run it from the issue worktree:
#
#   scripts/merge-to-main.sh
#
# 1. Rebase the branch onto main (no lock).
# 2. Run scripts/gate.sh on the result, here in the issue worktree (no lock), under
#    scripts/timeout.sh (PSH_GATE_TIMEOUT seconds, default 10800: the windows leg may
#    wait an hour for the VM's turn, D1072).
# 3. Take the merge lock (work/scratch/merge.lock in the primary checkout). If main is
#    still where step 1 put the branch, fast-forward main in the primary checkout and
#    release: the lock is held for seconds. If main moved meanwhile (another merge, the
#    coordinator's commits), release it and go back to 1. After two such lost races the
#    next attempt holds the lock from 1 to 3, so no branch starves.
#
# main only ever fast-forwards to a commit that passed the gate.
#
# Refuses to start when this worktree has uncommitted or untracked files (the gate would
# test something other than the commit). When the primary checkout has uncommitted
# changes at fast-forward time, it releases the lock and waits for them to be committed
# (polling, up to PSH_MERGE_DIRTY_WAIT seconds, default 1800), then rebases and gates
# again if that moved main; it never touches the primary checkout's changes. The lock
# wait is capped by PSH_MERGE_LOCK_WAIT (default 3600). A rebase conflict stops the
# script with the rebase in progress: resolve it, `git rebase --continue`, run it again.
#
# One line per run goes to work/scratch/merge-log.tsv in the primary checkout: start,
# branch, outcome, seconds, attempts.
set -euo pipefail

# bash reads a script while it runs it, and the rebase below can rewrite this file: run a
# copy. The sourced lock functions are read in full before the rebase.
if [ -z "${PSH_MERGE_SCRIPTS:-}" ]; then
  PSH_MERGE_SCRIPTS=$(cd "$(dirname "$0")" && pwd)
  export PSH_MERGE_SCRIPTS
  copy=$(mktemp "${TMPDIR:-/tmp}/merge-to-main.XXXXXX")
  cp "$0" "$copy"
  exec "$BASH" "$copy" "$@"
fi
rm -f "$0"
# shellcheck source=merge-lock.sh
source "$PSH_MERGE_SCRIPTS/merge-lock.sh"
# Child scripts (the gate's tests) must not mistake themselves for this copy.
unset PSH_MERGE_SCRIPTS

die() {
  echo "merge-to-main: $*" >&2
  exit 1
}

here=$(git rev-parse --show-toplevel)
cd "$here"
branch=$(git symbolic-ref -q --short HEAD || true)
case "$branch" in
  issue/*) ;;
  *) die "run this on an issue/* branch (current: '${branch:-detached HEAD}')" ;;
esac
main_root=$(primary_checkout)
[ "$here" != "$main_root" ] || die "run this from the issue worktree, not the primary checkout"
if [ -e "$(git rev-parse --git-path rebase-merge)" ] || [ -e "$(git rev-parse --git-path rebase-apply)" ]; then
  die "a rebase is in progress here; finish it (git rebase --continue) or abort it first"
fi
if [ -n "$(git status --porcelain)" ]; then
  git status --short >&2
  die "this worktree has uncommitted or untracked files; commit them (or gitignore them) first"
fi
primary_branch=$(git -C "$main_root" symbolic-ref -q --short HEAD || true)
[ "$primary_branch" = main ] || die "the primary checkout ($main_root) is on '${primary_branch:-detached HEAD}', not main"

scratch="$main_root/work/scratch"
lock="$scratch/merge.lock"
mkdir -p "$scratch"
lock_wait=${PSH_MERGE_LOCK_WAIT:-3600}
dirty_wait=${PSH_MERGE_DIRTY_WAIT:-1800}
poll=${PSH_MERGE_POLL:-2}

t_start=$(date +%s)
started=$(date '+%F %T')
outcome=failed
attempts=0
holding=0
take() {
  take_merge_lock "$lock" "merge-to-main $branch" "$lock_wait" || die "could not take the merge lock"
  holding=1
}
release() {
  if [ "$holding" = 1 ]; then
    release_merge_lock "$lock"
    holding=0
  fi
}
finish() {
  release
  printf '%s\t%s\t%s\t%s\t%s\n' "$started" "$branch" "$outcome" $(($(date +%s) - t_start)) "$attempts" \
    >>"$scratch/merge-log.tsv"
}
trap finish EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

primary_dirty() {
  [ -n "$(git -C "$main_root" status --porcelain --untracked-files=no)" ]
}

wait_primary_clean() {
  local waited=0
  echo "== the primary checkout ($main_root) has uncommitted changes:"
  git -C "$main_root" status --short --untracked-files=no | sed 's/^/   /'
  echo "   waiting up to ${dirty_wait}s for them to be committed (with scripts/with-merge-lock.sh)"
  while primary_dirty; do
    if [ "$waited" -ge "$dirty_wait" ]; then
      outcome=failed-primary-dirty
      die "the primary checkout still has uncommitted changes after ${waited}s; ask the coordinator"
    fi
    sleep "$poll"
    waited=$((waited + poll))
  done
}

# Sets $base (the main it rebased onto) and $sha (the commit to gate).
rebase_onto_main() {
  base=$(git rev-parse main)
  echo "== rebase $branch onto main ($(git rev-parse --short "$base"))"
  if ! git rebase -q main; then
    outcome=failed-rebase
    die "the rebase onto main conflicts; resolve it, run 'git rebase --continue', then run this again"
  fi
  sha=$(git rev-parse HEAD)
}

run_gate() {
  local t
  t=$(date +%s)
  echo "== gate on $(git rev-parse --short "$sha")"
  if ! "$here/scripts/timeout.sh" "${PSH_GATE_TIMEOUT:-10800}" "$here/scripts/gate.sh"; then
    outcome=failed-gate
    die "the gate failed; fix it, commit, run this again"
  fi
  echo "== gate passed in $(($(date +%s) - t))s"
}

gated=
lost=0
while :; do
  attempts=$((attempts + 1))
  hold=0
  if [ "$lost" -ge 2 ]; then
    echo "== main moved twice while gating; this attempt holds the merge lock throughout"
    hold=1
    take
  fi
  rebase_onto_main
  # Nothing to gate again when the rebase was a no-op (we only waited for the primary).
  if [ "$sha" != "$gated" ]; then
    run_gate
    gated=$sha
  fi
  [ "$hold" = 1 ] || take
  if [ "$(git rev-parse main)" != "$base" ]; then
    echo "== main moved while gating ($(git rev-parse --short "$base") -> $(git rev-parse --short main)); rebasing and gating again"
    release
    lost=$((lost + 1))
    continue
  fi
  if primary_dirty; then
    release
    wait_primary_clean
    continue
  fi
  echo "== fast-forward main to $(git rev-parse --short "$sha")"
  git -C "$main_root" merge -q --ff-only "$sha" || {
    outcome=failed-ff
    die "the fast-forward of main failed (see above)"
  }
  release
  outcome=merged
  echo "merge-to-main: main is now at $(git rev-parse --short main) ($branch)"
  break
done
