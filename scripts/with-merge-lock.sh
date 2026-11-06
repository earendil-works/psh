#!/usr/bin/env bash
# Run a command while holding the merge lock (the lock merge-to-main.sh takes before it
# fast-forwards main). The coordinator commits on main through this, so main never moves
# between an agent's "is main still where I gated?" check and its fast-forward:
#
#   scripts/with-merge-lock.sh git commit -qm "board: ..."
#   scripts/with-merge-lock.sh sh -c 'edit files && git commit -qam "docs: ..."'
#
# Waits for the lock up to PSH_MERGE_LOCK_WAIT seconds (default 3600). Exits with the
# command's status, or 1 when the lock could not be taken.
set -euo pipefail
if [ $# -eq 0 ]; then
  echo "usage: scripts/with-merge-lock.sh command [args...]" >&2
  exit 2
fi
# shellcheck source=merge-lock.sh
source "$(cd "$(dirname "$0")" && pwd)/merge-lock.sh"
main_root=$(primary_checkout)
lock="$main_root/work/scratch/merge.lock"
take_merge_lock "$lock" "with-merge-lock ($*)" "${PSH_MERGE_LOCK_WAIT:-3600}" || exit 1
trap 'release_merge_lock "$lock"' EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
"$@"
