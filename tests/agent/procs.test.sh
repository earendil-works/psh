# Agent shape: processes and failure modes. `timeout N cmd` and its 124 status,
# background jobs with `&` and `wait`/`wait $!`, `| head` ending a producer (SIGPIPE,
# PIPESTATUS), `|| true`, `; echo "exit=$?"`, `2>/dev/null`, `2>&1 | tail`, command not
# found (127), missing files, unmatched globs, a failing `cd` in a chain, `set -e`,
# `set -u`, `set -o pipefail` and exit codes from subshells. Tool runs (npm, node,
# python3) use local stubs, as in the corpus shapes `cd w1 && npm run w2`,
# `node --test w1`, `python3 - <<'EOF'`.

#### timeout lets a quick test run finish
mkdir -p bin && printf '#!/bin/sh\necho "ok 1 - parses config"\necho "ok 2 - writes report"\n' > bin/npm && chmod +x bin/npm && export PATH="$PWD/bin:$PATH"
timeout 60 npm test; echo "exit=$?"

#### timeout kills a hung command with 124
timeout 1 sleep 10; echo "exit=$?"

#### timeout with fractional seconds
timeout 0.2 sleep 5
echo "status=$?"

#### timeout keeps output printed before the kill
timeout 1 sh -c 'echo "starting server"; echo "listening on 8080"; sleep 10; echo never'
echo "exit=$?"

#### timeout output piped to tail
timeout 1 sh -c 'for i in 1 2 3 4 5; do echo "line $i"; done; sleep 10' 2>&1 | tail -3
echo "pipe=${PIPESTATUS[0]},${PIPESTATUS[1]}"

#### timeout passes through the command's own failure
timeout 5 sh -c 'echo "1 failing test" >&2; exit 3'; echo "exit=$?"

#### timeout with an s suffix
timeout 1s sleep 5; echo "exit=$?"

#### timeout of a missing command
timeout 5 no-such-tool-xyz --version 2>/dev/null; echo "exit=$?"

#### timeout with || echo fallback
timeout 1 sleep 5 || echo "timed out (status $?)"

#### sleep in the background and wait
sleep 0.1 & wait
echo "done status=$?"

#### cmd & then wait $!
sh -c 'exit 7' &
wait $!
echo "exit=$?"

#### save $! and wait on it later
sleep 0.1 & pid=$!
echo "started"
wait "$pid"; echo "exit=$?"

#### several background jobs write files, then wait
mkdir -p out
for n in a b c; do (sleep 0.1; echo "job $n done" > "out/$n.txt") & done
wait
cat out/a.txt out/b.txt out/c.txt

#### background jobs with distinct statuses
sh -c 'exit 0' & p1=$!
sh -c 'exit 2' & p2=$!
sh -c 'exit 5' & p3=$!
wait $p1; s1=$?
wait $p2; s2=$?
wait $p3; s3=$?
echo "statuses=$s1,$s2,$s3"

#### plain wait returns 0 even if a job failed
false &
sleep 0.1 &
wait
echo "wait=$?"

#### background server stub, then a check, then kill
mkdir -p bin && printf '#!/bin/sh\necho "serving" > server.log\nexec sleep 30\n' > bin/serve && chmod +x bin/serve && export PATH="$PWD/bin:$PATH"
serve & pid=$!
for i in 1 2 3 4 5 6 7 8 9 10; do [ -s server.log ] && break; sleep 0.1; done
cat server.log
kill "$pid"; wait "$pid"; echo "exit=$?"

#### kill -0 checks a live job
sleep 5 & pid=$!
kill -0 "$pid" && echo "alive"
kill -TERM "$pid"
wait "$pid" 2>/dev/null; echo "exit=$?"

#### background job output redirected to a log
(echo "build start"; echo "build ok") > build.log 2>&1 &
wait $!
cat build.log

#### background subshell inside a cd chain
mkdir -p pkg && printf 'x\ny\n' > pkg/data.txt
cd pkg && (sort -r data.txt > sorted.txt) & wait
cat pkg/sorted.txt

#### yes into head ends the producer
yes | head -n 3
echo "pipe=${PIPESTATUS[*]}"

#### seq into head -5, PIPESTATUS
seq 1 100000 | head -5
echo "pipe=${PIPESTATUS[0]},${PIPESTATUS[1]}"

#### SIGPIPE status 141 with pipefail
set -o pipefail
yes | head -n 1
echo "exit=$?"

#### producer loop ended by head
for i in $(seq 1 5000); do echo "row $i"; done | head -2

#### grep into head with no matches
printf 'alpha\nbeta\n' > notes.txt
grep -n "gamma" notes.txt | head -20; echo "pipe=${PIPESTATUS[*]}"

#### grep with no match and || true
printf 'info: ok\n' > app.log
grep ERROR app.log || true
echo "after status=$?"

#### set -e with || true keeps going
set -e
grep -q missing <<< "content" || true
echo "survived"

