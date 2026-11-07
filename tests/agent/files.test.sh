# Agent shape: file operations. The most frequent corpus shapes here are `ls` (with a
# directory, `-1`, `-a`, piped into `head`), `cat > file <<'EOF'` followed by running the
# file (`node f; rm f`, `node f; v=$?; rm -f f; exit $v`), `rm`/`rm -f`/`rm -rf`,
# `mv a b`, `cp a b`, `mkdir -p`, `chmod +x`, `printf ... > file`, and the listing chain
# `find . -maxdepth 2 -type f | sed 's#^./##' | sort | head`. Also: tee/tee -a, ln -s,
# test -f/-d/-x, `[ -e x ] || echo missing`, sed -i edits, cmp and diff -u.
# `ls -la` (the top shape) is left out: owners, sizes and times are not deterministic.

#### ls lists a directory
mkdir -p src tests
touch src/main.js src/util.js tests/main.test.js README.md
ls
echo ---
ls src

#### ls dir/ with a trailing slash
mkdir -p lib/core
touch lib/index.js lib/core/a.js
ls lib/

#### ls -1 one entry per line
mkdir docs
touch docs/intro.md docs/setup.md docs/api.md
ls -1 docs

#### ls -a shows dotfiles
mkdir app
touch app/.env app/.gitignore app/package.json
ls -a app

#### ls of a missing path with 2>/dev/null
mkdir -p build
ls build/ 2>/dev/null
echo "status=$?"
ls dist/ 2>/dev/null
echo "status=$?"

#### ls of a missing path reports an error
# dialect: gnu
ls nothing-here
echo "status=$?"

#### ls piped into head
mkdir logs
for i in 01 02 03 04 05 06 07 08 09 10 11 12; do touch "logs/run-$i.log"; done
ls logs | head

#### ls -d with a glob
mkdir -p pkg-a pkg-b other
touch pkg-file
ls -d pkg-*/

#### ls a file argument and a directory
mkdir conf
touch conf/app.toml Cargo.toml
ls Cargo.toml conf

#### ls a directory under HOME
mkdir -p ~/.config/tool
touch ~/.config/tool/settings.json ~/.config/tool/keys.json
ls ~/.config/tool/

#### find maxdepth listing sorted
mkdir -p src/components tests/unit docs
touch README.md package.json src/index.ts src/components/Button.tsx tests/unit/a.test.ts docs/guide.md
find . -maxdepth 2 -type f | sed 's#^./##' | sort | head -200

#### mkdir -p nested and idempotent
mkdir -p src/server/routes
mkdir -p src/server/routes
echo "status=$?"
find src -type d | sort

#### mkdir without -p on an existing directory fails
mkdir out
mkdir out 2>/dev/null
echo "status=$?"
mkdir a/b/c 2>/dev/null
echo "status=$?"

#### cat here-doc into a file then run it with node
mkdir -p bin scratch
printf '#!/bin/sh\necho "node $*"\nwhile IFS= read -r l; do echo "  | $l"; done < "$1"\n' > bin/node
chmod +x bin/node
export PATH="$PWD/bin:$PATH"
cd scratch && cat > check.mjs <<'EOF'
import { readFileSync } from "node:fs";
const pkg = JSON.parse(readFileSync("package.json", "utf8"));
console.log(pkg.name);
EOF
node check.mjs

#### run a script then remove it
mkdir -p bin
printf '#!/bin/sh\necho "ran $1"\n' > bin/node
chmod +x bin/node
export PATH="$PWD/bin:$PATH"
cat > probe.js <<'EOF'
console.log(process.version);
EOF
node probe.js; rm probe.js
ls probe.js 2>/dev/null || echo "probe.js removed"

#### run, keep the status, clean up, exit with it
mkdir -p bin
printf '#!/bin/sh\necho "test failed"\nexit 3\n' > bin/node
chmod +x bin/node
export PATH="$PWD/bin:$PATH"
cat > t.mjs <<'EOF'
throw new Error("boom");
EOF
node t.mjs; code=$?; rm -f t.mjs; exit $code

