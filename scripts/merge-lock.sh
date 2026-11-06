# Sourced by merge-to-main.sh and with-merge-lock.sh: the merge lock that guards `main`.
#
# The lock is the directory work/scratch/merge.lock of the primary checkout; mkdir is
# atomic, so whoever creates it holds it. Its `owner` file names the holder and its PID;
# a lock whose holder is gone (SIGKILL skips the EXIT trap that removes it) is detected
# and removed instead of blocking every later merge.
#
#   source scripts/merge-lock.sh
#   take_merge_lock LOCK WHO MAX_WAIT_SECONDS || exit 1
#   ... ; release_merge_lock LOCK
#
# PSH_MERGE_POLL (seconds, default 2) sets how often a waiter looks again.

# The primary checkout: the first entry of `git worktree list`.
primary_checkout() {
  git worktree list --porcelain | awk '/^worktree /{ sub(/^worktree /, ""); print; exit }'
}

take_merge_lock() {
  local lock=$1 who=$2 max=$3 waited=0 next_note=0 pid owner
  local poll=${PSH_MERGE_POLL:-2}
  mkdir -p "$(dirname "$lock")"
  until mkdir "$lock" 2>/dev/null; do
    owner=$(cat "$lock/owner" 2>/dev/null || true)
    pid=$(printf '%s\n' "$owner" | sed -n 's/.*pid=\([0-9][0-9]*\).*/\1/p')
    if [ -n "$pid" ] && ! kill -0 "$pid" 2>/dev/null; then
      echo "merge lock: removing a stale lock ($owner): pid $pid is gone" >&2
      rm -rf "$lock"
      continue
    fi
    if [ "$waited" -ge "$next_note" ]; then
      echo "merge lock: waiting for $lock, held by: ${owner:-unknown (no owner file yet)}" >&2
      next_note=$((next_note + 300))
    fi
    if [ "$waited" -ge "$max" ]; then
      echo "merge lock: gave up after ${waited}s; held by: ${owner:-unknown}." \
        "If that holder is stuck, tell the coordinator." >&2
      return 1
    fi
    sleep "$poll"
    waited=$((waited + poll))
  done
  echo "$who since $(date '+%F %T') host=$(hostname) pid=$$" >"$lock/owner"
}

release_merge_lock() {
  rm -rf "$1"
}
