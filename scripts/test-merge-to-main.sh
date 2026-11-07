#!/usr/bin/env bash
# Tests for scripts/merge-to-main.sh and scripts/with-merge-lock.sh, end to end in a
# throwaway repository (primary checkout + issue worktrees) with a stub gate. Run by the
# gate; takes a few seconds.
set -uo pipefail
src=$(cd "$(dirname "$0")" && pwd)
tmp=$(mktemp -d "${TMPDIR:-/tmp}/psh-merge-test.XXXXXX")
tmp=$(cd "$tmp" && pwd -P)
# KEEP=1 keeps the throwaway repository and outputs for a look after a failure.
trap '[ -n "${KEEP:-}" ] || rm -rf "$tmp"' EXIT

# Isolate git from the caller: no user config (signing, hooks), no repository env (the
# outer merge's gate runs this).
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE PSH_MERGE_SCRIPTS
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=test GIT_AUTHOR_EMAIL=test@example.com
export GIT_COMMITTER_NAME=test GIT_COMMITTER_EMAIL=test@example.com
export PSH_MERGE_POLL=1
failures=0

fail() {
  echo "FAIL: $*" >&2
  failures=$((failures + 1))
}

main=$tmp/repo
git init -q -b main "$main"
mkdir -p "$main/scripts"
cp "$src/merge-to-main.sh" "$src/merge-lock.sh" "$src/with-merge-lock.sh" "$src/timeout.sh" "$main/scripts/"
# The stub gate: fails when the tree has a file FAIL, counts its runs, and runs
# $GATE_HOOK in its first GATE_HOOK_TIMES runs (default 1), to move main while it gates.
cat >"$main/scripts/gate.sh" <<'EOF'
#!/usr/bin/env bash
echo run >>"$GATE_LOG"
if [ -n "${GATE_HOOK:-}" ] && [ "$(wc -l <"$GATE_LOG")" -le "${GATE_HOOK_TIMES:-1}" ]; then
  eval "$GATE_HOOK"
fi
[ ! -e FAIL ]
EOF
chmod +x "$main/scripts/"*.sh
printf 'work/scratch/\n.worktrees/\n' >"$main/.gitignore"
echo base >"$main/file"
git -C "$main" add -A
git -C "$main" commit -qm base
export GATE_LOG=$tmp/gate.log

# new_issue NAME: a worktree on issue/NAME with one commit adding NAME.txt.
new_issue() {
  git -C "$main" worktree add -q -b "issue/$1" "$main/.worktrees/$1" main
  echo "$1" >"$main/.worktrees/$1/$1.txt"
  git -C "$main/.worktrees/$1" add -A
  git -C "$main/.worktrees/$1" commit -qm "$1"
}

# merge NAME: run merge-to-main.sh in that issue's worktree; output to $tmp/out.
merge() {
  rm -f "$GATE_LOG"
  (cd "$main/.worktrees/$1" && "$main/scripts/timeout.sh" 60 scripts/merge-to-main.sh) >"$tmp/out" 2>&1
}

on_main() { git -C "$main" cat-file -e "main:$1" 2>/dev/null; }
gate_runs() { if [ -e "$GATE_LOG" ]; then wc -l <"$GATE_LOG" | tr -d ' '; else echo 0; fi; }
lock_free() { [ ! -e "$main/work/scratch/merge.lock" ]; }
check_linear() {
  [ "$(git -C "$main" rev-list --min-parents=2 --count main)" = 0 ] || fail "$1: main has a merge commit"
}

# 1. A plain merge: rebase, gate, fast-forward the primary checkout.
new_issue 0001-a
merge 0001-a || fail "plain merge failed: $(cat "$tmp/out")"
on_main 0001-a.txt || fail "plain merge: main lacks the branch's commit"
[ -e "$main/0001-a.txt" ] || fail "plain merge: the primary checkout's files were not updated"
[ "$(gate_runs)" = 1 ] || fail "plain merge: gate ran $(gate_runs) times"
lock_free || fail "plain merge: lock left behind"
grep -q "issue/0001-a	merged" "$main/work/scratch/merge-log.tsv" || fail "plain merge: no merge-log line"

