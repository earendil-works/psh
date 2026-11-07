# Agent shape: toolchain commands, run against stubs because the recording container has
# no node, npm, python, make, cargo, go, uv or gh: `npm run check`, `npm run X && npm run
# Y`, `npm test -- args`, `npm run -w @scope/pkg X`, `cd ../pkg && npm run X`, `make
# check`, `npx tsc --noEmit`, `node "$(git rev-parse --show-toplevel)/scripts/x.mjs"
# --run y`, `cd pkg && node ../../tools/x.mjs --run y`, `node --test file`, `cargo test
# -q 2>&1 | tail -n N`, `go test ./...`, `uv run pytest`, `gh issue view N --repo o/r
# --json a,b`, `gh run view N --log-failed`, `gh api X --jq ...`; and here-docs into
# interpreters: `python3 - <<'PY'`, `cd dir && python3 - <<'EOF' ... EOF` followed by
# `sed -n`/`grep -n`/`echo`/`git diff --stat`, `node - <<'NODE'`, `node
# --input-type=module <<'EOF'`, `uv run python - <<'PY'`, `cat > x.mjs <<'EOF'` then
# `node x.mjs`. The stub interpreters print the body's print()/console.log() lines,
# apply its open(...).write()/fs.appendFileSync() lines and honor sys.exit/process.exit,
# or simply cat the body to show what the here-doc delivered.

#### npm run check passes
mkdir -p bin && printf '#!/bin/sh\n' > bin/npm
cat >> bin/npm <<'STUB'
cmd=$1; shift
if [ "$1" = -w ]; then
  for p in packages/*/package.json; do [ "$(jq -r .name "$p")" = "$2" ] && cd "$(dirname "$p")"; done
  shift 2
fi
case $cmd in test) s=test ;; run) s=$1; shift ;; *) echo "npm stub: $cmd" >&2; exit 1 ;; esac
[ "$1" = -- ] && shift
body=$(jq -r --arg s "$s" '.scripts[$s] // empty' package.json)
if [ -z "$body" ]; then echo "npm error Missing script: \"$s\"" >&2; exit 1; fi
printf '\n> %s %s\n> %s\n\n' "$(jq -r '.name + "@" + .version' package.json)" "$s" "$body${*:+ $*}"
sh -c "$body${*:+ $*}"
STUB
chmod +x bin/npm && export PATH="$PWD/bin:$PATH"
cat > package.json <<'EOF'
{
  "name": "demo-app",
  "version": "1.2.0",
  "scripts": {
    "lint": "echo 'eslint: 0 problems'",
    "typecheck": "echo 'tsc: no errors'",
    "check": "npm run lint && npm run typecheck"
  }
}
EOF
npm run check
echo "status=$?"

#### npm run check fails on a lint error
mkdir -p bin && printf '#!/bin/sh\n' > bin/npm
cat >> bin/npm <<'STUB'
cmd=$1; shift
if [ "$1" = -w ]; then
  for p in packages/*/package.json; do [ "$(jq -r .name "$p")" = "$2" ] && cd "$(dirname "$p")"; done
  shift 2
fi
case $cmd in test) s=test ;; run) s=$1; shift ;; *) echo "npm stub: $cmd" >&2; exit 1 ;; esac
[ "$1" = -- ] && shift
body=$(jq -r --arg s "$s" '.scripts[$s] // empty' package.json)
if [ -z "$body" ]; then echo "npm error Missing script: \"$s\"" >&2; exit 1; fi
printf '\n> %s %s\n> %s\n\n' "$(jq -r '.name + "@" + .version' package.json)" "$s" "$body${*:+ $*}"
sh -c "$body${*:+ $*}"
STUB
chmod +x bin/npm && export PATH="$PWD/bin:$PATH"
mkdir -p src && printf 'export const a = 1;\nconsole.debug("left over");\n' > src/index.ts
cat > package.json <<'EOF'
{
  "name": "demo-app",
  "version": "1.2.0",
  "scripts": {
    "check": "grep -n 'console.debug' src/*.ts && exit 1 || echo clean"
  }
}
EOF
npm run check
echo "status=$?"

#### npm run check piped to tail
mkdir -p bin && printf '#!/bin/sh\n' > bin/npm
cat >> bin/npm <<'STUB'
cmd=$1; shift
if [ "$1" = -w ]; then
  for p in packages/*/package.json; do [ "$(jq -r .name "$p")" = "$2" ] && cd "$(dirname "$p")"; done
  shift 2
fi
case $cmd in test) s=test ;; run) s=$1; shift ;; *) echo "npm stub: $cmd" >&2; exit 1 ;; esac
[ "$1" = -- ] && shift
body=$(jq -r --arg s "$s" '.scripts[$s] // empty' package.json)
if [ -z "$body" ]; then echo "npm error Missing script: \"$s\"" >&2; exit 1; fi
printf '\n> %s %s\n> %s\n\n' "$(jq -r '.name + "@" + .version' package.json)" "$s" "$body${*:+ $*}"
sh -c "$body${*:+ $*}"
STUB
chmod +x bin/npm && export PATH="$PWD/bin:$PATH"
mkdir -p src && printf 'let x: any = 1;\nlet y: any = 2;\nlet z = 3;\n' > src/main.ts
cat > package.json <<'EOF'
{
  "name": "web",
  "version": "0.0.1",
  "scripts": {
    "check": "grep -n 'any' src/main.ts | sed 's/^/src\\/main.ts:/; s/$/  error  no-explicit-any/'; echo '2 problems' >&2; exit 1"
  }
}
EOF
npm run check 2>&1 | tail -n 4

#### npm run with a missing script
mkdir -p bin && printf '#!/bin/sh\n' > bin/npm
cat >> bin/npm <<'STUB'
cmd=$1; shift
if [ "$1" = -w ]; then
  for p in packages/*/package.json; do [ "$(jq -r .name "$p")" = "$2" ] && cd "$(dirname "$p")"; done
  shift 2
fi
case $cmd in test) s=test ;; run) s=$1; shift ;; *) echo "npm stub: $cmd" >&2; exit 1 ;; esac
[ "$1" = -- ] && shift
body=$(jq -r --arg s "$s" '.scripts[$s] // empty' package.json)
if [ -z "$body" ]; then echo "npm error Missing script: \"$s\"" >&2; exit 1; fi
printf '\n> %s %s\n> %s\n\n' "$(jq -r '.name + "@" + .version' package.json)" "$s" "$body${*:+ $*}"
sh -c "$body${*:+ $*}"
STUB
chmod +x bin/npm && export PATH="$PWD/bin:$PATH"
printf '{"name": "demo", "version": "1.0.0", "scripts": {"build": "echo build"}}\n' > package.json
npm run lint
echo "status=$?"

#### npm run lint and npm run build chained
mkdir -p bin && printf '#!/bin/sh\n' > bin/npm
cat >> bin/npm <<'STUB'
cmd=$1; shift
if [ "$1" = -w ]; then
  for p in packages/*/package.json; do [ "$(jq -r .name "$p")" = "$2" ] && cd "$(dirname "$p")"; done
  shift 2
fi
case $cmd in test) s=test ;; run) s=$1; shift ;; *) echo "npm stub: $cmd" >&2; exit 1 ;; esac
[ "$1" = -- ] && shift
body=$(jq -r --arg s "$s" '.scripts[$s] // empty' package.json)
if [ -z "$body" ]; then echo "npm error Missing script: \"$s\"" >&2; exit 1; fi
printf '\n> %s %s\n> %s\n\n' "$(jq -r '.name + "@" + .version' package.json)" "$s" "$body${*:+ $*}"
sh -c "$body${*:+ $*}"
STUB
chmod +x bin/npm && export PATH="$PWD/bin:$PATH"
printf '{"name": "app", "version": "0.3.0", "scripts": {"lint": "echo lint ok", "build": "echo build ok"}}\n' > package.json
npm run lint && npm run build
echo "status=$?"

#### failing npm run stops the chain
mkdir -p bin && printf '#!/bin/sh\n' > bin/npm
cat >> bin/npm <<'STUB'
cmd=$1; shift
if [ "$1" = -w ]; then
  for p in packages/*/package.json; do [ "$(jq -r .name "$p")" = "$2" ] && cd "$(dirname "$p")"; done
  shift 2
fi
case $cmd in test) s=test ;; run) s=$1; shift ;; *) echo "npm stub: $cmd" >&2; exit 1 ;; esac
[ "$1" = -- ] && shift
body=$(jq -r --arg s "$s" '.scripts[$s] // empty' package.json)
if [ -z "$body" ]; then echo "npm error Missing script: \"$s\"" >&2; exit 1; fi
printf '\n> %s %s\n> %s\n\n' "$(jq -r '.name + "@" + .version' package.json)" "$s" "$body${*:+ $*}"
sh -c "$body${*:+ $*}"
STUB
chmod +x bin/npm && export PATH="$PWD/bin:$PATH"
printf '{"name": "app", "version": "0.3.0", "scripts": {"lint": "echo lint failed >&2; exit 2", "build": "echo build ok"}}\n' > package.json
npm run lint && npm run build
echo "status=$?"

#### npm test
mkdir -p bin && printf '#!/bin/sh\n' > bin/npm
cat >> bin/npm <<'STUB'
cmd=$1; shift
if [ "$1" = -w ]; then
  for p in packages/*/package.json; do [ "$(jq -r .name "$p")" = "$2" ] && cd "$(dirname "$p")"; done
  shift 2
fi
case $cmd in test) s=test ;; run) s=$1; shift ;; *) echo "npm stub: $cmd" >&2; exit 1 ;; esac
[ "$1" = -- ] && shift
body=$(jq -r --arg s "$s" '.scripts[$s] // empty' package.json)
if [ -z "$body" ]; then echo "npm error Missing script: \"$s\"" >&2; exit 1; fi
printf '\n> %s %s\n> %s\n\n' "$(jq -r '.name + "@" + .version' package.json)" "$s" "$body${*:+ $*}"
sh -c "$body${*:+ $*}"
STUB
chmod +x bin/npm && export PATH="$PWD/bin:$PATH"
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
[ "$1" = --test ] && shift
awk '
function flush() { if (name != "") { n++; if (bad) { fail++; print "not ok " n " - " name } else { pass++; print "ok " n " - " name } } }
/^test\("/ { flush(); name = $0; sub(/^test\("/, "", name); sub(/".*$/, "", name); bad = 0; next }
/assert\.ok\(false/ { bad = 1 }
END { flush(); print "# tests " n; print "# pass " pass + 0; print "# fail " fail + 0; exit (fail > 0) }
' "$@"
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
mkdir -p tests
cat > tests/math.test.js <<'EOF'
const test = require("node:test");
const assert = require("node:assert");
test("adds", () => {
  assert.equal(1 + 1, 2);
});
test("subtracts", () => {
  assert.equal(3 - 1, 2);
});
EOF
printf '{"name": "calc", "version": "1.0.0", "scripts": {"test": "node --test tests/math.test.js"}}\n' > package.json
npm test
echo "status=$?"

#### npm test with extra arguments after --
mkdir -p bin && printf '#!/bin/sh\n' > bin/npm
cat >> bin/npm <<'STUB'
cmd=$1; shift
if [ "$1" = -w ]; then
  for p in packages/*/package.json; do [ "$(jq -r .name "$p")" = "$2" ] && cd "$(dirname "$p")"; done
  shift 2