#### run and remove only on success
mkdir -p bin
printf '#!/bin/sh\nexit 1\n' > bin/node
chmod +x bin/node
export PATH="$PWD/bin:$PATH"
printf 'process.exit(1)\n' > fail.js
node fail.js && rm fail.js
echo "status=$?"
ls

#### two here-doc files in one command
mkdir proj
cd proj && cat > a.txt <<'EOF'
alpha
  beta
EOF
cat > b.txt <<'EOF'
gamma $HOME
EOF
cat a.txt b.txt

#### here-doc message passed to a tool via $(cat)
mkdir -p bin
printf '#!/bin/sh\nprintf "argc=%%s\\n" "$#"\nprintf "[%%s]\\n" "$1"\n' > bin/tool
chmod +x bin/tool
cat > msg.txt <<'EOF'
fix: handle empty input

Closes the "edge" case.
EOF
cd bin && ./tool "$(cat ../msg.txt)"

#### python3 stdin script then write a file
mkdir -p bin work
printf '#!/bin/sh\necho "python3 got $(wc -l | tr -d " ") lines"\n' > bin/python3
chmod +x bin/python3
export PATH="$PWD/bin:$PATH"
cd work && python3 - <<'EOF'
import json
print(json.dumps({"ok": True}))
EOF
cat > out.json <<'EOF'
{"ok": true}
EOF
cat out.json

#### cp a file
printf 'port = 8080\n' > config.example.toml
cp config.example.toml config.toml
cat config.toml
cp config.toml 2>/dev/null || echo "cp needs a destination"

#### cp into a directory
mkdir backup
printf 'one\n' > notes.txt
cp notes.txt backup/
ls backup

#### cp -r copies a tree
mkdir -p template/src template/tests
printf 'main\n' > template/src/main.txt
printf 'test\n' > template/tests/t.txt
cp -r template app
find app -type f | sort
cat app/src/main.txt

#### cp -R into an existing directory nests it
mkdir -p assets/img dest
touch assets/img/logo.svg
cp -R assets dest
find dest | sort

#### cp -n does not overwrite
printf 'original\n' > keep.txt
printf 'new\n' > incoming.txt
cp -n incoming.txt keep.txt 2>/dev/null
cat keep.txt

#### mv renames a file
printf 'draft\n' > draft.md
mv draft.md final.md
ls
cat final.md

#### mv into a directory and a missing source
mkdir archive
touch old.log
mv old.log archive/
ls archive
mv nope.log archive/ 2>/dev/null
echo "status=$?"

#### mv over an existing file replaces it
printf 'v1\n' > app.conf
printf 'v2\n' > app.conf.new
mv app.conf.new app.conf
cat app.conf
ls

#### rm a file and a missing file
touch a.tmp
rm a.tmp
echo "status=$?"
rm a.tmp 2>/dev/null
echo "status=$?"

#### rm -f is quiet and succeeds on missing files
touch one.tmp two.tmp
rm -f one.tmp two.tmp three.tmp
echo "status=$?"
ls

#### rm -rf a tree and rm without -r on a directory
mkdir -p node_modules/pkg/lib dist
touch node_modules/pkg/lib/index.js
rm dist 2>/dev/null
echo "status=$?"
rm -rf node_modules dist
echo "status=$?"
ls

#### touch creates and keeps content
printf 'keep me\n' > existing.txt
touch existing.txt new.txt
cat existing.txt
test -s new.txt || echo "new.txt is empty"

#### chmod +x then run
cat > run.sh <<'EOF'
echo "running with $# args: $*"
EOF
./run.sh a b 2>/dev/null
echo "status=$?"
chmod +x run.sh
./run.sh a b
echo "status=$?"

