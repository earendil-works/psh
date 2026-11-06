#!/usr/bin/env bash
# The gate's Windows legs, run from the macOS host on the owner's Parallels VM (D027,
# D1070). TESTING.md "Windows" says when to run it.
#
#   scripts/windows-gate.sh            every leg, in order, stopping at the first failure
#   scripts/windows-gate.sh LEG...     only these legs
#   scripts/windows-gate.sh --list     the legs
#
# It snapshots the worktree (HEAD plus uncommitted and untracked, not ignored, files) as a
# commit, points refs/windows-gate/NAME at it (NAME: the worktree's directory name), and
# has the VM fetch that ref over the Parallels share into C:\psh-gate\NAME, a checkout of
# its own that keeps its target\ between runs. scripts/windows-gate.ps1 runs the legs
# there, each under its own timeout, and the output streams back.
#
# The VM is resumed or started if needed and put back into the state it was in afterwards
# (PSH_WINDOWS_KEEP_VM=1 leaves it running), unless another run still uses it.
#
#   PSH_WINDOWS_VM        the VM's name (default "Windows 11")
#   PSH_WINDOWS_KEEP_VM   1: leave the VM running afterwards
#
# Exit status: 0 passed, 1 a leg failed, 124 a leg (or the whole run) timed out, 2 setup
# failed, 3 the VM is not usable.
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/merge-lock.sh

vm=${PSH_WINDOWS_VM:-Windows 11}

# name:timeout-seconds, in the order they run. The commands are in windows-gate.ps1.
LEGS="
fmt:300
clippy:1800
test:1800
test-x64:1800
msrv:1800
"

legs=
for entry in $LEGS; do legs="$legs ${entry%%:*}"; done
if [ "${1:-}" = --list ]; then
  for l in $legs; do echo "$l"; done
  exit 0
fi