fi
case $cmd in test) s=test ;; run) s=$1; shift ;; *) echo "npm stub: $cmd" >&2; exit 1 ;; esac
[ "$1" = -- ] && shift
body=$(jq -r --arg s "$s" '.scripts[$s] // empty' package.json)
if [ -z "$body" ]; then echo "npm error Missing script: \"$s\"" >&2; exit 1; fi
printf '\n> %s %s\n> %s\n\n' "$(jq -r '.name + "@" + .version' package.json)" "$s" "$body${*:+ $*}"
sh -c "$body${*:+ $*}"
STUB
chmod +x bin/npm && export PATH="$PWD/bin:$PATH"
printf '{"name": "calc", "version": "1.0.0", "scripts": {"test": "echo running tests with"}}\n' > package.json
npm test -- --grep parser --reporter dot

#### npm test failing
mkdir -p bin && printf '#!/bin/sh\n' > bin/npm
cat >> bin/npm <<'STUB'
cmd=$1; shift
if [ "$1" = -w ]; then
  for p in packages/*/package.json; do [ "$(jq -r .name "$p")" = "$2" ] && cd "$(dirname "$p")"; done
  shift 2
fi
case $cmd in test) s=test ;; run) s=$1; shift ;; *) echo "npm stub: $cmd" >&2; exit 1 ;; esac
[ "$1" = -- ] && shift
body=$(jq -r --arg s "$s" '.scripts[$s] // empty' package.json)
if [ -z "$body" ]; then echo "npm error Missing script: \"$s\"" >&2; exit 1; fi
printf '\n> %s %s\n> %s\n\n' "$(jq -r '.name + "@" + .version' package.json)" "$s" "$body${*:+ $*}"
sh -c "$body${*:+ $*}"
STUB
chmod +x bin/npm && export PATH="$PWD/bin:$PATH"
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
[ "$1" = --test ] && shift
awk '
function flush() { if (name != "") { n++; if (bad) { fail++; print "not ok " n " - " name } else { pass++; print "ok " n " - " name } } }
/^test\("/ { flush(); name = $0; sub(/^test\("/, "", name); sub(/".*$/, "", name); bad = 0; next }
/assert\.ok\(false/ { bad = 1 }
END { flush(); print "# tests " n; print "# pass " pass + 0; print "# fail " fail + 0; exit (fail > 0) }
' "$@"
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
mkdir -p tests
cat > tests/parse.test.js <<'EOF'
const test = require("node:test");
const assert = require("node:assert");
test("parses numbers", () => {
  assert.ok(true);
});
test("parses strings", () => {
  assert.ok(false, "not implemented");
});
EOF
printf '{"name": "parse", "version": "1.0.0", "scripts": {"test": "node --test tests/parse.test.js"}}\n' > package.json
npm test 2>&1 | grep -E '^(not ok|# fail)'
npm test >/dev/null 2>&1
echo "status=$?"

#### cd into a package and npm run build
mkdir -p bin && printf '#!/bin/sh\n' > bin/npm
cat >> bin/npm <<'STUB'
cmd=$1; shift
if [ "$1" = -w ]; then
  for p in packages/*/package.json; do [ "$(jq -r .name "$p")" = "$2" ] && cd "$(dirname "$p")"; done
  shift 2
fi
case $cmd in test) s=test ;; run) s=$1; shift ;; *) echo "npm stub: $cmd" >&2; exit 1 ;; esac
[ "$1" = -- ] && shift
body=$(jq -r --arg s "$s" '.scripts[$s] // empty' package.json)
if [ -z "$body" ]; then echo "npm error Missing script: \"$s\"" >&2; exit 1; fi
printf '\n> %s %s\n> %s\n\n' "$(jq -r '.name + "@" + .version' package.json)" "$s" "$body${*:+ $*}"
sh -c "$body${*:+ $*}"
STUB
chmod +x bin/npm && export PATH="$PWD/bin:$PATH"
mkdir -p packages/web
printf '{"name": "@demo/web", "version": "0.1.0", "scripts": {"build": "echo bundling $(basename $(pwd))"}}\n' > packages/web/package.json
cd packages/web && npm run build

#### cd to a sibling and npm run
mkdir -p bin && printf '#!/bin/sh\n' > bin/npm
cat >> bin/npm <<'STUB'
cmd=$1; shift
if [ "$1" = -w ]; then
  for p in packages/*/package.json; do [ "$(jq -r .name "$p")" = "$2" ] && cd "$(dirname "$p")"; done
  shift 2
fi
case $cmd in test) s=test ;; run) s=$1; shift ;; *) echo "npm stub: $cmd" >&2; exit 1 ;; esac
[ "$1" = -- ] && shift
body=$(jq -r --arg s "$s" '.scripts[$s] // empty' package.json)
if [ -z "$body" ]; then echo "npm error Missing script: \"$s\"" >&2; exit 1; fi
printf '\n> %s %s\n> %s\n\n' "$(jq -r '.name + "@" + .version' package.json)" "$s" "$body${*:+ $*}"
sh -c "$body${*:+ $*}"
STUB
chmod +x bin/npm && export PATH="$PWD/bin:$PATH"
mkdir -p repo/api repo/web
printf '{"name": "web", "version": "0.1.0", "scripts": {"dev:check": "ls"}}\n' > repo/web/package.json
touch repo/web/vite.config.ts
cd repo/api
cd ../web && npm run dev:check

#### npm run in a workspace
mkdir -p bin && printf '#!/bin/sh\n' > bin/npm
cat >> bin/npm <<'STUB'
cmd=$1; shift
if [ "$1" = -w ]; then
  for p in packages/*/package.json; do [ "$(jq -r .name "$p")" = "$2" ] && cd "$(dirname "$p")"; done
  shift 2
fi
case $cmd in test) s=test ;; run) s=$1; shift ;; *) echo "npm stub: $cmd" >&2; exit 1 ;; esac
[ "$1" = -- ] && shift
body=$(jq -r --arg s "$s" '.scripts[$s] // empty' package.json)
if [ -z "$body" ]; then echo "npm error Missing script: \"$s\"" >&2; exit 1; fi
printf '\n> %s %s\n> %s\n\n' "$(jq -r '.name + "@" + .version' package.json)" "$s" "$body${*:+ $*}"
sh -c "$body${*:+ $*}"
STUB
chmod +x bin/npm && export PATH="$PWD/bin:$PATH"
mkdir -p packages/core packages/cli
printf '{"name": "monorepo", "version": "0.0.0", "private": true}\n' > package.json
printf '{"name": "@demo/core", "version": "1.0.0", "scripts": {"test": "echo core tests in $(basename $PWD)"}}\n' > packages/core/package.json
printf '{"name": "@demo/cli", "version": "1.0.0", "scripts": {"test": "echo cli tests"}}\n' > packages/cli/package.json
npm run -w @demo/core test

#### npm run check with output silenced
mkdir -p bin && printf '#!/bin/sh\n' > bin/npm
cat >> bin/npm <<'STUB'
cmd=$1; shift
if [ "$1" = -w ]; then
  for p in packages/*/package.json; do [ "$(jq -r .name "$p")" = "$2" ] && cd "$(dirname "$p")"; done
  shift 2
fi
case $cmd in test) s=test ;; run) s=$1; shift ;; *) echo "npm stub: $cmd" >&2; exit 1 ;; esac
[ "$1" = -- ] && shift
body=$(jq -r --arg s "$s" '.scripts[$s] // empty' package.json)
if [ -z "$body" ]; then echo "npm error Missing script: \"$s\"" >&2; exit 1; fi
printf '\n> %s %s\n> %s\n\n' "$(jq -r '.name + "@" + .version' package.json)" "$s" "$body${*:+ $*}"
sh -c "$body${*:+ $*}"
STUB
chmod +x bin/npm && export PATH="$PWD/bin:$PATH"
printf '{"name": "app", "version": "1.0.0", "scripts": {"check": "echo checking; exit 1"}}\n' > package.json
npm run check >/dev/null 2>&1 && echo "check ok" || echo "check failed"

#### npm run typecheck grep for errors
mkdir -p bin && printf '#!/bin/sh\n' > bin/npm
cat >> bin/npm <<'STUB'
cmd=$1; shift
if [ "$1" = -w ]; then
  for p in packages/*/package.json; do [ "$(jq -r .name "$p")" = "$2" ] && cd "$(dirname "$p")"; done
  shift 2
fi
case $cmd in test) s=test ;; run) s=$1; shift ;; *) echo "npm stub: $cmd" >&2; exit 1 ;; esac
[ "$1" = -- ] && shift
body=$(jq -r --arg s "$s" '.scripts[$s] // empty' package.json)
if [ -z "$body" ]; then echo "npm error Missing script: \"$s\"" >&2; exit 1; fi
printf '\n> %s %s\n> %s\n\n' "$(jq -r '.name + "@" + .version' package.json)" "$s" "$body${*:+ $*}"
sh -c "$body${*:+ $*}"
STUB
chmod +x bin/npm && export PATH="$PWD/bin:$PATH"
cat > package.json <<'EOF'
{
  "name": "app",
  "version": "1.0.0",
  "scripts": {
    "typecheck": "printf 'src/a.ts(3,1): error TS2304: Cannot find name foo.\\nsrc/b.ts(9,4): error TS2322: Type string is not assignable.\\nFound 2 errors.\\n'; exit 2"
  }
}
EOF
npm run typecheck 2>&1 | grep -n '^src/' | head -n 3

#### make check runs prerequisites
mkdir -p bin && printf '#!/bin/sh\n' > bin/make
cat >> bin/make <<'STUB'
t=${1:-$(sed -n 's/^\([a-z][a-z-]*\):.*/\1/p' Makefile | head -n 1)}
if ! grep -q "^$t:" Makefile; then echo "make: *** No rule to make target '$t'.  Stop." >&2; exit 2; fi
for d in $(sed -n "s/^$t:\(.*\)/\1/p" Makefile); do "$0" "$d" || exit 2; done
awk -v t="$t" '$0 ~ "^" t ":" { f = 1; next } f && /^\t/ { sub(/^\t/, ""); print; next } { f = 0 }' Makefile |
while IFS= read -r line; do
  echo "$line"
  sh -c "$line" </dev/null || { echo "make: *** [Makefile: $t] Error $?" >&2; exit 2; }
done
STUB
chmod +x bin/make && export PATH="$PWD/bin:$PATH"
printf '.PHONY: check lint test\ncheck: lint test\n\techo all checks passed\nlint:\n\techo lint ok\ntest:\n\techo test ok\n' > Makefile
make check
echo "status=$?"

#### make check fails in a prerequisite
mkdir -p bin && printf '#!/bin/sh\n' > bin/make
cat >> bin/make <<'STUB'
t=${1:-$(sed -n 's/^\([a-z][a-z-]*\):.*/\1/p' Makefile | head -n 1)}
if ! grep -q "^$t:" Makefile; then echo "make: *** No rule to make target '$t'.  Stop." >&2; exit 2; fi
for d in $(sed -n "s/^$t:\(.*\)/\1/p" Makefile); do "$0" "$d" || exit 2; done
awk -v t="$t" '$0 ~ "^" t ":" { f = 1; next } f && /^\t/ { sub(/^\t/, ""); print; next } { f = 0 }' Makefile |
while IFS= read -r line; do
  echo "$line"
  sh -c "$line" </dev/null || { echo "make: *** [Makefile: $t] Error $?" >&2; exit 2; }
