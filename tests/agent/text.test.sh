# Agent shape: text processing in pipelines. The utilities agents reach for after
# grep/sed/head: `sort -u`/`-rn`/`-k2`, `uniq -c`, `cut -d -f`/`-c1-200`, `tr -d`,
# awk with `-F`/`-v`/`NR>1`/sums, `printf` formatting, `paste -sd`, `comm`, `diff -u`
# (with `--label` and `-`), `jq -r`/`-c`/`-s` (also behind `gh api --jq`), `wc -c`/`-w`,
# `basename`/`dirname`, `seq`, `rev`, and `cat a && printf '\n---\n' && cat b`.

#### sort -u of collected names
mkdir -p src lib
printf 'import os\nimport sys\nimport json\n' > src/a.py
printf 'import sys\nimport re\nimport os\n' > lib/b.py
cat src/a.py lib/b.py | sort -u

#### sort -rn of counts
printf '3 tests\n12 src\n7 lib\n120 vendor\n' | sort -rn

#### sort -nr | head
printf '4 a.rs\n40 b.rs\n17 c.rs\n2 d.rs\n33 e.rs\n' | sort -nr | head -3

#### sort -k2 by the second field
printf 'b 3\na 1\nc 2\n' | sort -k2

#### sort -t: -k3 -n on a passwd-like file
printf 'root:x:0:0\nalice:x:1001:1001\nbob:x:1000:1000\ndaemon:x:1:1\n' > users.txt
sort -t: -k3 -n users.txt

#### sort -k2,2 -k1,1 with ties
printf 'gamma 2\nalpha 1\nbeta 2\ndelta 1\n' | sort -k2,2 -k1,1

#### sort -o in place
printf 'pear\napple\nfig\n' > fruit.txt
sort -o fruit.txt fruit.txt
cat fruit.txt

#### sort | uniq -c | sort -rn | head of log levels
cat > app.log <<'EOF'
INFO starting server
WARN slow query
INFO request /api/users
ERROR db timeout
INFO request /api/items
WARN slow query
INFO shutting down
EOF
awk '{print $1}' app.log | sort | uniq -c | sort -rn | head -n 5

#### uniq -d lists duplicates
printf 'a\nb\nb\nc\nc\nc\nd\n' | uniq -d

#### cut -d: -f1
printf 'src/main.rs:12:fn main() {\nsrc/lib.rs:3:pub mod util;\n' | cut -d: -f1

#### cut -d, -f2,3 of a csv
cat > data.csv <<'EOF'
id,name,score,team
1,ada,93,red
2,bob,71,blue
3,cy,88,red
EOF
cut -d, -f2,3 data.csv

#### cut -c1-20 truncates long lines
printf 'a short line\nthis line is definitely longer than twenty characters\n' | cut -c1-20

#### cut -d' ' -f2- keeps the rest
printf 'M src/main.rs\nA tests/new test.rs\n' | cut -d' ' -f2-

#### tr -d removes characters
printf '  42\n' | tr -d ' '
printf 'a\r\nb\r\n' | tr -d '\r' | od -c | head -n 2

#### tr -s squeezes spaces
echo "a    b  c     d" | tr -s ' '

#### tr -cd keeps only digits and dots
echo "version: v1.24.3-rc1" | tr -cd '0-9.'
echo

#### awk -F, print columns
cat > data.csv <<'EOF'
id,name,score
1,ada,93
2,bob,71
EOF
awk -F, '{print $2, $3}' data.csv

#### awk NR>1 skips the header
cat > data.csv <<'EOF'
id,name,score
1,ada,93
2,bob,71
3,cy,88
EOF
awk -F, 'NR>1 {print $2}' data.csv

#### awk sums a column
printf 'a.rs 120\nb.rs 45\nc.rs 300\n' | awk '{s += $2} END {print "total", s}'

#### awk printf formats a table
printf 'build 1.5\ntest 12.25\nlint 0.4\n' | awk '{printf "%-8s %6.2f\n", $1, $2}'

#### awk -v passes a variable
printf 'ok 1\nfail 2\nok 3\n' | awk -v want=ok '$1 == want {n++} END {print n}'

#### awk last field and NF
printf 'src/a/b/c.rs\nlib/x.rs\n' | awk -F/ '{print NF, $NF}'

#### awk counts with an array
printf 'red\nblue\nred\ngreen\nred\nblue\n' | awk '{c[$1]++} END {for (k in c) print k, c[k]}' | sort

#### printf with widths and padding
printf '%-10s|%5d|%05.1f\n' name 42 3.14159
printf '%s=%s\n' a 1 b 2 c 3

#### printf %q and %x
printf '%q\n' "it's a test"
printf '%x %o %X\n' 255 8 3054

#### printf separator between two files
printf 'first\n' > a.txt
printf 'second\n' > b.txt
cat a.txt && printf '\n---\n' && cat b.txt