selected=
backstop=600
for leg in ${*:-$legs}; do
  secs=
  for entry in $LEGS; do
    [ "${entry%%:*}" = "$leg" ] && secs=${entry#*:}
  done
  if [ -z "$secs" ]; then
    echo "windows-gate: unknown leg '$leg' (scripts/windows-gate.sh --list)" >&2
    exit 2
  fi
  selected="$selected${selected:+,}$leg:$secs"
  backstop=$((backstop + secs))
done

if ! command -v prlctl >/dev/null; then
  echo "windows-gate: prlctl not found (Parallels Desktop)" >&2
  exit 3
fi

# A host path as the VM sees it: Parallels shares the host home as \\Mac\Home.
to_share() {
  case "$1" in
    "$HOME"/*) ;;
    *)
      echo "windows-gate: $1 is not under $HOME, so the VM cannot see it" >&2
      exit 3
      ;;
  esac
  case "$1" in
    *[\"\ ]*)
      echo "windows-gate: $1 contains a space or quote, which prlctl exec cannot pass" >&2
      exit 3
      ;;
  esac
  printf '//Mac/Home/%s' "${1#"$HOME"/}"
}

top=$(pwd -P)
name=$(basename "$top" | tr -c 'A-Za-z0-9_.\n-' _)
common=$(cd "$(git rev-parse --git-common-dir)" && pwd -P)
repo_share=$(to_share "$common")
runner=$(to_share "$top/scripts/windows-gate.ps1" | tr / '\\')
toolchain=$(rustc --version | awk '{ print $2 }')
msrv=$(sed -n 's/^rust-version = "\(.*\)"/\1/p' Cargo.toml)

# --- the snapshot: a commit of the worktree as it is, in a scratch index, so neither the
# real index nor the stash is touched.
index=$(mktemp "${TMPDIR:-/tmp}/windows-gate-index.XXXXXX")
cp "$(git rev-parse --git-path index)" "$index"
GIT_INDEX_FILE=$index git add -A
tree=$(GIT_INDEX_FILE=$index git write-tree)
rm -f "$index"
commit=$(git rev-parse HEAD)
if [ "$tree" != "$(git rev-parse 'HEAD^{tree}')" ]; then
  commit=$(git commit-tree "$tree" -p HEAD -m "windows-gate snapshot of $name")
  echo "windows-gate: testing HEAD plus uncommitted changes (${commit:0:10})"
fi
git update-ref "refs/windows-gate/$name" "$commit"

# --- locks in the primary checkout: one per NAME (one run per checkout in the VM), and
# a short one around starting and stopping the VM.
locks="$(primary_checkout)/work/scratch/windows-gate"
run_lock="$locks/run-$name.lock"
vm_lock="$locks/vm.lock"
prior_state=
cleanup() {
  local st=$?
  trap - EXIT INT TERM HUP
  if [ -n "${run_pid:-}" ]; then
    kill -TERM "$run_pid" 2>/dev/null || true
    wait "$run_pid" 2>/dev/null || true
  fi
  if [ -n "${running:-}" ]; then
    # Killing prlctl does not stop the runner in the VM; this does, and its Job Object
    # takes the cargo and test processes along.
    scripts/timeout.sh 120 prlctl exec "$vm" --current-user powershell -NoProfile \
      -NonInteractive -ExecutionPolicy Bypass -File "$runner" -Name "$name" -Kill || true
  fi
  if [ -n "${have_run_lock:-}" ]; then
    take_merge_lock "$vm_lock" "windows-gate $name (stop)" 600 || exit "$st"
    release_merge_lock "$run_lock"
    if [ -n "$prior_state" ] && [ "${PSH_WINDOWS_KEEP_VM:-}" != 1 ] &&
      ! ls -d "$locks"/run-*.lock >/dev/null 2>&1; then
      case $prior_state in
        suspended | paused)
          echo "windows-gate: suspending the VM again"
          prlctl suspend "$vm" >/dev/null || true
          ;;
        stopped)
          echo "windows-gate: stopping the VM again"
          prlctl stop "$vm" >/dev/null || true
          ;;
      esac
    fi
    release_merge_lock "$vm_lock"
  fi
  exit "$st"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

take_merge_lock "$run_lock" "windows-gate $name" 3600 || exit 2
have_run_lock=1

vm_state() {
  prlctl list -a -o status,name | awk -v vm="$vm" 'NR > 1 {
    st = $1; sub(/^[^ ]+ +/, ""); if ($0 == vm) print st }'
}

take_merge_lock "$vm_lock" "windows-gate $name (start)" 600 || exit 2
state=$(vm_state)
case $state in
  running) ;;
  suspended | paused | stopped)
    prior_state=$state
    echo "windows-gate: the VM is $state; starting it"
    case $state in
      suspended | paused) prlctl resume "$vm" >/dev/null ;;
      stopped) prlctl start "$vm" >/dev/null ;;
    esac
    ;;
  "")
    echo "windows-gate: no VM named '$vm' (prlctl list -a; PSH_WINDOWS_VM)" >&2
    release_merge_lock "$vm_lock"
    exit 3
    ;;
  *)
    echo "windows-gate: the VM is $state; try again later" >&2
    release_merge_lock "$vm_lock"
    exit 3
    ;;
esac
release_merge_lock "$vm_lock"

# Wait until the guest tools answer (a fresh boot takes a while).
waited=0
until scripts/timeout.sh 30 prlctl exec "$vm" --current-user cmd /c "exit 0" >/dev/null 2>&1; do
  if [ "$waited" -ge 300 ]; then
    echo "windows-gate: the VM does not answer prlctl exec after ${waited}s" >&2
    exit 3
  fi
  sleep 5
  waited=$((waited + 5))
done

# In the background, so a signal to this script runs its trap at once instead of after
# prlctl returns (timeout.sh puts prlctl into a process group of its own, out of reach of
# a signal to ours).
running=1
scripts/timeout.sh "$backstop" prlctl exec "$vm" --current-user powershell -NoProfile \
  -NonInteractive -ExecutionPolicy Bypass -File "$runner" -Repo "$repo_share" \
  -Ref "refs/windows-gate/$name" -Name "$name" -Toolchain "$toolchain" -Msrv "$msrv" \
  -Legs "$selected" &
run_pid=$!
st=0
wait "$run_pid" || st=$?
run_pid=
# The runner's own ends (0, 1, 2, and 124 for a leg timeout) leave nothing behind: its
# exit closes the job. 124 can also be the backstop, and >128 a signal; then the cleanup
# kills the runner.
case $st in
  0 | 1 | 2) running= ;;
esac
case $st in
  0) echo "windows-gate: passed" ;;
  *) echo "windows-gate: failed (exit $st)" >&2 ;;
esac
exit "$st"