done
STUB
chmod +x bin/make && export PATH="$PWD/bin:$PATH"
printf 'check: lint test\nlint:\n\techo lint ok\ntest:\n\techo running tests\n\texit 1\n' > Makefile
make check
echo "status=$?"

#### make test piped to tail
mkdir -p bin && printf '#!/bin/sh\n' > bin/make
cat >> bin/make <<'STUB'
t=${1:-$(sed -n 's/^\([a-z][a-z-]*\):.*/\1/p' Makefile | head -n 1)}
if ! grep -q "^$t:" Makefile; then echo "make: *** No rule to make target '$t'.  Stop." >&2; exit 2; fi
for d in $(sed -n "s/^$t:\(.*\)/\1/p" Makefile); do "$0" "$d" || exit 2; done
awk -v t="$t" '$0 ~ "^" t ":" { f = 1; next } f && /^\t/ { sub(/^\t/, ""); print; next } { f = 0 }' Makefile |
while IFS= read -r line; do
  echo "$line"
  sh -c "$line" </dev/null || { echo "make: *** [Makefile: $t] Error $?" >&2; exit 2; }
done
STUB
chmod +x bin/make && export PATH="$PWD/bin:$PATH"
mkdir -p tests && printf 'case one\ncase two\ncase three\n' > tests/cases.txt
printf 'test:\n\twc -l < tests/cases.txt\n\tsed -n 2p tests/cases.txt\n\techo done\n' > Makefile
make test 2>&1 | tail -n 2

#### make with an unknown target
mkdir -p bin && printf '#!/bin/sh\n' > bin/make
cat >> bin/make <<'STUB'
t=${1:-$(sed -n 's/^\([a-z][a-z-]*\):.*/\1/p' Makefile | head -n 1)}
if ! grep -q "^$t:" Makefile; then echo "make: *** No rule to make target '$t'.  Stop." >&2; exit 2; fi
for d in $(sed -n "s/^$t:\(.*\)/\1/p" Makefile); do "$0" "$d" || exit 2; done
awk -v t="$t" '$0 ~ "^" t ":" { f = 1; next } f && /^\t/ { sub(/^\t/, ""); print; next } { f = 0 }' Makefile |
while IFS= read -r line; do
  echo "$line"
  sh -c "$line" </dev/null || { echo "make: *** [Makefile: $t] Error $?" >&2; exit 2; }
done
STUB
chmod +x bin/make && export PATH="$PWD/bin:$PATH"
printf 'build:\n\techo build\n' > Makefile
make deploy
echo "status=$?"

#### npx tsc --noEmit passes
mkdir -p bin && printf '#!/bin/sh\n' > bin/npx
cat >> bin/npx <<'STUB'
d=$PWD
while :; do
  if [ -x "$d/node_modules/.bin/$1" ]; then t=$1; shift; exec "$d/node_modules/.bin/$t" "$@"; fi
  [ "$d" = / ] && break
  d=$(dirname "$d")
done
echo "npm error could not determine executable to run" >&2
exit 1
STUB
chmod +x bin/npx && export PATH="$PWD/bin:$PATH"
mkdir -p node_modules/.bin && printf '#!/bin/sh\necho "tsc $*" >&2\n' > node_modules/.bin/tsc && chmod +x node_modules/.bin/tsc
npx tsc --noEmit && echo "typecheck ok"

#### npx tsc --noEmit reports errors
mkdir -p bin && printf '#!/bin/sh\n' > bin/npx
cat >> bin/npx <<'STUB'
d=$PWD
while :; do
  if [ -x "$d/node_modules/.bin/$1" ]; then t=$1; shift; exec "$d/node_modules/.bin/$t" "$@"; fi
  [ "$d" = / ] && break
  d=$(dirname "$d")
done
echo "npm error could not determine executable to run" >&2
exit 1
STUB
chmod +x bin/npx && export PATH="$PWD/bin:$PATH"
mkdir -p node_modules/.bin src
printf 'let n: number = "x";\n' > src/a.ts
printf '#!/bin/sh\ngrep -n "number = \\"" src/*.ts | sed "s/:.*/: error TS2322/"\nexit 2\n' > node_modules/.bin/tsc && chmod +x node_modules/.bin/tsc
npx tsc --noEmit
echo "status=$?"

#### cd into a package and npx a root script
mkdir -p bin && printf '#!/bin/sh\n' > bin/npx
cat >> bin/npx <<'STUB'
d=$PWD
while :; do
  if [ -x "$d/node_modules/.bin/$1" ]; then t=$1; shift; exec "$d/node_modules/.bin/$t" "$@"; fi
  [ "$d" = / ] && break
  d=$(dirname "$d")
done
echo "npm error could not determine executable to run" >&2
exit 1
STUB
chmod +x bin/npx && export PATH="$PWD/bin:$PATH"
mkdir -p node_modules/.bin packages/app scripts
printf '#!/bin/sh\necho "tsx $1"; shift; echo "args: $*"\n' > node_modules/.bin/tsx && chmod +x node_modules/.bin/tsx
printf 'console.log("seed")\n' > scripts/seed.ts
cd packages/app && npx tsx ../../scripts/seed.ts --run users fixtures

#### node script at the git toplevel with --run
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
run() { awk '
/^ *console\.log\(".*"\);?$/ { s = $0; sub(/^ *console\.log\("/, "", s); sub(/"\);?$/, "", s); print s; next }
/^ *fs\.appendFileSync\(".*", ".*\\n"\);?$/ { f = $0; sub(/^ *fs\.appendFileSync\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^[^"]*", "/, "", t); sub(/\\n"\);?$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *process\.exit\([0-9]+\);?$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"; }
case $1 in --input-type=*) shift ;; esac
case $1 in
  -e) printf '%s\n' "$2" | run ;;
  - | "") run ;;
  *) if [ ! -f "$1" ]; then echo "Error: Cannot find module '$(basename "$1")'" >&2; exit 1; fi
     s=$1; shift; echo "[$(basename "$s")]${*:+ $*}"; run "$s" ;;
esac
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
git init -q -b main
mkdir -p scripts && cat > scripts/dev.mjs <<'EOF'
import { argv } from "node:process";
console.log("dev runner");
EOF
git add . && git commit -qm init
node "$(git rev-parse --show-toplevel)/scripts/dev.mjs" --run lint

#### cd into a package and node the toplevel script
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
run() { awk '
/^ *console\.log\(".*"\);?$/ { s = $0; sub(/^ *console\.log\("/, "", s); sub(/"\);?$/, "", s); print s; next }
/^ *fs\.appendFileSync\(".*", ".*\\n"\);?$/ { f = $0; sub(/^ *fs\.appendFileSync\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^[^"]*", "/, "", t); sub(/\\n"\);?$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *process\.exit\([0-9]+\);?$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"; }
case $1 in --input-type=*) shift ;; esac
case $1 in
  -e) printf '%s\n' "$2" | run ;;
  - | "") run ;;
  *) if [ ! -f "$1" ]; then echo "Error: Cannot find module '$(basename "$1")'" >&2; exit 1; fi
     s=$1; shift; echo "[$(basename "$s")]${*:+ $*}"; run "$s" ;;
esac
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
git init -q -b main
mkdir -p scripts packages/core/src && cat > scripts/dev.mjs <<'EOF'
const task = process.argv[3];
console.log("dev runner");
EOF
printf 'export {};\n' > packages/core/src/index.ts
git add . && git commit -qm init
cd packages/core && node "$(git rev-parse --show-toplevel)/scripts/dev.mjs" --run test

#### toplevel script with several arguments
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
run() { awk '
/^ *console\.log\(".*"\);?$/ { s = $0; sub(/^ *console\.log\("/, "", s); sub(/"\);?$/, "", s); print s; next }
/^ *fs\.appendFileSync\(".*", ".*\\n"\);?$/ { f = $0; sub(/^ *fs\.appendFileSync\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^[^"]*", "/, "", t); sub(/\\n"\);?$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *process\.exit\([0-9]+\);?$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"; }
case $1 in --input-type=*) shift ;; esac
case $1 in
  -e) printf '%s\n' "$2" | run ;;
  - | "") run ;;
  *) if [ ! -f "$1" ]; then echo "Error: Cannot find module '$(basename "$1")'" >&2; exit 1; fi
     s=$1; shift; echo "[$(basename "$s")]${*:+ $*}"; run "$s" ;;
esac
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
git init -q -b main
mkdir -p scripts packages/core && printf 'console.log("ok");\n' > scripts/dev.mjs
git add . && git commit -qm init
cd packages/core && node "$(git rev-parse --show-toplevel)/scripts/dev.mjs" --run test unit --reporter=spec --bail

#### toplevel script with --run and nothing else
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
run() { awk '
/^ *console\.log\(".*"\);?$/ { s = $0; sub(/^ *console\.log\("/, "", s); sub(/"\);?$/, "", s); print s; next }
/^ *fs\.appendFileSync\(".*", ".*\\n"\);?$/ { f = $0; sub(/^ *fs\.appendFileSync\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^[^"]*", "/, "", t); sub(/\\n"\);?$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *process\.exit\([0-9]+\);?$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"; }
case $1 in --input-type=*) shift ;; esac
case $1 in
  -e) printf '%s\n' "$2" | run ;;
  - | "") run ;;
  *) if [ ! -f "$1" ]; then echo "Error: Cannot find module '$(basename "$1")'" >&2; exit 1; fi
     s=$1; shift; echo "[$(basename "$s")]${*:+ $*}"; run "$s" ;;
esac
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
git init -q -b main
mkdir -p scripts packages/ui && printf 'console.log("usage: dev.mjs --run <task>");\nprocess.exit(1);\n' > scripts/dev.mjs
git add . && git commit -qm init
cd packages/ui && node "$(git rev-parse --show-toplevel)/scripts/dev.mjs" --run
echo "status=$?"

#### cd into a package and node a relative script
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
run() { awk '
/^ *console\.log\(".*"\);?$/ { s = $0; sub(/^ *console\.log\("/, "", s); sub(/"\);?$/, "", s); print s; next }
/^ *fs\.appendFileSync\(".*", ".*\\n"\);?$/ { f = $0; sub(/^ *fs\.appendFileSync\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^[^"]*", "/, "", t); sub(/\\n"\);?$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *process\.exit\([0-9]+\);?$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"; }
case $1 in --input-type=*) shift ;; esac
case $1 in
  -e) printf '%s\n' "$2" | run ;;
  - | "") run ;;
  *) if [ ! -f "$1" ]; then echo "Error: Cannot find module '$(basename "$1")'" >&2; exit 1; fi
     s=$1; shift; echo "[$(basename "$s")]${*:+ $*}"; run "$s" ;;
esac
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
mkdir -p tools packages/core
cat > tools/run.mjs <<'EOF'
import { spawnSync } from "node:child_process";
console.log("building core");
EOF
cd packages/core && node ../../tools/run.mjs --run build

#### relative script from the wrong directory
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
run() { awk '
/^ *console\.log\(".*"\);?$/ { s = $0; sub(/^ *console\.log\("/, "", s); sub(/"\);?$/, "", s); print s; next }
/^ *fs\.appendFileSync\(".*", ".*\\n"\);?$/ { f = $0; sub(/^ *fs\.appendFileSync\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^[^"]*", "/, "", t); sub(/\\n"\);?$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *process\.exit\([0-9]+\);?$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"; }
case $1 in --input-type=*) shift ;; esac
case $1 in
  -e) printf '%s\n' "$2" | run ;;
  - | "") run ;;
  *) if [ ! -f "$1" ]; then echo "Error: Cannot find module '$(basename "$1")'" >&2; exit 1; fi
     s=$1; shift; echo "[$(basename "$s")]${*:+ $*}"; run "$s" ;;
