# Agent shapes: the most frequent corpus command shapes the topic files did not reach,
# written in exactly their form (tests/shapes/shapes.txt; coverage.py lists the next
# ones). Toolchain commands are local stubs in bin/.

#### cd into a package and run a repo script by its toplevel path
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p bin scripts packages/web
printf '#!/bin/sh\necho "node $(basename "$1") $2 $3"\n' > bin/node && chmod +x bin/node
export PATH="$PWD/bin:$PATH"
: > scripts/check.mjs
cd packages/web && node "$(git rev-parse --show-toplevel)/scripts/check.mjs" --run lint

#### cd into a package and run a script two levels up
mkdir -p bin tools packages/api
printf '#!/bin/sh\necho "node $1 $2 $3"\n' > bin/node && chmod +x bin/node
export PATH="$PWD/bin:$PATH"
cd packages/api && node ../../tools/build.mjs --run types
echo "status=$?"

#### cd and node --test one file
mkdir -p bin pkg/test
printf '#!/bin/sh\nshift\nfor f; do echo "ok - $f"; done\n' > bin/node && chmod +x bin/node
export PATH="$PWD/bin:$PATH"
cd pkg && node --test test/parse.test.js

#### cd and npx with --run
mkdir -p bin app
printf '#!/bin/sh\necho "npx $*"\n' > bin/npx && chmod +x bin/npx
export PATH="$PWD/bin:$PATH"
cd app && npx vitest ../../shared --run unit

#### npx tsc --noEmit
mkdir -p bin
printf '#!/bin/sh\necho "npx $*"; exit 0\n' > bin/npx && chmod +x bin/npx
export PATH="$PWD/bin:$PATH"
npx tsc --noEmit
echo "status=$?"

#### npm run build
mkdir -p bin
printf '#!/bin/sh\necho "> app@1.0.0 $2"\necho "built"\n' > bin/npm && chmod +x bin/npm
export PATH="$PWD/bin:$PATH"
npm run build

#### npm run two scripts with &&
mkdir -p bin
printf '#!/bin/sh\necho "> app@1.0.0 $2"\n' > bin/npm && chmod +x bin/npm
export PATH="$PWD/bin:$PATH"
npm run lint && npm run test

#### npm test with extra arguments
mkdir -p bin
printf '#!/bin/sh\necho "npm $*"\n' > bin/npm && chmod +x bin/npm
export PATH="$PWD/bin:$PATH"
npm test -- --run parser

#### npm test -- --run file
mkdir -p bin
printf '#!/bin/sh\necho "npm $*"\n' > bin/npm && chmod +x bin/npm
export PATH="$PWD/bin:$PATH"
npm test -- --run src/parse.test.ts

#### rg -n into head -100
mkdir -p src
printf 'fn item_a() {}\nlet x = 1;\nfn item_b() {}\n' > src/items.rs
rg -n "fn item" src | head -100

#### rg -n into head -200
mkdir -p src
for i in $(seq 1 250); do echo "let v$i = $i;"; done > src/vals.rs
rg -n "let v" src | head -200

#### rg -n into head -300
mkdir -p src
printf 'impl A {}\nimpl B {}\n' > src/impls.rs
rg -n "impl" src | head -300

#### rg -n into head -120
mkdir -p src
printf 'unsafe { a }\nsafe\n' > src/u.rs
rg -n "unsafe" src | head -120

#### rg -n into head -80
mkdir -p lib
for i in $(seq 1 90); do echo "TODO item $i"; done > lib/notes.txt
rg -n "TODO" lib | head -80

#### rg -n into head -30
printf 'error: a\nerror: b\nwarn: c\n' > build.log
rg -n "error" build.log | head -30

#### rg with a glob
mkdir -p src
printf 'fn main() {}\n' > src/main.rs
printf 'main = 1\n' > src/main.py
rg -n "main" src --glob '*.rs'

#### rg with two globs
mkdir -p src
printf 'fn run() {}\n' > src/a.rs
printf 'run = 1\n' > src/b.toml
printf 'run()\n' > src/c.py
rg -n "run" src --glob '*.rs' --glob '*.toml' | sort

#### rg with an excluding glob
mkdir -p src/vendor src/app
printf 'needle\n' > src/vendor/x.txt
printf 'needle\n' > src/app/y.txt
rg -n "needle" src --glob '!**/vendor/**'

#### rg for a call with an escaped paren
mkdir -p src
printf 'parse(x)\nparser = 1\nparse (y)\n' > src/p.js
rg -n "parse\(" src

#### rg with alternation and a paren
printf 'load(a)\nsave = 2\nstore(b)\n' > io.js
rg -n "load\(|save" io.js

#### rg for a dotted name
printf 'config.port = 1\nconfigXport = 2\n' > cfg.js
rg -n "config\.port" cfg.js

#### rg -C context
printf 'a\nb\nmatch\nc\nd\n' > f.txt
rg -n -C 1 match f.txt

#### cd and rg
mkdir -p proj/src
printf 'use std::io;\n' > proj/src/lib.rs
cd proj && rg -n "std::io" src

#### cd and rg into head
mkdir -p proj
for i in 1 2 3 4 5; do echo "warn $i"; done > proj/log.txt
cd proj && rg -n "warn" log.txt | head -3