#### printf section headers between three files
printf 'one\n' > x.txt
printf 'two\n' > y.txt
printf 'three\n' > z.txt
cat x.txt && printf '\n=== y ===\n' && cat y.txt && printf '\n=== z ===\n' && cat z.txt

#### paste -sd, joins lines
printf 'alpha\nbeta\ngamma\n' | paste -sd, -

#### paste -sd+ | bc-free sum via arithmetic
nums=$(printf '3\n4\n5\n' | paste -sd+ -)
echo "$nums = $(( nums ))"

#### comm of two sorted lists
printf 'a.rs\nb.rs\nc.rs\n' > before.txt
printf 'b.rs\nc.rs\nd.rs\n' > after.txt
comm -3 before.txt after.txt
echo ---
comm -12 before.txt after.txt

#### diff -u with labels
printf 'line1\nline2\nline3\n' > old.conf
printf 'line1\nline2 changed\nline3\nline4\n' > new.conf
diff -u --label a/app.conf --label b/app.conf old.conf new.conf
echo "status=$?"

#### diff -u skipping the timestamp header
printf 'a\nb\nc\n' > v1.txt
printf 'a\nB\nc\n' > v2.txt
diff -u v1.txt v2.txt | tail -n +3

#### diff against stdin
printf 'one\ntwo\n' > expected.txt
printf 'one\nthree\n' | diff expected.txt -
echo "status=$?"

#### diff -rq of two trees
mkdir -p old/src new/src
printf 'a\n' > old/src/a.txt; printf 'a\n' > new/src/a.txt
printf 'b\n' > old/src/b.txt; printf 'B\n' > new/src/b.txt
printf 'c\n' > new/src/c.txt
diff -rq old new | sort
echo "status=${PIPESTATUS[0]}"

#### jq -r of a key
cat > package.json <<'EOF'
{"name": "demo-app", "version": "1.4.2", "scripts": {"test": "node --test", "build": "tsc"}}
EOF
jq -r '.version' package.json
jq '.name' package.json

#### jq -r keys of scripts
cat > package.json <<'EOF'
{"name": "demo-app", "scripts": {"test": "node --test", "build": "tsc", "lint": "eslint ."}}
EOF
jq -r '.scripts | keys[]' package.json

#### jq -r .[] select
cat > items.json <<'EOF'
[{"name": "a", "state": "open", "n": 3}, {"name": "b", "state": "closed", "n": 5}, {"name": "c", "state": "open", "n": 8}]
EOF
jq -r '.[] | select(.state == "open") | .name' items.json

#### jq -c compact objects
cat > items.json <<'EOF'
[{"name": "a", "n": 3}, {"name": "b", "n": 5}]
EOF
jq -c '.[] | {name, double: (.n * 2)}' items.json

#### jq length and map
cat > items.json <<'EOF'
[{"tags": ["x", "y"]}, {"tags": []}, {"tags": ["z"]}]
EOF
jq 'length' items.json
jq -c 'map(.tags | length)' items.json

#### jq -r string interpolation into tsv
cat > items.json <<'EOF'
[{"id": 1, "title": "Fix parser"}, {"id": 2, "title": "Add tests"}]
EOF
jq -r '.[] | "\(.id)\t\(.title)"' items.json

#### jq -s slurps lines
printf '{"ms": 12}\n{"ms": 30}\n{"ms": 8}\n' | jq -s 'map(.ms) | add'

#### jq --arg filter
cat > deps.json <<'EOF'
{"dependencies": {"left-pad": "1.3.0", "lodash": "4.17.21"}}
EOF
jq -r --arg d lodash '.dependencies[$d]' deps.json

#### jq -e exit status on null
echo '{"a": 1}' | jq -e '.b'
echo "status=$?"

#### gh api --jq through a stub
mkdir -p bin
cat > pulls.json <<'EOF'
[{"number": 7, "title": "Speed up lexer", "user": {"login": "ada"}}, {"number": 9, "title": "Docs", "user": {"login": "bob"}}]
EOF
cat > bin/gh <<'EOF'
#!/bin/sh
filter=.
while [ $# -gt 0 ]; do
  case $1 in
    --jq) filter=$2; shift ;;
  esac
  shift
done
exec jq -c "$filter" pulls.json
EOF
chmod +x bin/gh
export PATH="$PWD/bin:$PATH"
gh api repos/acme/demo/pulls --paginate --jq '.[] | {number, title, author: .user.login}'

#### basename and dirname
basename src/components/Button.tsx
basename src/components/Button.tsx .tsx
dirname src/components/Button.tsx
dirname Button.tsx
basename /usr/local/bin/

#### seq forms
seq 3
seq 2 4
seq 0 5 20
seq -s, 1 5
seq -w 8 11

#### rev reverses lines
printf 'abc\nhello world\n' | rev
printf 'src/a/b.rs\n' | rev | cut -d/ -f1 | rev

#### wc -c and -w
printf 'hello world\n' > greet.txt
wc -c < greet.txt
wc -w < greet.txt
wc -c greet.txt
printf 'one two\nthree\n' | wc -w