esac
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
mkdir -p tools packages/core && printf 'console.log("never");\n' > tools/run.mjs
node ../../tools/run.mjs --run build
echo "status=$?"

#### cd into a package and node a local script
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
run() { awk '
/^ *console\.log\(".*"\);?$/ { s = $0; sub(/^ *console\.log\("/, "", s); sub(/"\);?$/, "", s); print s; next }
/^ *fs\.appendFileSync\(".*", ".*\\n"\);?$/ { f = $0; sub(/^ *fs\.appendFileSync\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^[^"]*", "/, "", t); sub(/\\n"\);?$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *process\.exit\([0-9]+\);?$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"; }
case $1 in --input-type=*) shift ;; esac
case $1 in
  -e) printf '%s\n' "$2" | run ;;
  - | "") run ;;
  *) if [ ! -f "$1" ]; then echo "Error: Cannot find module '$(basename "$1")'" >&2; exit 1; fi
     s=$1; shift; echo "[$(basename "$s")]${*:+ $*}"; run "$s" ;;
esac
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
mkdir -p pkg/scripts && printf 'console.log("generated 3 files");\n' > pkg/scripts/gen.mjs
cd pkg && node scripts/gen.mjs --run all

#### node script that exits with an error
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
run() { awk '
/^ *console\.log\(".*"\);?$/ { s = $0; sub(/^ *console\.log\("/, "", s); sub(/"\);?$/, "", s); print s; next }
/^ *fs\.appendFileSync\(".*", ".*\\n"\);?$/ { f = $0; sub(/^ *fs\.appendFileSync\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^[^"]*", "/, "", t); sub(/\\n"\);?$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *process\.exit\([0-9]+\);?$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"; }
case $1 in --input-type=*) shift ;; esac
case $1 in
  -e) printf '%s\n' "$2" | run ;;
  - | "") run ;;
  *) if [ ! -f "$1" ]; then echo "Error: Cannot find module '$(basename "$1")'" >&2; exit 1; fi
     s=$1; shift; echo "[$(basename "$s")]${*:+ $*}"; run "$s" ;;
esac
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
mkdir -p scripts && cat > scripts/verify.mjs <<'EOF'
console.log("verifying fixtures");
console.log("fixture users.json: missing field id");
process.exit(3);
EOF
node scripts/verify.mjs --run fixtures
echo "status=$?"

#### node an absolute script then remove it
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
run() { awk '
/^ *console\.log\(".*"\);?$/ { s = $0; sub(/^ *console\.log\("/, "", s); sub(/"\);?$/, "", s); print s; next }
/^ *fs\.appendFileSync\(".*", ".*\\n"\);?$/ { f = $0; sub(/^ *fs\.appendFileSync\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^[^"]*", "/, "", t); sub(/\\n"\);?$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *process\.exit\([0-9]+\);?$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"; }
case $1 in --input-type=*) shift ;; esac
case $1 in
  -e) printf '%s\n' "$2" | run ;;
  - | "") run ;;
  *) if [ ! -f "$1" ]; then echo "Error: Cannot find module '$(basename "$1")'" >&2; exit 1; fi
     s=$1; shift; echo "[$(basename "$s")]${*:+ $*}"; run "$s" ;;
esac
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
cat > probe.mjs <<'EOF'
console.log("probe ok");
EOF
node "$PWD/probe.mjs"; rm "$PWD/probe.mjs"
ls

#### node a tool with quoted options
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
run() { awk '
/^ *console\.log\(".*"\);?$/ { s = $0; sub(/^ *console\.log\("/, "", s); sub(/"\);?$/, "", s); print s; next }
/^ *fs\.appendFileSync\(".*", ".*\\n"\);?$/ { f = $0; sub(/^ *fs\.appendFileSync\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^[^"]*", "/, "", t); sub(/\\n"\);?$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *process\.exit\([0-9]+\);?$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"; }
case $1 in --input-type=*) shift ;; esac
case $1 in
  -e) printf '%s\n' "$2" | run ;;
  - | "") run ;;
  *) if [ ! -f "$1" ]; then echo "Error: Cannot find module '$(basename "$1")'" >&2; exit 1; fi
     s=$1; shift; echo "[$(basename "$s")]${*:+ $*}"; run "$s" ;;
esac
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
mkdir -p tools && printf 'console.log("asking reviewer");\n' > tools/ask.mjs
node "$PWD/tools/ask.mjs" "Is the parser change safe?" --purpose "code review" --timeout 120

#### node a tool that runs a shell command
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
run() { awk '
/^ *console\.log\(".*"\);?$/ { s = $0; sub(/^ *console\.log\("/, "", s); sub(/"\);?$/, "", s); print s; next }
/^ *fs\.appendFileSync\(".*", ".*\\n"\);?$/ { f = $0; sub(/^ *fs\.appendFileSync\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^[^"]*", "/, "", t); sub(/\\n"\);?$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *process\.exit\([0-9]+\);?$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"; }
case $1 in --input-type=*) shift ;; esac
case $1 in
  -e) printf '%s\n' "$2" | run ;;
  - | "") run ;;
  *) if [ ! -f "$1" ]; then echo "Error: Cannot find module '$(basename "$1")'" >&2; exit 1; fi
     s=$1; shift; echo "[$(basename "$s")]${*:+ $*}"; run "$s" ;;
esac
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
mkdir -p tools && printf 'console.log("sandbox ready");\n' > tools/agent.mjs
node tools/agent.mjs exec --sandbox read-only --cwd . -- /bin/sh -lc 'ls src | head -n 3'

#### node --test a single file
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
[ "$1" = --test ] && shift
awk '
function flush() { if (name != "") { n++; if (bad) { fail++; print "not ok " n " - " name } else { pass++; print "ok " n " - " name } } }
/^test\("/ { flush(); name = $0; sub(/^test\("/, "", name); sub(/".*$/, "", name); bad = 0; next }
/assert\.ok\(false/ { bad = 1 }
END { flush(); print "# tests " n; print "# pass " pass + 0; print "# fail " fail + 0; exit (fail > 0) }
' "$@"
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
mkdir -p tests && cat > tests/parser.test.js <<'EOF'
import test from "node:test";
import assert from "node:assert/strict";
test("parses empty input", () => {
  assert.deepEqual(parse(""), []);
});
test("parses a word", () => {
  assert.deepEqual(parse("a"), ["a"]);
});
EOF
node --test tests/parser.test.js

#### cd into a package and node --test two files
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
[ "$1" = --test ] && shift
awk '
function flush() { if (name != "") { n++; if (bad) { fail++; print "not ok " n " - " name } else { pass++; print "ok " n " - " name } } }
/^test\("/ { flush(); name = $0; sub(/^test\("/, "", name); sub(/".*$/, "", name); bad = 0; next }
/assert\.ok\(false/ { bad = 1 }
END { flush(); print "# tests " n; print "# pass " pass + 0; print "# fail " fail + 0; exit (fail > 0) }
' "$@"
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
mkdir -p pkg/tests
printf 'test("a works", () => {\n  assert.ok(true);\n});\n' > pkg/tests/a.test.js
printf 'test("b works", () => {\n  assert.ok(true);\n});\ntest("b fails", () => {\n  assert.ok(false);\n});\n' > pkg/tests/b.test.js
cd pkg && node --test tests/a.test.js tests/b.test.js
echo "status=$?"

#### node -e with a one-liner
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
run() { awk '
/^ *console\.log\(".*"\);?$/ { s = $0; sub(/^ *console\.log\("/, "", s); sub(/"\);?$/, "", s); print s; next }
/^ *fs\.appendFileSync\(".*", ".*\\n"\);?$/ { f = $0; sub(/^ *fs\.appendFileSync\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^[^"]*", "/, "", t); sub(/\\n"\);?$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *process\.exit\([0-9]+\);?$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"; }
case $1 in --input-type=*) shift ;; esac
case $1 in
  -e) printf '%s\n' "$2" | run ;;
  - | "") run ;;
  *) if [ ! -f "$1" ]; then echo "Error: Cannot find module '$(basename "$1")'" >&2; exit 1; fi
     s=$1; shift; echo "[$(basename "$s")]${*:+ $*}"; run "$s" ;;
esac
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
node -e 'console.log("hello from node")'

#### node - with a NODE here-doc
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
run() { awk '
/^ *console\.log\(".*"\);?$/ { s = $0; sub(/^ *console\.log\("/, "", s); sub(/"\);?$/, "", s); print s; next }
/^ *fs\.appendFileSync\(".*", ".*\\n"\);?$/ { f = $0; sub(/^ *fs\.appendFileSync\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^[^"]*", "/, "", t); sub(/\\n"\);?$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *process\.exit\([0-9]+\);?$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"; }
case $1 in --input-type=*) shift ;; esac
case $1 in
  -e) printf '%s\n' "$2" | run ;;
  - | "") run ;;
  *) if [ ! -f "$1" ]; then echo "Error: Cannot find module '$(basename "$1")'" >&2; exit 1; fi
     s=$1; shift; echo "[$(basename "$s")]${*:+ $*}"; run "$s" ;;
esac
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
node - <<'NODE'
const fs = require("node:fs");
const pkg = { name: "demo" };
console.log("package scanned");
console.log("2 dependencies");
NODE

#### node --input-type=module with an EOF here-doc
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
run() { awk '
/^ *console\.log\(".*"\);?$/ { s = $0; sub(/^ *console\.log\("/, "", s); sub(/"\);?$/, "", s); print s; next }
/^ *fs\.appendFileSync\(".*", ".*\\n"\);?$/ { f = $0; sub(/^ *fs\.appendFileSync\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^[^"]*", "/, "", t); sub(/\\n"\);?$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *process\.exit\([0-9]+\);?$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"; }
case $1 in --input-type=*) shift ;; esac
case $1 in
  -e) printf '%s\n' "$2" | run ;;
  - | "") run ;;
  *) if [ ! -f "$1" ]; then echo "Error: Cannot find module '$(basename "$1")'" >&2; exit 1; fi
     s=$1; shift; echo "[$(basename "$s")]${*:+ $*}"; run "$s" ;;
esac
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
node --input-type=module <<'EOF'
import { readFileSync } from "node:fs";
const lines = ["a", "b"];
console.log("module mode");
EOF
echo "status=$?"

#### node --input-type=module - with a NODE here-doc
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
run() { awk '
/^ *console\.log\(".*"\);?$/ { s = $0; sub(/^ *console\.log\("/, "", s); sub(/"\);?$/, "", s); print s; next }
/^ *fs\.appendFileSync\(".*", ".*\\n"\);?$/ { f = $0; sub(/^ *fs\.appendFileSync\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^[^"]*", "/, "", t); sub(/\\n"\);?$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *process\.exit\([0-9]+\);?$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"; }
case $1 in --input-type=*) shift ;; esac
case $1 in
  -e) printf '%s\n' "$2" | run ;;
  - | "") run ;;
  *) if [ ! -f "$1" ]; then echo "Error: Cannot find module '$(basename "$1")'" >&2; exit 1; fi
     s=$1; shift; echo "[$(basename "$s")]${*:+ $*}"; run "$s" ;;
esac
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
node --input-type=module - <<'NODE'
import path from "node:path";
console.log("joined");
NODE

