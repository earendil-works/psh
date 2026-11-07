# Agent shape: searching a project. `rg -n "pat" path...` (with -S, -g/--glob, -C,
# -l, `| head -n N`) is the most frequent search shape, then `grep -n "a\|b" file`
# after an edit (often behind `cd dir && python3 - <<'EOF'`), `grep -rn --include`,
# `find DIR -maxdepth N -type f | sort`, `find ... -print0 | xargs -0` and
# `xargs -n1 / -I{}`. rg, grep -r and find visit files in an unspecified order, so
# every case whose output spans several files sorts it or has a single matching file.

#### rg -n in one file
mkdir -p src
printf 'fn main() {\n    let cfg = load_config();\n    run(cfg);\n}\n' > src/main.rs
rg -n "load_config" src/main.rs

#### rg -n in a directory with one matching file
mkdir -p src lib
printf 'pub fn parse_args() {}\npub fn helper() {}\n' > src/cli.rs
printf 'pub fn other() {}\n' > lib/util.rs
rg -n "parse_args" src

#### rg -n over two paths, sorted
mkdir -p src tests
printf 'fn render() {}\nfn render_all() {}\n' > src/view.rs
printf 'fn test_render() { render(); }\n' > tests/view_test.rs
rg -n "render" src tests | sort

#### rg pattern before -n
mkdir -p src
printf 'const MAX_RETRIES = 3;\nfunction retry() {}\n' > src/net.js
rg "MAX_RETRIES" -n src

#### rg -n after the path
mkdir -p src
printf 'export function fetchUser() {}\nexport function fetchTeam() {}\n' > src/api.ts
rg "fetchTeam" src -n

#### rg -n with single quotes
mkdir -p docs
printf 'Install with the setup script.\nThen run the server.\n' > docs/install.md
rg -n 'server' docs

#### rg -n with three paths, sorted
mkdir -p src lib tests
printf 'TODO: split this\n' > src/a.py
printf 'ok\n' > lib/b.py
printf 'TODO: add cases\n' > tests/c.py
rg -n "TODO" src lib tests | sort

#### rg -n -S smart case lowercase matches all cases
mkdir -p src
printf 'Config\nconfig\nCONFIG\n' > src/names.txt
rg -n "config" src -S

#### rg -n -S smart case uppercase is case sensitive
mkdir -p src
printf 'Config\nconfig\nCONFIG\n' > src/names.txt
rg -n "Config" src -S

#### rg -i ignores case
printf 'Error: disk\nerror: net\nok\n' > app.log
rg -i "ERROR" app.log

#### rg -n with an escaped paren
mkdir -p src
printf 'fn build(x: u32) {}\nlet b = build(3);\nlet build_dir = 1;\n' > src/lib.rs
rg -n "build\\(" src

#### rg -n alternation with an escaped paren
mkdir -p src
printf 'init();\ninit_done = true;\nshutdown();\nshutdown_hook = 0;\n' > src/main.c
rg -n "init\\(|shutdown_hook" src

#### rg -n with an escaped dot
mkdir -p src
printf 'self.config\nselfXconfig\n' > src/obj.py
rg -n "self\\.config" src

#### rg -n into head -n
mkdir -p src
for i in 1 2 3 4 5 6 7 8; do echo "log_line $i"; done > src/out.txt
rg -n "log_line" src | head -n 3

#### rg -n into head -N
mkdir -p src
seq 1 50 | sed 's/^/item /' > src/items.txt
rg -n "item" src/items.txt | head -5

#### rg -n -g with a glob
mkdir -p src
printf 'import os\n' > src/a.py
printf 'import os from "x";\n' > src/b.js
rg -n "import" src -g '*.py'

#### rg -n -g glued to the glob
mkdir -p src tests
printf 'const port = 8080;\n' > src/server.ts
printf 'port = 8080\n' > src/server.toml
printf 'const port = 1;\n' > tests/server.test.ts
rg -n "port" src tests -g'*.toml'

#### rg -n --glob
mkdir -p src lib
printf 'type User = {}\n' > src/user.ts
printf 'User docs\n' > src/user.md
printf 'class User {}\n' > lib/user.ts
rg -n "User" src lib --glob '*.ts' | sort

#### rg --glob negated
mkdir -p src
printf 'describe("x")\n' > src/a.test.js
printf 'describe = 1\n' > src/a.js
rg -n "describe" src --glob '!*.test.js'

#### rg -n on dot
mkdir -p pkg
printf 'version = "1.2.0"\n' > pkg/meta.toml
printf 'nothing here\n' > notes.txt
rg -n "version" .

#### rg -n -S on dot
mkdir -p src
printf 'let Timeout = 5;\n' > src/t.rs
rg -n "timeout" -S .

#### rg -l lists files
mkdir -p src
printf 'use serde;\n' > src/a.rs
printf 'use std;\n' > src/b.rs
printf 'use serde_json;\n' > src/c.rs
rg -l "serde" src | sort

