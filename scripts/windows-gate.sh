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
# has the VM fetch that ref over the Parallels share into C:\psh-gate\shared, the one
# checkout every run uses, with a warm target\. scripts/windows-gate.ps1 runs the legs
# there, each under its own timeout, and the output streams back.
#
# Runs take turns (D1072): a run waits for the one before it ("waiting for the Windows
# VM"), at most PSH_WINDOWS_QUEUE_WAIT seconds (default 3600); the wait does not count
# against the legs' timeouts. The VM is resumed or started if needed; a run that ends
# with no other run waiting puts it back to sleep, and only if a run woke it: a VM that
# was already running (the owner's) stays running.
#
#   PSH_WINDOWS_VM          the VM's name (default "Windows 11")
#   PSH_WINDOWS_KEEP_VM     1: leave the VM running afterwards
#   PSH_WINDOWS_QUEUE_WAIT  seconds to wait for the VM's turn (default 3600)
#
# Exit status: 0 passed, 1 a leg failed, 124 a leg (or the whole run) timed out, 2 setup
# failed, 3 the VM is not usable (or the turn did not come).
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/merge-lock.sh

vm=${PSH_WINDOWS_VM:-Windows 11}

# name:timeout-seconds, in the order they run. The commands are in windows-gate.ps1.
LEGS="
fmt:300
clippy:1200
test:1200
test-x64:1200
msrv:900
spec:900
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
# The whole run, setup included, after its turn came; 99-windows.sh's timeout allows for
# the longest queue wait plus this.
[ "$backstop" -le 3600 ] || backstop=3600

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

# --- one run at a time (D1072): the VM has few cores, and the runs share one checkout
# and target\ in it. In the primary checkout's work/scratch/windows-gate/:
#   run.lock       the run using the VM (its owner file names it and its PID, so the
#                  lock of a run killed with SIGKILL is detected and taken over)
#   waiting/       one file NAME.PID per queued run
#   resumed        exists while the VM runs because a run woke it; holds the state it
#                  was in. Without it a running VM is the owner's and stays running.
# The lock is held from looking at the VM until it is asleep again, so no run ever sees
# it between states. A run that ends with nobody queued puts the VM back to sleep, and
# only if a run woke it.
locks="$(primary_checkout)/work/scratch/windows-gate"
run_lock="$locks/run.lock"
legacy_lock="$locks/run-$name.lock"
waiting="$locks/waiting"
resumed="$locks/resumed"
queue_wait=${PSH_WINDOWS_QUEUE_WAIT:-3600}
mkdir -p "$waiting"

lock_pid() { sed -n 's/.*pid=\([0-9][0-9]*\).*/\1/p' "$1" 2>/dev/null || true; }