# 2. Refusals: not an issue branch, the primary checkout, a dirty worktree.
new_issue 0002-b
git -C "$main/.worktrees/0002-b" checkout -q -b feature
merge 0002-b && fail "merged from a non-issue branch"
git -C "$main/.worktrees/0002-b" checkout -q issue/0002-b
(cd "$main" && git checkout -q -b issue/9999-x && scripts/timeout.sh 60 scripts/merge-to-main.sh) >"$tmp/out" 2>&1 &&
  fail "merged from the primary checkout"
git -C "$main" checkout -q main
git -C "$main" branch -q -D issue/9999-x
echo dirty >>"$main/.worktrees/0002-b/0002-b.txt"
merge 0002-b && fail "merged a dirty worktree"
git -C "$main/.worktrees/0002-b" checkout -q -- .
echo untracked >"$main/.worktrees/0002-b/new.txt"
merge 0002-b && fail "merged a worktree with untracked files"
rm "$main/.worktrees/0002-b/new.txt"
on_main 0002-b.txt && fail "a refused merge moved main"

# 3. The gate fails: main stays, the lock is free.
echo x >"$main/.worktrees/0002-b/FAIL"
git -C "$main/.worktrees/0002-b" add -A
git -C "$main/.worktrees/0002-b" commit -qm "fail the gate"
merge 0002-b && fail "merged past a failing gate"
on_main 0002-b.txt && fail "a failing gate moved main"
lock_free || fail "failing gate: lock left behind"
git -C "$main/.worktrees/0002-b" rm -q FAIL
git -C "$main/.worktrees/0002-b" commit -qm "pass the gate"

# 4. The lock is held from the start of the gate: a coordinator commit through
# with-merge-lock while the gate runs waits until main has fast-forwarded and lands on
# top; the gate runs once.
# (The edit is made under the lock too: an edit before it makes the primary dirty, the
# case of section 5.)
echo "cd '$main' && scripts/with-merge-lock.sh sh -c 'echo c >c.txt && git add c.txt && git commit -qm c'" \
  >"$tmp/move-main.sh"
GATE_HOOK="test -d '$main/work/scratch/merge.lock' || echo unlocked >>'$tmp/unlocked'; \
  (bash $tmp/move-main.sh >'$tmp/coord.out' 2>&1 &)" merge 0002-b ||
  fail "merge with a waiting commit failed: $(cat "$tmp/out")"
[ -e "$tmp/unlocked" ] && fail "lock held: the gate ran without the merge lock"
[ "$(gate_runs)" = 1 ] || fail "lock held: gate ran $(gate_runs) times, want 1"
for _ in $(seq 1 20); do on_main c.txt && break; sleep 1; done
on_main 0002-b.txt && on_main c.txt || fail "lock held: main lacks a commit"
[ "$(git -C "$main" log -1 --format=%s main)" = c ] || fail "lock held: the waiting commit did not land on top"
grep -q "waiting for the merge lock (held by merge-to-main issue/0002-b since " "$tmp/coord.out" ||
  fail "lock held: no waiting message: $(cat "$tmp/coord.out")"
check_linear "lock held"
lock_free || fail "lock held: lock left behind"

# ... two merges at once are serialized: the second waits for the first's lock, then
# rebases onto it and gates once; no gate is wasted.
new_issue 0006-f
new_issue 0007-g
rm -f "$GATE_LOG"
(cd "$main/.worktrees/0006-f" && GATE_HOOK="sleep 3" "$main/scripts/timeout.sh" 60 scripts/merge-to-main.sh) \
  >"$tmp/out-f" 2>&1 &
first=$!
for _ in $(seq 1 20); do [ -e "$GATE_LOG" ] && break; sleep 0.2; done
(cd "$main/.worktrees/0007-g" && "$main/scripts/timeout.sh" 60 scripts/merge-to-main.sh) >"$tmp/out-g" 2>&1 ||
  fail "serialized: the second merge failed: $(cat "$tmp/out-g")"
wait "$first" || fail "serialized: the first merge failed: $(cat "$tmp/out-f")"
on_main 0006-f.txt && on_main 0007-g.txt || fail "serialized: main lacks a commit"
[ "$(gate_runs)" = 2 ] || fail "serialized: gate ran $(gate_runs) times, want 2"
grep -q "waiting for the merge lock (held by merge-to-main issue/0006-f since " "$tmp/out-g" ||
  fail "serialized: no waiting message: $(cat "$tmp/out-g")"
