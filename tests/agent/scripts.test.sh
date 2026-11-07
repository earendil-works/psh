# Agent shape: multi-line scripts agents write in one command: `set -euo pipefail`
# headers, functions, loops over files (`for f in src/*.c; do ... || exit 1; done`),
# `while IFS= read -r line`, if/elif chains, case, arrays, `trap cleanup EXIT`,
# counters with $((...)), here-doc config written then checked, early `exit 1`,
# `|| { echo ...; exit 1; }` guards, `command -v x >/dev/null || echo missing`,
# retry loops (`for i in $(seq 1 N); do if cmd; then exit; fi; sleep 1; done; exit 2`)
# and scripts that fail on purpose under set -e, set -u and pipefail.

#### strict header then a loop over sources
set -euo pipefail
mkdir -p src
printf 'int a;\n' > src/a.c
printf 'int b;\nint c;\n' > src/b.c
for f in src/*.c; do
  echo "$f: $(wc -l < "$f" | tr -d ' ') lines"
done
echo done

#### node --check loop with exit 1 on failure
mkdir -p bin scripts
printf '#!/bin/sh\ncase "$2" in *bad*) echo "SyntaxError in $2" >&2; exit 1;; esac\n' > bin/node
chmod +x bin/node
export PATH="$PWD/bin:$PATH"
printf 'ok\n' > scripts/a.js
printf 'ok\n' > scripts/b.js
printf 'ok\n' > main.js
for f in scripts/*.js main.js; do node --check "$f" || exit 1; done
echo "all ok status=$?"

#### node --check loop stops at the first bad file
mkdir -p bin scripts
printf '#!/bin/sh\necho "check $2"\ncase "$2" in *bad*) exit 1;; esac\n' > bin/node
chmod +x bin/node
export PATH="$PWD/bin:$PATH"
touch scripts/a.js scripts/bad.js scripts/c.js
(for f in scripts/*.js; do node --check "$f" || exit 1; done; echo "not reached")
echo "status=$?"

#### retry loop exits on success
mkdir -p bin
printf '#!/bin/sh\nn=$(cat count 2>/dev/null || echo 0)\nn=$((n+1))\necho $n > count\n[ $n -ge 3 ] && { echo "v1.2.3"; exit 0; }\nexit 1\n' > bin/server
chmod +x bin/server
(B="$PWD/bin/server"; for i in $(seq 1 5);do if "$B" version;then exit;fi;sleep 0.1;done;exit 2)
echo "status=$? tries=$(cat count)"

#### retry loop gives up with exit 2
mkdir -p bin
printf '#!/bin/sh\nexit 1\n' > bin/server
chmod +x bin/server
(B="$PWD/bin/server"; for i in $(seq 1 3);do if "$B" version;then exit;fi;sleep 0.05;done;exit 2)
echo "status=$?"

#### function with arguments and return status
check_file() {
  local f="$1"
  if [ ! -f "$f" ]; then
    echo "missing: $f"
    return 1
  fi
  echo "ok: $f"
}
touch README.md
check_file README.md
check_file LICENSE
echo "status=$?"

#### function counts matches across files
mkdir -p src
printf 'TODO one\nfine\nTODO two\n' > src/a.rs
printf 'fine\n' > src/b.rs
count_todos() {
  local total=0 n
  for f in "$@"; do
    n=$(grep -c TODO "$f" || true)
    total=$((total + n))
  done
  echo "$total"
}
echo "todos=$(count_todos src/*.rs)"

#### while IFS= read -r keeps whitespace and backslashes
printf '  indented line\nback\\slash\ntrailing  \n' > in.txt
while IFS= read -r line; do
  printf '[%s]\n' "$line"
done < in.txt

#### while read -r from a pipe with a counter
printf 'src/a.ts\nsrc/b.ts\nREADME.md\n' > files.txt
n=0
grep '\.ts$' files.txt | while IFS= read -r f; do
  n=$((n+1))
  echo "$n $f"
done
echo "after pipe n=$n"

#### while read from process substitution keeps the counter
printf 'src/a.ts\nsrc/b.ts\nREADME.md\n' > files.txt
n=0
while IFS= read -r f; do
  n=$((n+1))
done < <(grep '\.ts$' files.txt)
echo "n=$n"

#### while read splits fields with IFS
printf 'alice:admin:1\nbob:dev:2\n' > users.txt
while IFS=: read -r name role id; do
  echo "$id $name ($role)"
done < users.txt

#### while read handles a last line without newline
printf 'one\ntwo' > nonl.txt
while IFS= read -r line || [ -n "$line" ]; do
  echo "got $line"
done < nonl.txt

#### if elif else chain on file kinds
mkdir -p proj
touch proj/Cargo.toml
detect() {
  if [ -f "$1/package.json" ]; then echo node
  elif [ -f "$1/Cargo.toml" ]; then echo rust
  elif [ -f "$1/pyproject.toml" ]; then echo python
  else echo unknown
  fi
}
detect proj
detect .

#### if elif on exit status of grep
printf 'error: boom\nwarning: meh\n' > build.log
if grep -q '^error' build.log; then
  echo "build failed"
elif grep -q '^warning' build.log; then
  echo "build has warnings"
else
  echo "build clean"
fi

#### case on file extension in a loop
mkdir -p src
touch src/main.rs src/app.ts src/util.py src/notes.txt
for f in src/*; do
  case "$f" in
    *.rs) kind=rust ;;
    *.ts|*.tsx) kind=typescript ;;
    *.py) kind=python ;;
    *) kind=skip ;;
  esac
  echo "$kind ${f#src/}"
done

#### case on a subcommand argument
run() {
  case "$1" in
    build) echo "building" ;;
    test|check) echo "testing" ;;
    -h|--help) echo "usage: run build|test" ;;
    *) echo "unknown: $1" >&2; return 2 ;;
  esac
}
run build; run check; run --help
run deploy 2>&1
echo "status=$?"

#### indexed array of files
files=(src/a.c src/b.c "docs/my notes.md")
echo "count=${#files[@]}"
for f in "${files[@]}"; do echo "- $f"; done
echo "first=${files[0]} last=${files[-1]}"

#### array built in a loop then joined
mkdir -p tests
touch tests/a_test.sh tests/b_test.sh tests/helper.sh
failed=()
for t in tests/*_test.sh; do
  failed+=("$(basename "$t" .sh)")
done
echo "failed: ${failed[*]}"
(IFS=,; echo "csv: ${failed[*]}")

#### empty array is fine under set -u
set -u
args=()
echo "n=${#args[@]}"
echo "args: ${args[@]+"${args[@]}"}"

#### mapfile reads lines into an array
printf 'lib/x.js\nlib/y.js\nlib/z.js\n' > list.txt
mapfile -t paths < list.txt
echo "${#paths[@]} ${paths[1]}"

#### associative array counts extensions
mkdir -p src
touch src/a.ts src/b.ts src/c.js src/d.md
declare -A count
for f in src/*; do
  ext="${f##*.}"
  count[$ext]=$(( ${count[$ext]:-0} + 1 ))
done
for k in js md ts; do echo "$k=${count[$k]}"; done

#### trap cleanup on EXIT removes a temp dir
cleanup() { rm -rf work.tmp; echo "cleaned"; }
(
  trap cleanup EXIT
  mkdir work.tmp
  echo data > work.tmp/f
  echo "working"
)
[ -d work.tmp ] && echo "still there" || echo "gone"

#### trap EXIT runs after a set -e failure
(
  set -e
  trap 'echo "cleanup ran"' EXIT
  echo "step 1"
  false
  echo "step 2"
)
echo "status=$?"

#### trap EXIT sees the exit status
(
  trap 'rc=$?; echo "exiting with $rc"' EXIT
  exit 3
)
echo "status=$?"

#### counters with arithmetic
pass=0 fail=0
for r in ok ok fail ok fail; do
  if [ "$r" = ok ]; then pass=$((pass + 1)); else fail=$((fail + 1)); fi
done
echo "pass=$pass fail=$fail total=$((pass + fail))"

#### arithmetic increment under set -e
set -e
i=0
((i++)) || true
i=$((i + 1))
echo "i=$i"

#### ((i++)) from zero trips set -e
(
  set -e
  i=0
  ((i++))
  echo "not reached"
)
echo "status=$?"

#### here-doc config written then checked
cat > config.toml <<'EOF'
[server]
port = 8080
host = "127.0.0.1"
[log]
level = "debug"
EOF
grep -n '^port' config.toml
grep -q 'level = "debug"' config.toml && echo "debug enabled"
wc -l < config.toml

#### here-doc env file sourced by a script
cat > .env <<'EOF'
APP_NAME=demo
APP_PORT=3000
EOF
set -a
. ./.env
set +a
echo "$APP_NAME on $APP_PORT"

#### unquoted here-doc expands variables into a config
name=svc port=9000
cat > app.conf <<EOF
name=$name
port=$port
url=http://localhost:$port/
EOF
cat app.conf

#### guard with or-brace exits
mkdir -p src
(
  [ -d src ] || { echo "no src dir"; exit 1; }
  [ -f src/main.c ] || { echo "error: src/main.c missing" >&2; exit 1; }
  echo "not reached"
) 2>&1
echo "status=$?"

#### cd guard before work
(
  cd missing-dir 2>/dev/null || { echo "cannot cd"; exit 1; }
  echo "in dir"
)
echo "status=$?"

#### command -v checks for tools
mkdir -p bin
printf '#!/bin/sh\necho stub\n' > bin/rg-stub
chmod +x bin/rg-stub
export PATH="$PWD/bin:$PATH"
command -v rg-stub >/dev/null || echo "rg-stub missing"
command -v no-such-tool >/dev/null || echo "no-such-tool missing"
command -v sh >/dev/null && echo "sh found"

#### command -v picks a fallback tool
if command -v no-such-grep >/dev/null 2>&1; then
  G=no-such-grep
else
  G=grep
fi
printf 'alpha\nbeta\n' > f.txt
"$G" -n beta f.txt

#### early exit 1 when a check fails
printf 'version = "1.0"\n' > Cargo.toml
(
  want=2.0
  have=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml)
  if [ "$have" != "$want" ]; then
    echo "version mismatch: have $have want $want"
    exit 1
  fi
  echo "versions match"
)
echo "status=$?"

#### set -e stops a script mid-way
(
  set -e
  echo "build"
  ls missing-file >/dev/null 2>&1
  echo "deploy"
)
echo "status=$?"

#### set -u fails on an unset variable
(
  set -u
  echo "start"
  echo "target=$TARGET_DIR"
  echo "not reached"
) 2>/dev/null
echo "status=$?"

#### set -u with a default is fine
set -u
echo "out=${OUT_DIR:-dist}"
: "${MODE:=release}"
echo "mode=$MODE"

#### pipefail reports a failing producer
set -o pipefail
cat missing.txt 2>/dev/null | sort | head -n 1
echo "status=$?"
set +o pipefail
cat missing.txt 2>/dev/null | sort | head -n 1
echo "status=$?"

#### set -euo pipefail aborts on a failing pipeline
(
  set -euo pipefail
  printf 'a\nb\n' > in.txt
  grep zzz in.txt | wc -l
  echo "not reached"
)
echo "status=$?"

#### set -e ignores failures in conditions and || chains
set -e
if grep -q zzz /dev/null; then echo found; else echo "not found"; fi
grep -q zzz /dev/null || echo "fallback"
! false
echo "still running"

#### set -e does not apply inside a function called from if
f() { false; echo "after false in f"; }
set -e
if f; then echo "f ok"; fi

#### status captured with || rc=$?
rc=0
grep -q missing /dev/null || rc=$?
echo "rc=$rc"

#### functions as pipeline steps in a script
set -euo pipefail
mkdir -p logs
printf 'INFO a\nERROR b\nINFO c\nERROR d\n' > logs/app.log
errors() { grep '^ERROR' "$1"; }
fmt() { while IFS= read -r l; do echo "!! ${l#ERROR }"; done; }
errors logs/app.log | fmt
echo "n=$(errors logs/app.log | wc -l | tr -d ' ')"

#### test runner script with summary
mkdir -p tests
printf 'exit 0\n' > tests/t1.sh
printf 'exit 1\n' > tests/t2.sh
printf 'exit 0\n' > tests/t3.sh
pass=0; fail=0; failed=()
for t in tests/*.sh; do
  if sh "$t" >/dev/null 2>&1; then
    pass=$((pass+1))
  else
    fail=$((fail+1)); failed+=("$t")
  fi
done
echo "pass=$pass fail=$fail"
[ "$fail" -eq 0 ] || { echo "failed: ${failed[*]}"; exit 1; }

#### script written to a file then run
cat > check.sh <<'EOF'
set -euo pipefail
dir="${1:?usage: check.sh DIR}"
[ -d "$dir" ] || { echo "no such dir: $dir"; exit 1; }
count=$(find "$dir" -type f -name '*.md' | wc -l | tr -d ' ')
echo "$dir has $count markdown files"
EOF
mkdir -p docs && touch docs/a.md docs/b.md docs/c.txt
bash check.sh docs
bash check.sh nope
echo "status=$?"
bash check.sh 2>&1
echo "status=$?"

#### python stub with heredoc then grep result
mkdir -p bin src
printf '#!/bin/sh\nexec sh "$@"\n' > bin/python3
chmod +x bin/python3
export PATH="$PWD/bin:$PATH"
printf 'const a = 1;\nconst debug = true;\n' > src/app.js
python3 - <<'EOF'
sed 's/debug = true/debug = false/' src/app.js > src/app.js.new
mv src/app.js.new src/app.js
EOF
grep -n "debug" src/app.js

#### loop with break on first match
mkdir -p a b c
touch b/target.cfg c/target.cfg
found=""
for d in a b c; do
  if [ -f "$d/target.cfg" ]; then found=$d; break; fi
done
echo "found in ${found:-none}"

#### loop with continue skipping files
mkdir -p src
printf 'x\n' > src/a.c
: > src/empty.c
printf 'y\n' > src/b.c
for f in src/*.c; do
  [ -s "$f" ] || { echo "skip empty $f"; continue; }
  echo "process $f"
done

#### nested loops with a status flag
mkdir -p pkg1 pkg2
printf 'name=one\n' > pkg1/meta
printf 'nam=two\n' > pkg2/meta
ok=1
for p in pkg1 pkg2; do
  for key in name; do
    grep -q "^$key=" "$p/meta" || { echo "$p: missing $key"; ok=0; }
  done
done
[ "$ok" -eq 1 ] && echo "all good" || echo "problems found"

#### until loop waits for a file
n=0
until [ -f ready.flag ]; do
  n=$((n+1))
  [ $n -eq 3 ] && touch ready.flag
done
echo "ready after $n"

#### local variables do not leak
x=global
f() { local x=local; echo "in f: $x"; }
f
echo "after f: $x"

#### function output captured and status checked
get_version() {
  sed -n 's/^version: //p' "$1"
  [ -s "$1" ]
}
printf 'name: app\nversion: 0.4.1\n' > meta.yml
if v=$(get_version meta.yml); then echo "version=$v"; fi
if ! v=$(get_version missing.yml 2>/dev/null); then echo "no version"; fi

#### exit status of the last command ends the script
(
  echo "checking"
  [ -f nothere ]
)
echo "status=$?"