#### rg -c counts per file
mkdir -p src
printf 'x\nx\ny\n' > src/one.txt
printf 'x\n' > src/two.txt
rg -c "x" src | sort

#### rg -o prints only the match
printf 'id=17 name=a\nid=42 name=b\n' > rows.txt
rg -o "id=[0-9]+" rows.txt

#### rg -C context
seq 1 12 | sed 's/^/line /' > f.txt
rg -n -C 1 "line (4|9)$" f.txt

#### rg -A after context
printf 'fn a() {\n    one\n    two\n}\nfn b() {\n}\n' > m.rs
rg -n -A 2 "fn a" m.rs

#### rg -F fixed string
printf 'a.b(c)\naxb(c)\n' > s.txt
rg -n -F "a.b(c)" s.txt

#### rg -w word match
printf 'id\nuser_id\nid_map\nthe id here\n' > w.txt
rg -n -w "id" w.txt

#### rg -v inverts
printf 'keep\ndrop me\nkeep too\n' > v.txt
rg -v "drop" v.txt

#### rg --files sorted
mkdir -p src/sub docs
touch src/a.rs src/sub/b.rs docs/guide.md
rg --files | sort

#### rg --files with a glob
mkdir -p src docs
touch src/a.rs src/b.rs docs/guide.md
rg --files -g '*.md'

#### rg --type
mkdir -p src
printf 'fn go() {}\n' > src/a.rs
printf 'def go(): pass\n' > src/a.py
rg -n "go" --type py src

#### rg respects gitignore in a repo
export GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com GIT_AUTHOR_DATE=2024-01-01T00:00:00Z GIT_COMMITTER_DATE=2024-01-01T00:00:00Z GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
git init -q -b main
mkdir -p src target
printf 'target/\n' > .gitignore
printf 'needle\n' > src/a.txt
printf 'needle\n' > target/b.txt
rg -n "needle" .

#### rg skips hidden files unless --hidden
mkdir -p .config
printf 'token_name\n' > .config/app.ini
printf 'token_name\n' > visible.ini
echo "default:"
rg -l "token_name" . | sort
echo "hidden:"
rg -l --hidden "token_name" . | sort

#### rg no match status
printf 'alpha\n' > a.txt
rg -n "zeta" a.txt
echo "status=$?"

#### rg missing path status
rg -n "x" nope.txt 2>/dev/null
echo "status=$?"

#### rg from stdin
printf 'one\ntwo\nthree\n' | rg -n "t"

#### cd then rg into head
mkdir -p proj/src
printf 'warn!("a");\ninfo!("b");\nwarn!("c");\n' > proj/src/log.rs
cd proj && rg -n "warn" src | head

#### python edit then grep -n
mkdir -p proj bin
printf '#!/bin/sh\ncat > /dev/null\necho "patched"\n' > bin/python3 && chmod +x bin/python3
export PATH="$PWD/bin:$PATH"
printf 'DEBUG = False\nPORT = 8000\n' > proj/settings.py
cd proj && python3 - <<'EOF'
from pathlib import Path
p = Path("settings.py")
p.write_text(p.read_text().replace("8000", "9000"))
EOF
grep -n "PORT" settings.py

#### python edit then grep -n alternation
mkdir -p proj bin
printf '#!/bin/sh\nexec sh -s\n' > bin/python3 && chmod +x bin/python3
export PATH="$PWD/bin:$PATH"
printf 'host = a\nport = 1\nuser = x\n' > proj/app.conf
cd proj && python3 - <<'EOF'
sed 's/port = 1/port = 2/' app.conf > app.tmp && mv app.tmp app.conf
EOF
grep -n "host\|port" app.conf

#### grep -n three-way alternation into head
mkdir -p proj
printf 'import a\nfrom b import c\nx = 1\nexport D\n' > proj/mod.py
cd proj && grep -n "import\|export\|from" mod.py | head -20

#### grep -n anchored
printf 'def a():\n    def inner():\n\ndef b():\n' > m.py
grep -n "^def" m.py

#### node test into grep -E into head
mkdir -p bin
printf '#!/bin/sh\necho "TAP version 13"\necho "ok 1 - adds"\necho "not ok 2 - subtracts"\necho "# pass 1"\necho "# fail 1"\nexit 1\n' > bin/node && chmod +x bin/node
export PATH="$PWD/bin:$PATH"
node --test tests/math.test.js 2>&1 | grep -E "^(ok|not ok)" | head

#### grep -rn with --include, sorted
mkdir -p src/a src/b
printf 'TODO one\n' > src/a/x.js
printf 'TODO two\n' > src/b/y.ts
printf 'TODO three\n' > src/b/z.md
grep -rn "TODO" src --include='*.js' --include='*.ts' | sort

