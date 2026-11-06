#!/usr/bin/env bash
# Tests for scripts/timeout.sh. Run by the gate; takes a few seconds.
set -uo pipefail
cd "$(dirname "$0")/.."
t=scripts/timeout.sh
tmp=$(mktemp -d "${TMPDIR:-/tmp}/psh-timeout-test.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
export PSH_TIMEOUT_GRACE=1
# The nice test below needs timeout.sh to nice; the gate sets PSH_GATE=1.
unset PSH_GATE
failures=0

fail() {
  echo "FAIL: $*" >&2
  failures=$((failures + 1))
}

# expect_status WANT CMD...: run CMD, compare its exit status.
expect_status() {
  local want=$1 got
  shift
  "$@" >/dev/null 2>&1
  got=$?
  [ "$got" = "$want" ] || fail "$* exited $got, want $want"
}

# alive PID: whether the process exists (and is not a zombie).
alive() {
  local state
  state=$(ps -o stat= -p "$1" 2>/dev/null) || return 1
  case "$state" in Z*) return 1 ;; esac
  return 0
}

# The exit status passes through, and signal deaths become 128+N.
expect_status 0 $t 5 true
expect_status 3 $t 5 sh -c 'exit 3'
expect_status 137 $t 5 sh -c 'kill -KILL $$'
expect_status 127 $t 5 "$tmp/no-such-command"
expect_status 125 $t 5x true
expect_status 125 $t 5

# Arguments, stdin and stdout pass through unchanged.
out=$(printf 'in\n' | $t 5 sh -c 'printf "%s|" "$@"; cat' sh 'a b' '' c)
[ "$out" = "a b||c|in" ] || fail "passthrough: got '$out'"

# Timeout: status 124, the whole group dies, including a background grandchild.
start=$(date +%s)
expect_status 124 $t 1 sh -c 'sleep 30 & echo $! > "$1"; wait' sh "$tmp/pid1"
elapsed=$(($(date +%s) - start))
[ "$elapsed" -le 4 ] || fail "timeout took ${elapsed}s, want about 1s"
sleep 0.2
alive "$(cat "$tmp/pid1")" && fail "grandchild survived the timeout"

# Fractional durations work.
expect_status 124 $t 0.3 sleep 5

# A group that ignores TERM gets KILL after the grace period.
start=$(date +%s)
expect_status 124 $t 1 sh -c 'trap "" TERM; sleep 30 & echo $! > "$1"; while :; do sleep 0.1; done' sh "$tmp/pid2"
elapsed=$(($(date +%s) - start))
[ "$elapsed" -le 6 ] || fail "TERM-ignoring group took ${elapsed}s to die"
sleep 0.2
alive "$(cat "$tmp/pid2")" && fail "TERM-ignoring grandchild survived"

# A TERM sent to timeout.sh is forwarded to the group (status 143).
$t 30 sh -c 'echo $$ > "$1"; sleep 30 & echo $! >> "$1"; wait' sh "$tmp/pid3" >/dev/null 2>&1 &
tpid=$!
i=0
while [ "$(wc -l 2>/dev/null <"$tmp/pid3" || echo 0)" -lt 2 ] && [ $i -lt 100 ]; do
  sleep 0.05
  i=$((i + 1))
done
kill -TERM "$tpid"
wait "$tpid"
st=$?
[ "$st" = 143 ] || fail "TERM to timeout.sh: status $st, want 143"
sleep 0.2
for p in $(cat "$tmp/pid3"); do alive "$p" && fail "process $p survived TERM to timeout.sh"; done

# The command runs niced by 10, except under PSH_GATE=1.
base=$(ps -o nice= -p $$ | tr -d ' ')
want=$((base + 10))
[ "$want" -gt 19 ] && want=19
got=$($t 5 sh -c 'ps -o nice= -p $$' | tr -d ' ')
[ "$got" = "$want" ] || fail "nice: got $got, want $want"
got=$(PSH_GATE=1 $t 5 sh -c 'ps -o nice= -p $$' | tr -d ' ')
[ "$got" = "$base" ] || fail "nice under PSH_GATE=1: got $got, want $base"

if [ $failures -gt 0 ]; then
  echo "test-timeout: $failures failure(s)" >&2
  exit 1
fi
echo "test-timeout: ok"