#### node --input-type=commonjs with a JS here-doc
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
run() { awk '
/^ *console\.log\(".*"\);?$/ { s = $0; sub(/^ *console\.log\("/, "", s); sub(/"\);?$/, "", s); print s; next }
/^ *fs\.appendFileSync\(".*", ".*\\n"\);?$/ { f = $0; sub(/^ *fs\.appendFileSync\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^[^"]*", "/, "", t); sub(/\\n"\);?$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *process\.exit\([0-9]+\);?$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"; }
case $1 in --input-type=*) shift ;; esac
case $1 in
  -e) printf '%s\n' "$2" | run ;;
  - | "") run ;;
  *) if [ ! -f "$1" ]; then echo "Error: Cannot find module '$(basename "$1")'" >&2; exit 1; fi
     s=$1; shift; echo "[$(basename "$s")]${*:+ $*}"; run "$s" ;;
esac
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
node --input-type=commonjs <<'JS'
const os = require("node:os");
console.log("commonjs mode");
JS

#### node here-doc that exits non-zero
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
run() { awk '
/^ *console\.log\(".*"\);?$/ { s = $0; sub(/^ *console\.log\("/, "", s); sub(/"\);?$/, "", s); print s; next }
/^ *fs\.appendFileSync\(".*", ".*\\n"\);?$/ { f = $0; sub(/^ *fs\.appendFileSync\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^[^"]*", "/, "", t); sub(/\\n"\);?$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *process\.exit\([0-9]+\);?$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"; }
case $1 in --input-type=*) shift ;; esac
case $1 in
  -e) printf '%s\n' "$2" | run ;;
  - | "") run ;;
  *) if [ ! -f "$1" ]; then echo "Error: Cannot find module '$(basename "$1")'" >&2; exit 1; fi
     s=$1; shift; echo "[$(basename "$s")]${*:+ $*}"; run "$s" ;;
esac
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
node - <<'NODE'
const missing = ["src/a.ts"];
console.log("missing files: 1");
process.exit(4);
NODE
echo "status=$?"

#### node here-doc that appends to a file
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
run() { awk '
/^ *console\.log\(".*"\);?$/ { s = $0; sub(/^ *console\.log\("/, "", s); sub(/"\);?$/, "", s); print s; next }
/^ *fs\.appendFileSync\(".*", ".*\\n"\);?$/ { f = $0; sub(/^ *fs\.appendFileSync\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^[^"]*", "/, "", t); sub(/\\n"\);?$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *process\.exit\([0-9]+\);?$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"; }
case $1 in --input-type=*) shift ;; esac
case $1 in
  -e) printf '%s\n' "$2" | run ;;
  - | "") run ;;
  *) if [ ! -f "$1" ]; then echo "Error: Cannot find module '$(basename "$1")'" >&2; exit 1; fi
     s=$1; shift; echo "[$(basename "$s")]${*:+ $*}"; run "$s" ;;
esac
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
printf 'alpha\n' > notes.txt
node - <<'NODE'
const fs = require("node:fs");
fs.appendFileSync("notes.txt", "beta\n");
console.log("appended");
NODE
cat notes.txt

#### node here-doc body arrives verbatim
mkdir -p bin && printf '#!/bin/sh\ncat\n' > bin/node && chmod +x bin/node && export PATH="$PWD/bin:$PATH"
node --input-type=module <<'EOF'
const name = process.env.USER ?? "nobody";
console.log(`hello ${name}`);
const re = /\d+$/;
EOF

#### cat a module to a file then run it
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
run() { awk '
/^ *console\.log\(".*"\);?$/ { s = $0; sub(/^ *console\.log\("/, "", s); sub(/"\);?$/, "", s); print s; next }
/^ *fs\.appendFileSync\(".*", ".*\\n"\);?$/ { f = $0; sub(/^ *fs\.appendFileSync\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^[^"]*", "/, "", t); sub(/\\n"\);?$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *process\.exit\([0-9]+\);?$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"; }
case $1 in --input-type=*) shift ;; esac
case $1 in
  -e) printf '%s\n' "$2" | run ;;
  - | "") run ;;
  *) if [ ! -f "$1" ]; then echo "Error: Cannot find module '$(basename "$1")'" >&2; exit 1; fi
     s=$1; shift; echo "[$(basename "$s")]${*:+ $*}"; run "$s" ;;
esac
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
mkdir -p app && cd app && cat > gen.mjs <<'EOF'
import { writeFileSync } from "node:fs";
console.log("wrote config.json");
EOF
node "$PWD/gen.mjs"
ls