# Whether another run is queued (dropping the entries of dead ones), or still uses the
# VM through the older per-worktree locks (run-NAME.lock, until every branch has rebased).
others_queued() {
  local f pid found=1
  for f in "$waiting"/*; do
    [ -f "$f" ] && [ "$f" != "$waiting/$name.$$" ] || continue
    pid=${f##*.}
    if kill -0 "$pid" 2>/dev/null; then found=0; else rm -f "$f"; fi
  done
  for f in "$locks"/run-*.lock; do
    [ -d "$f" ] && [ "$f" != "$legacy_lock" ] || continue
    pid=$(lock_pid "$f/owner")
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then found=0; fi
  done
  return $found
}

take_vm() {
  local waited=0 next_note=0 owner pid
  : >"$waiting/$name.$$"
  until mkdir "$run_lock" 2>/dev/null; do
    owner=$(cat "$run_lock/owner" 2>/dev/null || true)
    pid=$(lock_pid "$run_lock/owner")
    if [ -n "$pid" ] && ! kill -0 "$pid" 2>/dev/null; then
      echo "windows-gate: taking over the lock of a dead run ($owner)" >&2
      rm -rf "$run_lock"
      continue
    fi
    if [ "$waited" -ge "$next_note" ]; then
      echo "windows-gate: waiting for the Windows VM (held by ${owner:-a run just starting})" >&2
      next_note=$((next_note + 300))
    fi
    if [ "$waited" -ge "$queue_wait" ]; then
      echo "windows-gate: gave up waiting for the Windows VM after ${waited}s" \
        "(PSH_WINDOWS_QUEUE_WAIT); held by: ${owner:-unknown}" >&2
      rm -f "$waiting/$name.$$"
      return 1
    fi
    sleep 2
    waited=$((waited + 2))
  done
  echo "$name since $(date '+%F %T') pid=$$" >"$run_lock/owner"
  rm -f "$waiting/$name.$$"
  # Until every branch has rebased: the older script counts the runs by run-NAME.lock
  # and would suspend the VM under this run without one.
  if mkdir "$legacy_lock" 2>/dev/null; then
    echo "windows-gate $name pid=$$" >"$legacy_lock/owner"
    have_legacy=1
  fi
}

# prlctl's VM management hangs at times; every call is bounded.
prl() { scripts/timeout.sh "$1" prlctl "${@:2}"; }

vm_state() {
  prl 60 list -a -o status,name | awk -v vm="$vm" 'NR > 1 {
    st = $1; sub(/^[^ ]+ +/, ""); if ($0 == vm) print st }'
}

# The VM's state once it is not between states (suspending, resuming, starting, ...: the
# owner, or Parallels itself, may have started a transition); empty if there is no such VM
# or prlctl does not answer. Waits at most 300s, then prints the transitional state.
settled_state() {
  local st waited=0
  while :; do
    st=$(vm_state || true)
    case $st in
      running | suspended | paused | stopped | "") break ;;
    esac
    [ "$waited" -ge 300 ] && break
    [ "$waited" = 0 ] && echo "windows-gate: the VM is $st; waiting for it to settle" >&2
    sleep 2
    waited=$((waited + 2))
  done
  printf '%s\n' "$st"
}

# Makes the VM running and answering prlctl exec, waking it if needed.
ensure_vm() {
  local state ok waited=0
  state=$(settled_state)
  case $state in
    running) ;;
    suspended | paused | stopped)
      echo "windows-gate: the VM is $state; starting it"
      [ -f "$resumed" ] || echo "$state" >"$resumed"
      case $state in
        suspended | paused) ok=$(prl 180 resume "$vm" >/dev/null && echo 1 || true) ;;
        stopped) ok=$(prl 300 start "$vm" >/dev/null && echo 1 || true) ;;
      esac
      state=$(settled_state)
      if [ -z "$ok" ] || [ "$state" != running ]; then
        echo "windows-gate: could not start the VM '$vm' (now: ${state:-unknown})" >&2
        return 3
      fi
      ;;
    "")
      echo "windows-gate: no VM named '$vm', or prlctl does not answer (PSH_WINDOWS_VM)" >&2
      return 3
      ;;
    *)
      echo "windows-gate: the VM is still '$state' after 300s; cannot use it now" >&2
      return 3
      ;;
  esac
  # Wait until the guest tools answer for the logged-in user (a fresh boot takes a
  # while; without a logged-in user --current-user never works).
  until prl 30 exec "$vm" --current-user cmd /c "exit 0" >/dev/null 2>&1; do
    if [ "$waited" -ge 300 ]; then
      echo "windows-gate: the VM '$vm' does not answer prlctl exec --current-user after" \
        "${waited}s (is a user logged in?)" >&2
      return 3
    fi
    sleep 5
    waited=$((waited + 5))
  done
}

cleanup() {
  local st=$? prior
  trap - EXIT INT TERM HUP
  rm -f "$waiting/$name.$$"
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
  if [ -n "${have_lock:-}" ]; then
    if [ -f "$resumed" ] && ! others_queued; then
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
        settled_state >/dev/null
      fi
    fi
    [ -z "${have_legacy:-}" ] || rm -rf "$legacy_lock"
    rm -rf "$run_lock"
  fi
  exit "$st"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

take_vm || exit 3
have_lock=1

# In the background, so a signal to this script runs its trap at once instead of after
# prlctl returns (timeout.sh puts prlctl into a process group of its own, out of reach of
# a signal to ours).
# The runner also writes its exit status to a file on the share: prlctl now and then
# fails to fetch a guest's exit code (`PrlJob_GetRetCode`, status 255) for a run that
# went through, and the file still says how it ended.
result="$locks/result-$name-$$.txt"
result_share=$(to_share "$result" | tr / '\\')
run() {
  running=1
  rm -f "$result"
  scripts/timeout.sh "$backstop" prlctl exec "$vm" --current-user powershell -NoProfile \
    -NonInteractive -ExecutionPolicy Bypass -File "$runner" -Repo "$repo_share" \
    -Ref "refs/windows-gate/$name" -Name "$name" -Toolchain "$toolchain" -Msrv "$msrv" \
    -Legs "$selected" -ResultFile "$result_share" &
  run_pid=$!
  st=0
  wait "$run_pid" || st=$?
  run_pid=
  if [ "$st" = 255 ] && [ -s "$result" ]; then
    st=$(tr -dc '0-9' <"$result")
    echo "windows-gate: prlctl lost the runner's exit status; its result file says $st" >&2
  fi
  rm -f "$result"
}

st=0
ensure_vm || st=$?
[ "$st" = 0 ] || exit "$st"
run
# 255 is prlctl's own failure (PrlJob_...), never the runner's: the VM went away under
# the run (the owner suspended it, say). Once more, from a settled VM; a runner that
# survived is killed by the next one (windows-gate.ps1).
if [ "$st" = 255 ]; then
  echo "windows-gate: prlctl exec failed (the VM is $(vm_state || true)); retrying once" >&2
  st=0
  ensure_vm || st=$?
  [ "$st" = 0 ] || exit "$st"
  run
fi
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