grep -q "main moved" "$tmp/out-g" && fail "serialized: the second merge gated twice"
[ "$(git -C "$main" log -1 --format=%s main)" = 0007-g ] || fail "serialized: wrong order"
check_linear "serialized"
lock_free || fail "serialized: lock left behind"

# ... and a commit made without the lock while the gate runs: rebase and gate again.
new_issue 0008-h
echo "cd '$main' && date >>moves.txt && git add moves.txt && git commit -qm move" >"$tmp/move-main.sh"
GATE_HOOK="bash $tmp/move-main.sh" merge 0008-h || fail "merge after an unlocked move failed: $(cat "$tmp/out")"
on_main 0008-h.txt && on_main moves.txt || fail "unlocked move: main lacks a commit"
[ "$(gate_runs)" = 2 ] || fail "unlocked move: gate ran $(gate_runs) times, want 2"
grep -q "main moved while gating, without the merge lock" "$tmp/out" || fail "unlocked move: no message"
check_linear "unlocked move"
lock_free || fail "unlocked move: lock left behind"

# 5. A dirty primary checkout: wait until it is committed, then land on top of it.
new_issue 0003-c
echo edit >>"$main/file"
(
  sleep 3
  cd "$main" && scripts/with-merge-lock.sh git commit -qam "coordinator edit"
) &
merge 0003-c || fail "merge with a dirty primary failed: $(cat "$tmp/out")"
wait
grep -q "uncommitted changes" "$tmp/out" || fail "dirty primary: no waiting message"
on_main 0003-c.txt || fail "dirty primary: main lacks the branch's commit"
[ "$(git -C "$main" log -1 --format=%s main)" = 0003-c ] || fail "dirty primary: not rebased onto the edit"
[ "$(gate_runs)" = 2 ] || fail "dirty primary: gate ran $(gate_runs) times, want 2 (before and after the edit)"
check_linear "dirty primary"

# ... and gives up after PSH_MERGE_DIRTY_WAIT.
new_issue 0004-d
echo edit2 >>"$main/file"
PSH_MERGE_DIRTY_WAIT=1 merge 0004-d && fail "merged into a dirty primary checkout"
on_main 0004-d.txt && fail "dirty primary: main moved anyway"
lock_free || fail "dirty primary: lock left behind"
git -C "$main" checkout -q -- file

# 6. A rebase conflict stops with the rebase in progress.
new_issue 0005-e
echo e1 >"$main/.worktrees/0005-e/file"
git -C "$main/.worktrees/0005-e" commit -qam "conflict"
echo main-side >"$main/file"
git -C "$main" commit -qam "main side"
merge 0005-e && fail "merged through a rebase conflict"
grep -q "conflicts" "$tmp/out" || fail "conflict: no message"
git -C "$main/.worktrees/0005-e" rebase --abort

# 7. A stale lock (its holder is gone) is removed; a live one is waited for.
mkdir -p "$main/work/scratch/merge.lock"
echo "ghost since then pid=999999" >"$main/work/scratch/merge.lock/owner"
merge 0004-d || fail "stale lock blocked the merge: $(cat "$tmp/out")"
grep -q "stale lock" "$tmp/out" || fail "stale lock: no message"
sleep 30 &
holder=$!
mkdir -p "$main/work/scratch/merge.lock"
echo "someone pid=$holder" >"$main/work/scratch/merge.lock/owner"
(cd "$main" && PSH_MERGE_LOCK_WAIT=1 scripts/with-merge-lock.sh true) >"$tmp/out" 2>&1 &&
  fail "with-merge-lock ran while the lock was held"
grep -q "gave up" "$tmp/out" || fail "live lock: no message"
kill "$holder"
wait "$holder" 2>/dev/null
rm -rf "$main/work/scratch/merge.lock"

# 8. with-merge-lock passes the status through and releases the lock.
(cd "$main" && scripts/with-merge-lock.sh sh -c 'test -d work/scratch/merge.lock && exit 7')
st=$?
[ "$st" = 7 ] || fail "with-merge-lock: status $st, want 7 (lock held while running)"
lock_free || fail "with-merge-lock: lock left behind"

if [ $failures -gt 0 ]; then
  echo "test-merge-to-main: $failures failure(s)" >&2
  exit 1
fi
echo "test-merge-to-main: ok"