#### cd and grep -n
mkdir -p svc
printf 'PORT=8080\nHOST=localhost\n' > svc/.env.example
cd svc && grep -n "PORT" .env.example

#### find -maxdepth -type into sort
mkdir -p a/b/c d
touch a/x a/b/y a/b/c/z d/w
find . -maxdepth 2 -type f | sort

#### git commit -m alone
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
echo hello > README.md
git add README.md
git commit -m "Add README"

#### git add then status --short
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
printf 'a\n' > a.txt
printf 'b\n' > b.txt
git add a.txt && git status --short

#### git add, commit, status
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
printf 'x\n' > x.txt
git add x.txt && git commit -m "Add x" && git status --short

#### git add, status, commit
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
printf 'y\n' > y.txt
git add y.txt && git status --short && git commit -m "Add y"

#### git diff --check then diff of a file
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
printf 'one\n' > f.txt
git add f.txt && git commit -qm init
printf 'one\ntwo\n' > f.txt
git diff --check && git diff -- f.txt

#### git diff --stat then diff of a file
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
printf 'a\nb\n' > g.txt
git add g.txt && git commit -qm init
printf 'a\nc\n' > g.txt
git diff --stat && git diff -- g.txt

#### git status, diff --stat and a file diff through sed -n
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
printf '1\n2\n3\n' > h.txt
git add h.txt && git commit -qm init
printf '1\n2\n4\n' > h.txt
git status --short && git diff --stat && git diff -- h.txt | sed -n '1,4p'

#### git diff with a unified width and a path
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
printf 'a\nb\nc\nd\ne\n' > u.txt
git add u.txt && git commit -qm init
printf 'a\nb\nX\nd\ne\n' > u.txt
git diff --unified=0 HEAD -- u.txt

#### cd and git diff a revision
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
mkdir repo && git -C repo init -q -b main
printf 'v1\n' > repo/v.txt
git -C repo add v.txt && git -C repo commit -qm one
printf 'v2\n' > repo/v.txt
cd repo && git diff HEAD

#### cd; sed -n twice
mkdir -p src
printf 'l1\nl2\nl3\nl4\nl5\n' > src/a.c
cd src; sed -n 1,2p a.c; sed -n 4,5p a.c

#### cd && four sed -n ranges
mkdir -p doc
for i in $(seq 1 20); do echo "line $i"; done > doc/guide.md
cd doc && sed -n 1,2p guide.md; sed -n 5,6p guide.md; sed -n 10,11p guide.md; sed -n 19,20p guide.md

#### cd && sed -n && sed -n
mkdir -p src
printf 'a\nb\nc\n' > src/x.go
printf 'd\ne\n' > src/y.go
cd src && sed -n 1,2p x.go && sed -n 2p y.go

#### three sed -n ranges without cd
for i in $(seq 1 12); do echo "row $i"; done > t.txt
sed -n 1,2p t.txt; sed -n 6,7p t.txt; sed -n 11,12p t.txt

#### cd and python3 here-doc, then echo
mkdir -p bin proj
printf '#!/bin/sh\ncat >/dev/null\necho "migrated 3 rows"\n' > bin/python3 && chmod +x bin/python3
export PATH="$PWD/bin:$PATH"
cd proj && python3 - <<'EOF'
import sqlite3
rows = [1, 2, 3]
print("migrated", len(rows), "rows")
EOF
echo done

#### uv run --with a package and a python here-doc
mkdir -p bin
printf '#!/bin/sh\nwhile [ "$1" != - ]; do shift; done\ncat\n' > bin/uv && chmod +x bin/uv
export PATH="$PWD/bin:$PATH"
uv run --with requests python - <<'PY'
import requests
print("ok")
PY

#### cd and uv run a tool quietly
mkdir -p bin svc
printf '#!/bin/sh\necho "uv $*"\n' > bin/uv && chmod +x bin/uv
export PATH="$PWD/bin:$PATH"
cd svc && uv run pytest -q

#### bash a script with a flag
printf 'for a; do echo "arg: $a"; done\n' > find-root.sh
bash find-root.sh tools --path-only

#### gh pr view with --json
mkdir -p bin
printf '#!/bin/sh\necho "{\\"number\\":7,\\"title\\":\\"Fix parser\\"}"\n' > bin/gh && chmod +x bin/gh
export PATH="$PWD/bin:$PATH"
gh pr view 7 --repo acme/app --json number,title | jq -r .title

#### node with a purpose and a timeout
mkdir -p bin
printf '#!/bin/sh\necho "node $1 purpose=$3 timeout=$5"\n' > bin/node && chmod +x bin/node
export PATH="$PWD/bin:$PATH"
node scripts/ask.mjs --purpose "summarize the log" --timeout 30

#### cd and run a local command
mkdir -p tool
printf '#!/bin/sh\necho "ran in tool"\n' > tool/run && chmod +x tool/run
cd tool && ./run

#### git push to a named remote
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q --bare -b main origin.git
git init -q -b main work && cd work
echo hi > hi.txt && git add hi.txt && git commit -qm hi
git remote add origin ../origin.git
git push -q origin main
git -C ../origin.git log --oneline