#### python3 - with a PY here-doc
mkdir -p bin && printf '#!/bin/sh\n' > bin/python3
cat >> bin/python3 <<'STUB'
exec awk '
/^ *print\(".*"\)$/ { s = $0; sub(/^ *print\("/, "", s); sub(/"\)$/, "", s); print s; next }
/^ *open\(".*", "a"\)\.write\(".*\\n"\)$/ { f = $0; sub(/^ *open\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^.*\.write\("/, "", t); sub(/\\n"\)$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *sys\.exit\([0-9]+\)$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"
STUB
chmod +x bin/python3 && export PATH="$PWD/bin:$PATH"
python3 - <<'PY'
import json
from pathlib import Path
print("scanning")
print("3 files")
PY

#### python - with a PY here-doc
mkdir -p bin && printf '#!/bin/sh\n' > bin/python
cat >> bin/python <<'STUB'
exec awk '
/^ *print\(".*"\)$/ { s = $0; sub(/^ *print\("/, "", s); sub(/"\)$/, "", s); print s; next }
/^ *open\(".*", "a"\)\.write\(".*\\n"\)$/ { f = $0; sub(/^ *open\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^.*\.write\("/, "", t); sub(/\\n"\)$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *sys\.exit\([0-9]+\)$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"
STUB
chmod +x bin/python && export PATH="$PWD/bin:$PATH"
python - <<'PY'
import sys
print("python ok")
PY
echo "status=$?"

#### python3 here-doc that exits with a status
mkdir -p bin && printf '#!/bin/sh\n' > bin/python3
cat >> bin/python3 <<'STUB'
exec awk '
/^ *print\(".*"\)$/ { s = $0; sub(/^ *print\("/, "", s); sub(/"\)$/, "", s); print s; next }
/^ *open\(".*", "a"\)\.write\(".*\\n"\)$/ { f = $0; sub(/^ *open\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^.*\.write\("/, "", t); sub(/\\n"\)$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *sys\.exit\([0-9]+\)$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"
STUB
chmod +x bin/python3 && export PATH="$PWD/bin:$PATH"
python3 - <<'PY'
import sys
print("validation failed: 2 errors")
sys.exit(2)
print("not reached")
PY
echo "status=$?"

#### python3 here-doc body arrives verbatim
mkdir -p bin && printf '#!/bin/sh\ncat\n' > bin/python3 && chmod +x bin/python3 && export PATH="$PWD/bin:$PATH"
python3 - <<'PY'
import os
name = os.environ.get("HOME", "$HOME")
print(f"{name!r} `not a command` \n")
PY

#### python3 with an unquoted here-doc expands variables
mkdir -p bin && printf '#!/bin/sh\n' > bin/python3
cat >> bin/python3 <<'STUB'
exec awk '
/^ *print\(".*"\)$/ { s = $0; sub(/^ *print\("/, "", s); sub(/"\)$/, "", s); print s; next }
/^ *open\(".*", "a"\)\.write\(".*\\n"\)$/ { f = $0; sub(/^ *open\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^.*\.write\("/, "", t); sub(/\\n"\)$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *sys\.exit\([0-9]+\)$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"
STUB
chmod +x bin/python3 && export PATH="$PWD/bin:$PATH"
pkg=core
python3 - <<EOF
print("package $pkg")
print("files $(printf '%s' 3)")
EOF

#### python3 here-doc output to a file
mkdir -p bin && printf '#!/bin/sh\n' > bin/python3
cat >> bin/python3 <<'STUB'
exec awk '
/^ *print\(".*"\)$/ { s = $0; sub(/^ *print\("/, "", s); sub(/"\)$/, "", s); print s; next }
/^ *open\(".*", "a"\)\.write\(".*\\n"\)$/ { f = $0; sub(/^ *open\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^.*\.write\("/, "", t); sub(/\\n"\)$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *sys\.exit\([0-9]+\)$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"
STUB
chmod +x bin/python3 && export PATH="$PWD/bin:$PATH"
python3 - <<'PY' > report.txt
print("ok: parser")
print("ok: lexer")
print("fail: runtime")
PY
grep -c '^ok' report.txt

#### cd and python3 edit then sed -n
mkdir -p bin && printf '#!/bin/sh\n' > bin/python3
cat >> bin/python3 <<'STUB'
exec awk '
/^ *print\(".*"\)$/ { s = $0; sub(/^ *print\("/, "", s); sub(/"\)$/, "", s); print s; next }
/^ *open\(".*", "a"\)\.write\(".*\\n"\)$/ { f = $0; sub(/^ *open\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^.*\.write\("/, "", t); sub(/\\n"\)$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *sys\.exit\([0-9]+\)$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"
STUB
chmod +x bin/python3 && export PATH="$PWD/bin:$PATH"
mkdir -p proj/src && printf 'def main():\n    return 0\n' > proj/src/app.py
cd proj && python3 - <<'EOF'
from pathlib import Path
open("src/app.py", "a").write("def helper():\n")
open("src/app.py", "a").write("    return 1\n")
print("patched src/app.py")
EOF
sed -n 1,10p src/app.py

#### cd and python3 edit then sed -n a range
mkdir -p bin && printf '#!/bin/sh\n' > bin/python3
cat >> bin/python3 <<'STUB'
exec awk '
/^ *print\(".*"\)$/ { s = $0; sub(/^ *print\("/, "", s); sub(/"\)$/, "", s); print s; next }
/^ *open\(".*", "a"\)\.write\(".*\\n"\)$/ { f = $0; sub(/^ *open\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^.*\.write\("/, "", t); sub(/\\n"\)$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *sys\.exit\([0-9]+\)$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"
STUB
chmod +x bin/python3 && export PATH="$PWD/bin:$PATH"
mkdir -p proj && seq 1 5 | sed 's/^/line /' > proj/data.txt
cd proj && python3 - <<'EOF'
open("data.txt", "a").write("line 6\n")
open("data.txt", "a").write("line 7\n")
EOF
sed -n 4,7p data.txt

#### cd and python3 then grep -n
mkdir -p bin && printf '#!/bin/sh\n' > bin/python3
cat >> bin/python3 <<'STUB'
exec awk '
/^ *print\(".*"\)$/ { s = $0; sub(/^ *print\("/, "", s); sub(/"\)$/, "", s); print s; next }
/^ *open\(".*", "a"\)\.write\(".*\\n"\)$/ { f = $0; sub(/^ *open\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^.*\.write\("/, "", t); sub(/\\n"\)$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *sys\.exit\([0-9]+\)$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"
STUB
chmod +x bin/python3 && export PATH="$PWD/bin:$PATH"
mkdir -p proj && printf 'import os\n\ndef run():\n    pass\n' > proj/main.py
cd proj && python3 - <<'EOF'
open("main.py", "a").write("def stop():\n")
open("main.py", "a").write("    pass\n")
EOF
grep -n "def " main.py

#### cd and python3 then grep -n alternatives
mkdir -p bin && printf '#!/bin/sh\n' > bin/python3
cat >> bin/python3 <<'STUB'
exec awk '
/^ *print\(".*"\)$/ { s = $0; sub(/^ *print\("/, "", s); sub(/"\)$/, "", s); print s; next }
/^ *open\(".*", "a"\)\.write\(".*\\n"\)$/ { f = $0; sub(/^ *open\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^.*\.write\("/, "", t); sub(/\\n"\)$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *sys\.exit\([0-9]+\)$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"
STUB
chmod +x bin/python3 && export PATH="$PWD/bin:$PATH"
mkdir -p proj && printf 'TODO: docs\nok\nFIXME: tests\n' > proj/NOTES.md
cd proj && python3 - <<'EOF'
open("NOTES.md", "a").write("TODO: release\n")
EOF
grep -n "TODO\|FIXME" NOTES.md

#### cd and python3 then grep -n alternatives into head
mkdir -p bin && printf '#!/bin/sh\n' > bin/python3
cat >> bin/python3 <<'STUB'
exec awk '
/^ *print\(".*"\)$/ { s = $0; sub(/^ *print\("/, "", s); sub(/"\)$/, "", s); print s; next }
/^ *open\(".*", "a"\)\.write\(".*\\n"\)$/ { f = $0; sub(/^ *open\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^.*\.write\("/, "", t); sub(/\\n"\)$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *sys\.exit\([0-9]+\)$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"
STUB
chmod +x bin/python3 && export PATH="$PWD/bin:$PATH"
mkdir -p proj && printf 'error: one\nwarn: two\nerror: three\nwarn: four\nerror: five\n' > proj/build.log
cd proj && python3 - <<'EOF'
print("log rotated")
EOF
grep -n "error\|warn" build.log | head

#### cd and python3 then echo
mkdir -p bin && printf '#!/bin/sh\n' > bin/python3
cat >> bin/python3 <<'STUB'
exec awk '
/^ *print\(".*"\)$/ { s = $0; sub(/^ *print\("/, "", s); sub(/"\)$/, "", s); print s; next }
/^ *open\(".*", "a"\)\.write\(".*\\n"\)$/ { f = $0; sub(/^ *open\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^.*\.write\("/, "", t); sub(/\\n"\)$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *sys\.exit\([0-9]+\)$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"
STUB
chmod +x bin/python3 && export PATH="$PWD/bin:$PATH"
mkdir -p proj && printf 'v1\n' > proj/VERSION
cd proj && python3 - <<'EOF'
open("VERSION", "a").write("v2\n")
EOF
echo done; cat VERSION

#### cd and python3 edit then git diff --stat
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
mkdir -p bin && printf '#!/bin/sh\n' > bin/python3
cat >> bin/python3 <<'STUB'
exec awk '
/^ *print\(".*"\)$/ { s = $0; sub(/^ *print\("/, "", s); sub(/"\)$/, "", s); print s; next }
/^ *open\(".*", "a"\)\.write\(".*\\n"\)$/ { f = $0; sub(/^ *open\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^.*\.write\("/, "", t); sub(/\\n"\)$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *sys\.exit\([0-9]+\)$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"
STUB
chmod +x bin/python3 && export PATH="$PWD/bin:$PATH"
mkdir -p proj/src && printf 'def a():\n    return 1\n' > proj/src/mod.py
git -C proj init -q -b main && git -C proj add . && git -C proj commit -qm init
cd proj && python3 - <<'EOF'
from pathlib import Path
open("src/mod.py", "a").write("def b():\n")
open("src/mod.py", "a").write("    return 2\n")
print("added b")
EOF
git diff --stat

#### cd and python3 then node --test grep
mkdir -p bin && printf '#!/bin/sh\n' > bin/python3
cat >> bin/python3 <<'STUB'
exec awk '
/^ *print\(".*"\)$/ { s = $0; sub(/^ *print\("/, "", s); sub(/"\)$/, "", s); print s; next }
/^ *open\(".*", "a"\)\.write\(".*\\n"\)$/ { f = $0; sub(/^ *open\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^.*\.write\("/, "", t); sub(/\\n"\)$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *sys\.exit\([0-9]+\)$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"
STUB
chmod +x bin/python3 && export PATH="$PWD/bin:$PATH"
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
[ "$1" = --test ] && shift
awk '
function flush() { if (name != "") { n++; if (bad) { fail++; print "not ok " n " - " name } else { pass++; print "ok " n " - " name } } }
/^test\("/ { flush(); name = $0; sub(/^test\("/, "", name); sub(/".*$/, "", name); bad = 0; next }
/assert\.ok\(false/ { bad = 1 }
END { flush(); print "# tests " n; print "# pass " pass + 0; print "# fail " fail + 0; exit (fail > 0) }
' "$@"
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
mkdir -p proj/test
printf 'test("keeps order", () => {\n  assert.ok(true);\n});\n' > proj/test/sort.test.js
cd proj && python3 - <<'EOF'
open("test/sort.test.js", "a").write("test(\"handles ties\", () => {\n")
open("test/sort.test.js", "a").write("  assert.ok(false);\n")
open("test/sort.test.js", "a").write("});\n")
EOF
node --test test/sort.test.js 2>&1 | grep -E "^(ok|not ok)" | head

#### cat a python script to a file then run it
mkdir -p bin && printf '#!/bin/sh\n' > bin/python3
cat >> bin/python3 <<'STUB'
exec awk '
/^ *print\(".*"\)$/ { s = $0; sub(/^ *print\("/, "", s); sub(/"\)$/, "", s); print s; next }
/^ *open\(".*", "a"\)\.write\(".*\\n"\)$/ { f = $0; sub(/^ *open\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^.*\.write\("/, "", t); sub(/\\n"\)$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *sys\.exit\([0-9]+\)$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"
STUB
chmod +x bin/python3 && export PATH="$PWD/bin:$PATH"
cat > check_env.py <<'EOF'
import os
print("env check passed")
EOF
python3 check_env.py && rm check_env.py
ls

#### python3 and node here-docs in one chain
mkdir -p bin && printf '#!/bin/sh\n' > bin/python3
cat >> bin/python3 <<'STUB'
exec awk '
/^ *print\(".*"\)$/ { s = $0; sub(/^ *print\("/, "", s); sub(/"\)$/, "", s); print s; next }
/^ *open\(".*", "a"\)\.write\(".*\\n"\)$/ { f = $0; sub(/^ *open\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^.*\.write\("/, "", t); sub(/\\n"\)$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *sys\.exit\([0-9]+\)$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"
STUB
chmod +x bin/python3 && export PATH="$PWD/bin:$PATH"
mkdir -p bin && printf '#!/bin/sh\n' > bin/node
cat >> bin/node <<'STUB'
run() { awk '
/^ *console\.log\(".*"\);?$/ { s = $0; sub(/^ *console\.log\("/, "", s); sub(/"\);?$/, "", s); print s; next }
/^ *fs\.appendFileSync\(".*", ".*\\n"\);?$/ { f = $0; sub(/^ *fs\.appendFileSync\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^[^"]*", "/, "", t); sub(/\\n"\);?$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *process\.exit\([0-9]+\);?$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"; }
case $1 in --input-type=*) shift ;; esac
case $1 in
  -e) printf '%s\n' "$2" | run ;;
  - | "") run ;;
  *) if [ ! -f "$1" ]; then echo "Error: Cannot find module '$(basename "$1")'" >&2; exit 1; fi
     s=$1; shift; echo "[$(basename "$s")]${*:+ $*}"; run "$s" ;;
esac
STUB
chmod +x bin/node && export PATH="$PWD/bin:$PATH"
python3 - <<'PY' && node - <<'NODE'
print("python side")
PY
console.log("node side");
NODE

#### uv run python - with a PY here-doc
mkdir -p bin && printf '#!/bin/sh\n' > bin/uv
cat >> bin/uv <<'STUB'
[ "$1" = run ] && shift
while [ "$1" = --with ]; do shift 2; done
exec "$@"
STUB
chmod +x bin/uv && export PATH="$PWD/bin:$PATH"
mkdir -p bin && printf '#!/bin/sh\n' > bin/python
cat >> bin/python <<'STUB'
exec awk '
/^ *print\(".*"\)$/ { s = $0; sub(/^ *print\("/, "", s); sub(/"\)$/, "", s); print s; next }
/^ *open\(".*", "a"\)\.write\(".*\\n"\)$/ { f = $0; sub(/^ *open\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^.*\.write\("/, "", t); sub(/\\n"\)$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *sys\.exit\([0-9]+\)$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"
STUB
chmod +x bin/python && export PATH="$PWD/bin:$PATH"
uv run python - <<'PY'
import tomllib
print("uv python ok")
PY

#### uv run --with a package and python -
mkdir -p bin && printf '#!/bin/sh\n' > bin/uv
cat >> bin/uv <<'STUB'
[ "$1" = run ] && shift
while [ "$1" = --with ]; do shift 2; done
exec "$@"
STUB
chmod +x bin/uv && export PATH="$PWD/bin:$PATH"
mkdir -p bin && printf '#!/bin/sh\n' > bin/python
cat >> bin/python <<'STUB'
exec awk '
/^ *print\(".*"\)$/ { s = $0; sub(/^ *print\("/, "", s); sub(/"\)$/, "", s); print s; next }
/^ *open\(".*", "a"\)\.write\(".*\\n"\)$/ { f = $0; sub(/^ *open\("/, "", f); t = f; sub(/".*$/, "", f); sub(/^.*\.write\("/, "", t); sub(/\\n"\)$/, "", t); gsub(/\\"/, "\"", t); print t >> f; next }
/^ *sys\.exit\([0-9]+\)$/ { s = $0; gsub(/[^0-9]/, "", s); exit s }
' "$@"
STUB
chmod +x bin/python && export PATH="$PWD/bin:$PATH"
uv run --with rich --with httpx python - <<'PY'
from rich import print as rprint
print("with extras")
PY
echo "status=$?"

#### uv run pytest
mkdir -p bin && printf '#!/bin/sh\n' > bin/uv
cat >> bin/uv <<'STUB'
[ "$1" = run ] && shift
while [ "$1" = --with ]; do shift 2; done
exec "$@"
STUB
chmod +x bin/uv && export PATH="$PWD/bin:$PATH"
mkdir -p bin && printf '#!/bin/sh\n' > bin/pytest
cat >> bin/pytest <<'STUB'
n=$(cat tests/test_*.py | grep -c '^def test_')
f=$(cat tests/test_*.py | grep -c 'assert False')
if [ "$f" -gt 0 ]; then echo "$((n - f)) passed, $f failed"; exit 1; fi
echo "$n passed"
STUB
chmod +x bin/pytest && export PATH="$PWD/bin:$PATH"
mkdir -p tests
printf 'def test_add():\n    assert 1 + 1 == 2\n\ndef test_sub():\n    assert 2 - 1 == 1\n' > tests/test_math.py
uv run pytest -q tests
echo "status=$?"

#### cargo test -q piped to tail
mkdir -p bin && printf '#!/bin/sh\n' > bin/cargo
cat >> bin/cargo <<'STUB'
echo "   Compiling demo v0.1.0" >&2
echo "    Finished test profile [unoptimized + debuginfo] target(s)" >&2
awk '
/^fn [a-z_0-9]+\(\)/ { name = $2; sub(/\(.*/, "", name); names[++n] = name; next }
/panic!\(/ { bad[n] = 1 }
END {
  printf "\nrunning %d tests\n", n
  for (i = 1; i <= n; i++) { if (bad[i]) { fail++; print "test " names[i] " ... FAILED" } else { pass++; print "test " names[i] " ... ok" } }
  printf "\ntest result: %s. %d passed; %d failed; 0 ignored; 0 measured; 0 filtered out\n", (fail ? "FAILED" : "ok"), pass, fail
  exit (fail ? 101 : 0)
}' tests/*.rs
STUB
chmod +x bin/cargo && export PATH="$PWD/bin:$PATH"
mkdir -p tests
printf 'fn parses_empty() {\n    assert!(true);\n}\nfn parses_word() {\n    assert!(true);\n}\n' > tests/parse.rs
cargo test -q 2>&1 | tail -n 3

#### cargo test failing
mkdir -p bin && printf '#!/bin/sh\n' > bin/cargo
cat >> bin/cargo <<'STUB'
echo "   Compiling demo v0.1.0" >&2
echo "    Finished test profile [unoptimized + debuginfo] target(s)" >&2
awk '
/^fn [a-z_0-9]+\(\)/ { name = $2; sub(/\(.*/, "", name); names[++n] = name; next }
/panic!\(/ { bad[n] = 1 }
END {
  printf "\nrunning %d tests\n", n
  for (i = 1; i <= n; i++) { if (bad[i]) { fail++; print "test " names[i] " ... FAILED" } else { pass++; print "test " names[i] " ... ok" } }
  printf "\ntest result: %s. %d passed; %d failed; 0 ignored; 0 measured; 0 filtered out\n", (fail ? "FAILED" : "ok"), pass, fail
  exit (fail ? 101 : 0)
}' tests/*.rs
STUB
chmod +x bin/cargo && export PATH="$PWD/bin:$PATH"
mkdir -p tests
printf 'fn ok_case() {\n}\nfn broken_case() {\n    panic!("boom");\n}\n' > tests/it.rs
cargo test -q
echo "status=$?"

#### cd and go test ./...
mkdir -p bin && printf '#!/bin/sh\n' > bin/go
cat >> bin/go <<'STUB'
mod=$(sed -n 's/^module //p' go.mod)
rc=0
for dir in $(find . -name '*.go' | sed 's|/[^/]*$||' | sort -u); do
  pkg=$mod${dir#.}
  if ! ls "$dir"/*_test.go >/dev/null 2>&1; then printf '?   \t%s\t[no test files]\n' "$pkg"; continue; fi
  if grep -q 't.Fatal' "$dir"/*_test.go; then
    grep -h '^func Test' "$dir"/*_test.go | sed 's/^func \(Test[A-Za-z0-9]*\).*/--- FAIL: \1 (0.00s)/'
    printf 'FAIL\nFAIL\t%s\t0.01s\n' "$pkg"; rc=1
  else
    printf 'ok  \t%s\t0.01s\n' "$pkg"
  fi
done
[ $rc = 1 ] && echo FAIL
exit $rc
STUB
chmod +x bin/go && export PATH="$PWD/bin:$PATH"
mkdir -p svc/api svc/store svc/cmd
printf 'module example.com/svc\n\ngo 1.22\n' > svc/go.mod
printf 'package api\n' > svc/api/api.go
printf 'package api\n\nfunc TestRoutes(t *testing.T) {}\n' > svc/api/api_test.go
printf 'package store\n' > svc/store/store.go
printf 'package store\n\nfunc TestGet(t *testing.T) {}\n' > svc/store/store_test.go
printf 'package main\n' > svc/cmd/main.go
cd svc && go test ./...
echo "status=$?"

#### go test ./... with a failing package
mkdir -p bin && printf '#!/bin/sh\n' > bin/go
cat >> bin/go <<'STUB'
mod=$(sed -n 's/^module //p' go.mod)
rc=0
for dir in $(find . -name '*.go' | sed 's|/[^/]*$||' | sort -u); do
  pkg=$mod${dir#.}
  if ! ls "$dir"/*_test.go >/dev/null 2>&1; then printf '?   \t%s\t[no test files]\n' "$pkg"; continue; fi
  if grep -q 't.Fatal' "$dir"/*_test.go; then
    grep -h '^func Test' "$dir"/*_test.go | sed 's/^func \(Test[A-Za-z0-9]*\).*/--- FAIL: \1 (0.00s)/'
    printf 'FAIL\nFAIL\t%s\t0.01s\n' "$pkg"; rc=1
  else
    printf 'ok  \t%s\t0.01s\n' "$pkg"
  fi
done
[ $rc = 1 ] && echo FAIL
exit $rc
STUB
chmod +x bin/go && export PATH="$PWD/bin:$PATH"
mkdir -p svc/api
printf 'module example.com/svc\n' > svc/go.mod
printf 'package api\n' > svc/api/api.go
printf 'package api\n\nfunc TestLogin(t *testing.T) {\n\tt.Fatal("bad token")\n}\n' > svc/api/api_test.go
cd svc && go test ./... 2>&1 | grep -v '^ok'
echo "status=${PIPESTATUS[0]}"

#### gh issue view with --json fields
mkdir -p bin && printf '#!/bin/sh\n' > bin/gh
cat >> bin/gh <<'STUB'
kind=$1; shift
if [ "$kind" = api ]; then path=$1; shift; else verb=$1 id=$2; shift 2; fi
fields= jqx= body= comments=
while [ $# -gt 0 ]; do
  case $1 in --json) fields=$2; shift ;; --jq) jqx=$2; shift ;; --body-file) body=$2; shift ;; --comments) comments=1 ;; esac
  shift
done
case "$kind $verb" in
  "issue view")
    f=.gh/issue-$id.json
    if [ ! -f "$f" ]; then echo "GraphQL: Could not resolve to an issue or pull request with the number of $id. (repository.issue)" >&2; exit 1; fi
    if [ -n "$fields" ]; then jq -c --arg f "$fields" '. as $i | [$f | split(",")[] | {(.): $i[.]}] | add' "$f"
    else jq -r --arg c "$comments" '"title:\t" + .title, "state:\t" + .state, "--", .body, (if $c == "1" then .comments[] | "--", "author: " + .author, .body else empty end)' "$f"; fi ;;
  "issue comment") echo "commented on issue $id with $(wc -l < "$body" | tr -d ' ') lines" ;;
  "run view") cat ".gh/run-$id.log" ;;
  "run watch") s=$(cat ".gh/run-$id.status"); echo "Run $id completed with '$s'"; [ "$s" = success ] ;;
  "api ") jq -c "${jqx:-.}" ".gh/api/$(printf '%s' "$path" | tr / _).json" ;;
  *) echo "gh stub: $kind $verb" >&2; exit 1 ;;
esac
STUB
chmod +x bin/gh && export PATH="$PWD/bin:$PATH"
mkdir -p .gh
printf '{"number": 42, "title": "Crash on empty input", "state": "OPEN", "labels": [{"name": "bug"}], "author": {"login": "octo"}, "body": "Steps to reproduce", "comments": []}\n' > .gh/issue-42.json
gh issue view 42 --repo acme/widgets --json title,state,labels

#### gh issue view with many --json fields
mkdir -p bin && printf '#!/bin/sh\n' > bin/gh
cat >> bin/gh <<'STUB'
kind=$1; shift
if [ "$kind" = api ]; then path=$1; shift; else verb=$1 id=$2; shift 2; fi
fields= jqx= body= comments=
while [ $# -gt 0 ]; do
  case $1 in --json) fields=$2; shift ;; --jq) jqx=$2; shift ;; --body-file) body=$2; shift ;; --comments) comments=1 ;; esac
  shift
done
case "$kind $verb" in
  "issue view")
    f=.gh/issue-$id.json
    if [ ! -f "$f" ]; then echo "GraphQL: Could not resolve to an issue or pull request with the number of $id. (repository.issue)" >&2; exit 1; fi
    if [ -n "$fields" ]; then jq -c --arg f "$fields" '. as $i | [$f | split(",")[] | {(.): $i[.]}] | add' "$f"
    else jq -r --arg c "$comments" '"title:\t" + .title, "state:\t" + .state, "--", .body, (if $c == "1" then .comments[] | "--", "author: " + .author, .body else empty end)' "$f"; fi ;;
  "issue comment") echo "commented on issue $id with $(wc -l < "$body" | tr -d ' ') lines" ;;
  "run view") cat ".gh/run-$id.log" ;;
  "run watch") s=$(cat ".gh/run-$id.status"); echo "Run $id completed with '$s'"; [ "$s" = success ] ;;
  "api ") jq -c "${jqx:-.}" ".gh/api/$(printf '%s' "$path" | tr / _).json" ;;
  *) echo "gh stub: $kind $verb" >&2; exit 1 ;;
esac
STUB
chmod +x bin/gh && export PATH="$PWD/bin:$PATH"
mkdir -p .gh
printf '{"number": 7, "title": "Docs typo", "state": "CLOSED", "labels": [], "author": {"login": "dev"}, "body": "fix it", "url": "https://github.com/acme/widgets/issues/7", "comments": []}\n' > .gh/issue-7.json
gh issue view 7 --repo acme/widgets --json number,title,state,labels,author,body,url | jq -r '.title + " by " + .author.login'

#### gh issue view with comments
mkdir -p bin && printf '#!/bin/sh\n' > bin/gh
cat >> bin/gh <<'STUB'
kind=$1; shift
if [ "$kind" = api ]; then path=$1; shift; else verb=$1 id=$2; shift 2; fi
fields= jqx= body= comments=
while [ $# -gt 0 ]; do
  case $1 in --json) fields=$2; shift ;; --jq) jqx=$2; shift ;; --body-file) body=$2; shift ;; --comments) comments=1 ;; esac
  shift
done
case "$kind $verb" in
  "issue view")
    f=.gh/issue-$id.json
    if [ ! -f "$f" ]; then echo "GraphQL: Could not resolve to an issue or pull request with the number of $id. (repository.issue)" >&2; exit 1; fi
    if [ -n "$fields" ]; then jq -c --arg f "$fields" '. as $i | [$f | split(",")[] | {(.): $i[.]}] | add' "$f"
    else jq -r --arg c "$comments" '"title:\t" + .title, "state:\t" + .state, "--", .body, (if $c == "1" then .comments[] | "--", "author: " + .author, .body else empty end)' "$f"; fi ;;
  "issue comment") echo "commented on issue $id with $(wc -l < "$body" | tr -d ' ') lines" ;;
  "run view") cat ".gh/run-$id.log" ;;
  "run watch") s=$(cat ".gh/run-$id.status"); echo "Run $id completed with '$s'"; [ "$s" = success ] ;;
  "api ") jq -c "${jqx:-.}" ".gh/api/$(printf '%s' "$path" | tr / _).json" ;;
  *) echo "gh stub: $kind $verb" >&2; exit 1 ;;
esac
STUB
chmod +x bin/gh && export PATH="$PWD/bin:$PATH"
mkdir -p .gh
printf '{"number": 42, "title": "Crash on empty input", "state": "OPEN", "body": "It panics.", "comments": [{"author": "octo", "body": "Repro attached."}, {"author": "dev", "body": "Fixed on main."}]}\n' > .gh/issue-42.json
gh issue view 42 --repo acme/widgets --comments

#### gh issue view for a missing issue
mkdir -p bin && printf '#!/bin/sh\n' > bin/gh
cat >> bin/gh <<'STUB'
kind=$1; shift
if [ "$kind" = api ]; then path=$1; shift; else verb=$1 id=$2; shift 2; fi
fields= jqx= body= comments=
while [ $# -gt 0 ]; do
  case $1 in --json) fields=$2; shift ;; --jq) jqx=$2; shift ;; --body-file) body=$2; shift ;; --comments) comments=1 ;; esac
  shift
done
case "$kind $verb" in
  "issue view")
    f=.gh/issue-$id.json
    if [ ! -f "$f" ]; then echo "GraphQL: Could not resolve to an issue or pull request with the number of $id. (repository.issue)" >&2; exit 1; fi
    if [ -n "$fields" ]; then jq -c --arg f "$fields" '. as $i | [$f | split(",")[] | {(.): $i[.]}] | add' "$f"
    else jq -r --arg c "$comments" '"title:\t" + .title, "state:\t" + .state, "--", .body, (if $c == "1" then .comments[] | "--", "author: " + .author, .body else empty end)' "$f"; fi ;;
  "issue comment") echo "commented on issue $id with $(wc -l < "$body" | tr -d ' ') lines" ;;
  "run view") cat ".gh/run-$id.log" ;;
  "run watch") s=$(cat ".gh/run-$id.status"); echo "Run $id completed with '$s'"; [ "$s" = success ] ;;
  "api ") jq -c "${jqx:-.}" ".gh/api/$(printf '%s' "$path" | tr / _).json" ;;
  *) echo "gh stub: $kind $verb" >&2; exit 1 ;;
esac
STUB
chmod +x bin/gh && export PATH="$PWD/bin:$PATH"
mkdir -p .gh
gh issue view 999 --repo acme/widgets --json title
echo "status=$?"

#### gh run view --log-failed piped to tail
mkdir -p bin && printf '#!/bin/sh\n' > bin/gh
cat >> bin/gh <<'STUB'
kind=$1; shift
if [ "$kind" = api ]; then path=$1; shift; else verb=$1 id=$2; shift 2; fi
fields= jqx= body= comments=
while [ $# -gt 0 ]; do
  case $1 in --json) fields=$2; shift ;; --jq) jqx=$2; shift ;; --body-file) body=$2; shift ;; --comments) comments=1 ;; esac
  shift
done
case "$kind $verb" in
  "issue view")
    f=.gh/issue-$id.json
    if [ ! -f "$f" ]; then echo "GraphQL: Could not resolve to an issue or pull request with the number of $id. (repository.issue)" >&2; exit 1; fi
    if [ -n "$fields" ]; then jq -c --arg f "$fields" '. as $i | [$f | split(",")[] | {(.): $i[.]}] | add' "$f"
    else jq -r --arg c "$comments" '"title:\t" + .title, "state:\t" + .state, "--", .body, (if $c == "1" then .comments[] | "--", "author: " + .author, .body else empty end)' "$f"; fi ;;
  "issue comment") echo "commented on issue $id with $(wc -l < "$body" | tr -d ' ') lines" ;;
  "run view") cat ".gh/run-$id.log" ;;
  "run watch") s=$(cat ".gh/run-$id.status"); echo "Run $id completed with '$s'"; [ "$s" = success ] ;;
  "api ") jq -c "${jqx:-.}" ".gh/api/$(printf '%s' "$path" | tr / _).json" ;;
  *) echo "gh stub: $kind $verb" >&2; exit 1 ;;