#### ; echo exit= after a failing test runner
mkdir -p bin && printf '#!/bin/sh\necho "not ok 1 - handles empty input"\necho "# fail 1"\nexit 1\n' > bin/node && chmod +x bin/node && export PATH="$PWD/bin:$PATH"
mkdir -p tests && touch tests/parse.test.js
node --test tests/parse.test.js; echo "exit=$?"

#### npm run chain stops at the failure
mkdir -p bin && printf '#!/bin/sh\necho "> $2"\n[ "$2" = lint ] && exit 2\nexit 0\n' > bin/npm && chmod +x bin/npm && export PATH="$PWD/bin:$PATH"
npm run lint && npm run test
echo "exit=$?"

#### npm run check 2>&1 | tail
mkdir -p bin && printf '#!/bin/sh\nfor i in 1 2 3 4 5 6; do echo "step $i"; done\necho "error: type mismatch" >&2\nexit 1\n' > bin/npm && chmod +x bin/npm && export PATH="$PWD/bin:$PATH"
npm run check 2>&1 | tail -3

#### 2>&1 | tail hides the exit status without pipefail
sh -c 'echo out; echo err >&2; exit 4' 2>&1 | tail -n 5
echo "exit=$? first=${PIPESTATUS[0]}"

#### 2>&1 | tail with pipefail keeps the status
set -o pipefail
sh -c 'echo out; echo err >&2; exit 4' 2>&1 | tail -n 5
echo "exit=$?"

#### 2>/dev/null hides the error message
cat missing.txt 2>/dev/null
echo "exit=$?"

#### 2>/dev/null with || echo fallback
cat config.local.json 2>/dev/null || echo "no local config"

#### command not found is 127
nosuchcommand-abc --help
echo "exit=$?"

#### command not found with 2>/dev/null
nosuchcommand-abc 2>/dev/null; echo "exit=$?"

#### command not found inside a pipeline
nosuchcommand-abc | head -n 1
echo "pipe=${PIPESTATUS[*]}"

#### command -v to probe for a tool
command -v nosuchcommand-abc >/dev/null 2>&1 || echo "tool not installed"
command -v sh >/dev/null && echo "sh available"

#### non-executable file is 126
printf 'echo hi\n' > run.sh
./run.sh; echo "exit=$?"

#### cat of a missing file
cat does-not-exist.txt; echo "exit=$?"

#### cat of one missing and one present file
printf 'present\n' > a.txt
cat a.txt b.txt; echo "exit=$?"

#### ls of a missing path
ls nothing-here; echo "exit=$?"

#### unmatched glob stays literal in echo
mkdir -p src && touch src/main.rs
echo src/*.nope

#### ls *.nope fails with the literal name
touch README.md
ls *.nope; echo "exit=$?"

#### unmatched glob in a for loop with a -e guard
for f in logs/*.log; do [ -e "$f" ] || { echo "no logs"; continue; }; echo "$f"; done

#### rm -f with an unmatched glob
rm -f build/*.o; echo "exit=$?"

#### cd into a missing dir stops the chain
cd /nonexistent-dir-xyz && sed -n 1,5p README.md
echo "exit=$?"

#### cd into a missing dir with ; runs the rest
printf 'top\n' > here.txt
cd missing-dir; cat here.txt

#### cd failure with 2>/dev/null and fallback
cd packages/web 2>/dev/null || echo "no packages/web"

#### set -e stops a chain
set -e
echo "step 1"
false
echo "step 2"

#### set -e in a subshell, exit status
(set -e; echo before; sh -c "exit 9"; echo after)
echo "exit=$?"

#### set -e ignores failures in && lists
set -e
false && echo "not printed"
echo "still running"

#### set -e and a failing command substitution
set -e
v=$(sh -c 'exit 3')
echo "not reached $v"

#### set -u unbound variable
set -u
echo "value: $UNSET_THING"
echo "not reached"

#### set -u with a default value
set -u
echo "mode=${BUILD_MODE:-debug}"

#### set -u in a subshell, status
(set -u; echo "$NOPE_VAR") 2>/dev/null
echo "exit=$?"

#### set -euo pipefail stops at a failing pipeline
set -euo pipefail
printf 'a\n' | grep -q zzz | cat
echo "not reached"

#### pipefail picks the last failing status
set -o pipefail
sh -c 'exit 2' | sh -c 'exit 3' | true
echo "exit=$?"

#### exit code from a subshell
(echo "in subshell"; exit 42)
echo "exit=$?"

#### exit in a subshell does not end the shell
(exit 1) || echo "subshell failed"
echo "still here"

#### exit status of a sh -c child
sh -c 'exit 300'; echo "exit=$?"

#### python3 heredoc script failing
mkdir -p bin && printf '#!/bin/sh\ncat >/dev/null\necho "Traceback: KeyError" >&2\nexit 1\n' > bin/python3 && chmod +x bin/python3 && export PATH="$PWD/bin:$PATH"
python3 - <<'EOF' 2>&1 | tail -1
import json
print(json.load(open("missing.json"))["k"])
EOF
echo "exit=${PIPESTATUS[0]}"