#### grep -rn on dot, sorted
mkdir -p lib
printf 'panic!("x")\n' > lib/a.rs
printf 'ok\npanic!("y")\n' > b.rs
grep -rn "panic" . | sort

#### grep -nE extended regex
printf 'v1.2.3\nv10.0\nversion\n' > tags.txt
grep -nE "^v[0-9]+\.[0-9]+(\.[0-9]+)?$" tags.txt

#### grep -v filters out lines
printf 'keep\n  \nskip this\nkeep 2\n' > in.txt
grep -v "skip" in.txt

#### grep -c counts
printf 'ERROR a\nINFO b\nERROR c\n' > app.log
grep -c "ERROR" app.log

#### grep -c with no match prints 0 and fails
printf 'INFO\n' > app.log
grep -c "ERROR" app.log
echo "status=$?"

#### grep -l lists files
printf 'needle\n' > a.txt
printf 'hay\n' > b.txt
printf 'needle\n' > c.txt
grep -l "needle" a.txt b.txt c.txt

#### grep -o only matching
printf 'see http://a.example/x and http://b.example/y\n' > links.txt
grep -oE "http://[a-z.]+/[a-z]+" links.txt

#### grep -A after context
printf 'fn one() {\n  a\n  b\n}\nfn two() {\n  c\n}\n' > code.rs
grep -n -A 2 "fn one" code.rs

#### grep -B and -C context
seq 1 10 | sed 's/^/row /' > rows.txt
grep -B 1 "row 3$" rows.txt
echo "--- C"
grep -C 1 "row 8$" rows.txt

#### grep -e twice
printf 'apple\nbanana\ncherry\n' > fruit.txt
grep -n -e "apple" -e "cherry" fruit.txt

#### grep -i -w
printf 'Error\nerrors\nERROR here\n' > e.txt
grep -i -w "error" e.txt

#### grep -q in a condition
printf 'feature = true\n' > cfg.toml
if grep -q "feature = true" cfg.toml; then echo "enabled"; else echo "disabled"; fi

#### grep missing file status
grep -n "x" missing.txt 2>/dev/null
echo "status=$?"

#### find -maxdepth -type f sorted
mkdir -p src/a/b/c docs
touch README.md src/main.rs src/a/mod.rs src/a/b/deep.rs src/a/b/c/deeper.rs docs/x.md
find . -maxdepth 3 -type f | sort

#### find -maxdepth -type d sorted
mkdir -p src/a/b tests/unit docs
find . -maxdepth 2 -type d | sort

#### find -name
mkdir -p src/x tests
touch src/x/a.test.js src/b.js tests/c.test.js
find . -name '*.test.js' | sort

#### find -name or -name
mkdir -p cfg
touch cfg/a.json cfg/b.yaml cfg/c.toml
find cfg \( -name '*.json' -o -name '*.yaml' \) | sort

#### find -path with -not
mkdir -p src node_modules/dep/lib
touch src/a.js node_modules/dep/lib/b.js
find . -name '*.js' -not -path './node_modules/*' | sort

#### find -prune
mkdir -p src .git/objects build
touch src/a.c .git/objects/x build/a.o README
find . -name .git -prune -o -type f -print | sort

#### find -print0 into xargs -0
mkdir -p 'my docs'
printf 'alpha\n' > 'my docs/one file.txt'
printf 'beta\n' > 'my docs/two.txt'
find 'my docs' -name '*.txt' -print0 | sort -z | xargs -0 cat

#### find -exec with \;
mkdir -p logs
printf 'a\nb\n' > logs/1.log
printf 'c\n' > logs/2.log
find logs -name '*.log' -exec grep -H . {} \; | sort

#### find -exec with +
mkdir -p src
printf 'TODO x\n' > src/a.py
printf 'done\n' > src/b.py
find src -name '*.py' -exec grep -l TODO {} +

#### find -type f into wc -l
mkdir -p a/b
touch a/1 a/2 a/b/3
find a -type f | wc -l

#### find a missing directory
find nope -type f 2>/dev/null
echo "status=$?"

#### xargs -n1
printf 'a b c\n' | xargs -n1 echo item

#### xargs -I{}
printf 'one\ntwo\n' | xargs -I{} echo "file-{}.txt"

#### xargs -0 with spaces
printf 'a b\0c\0' | xargs -0 -n1 echo "arg:"

#### grep -l into xargs sed
mkdir -p src
printf 'old_name()\n' > src/a.js
printf 'other()\n' > src/b.js
grep -l "old_name" src/*.js | xargs sed -n 's/old_name/new_name/p'

#### find into xargs grep -n, sorted
mkdir -p src/sub
printf 'unwrap()\n' > src/a.rs
printf 'x\nunwrap()\n' > src/sub/b.rs
find src -type f -name '*.rs' | xargs grep -n "unwrap" | sort

#### xargs -r with empty input
printf '' | xargs -r echo "should not print"
echo "status=$?"
