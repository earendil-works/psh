#!/usr/bin/env bash
# Tests for the gate cache in scripts/gate.sh, in a throwaway repository with stub legs.
# Run by the gate; takes a few seconds.
set -uo pipefail
src=$(cd "$(dirname "$0")" && pwd)
tmp=$(mktemp -d "${TMPDIR:-/tmp}/psh-gate-cache-test.XXXXXX")
tmp=$(cd "$tmp" && pwd -P)
trap 'rm -rf "$tmp"' EXIT

unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE PSH_GATE_NOCACHE
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=test GIT_AUTHOR_EMAIL=test@example.com
export GIT_COMMITTER_NAME=test GIT_COMMITTER_EMAIL=test@example.com
failures=0
fail() {
  echo "FAIL: $*" >&2
  failures=$((failures + 1))
}

repo=$tmp/repo
git init -q -b main "$repo"
mkdir -p "$repo/scripts/gate.d" "$repo/src"
cp "$src/gate.sh" "$src/timeout.sh" "$repo/scripts/"
# a: cached by src/; b: no inputs, never cached; c: opts out of the cache; d: fails on
# demand. Each counts its runs in $COUNTS/<leg>.
cat >"$repo/scripts/gate.d/10-a.sh" <<'EOF'
# timeout: 10
# inputs: src/
echo run >>"$COUNTS/a"
EOF
cat >"$repo/scripts/gate.d/20-b.sh" <<'EOF'
# timeout: 10
echo run >>"$COUNTS/b"
EOF
cat >"$repo/scripts/gate.d/30-c.sh" <<'EOF'
# timeout: 10
# inputs: src/
echo run >>"$COUNTS/c"
touch "$PSH_GATE_NOCACHE"
EOF
cat >"$repo/scripts/gate.d/40-d.sh" <<'EOF'
# timeout: 10
# inputs: src/
echo run >>"$COUNTS/d"
[ -z "${FAIL_D:-}" ]
EOF
printf 'work/scratch/\n' >"$repo/.gitignore"
echo one >"$repo/src/x"
echo doc >"$repo/notes.md"
git -C "$repo" add -A
git -C "$repo" commit -qm base
export COUNTS=$tmp/counts
mkdir -p "$COUNTS"

gate() { (cd "$repo" && scripts/gate.sh "$@") >"$tmp/out" 2>&1; }
runs() { if [ -f "$COUNTS/$1" ]; then wc -l <"$COUNTS/$1" | tr -d ' '; else echo 0; fi; }
expect() { # expect LEG N WHAT
  [ "$(runs "$1")" = "$2" ] || fail "$3: leg $1 ran $(runs "$1") times, want $2"
}
commit() { git -C "$repo" add -A && git -C "$repo" commit -qm "$1"; }

gate || fail "first gate failed: $(cat "$tmp/out")"
expect a 1 "first run"
expect b 1 "first run"
expect c 1 "first run"

gate || fail "second gate failed"
expect a 1 "unchanged inputs"
grep -q "== gate: a cached" "$tmp/out" || fail "no cache note"
expect b 2 "a leg without inputs"
expect c 2 "a leg that opted out"

echo more >>"$repo/notes.md"
commit "docs only"
gate || fail "docs-only gate failed"
expect a 1 "a change outside the inputs"

echo two >"$repo/src/x"
commit "source change"
gate || fail "gate after a source change failed"
expect a 2 "a change of an input"

echo dirty >>"$repo/src/x"
gate || fail "gate on a dirty input failed"
expect a 3 "an uncommitted input"
gate || fail "gate on a dirty input failed"
expect a 4 "an uncommitted input is never cached"
git -C "$repo" checkout -q -- src/x
echo new >"$repo/src/untracked"
gate || fail "gate with an untracked input failed"
expect a 5 "an untracked input"
rm "$repo/src/untracked"
gate || fail "gate on the clean tree failed"
expect a 5 "the clean tree was cached before"

gate --full || fail "--full gate failed"
expect a 6 "--full"

echo "# a comment" >>"$repo/scripts/gate.d/10-a.sh"
commit "leg change"
gate || fail "gate after a leg change failed"
expect a 7 "a change of the leg file"

echo three >"$repo/src/x"
commit "for d"
FAIL_D=1 gate && fail "a failing leg passed"
expect d 7 "failing leg"
gate || fail "gate after the failure failed"
expect d 8 "a failed leg is not cached"

[ -d "$repo/work/scratch/gate-cache" ] || fail "no cache directory in the primary checkout"

if [ $failures -gt 0 ]; then
  echo "test-gate-cache: $failures failure(s)" >&2
  exit 1
fi
echo "test-gate-cache: ok"
