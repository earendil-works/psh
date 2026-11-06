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
# The VM is resumed or started if needed. The last run out puts it back to sleep, and
# only if a run woke it: a VM that was already running (the owner's) stays running.
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

# --- sharing the VM between concurrent runs (several gates run at once, D1010). In the
# primary checkout's work/scratch/windows-gate/:
#   run-NAME.lock  one per run (and one run per NAME: it owns C:\psh-gate\NAME); the
#                  runs using the VM, a reference count with PIDs, so a run killed with
#                  SIGKILL does not count forever
#   vm.lock        held briefly while a run starts the VM or decides to put it back
#   resumed        exists while the VM runs because a run started it; holds the state
#                  it was in. Without it a running VM is the owner's and stays running.
# The last run out puts the VM back, and only if a run started it.
locks="$(primary_checkout)/work/scratch/windows-gate"
run_lock="$locks/run-$name.lock"
vm_lock="$locks/vm.lock"
resumed="$locks/resumed"

# Whether a run other than this one still uses the VM; drops the locks of dead runs.
other_runs() {
  local l pid found=1
  for l in "$locks"/run-*.lock; do
    [ -d "$l" ] && [ "$l" != "$run_lock" ] || continue
    pid=$(sed -n 's/.*pid=\([0-9][0-9]*\).*/\1/p' "$l/owner" 2>/dev/null || true)
    if [ -n "$pid" ] && ! kill -0 "$pid" 2>/dev/null; then
      rm -rf "$l"
    else
      found=0
    fi
  done
  return $found
}

# prlctl's VM management hangs at times; every call is bounded.
prl() { scripts/timeout.sh "$1" prlctl "${@:2}"; }

cleanup() {
  local st=$? prior
  trap - EXIT INT TERM HUP
  if [ -n "${run_pid:-}" ]; then
    kill -TERM "$run_pid" 2>/dev/null || true
    wait "$run_pid" 2>/dev/null || true
  fi
  if [ -n "${running:-}" ]; then
    # Killing prlctl does not stop the runner in the VM; this does, and its Job Object
    # takes the cargo and test processes along.
    prl 60 exec "$vm" --current-user powershell -NoProfile -NonInteractive \
      -ExecutionPolicy Bypass -File "$runner" -Name "$name" -Kill || true
  fi
  if [ -n "${have_run_lock:-}" ]; then
    take_merge_lock "$vm_lock" "windows-gate $name (stop)" 300 || exit "$st"
    release_merge_lock "$run_lock"
    if [ -f "$resumed" ] && ! other_runs; then
      prior=$(cat "$resumed")
      rm -f "$resumed"
      if [ "${PSH_WINDOWS_KEEP_VM:-}" = 1 ]; then
        echo "windows-gate: leaving the VM running (PSH_WINDOWS_KEEP_VM=1)"
      else
        case $prior in
          stopped)
            echo "windows-gate: shutting the VM down again"
            prl 300 stop "$vm" >/dev/null || true
            ;;
          *)
            echo "windows-gate: suspending the VM again"
            prl 300 suspend "$vm" >/dev/null || true
            ;;
        esac
      fi
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
  prl 60 list -a -o status,name | awk -v vm="$vm" 'NR > 1 {
    st = $1; sub(/^[^ ]+ +/, ""); if ($0 == vm) print st }'
}

take_merge_lock "$vm_lock" "windows-gate $name (start)" 300 || exit 2
state=$(vm_state || true)
case $state in
  running) ;;
  suspended | paused | stopped)
    echo "windows-gate: the VM is $state; starting it"
    [ -f "$resumed" ] || echo "$state" >"$resumed"
    case $state in
      suspended | paused) ok=$(prl 180 resume "$vm" >/dev/null && echo 1 || true) ;;
      stopped) ok=$(prl 300 start "$vm" >/dev/null && echo 1 || true) ;;
    esac
    if [ -z "$ok" ]; then
      echo "windows-gate: could not start the VM '$vm' (prlctl $state -> running failed)" >&2
      release_merge_lock "$vm_lock"
      exit 3
    fi
    ;;
  "")
    echo "windows-gate: no VM named '$vm', or prlctl does not answer (PSH_WINDOWS_VM)" >&2
    release_merge_lock "$vm_lock"
    exit 3
    ;;
  *)
    echo "windows-gate: the VM is '$state'; cannot use it now" >&2
    release_merge_lock "$vm_lock"
    exit 3
    ;;
esac
release_merge_lock "$vm_lock"

# Wait until the guest tools answer for the logged-in user (a fresh boot takes a while;
# without a logged-in user --current-user never works).
waited=0
until prl 30 exec "$vm" --current-user cmd /c "exit 0" >/dev/null 2>&1; do
  if [ "$waited" -ge 300 ]; then
    echo "windows-gate: the VM '$vm' does not answer prlctl exec --current-user after" \
      "${waited}s (is a user logged in?)" >&2
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