esac
STUB
chmod +x bin/gh && export PATH="$PWD/bin:$PATH"
mkdir -p .gh
printf 'test\tRun tests\tok 1 parser\ntest\tRun tests\tnot ok 2 lexer\ntest\tRun tests\tProcess completed with exit code 1.\n' > .gh/run-7001.log
gh run view 7001 --repo acme/widgets --log-failed | tail -n 2

#### gh run watch with --exit-status
mkdir -p bin && printf '#!/bin/sh\n' > bin/gh
cat >> bin/gh <<'STUB'
kind=$1; shift
if [ "$kind" = api ]; then path=$1; shift; else verb=$1 id=$2; shift 2; fi
fields= jqx= body= comments=
while [ $# -gt 0 ]; do
  case $1 in --json) fields=$2; shift ;; --jq) jqx=$2; shift ;; --body-file) body=$2; shift ;; --comments) comments=1 ;; esac
  shift
done
case "$kind $verb" in
  "issue view")
    f=.gh/issue-$id.json
    if [ ! -f "$f" ]; then echo "GraphQL: Could not resolve to an issue or pull request with the number of $id. (repository.issue)" >&2; exit 1; fi
    if [ -n "$fields" ]; then jq -c --arg f "$fields" '. as $i | [$f | split(",")[] | {(.): $i[.]}] | add' "$f"
    else jq -r --arg c "$comments" '"title:\t" + .title, "state:\t" + .state, "--", .body, (if $c == "1" then .comments[] | "--", "author: " + .author, .body else empty end)' "$f"; fi ;;
  "issue comment") echo "commented on issue $id with $(wc -l < "$body" | tr -d ' ') lines" ;;
  "run view") cat ".gh/run-$id.log" ;;
  "run watch") s=$(cat ".gh/run-$id.status"); echo "Run $id completed with '$s'"; [ "$s" = success ] ;;
  "api ") jq -c "${jqx:-.}" ".gh/api/$(printf '%s' "$path" | tr / _).json" ;;
  *) echo "gh stub: $kind $verb" >&2; exit 1 ;;