#### chmod +x on a script with a shebang
printf '#!/bin/sh\necho "build ok"\n' > build.sh
chmod +x build.sh && ./build.sh
test -x build.sh && echo executable

#### ln -s and read through the link
mkdir -p config
printf 'debug = true\n' > config/dev.toml
ln -s config/dev.toml current.toml
cat current.toml
readlink current.toml
test -L current.toml && echo "is a link"

#### ln -sf replaces a link
printf 'a\n' > a.txt
printf 'b\n' > b.txt
ln -s a.txt cur
ln -sf b.txt cur
cat cur

#### dangling symlink
ln -s missing-target dangling
test -e dangling || echo "target missing"
test -L dangling && echo "link exists"

#### test -f -d -x -s
mkdir dir
touch empty
printf 'x\n' > full
printf '#!/bin/sh\n' > tool && chmod +x tool
for p in dir empty full tool none; do
  r=""
  test -f "$p" && r="$r f"
  test -d "$p" && r="$r d"
  test -x "$p" && r="$r x"
  test -s "$p" && r="$r s"
  echo "$p:$r"
done

#### [ -e x ] || echo missing
touch present.txt
[ -e present.txt ] || echo "present.txt missing"
[ -e absent.txt ] || echo "absent.txt missing"
[ -d src ] && echo "has src" || echo "no src"

#### printf > and >>
printf '%s\n' '// generated' > header.ts
printf '%s\n' 'export const x = 1;' >> header.ts
printf 'export const y = %d;\n' 2 >> header.ts
cat header.ts

#### tee writes a file and stdout
printf 'line1\nline2\n' | tee out.txt
echo ---
cat out.txt

#### tee -a appends
echo "first" > build.log
echo "second" | tee -a build.log > /dev/null
echo "third" | tee -a build.log
echo ---
cat build.log

#### sed -i in place
# dialect: gnu
printf 'version = "1.0.0"\nname = "demo"\n' > Cargo.toml
sed -i 's/1\.0\.0/1.1.0/' Cargo.toml
cat Cargo.toml

#### sed -i with several expressions and a line delete
# dialect: gnu
printf 'a\nTODO remove\nb\nc\n' > list.txt
sed -i -e '/^TODO/d' -e 's/^b$/B/' list.txt
cat list.txt

#### sed -i.bak keeps a backup
printf 'host=localhost\nport=80\n' > app.env
sed -i.bak 's/port=80/port=8080/' app.env
cat app.env
cat app.env.bak

#### sed -n prints a line range
printf 'l1\nl2\nl3\nl4\nl5\nl6\n' > file.txt
sed -n '2,4p' file.txt

#### cmp -s identical and different
printf 'same\n' > a
printf 'same\n' > b
printf 'diff\n' > c
cmp -s a b && echo "a b equal"
cmp -s a c || echo "a c differ"

#### cmp reports the first difference
# dialect: gnu
printf 'abcdef\n' > x
printf 'abXdef\n' > y
cmp x y
echo "status=$?"

#### diff -u with labels
printf 'one\ntwo\nthree\n' > old.txt
printf 'one\n2\nthree\nfour\n' > new.txt
diff -u --label a/file.txt --label b/file.txt old.txt new.txt
echo "status=$?"

#### diff -u without the header lines
printf 'alpha\nbeta\n' > before
printf 'alpha\ngamma\n' > after
diff -u before after | tail -n +3

#### diff -q and identical files
printf 'x\n' > p
printf 'x\n' > q
printf 'y\n' > r
diff p q && echo "p q same"
diff -q p r
echo "status=$?"

#### diff -rq between trees
mkdir -p v1/src v2/src
printf 'a\n' > v1/src/a.txt
printf 'a\n' > v2/src/a.txt
printf 'b\n' > v1/src/b.txt
printf 'B\n' > v2/src/b.txt
touch v2/src/new.txt
diff -rq v1 v2 | sort