esac
STUB
chmod +x bin/gh && export PATH="$PWD/bin:$PATH"
mkdir -p .gh && echo failure > .gh/run-7002.status
gh run watch 7002 --repo acme/widgets --exit-status
echo "status=$?"

#### gh api with --jq
mkdir -p bin && printf '#!/bin/sh\n' > bin/gh
cat >> bin/gh <<'STUB'
kind=$1; shift
if [ "$kind" = api ]; then path=$1; shift; else verb=$1 id=$2; shift 2; fi
fields= jqx= body= comments=
while [ $# -gt 0 ]; do
  case $1 in --json) fields=$2; shift ;; --jq) jqx=$2; shift ;; --body-file) body=$2; shift ;; --comments) comments=1 ;; esac
  shift
done
case "$kind $verb" in
  "issue view")
    f=.gh/issue-$id.json
    if [ ! -f "$f" ]; then echo "GraphQL: Could not resolve to an issue or pull request with the number of $id. (repository.issue)" >&2; exit 1; fi
    if [ -n "$fields" ]; then jq -c --arg f "$fields" '. as $i | [$f | split(",")[] | {(.): $i[.]}] | add' "$f"
    else jq -r --arg c "$comments" '"title:\t" + .title, "state:\t" + .state, "--", .body, (if $c == "1" then .comments[] | "--", "author: " + .author, .body else empty end)' "$f"; fi ;;
  "issue comment") echo "commented on issue $id with $(wc -l < "$body" | tr -d ' ') lines" ;;
  "run view") cat ".gh/run-$id.log" ;;
  "run watch") s=$(cat ".gh/run-$id.status"); echo "Run $id completed with '$s'"; [ "$s" = success ] ;;
  "api ") jq -c "${jqx:-.}" ".gh/api/$(printf '%s' "$path" | tr / _).json" ;;
  *) echo "gh stub: $kind $verb" >&2; exit 1 ;;
esac
STUB
chmod +x bin/gh && export PATH="$PWD/bin:$PATH"
mkdir -p .gh/api
printf '{"name": "widgets", "default_branch": "main", "open_issues_count": 3}\n' > .gh/api/repos_acme_widgets.json
gh api repos/acme/widgets --jq '{default_branch, open_issues_count}'

#### gh api --paginate with --jq
mkdir -p bin && printf '#!/bin/sh\n' > bin/gh
cat >> bin/gh <<'STUB'
kind=$1; shift
if [ "$kind" = api ]; then path=$1; shift; else verb=$1 id=$2; shift 2; fi
fields= jqx= body= comments=
while [ $# -gt 0 ]; do
  case $1 in --json) fields=$2; shift ;; --jq) jqx=$2; shift ;; --body-file) body=$2; shift ;; --comments) comments=1 ;; esac
  shift
done
case "$kind $verb" in
  "issue view")
    f=.gh/issue-$id.json
    if [ ! -f "$f" ]; then echo "GraphQL: Could not resolve to an issue or pull request with the number of $id. (repository.issue)" >&2; exit 1; fi
    if [ -n "$fields" ]; then jq -c --arg f "$fields" '. as $i | [$f | split(",")[] | {(.): $i[.]}] | add' "$f"
    else jq -r --arg c "$comments" '"title:\t" + .title, "state:\t" + .state, "--", .body, (if $c == "1" then .comments[] | "--", "author: " + .author, .body else empty end)' "$f"; fi ;;
  "issue comment") echo "commented on issue $id with $(wc -l < "$body" | tr -d ' ') lines" ;;
  "run view") cat ".gh/run-$id.log" ;;
  "run watch") s=$(cat ".gh/run-$id.status"); echo "Run $id completed with '$s'"; [ "$s" = success ] ;;
  "api ") jq -c "${jqx:-.}" ".gh/api/$(printf '%s' "$path" | tr / _).json" ;;
  *) echo "gh stub: $kind $verb" >&2; exit 1 ;;
esac
STUB
chmod +x bin/gh && export PATH="$PWD/bin:$PATH"
mkdir -p .gh/api
printf '[{"number": 12, "title": "Add parser", "draft": false}, {"number": 15, "title": "WIP lexer", "draft": true}]\n' > .gh/api/repos_acme_widgets_pulls.json
gh api repos/acme/widgets/pulls --paginate --jq '.[] | {number, title}'

#### gh issue comment with --body-file
mkdir -p bin && printf '#!/bin/sh\n' > bin/gh
cat >> bin/gh <<'STUB'
kind=$1; shift
if [ "$kind" = api ]; then path=$1; shift; else verb=$1 id=$2; shift 2; fi
fields= jqx= body= comments=
while [ $# -gt 0 ]; do
  case $1 in --json) fields=$2; shift ;; --jq) jqx=$2; shift ;; --body-file) body=$2; shift ;; --comments) comments=1 ;; esac
  shift
done
case "$kind $verb" in
  "issue view")
    f=.gh/issue-$id.json
    if [ ! -f "$f" ]; then echo "GraphQL: Could not resolve to an issue or pull request with the number of $id. (repository.issue)" >&2; exit 1; fi
    if [ -n "$fields" ]; then jq -c --arg f "$fields" '. as $i | [$f | split(",")[] | {(.): $i[.]}] | add' "$f"
    else jq -r --arg c "$comments" '"title:\t" + .title, "state:\t" + .state, "--", .body, (if $c == "1" then .comments[] | "--", "author: " + .author, .body else empty end)' "$f"; fi ;;
  "issue comment") echo "commented on issue $id with $(wc -l < "$body" | tr -d ' ') lines" ;;
  "run view") cat ".gh/run-$id.log" ;;
  "run watch") s=$(cat ".gh/run-$id.status"); echo "Run $id completed with '$s'"; [ "$s" = success ] ;;
  "api ") jq -c "${jqx:-.}" ".gh/api/$(printf '%s' "$path" | tr / _).json" ;;
  *) echo "gh stub: $kind $verb" >&2; exit 1 ;;
esac
STUB
chmod +x bin/gh && export PATH="$PWD/bin:$PATH"
cat > comment.md <<'EOF'
Fixed in the latest commit.

The parser now handles empty input.
EOF
gh issue comment 42 --repo acme/widgets --body-file comment.md
echo "status=$?"
