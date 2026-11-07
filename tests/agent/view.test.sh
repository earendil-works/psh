# Agent shape: reading files. The most common agent command family is viewing a slice
# of a file: `cd DIR && sed -n A,Bp FILE`, chains of several `sed -n A,Bp` on one or
# more files separated by `;` or `&&`, `nl -ba FILE | sed -n 'A,Bp'`, address ranges
# `sed -n '/start/,/end/p'`, `head -n N` / `head -N`, `tail -n N` / `tail -n +N`,
# `cat` of one or several files (with printf separators), `wc -l`, `cat FILE | head`,
# `cd DIR; sed -n ...`, and `cd ~/proj && ...` on projects under HOME.

#### cd dir && sed -n range
mkdir -p app/src
seq 1 80 | sed 's/^/    let v = /; s/$/;/' > app/src/main.rs
cd app && sed -n 10,15p src/main.rs

#### cd dir && sed -n 1,N from the top
mkdir -p svc
cat > svc/config.toml <<'EOF'
[server]
host = "127.0.0.1"
port = 8080
workers = 4

[database]
url = "postgres://localhost/app"
pool = 10
EOF
cd svc && sed -n 1,5p config.toml

#### cd dir && two sed ranges on the same file
mkdir -p lib
cat > lib/util.py <<'EOF'
import os
import sys


def read_config(path):
    with open(path) as f:
        return f.read()


def write_config(path, data):
    with open(path, "w") as f:
        f.write(data)


def main():
    cfg = read_config(sys.argv[1])
    print(cfg)


if __name__ == "__main__":
    main()
EOF
cd lib && sed -n 1,3p util.py; sed -n 15,18p util.py

#### cd dir && sed ranges on two different files
mkdir -p proj/src proj/tests
seq 1 30 | sed 's/^/src line /' > proj/src/lib.rs
seq 1 30 | sed 's/^/test line /' > proj/tests/it.rs
cd proj && sed -n 3,5p src/lib.rs; sed -n 20,22p tests/it.rs

#### cd dir && three sed ranges on one file
mkdir -p web
seq 1 120 | sed 's/^/<div id="n/; s/$/"><\/div>/' > web/index.html
cd web && sed -n 1,2p index.html; sed -n 50,52p index.html; sed -n 118,120p index.html

#### cd dir && four sed ranges on one file
mkdir -p pkg
seq 1 200 | awk '{print "export const k" $1 " = " $1 * 2 ";"}' > pkg/consts.ts
cd pkg && sed -n 1,2p consts.ts; sed -n 40,41p consts.ts; sed -n 99,100p consts.ts; sed -n 199,200p consts.ts

#### cd dir && sed ranges, second file read twice
mkdir -p repo/docs
seq 1 20 | sed 's/^/readme /' > repo/README.md
seq 1 60 | sed 's/^/guide /' > repo/docs/guide.md
cd repo && sed -n 1,3p README.md; sed -n 10,12p docs/guide.md; sed -n 55,60p docs/guide.md

#### cd dir && three sed ranges on three files
mkdir -p m/a m/b m/c
seq 1 10 | sed 's/^/a/' > m/a/x.go
seq 1 10 | sed 's/^/b/' > m/b/y.go
seq 1 10 | sed 's/^/c/' > m/c/z.go
cd m && sed -n 1,2p a/x.go; sed -n 5,6p b/y.go; sed -n 9,10p c/z.go

#### cd dir && sed range then sed 1,N on another file
mkdir -p tool
seq 1 40 | sed 's/^/body /' > tool/run.sh
printf 'name: tool\nversion: 0.1.0\nlicense: MIT\n' > tool/meta.yml
cd tool && sed -n 30,33p run.sh; sed -n 1,2p meta.yml

#### cd dir && sed ranges joined with &&
mkdir -p api
seq 1 50 | sed 's/^/route /' > api/routes.js
cd api && sed -n 5,7p routes.js && sed -n 45,47p routes.js

#### cd dir && sed with quoted script
mkdir -p cfg
printf 'alpha\nbeta\ngamma\ndelta\nepsilon\n' > cfg/list.txt
cd cfg && sed -n '2,4p' list.txt

#### cd dir; sed range
mkdir -p crate/src
seq 1 25 | sed 's/^/fn f/; s/$/() {}/' > crate/src/lib.rs
cd crate; sed -n 20,25p src/lib.rs

#### cd dir; sed 1,N
mkdir -p notes
printf 'todo one\ntodo two\ntodo three\ntodo four\n' > notes/todo.md
cd notes; sed -n 1,3p todo.md

#### cd missing dir; sed still runs in the old directory
seq 1 5 > here.txt
cd nowhere 2>/dev/null; sed -n 2,3p here.txt
echo "status=$?"

#### cd missing dir && sed is skipped
seq 1 5 > here.txt
cd nowhere 2>/dev/null && sed -n 2,3p here.txt
echo "status=$?"

#### cd ~/proj && sed range
mkdir -p ~/proj/src
seq 1 40 | sed 's/^/home line /' > ~/proj/src/app.ts
cd ~/proj && sed -n 12,14p src/app.ts

#### cd ~/proj && two sed ranges on one file
mkdir -p ~/proj
seq 1 90 | sed 's/^/entry /' > ~/proj/CHANGELOG.md
cd ~/proj && sed -n 1,2p CHANGELOG.md; sed -n 88,90p CHANGELOG.md

#### cd ~/proj && sed 1,N
mkdir -p ~/proj
printf '{\n  "name": "demo",\n  "version": "1.0.0",\n  "private": true\n}\n' > ~/proj/package.json
cd ~/proj && sed -n 1,3p package.json

#### cd ~/proj; sed range
mkdir -p ~/proj/lib
seq 1 12 | sed 's/^/lib /' > ~/proj/lib/mod.rb
cd ~/proj; sed -n 4,6p lib/mod.rb

#### sed -n range without cd
seq 1 30 | sed 's/^/row /' > data.csv
sed -n 10,12p data.csv

#### sed -n single line
printf 'a\nb\nc\nd\n' > f.txt
sed -n '3p' f.txt

#### sed -n last line
printf 'a\nb\nc\nlast\n' > f.txt
sed -n '$p' f.txt

#### sed -n from line to end
seq 1 12 > n.txt
sed -n '9,$p' n.txt

#### sed -n range past end of file
seq 1 5 > short.txt
sed -n 3,100p short.txt
echo "status=$?"

#### sed -n range that starts past end prints nothing
seq 1 5 > short.txt
sed -n 50,60p short.txt
echo "status=$?"

#### sed -n reversed range prints only the first line
seq 1 20 > n.txt
sed -n 10,5p n.txt

#### sed -n two ranges in one script
seq 1 30 > n.txt
sed -n '1,2p;28,30p' n.txt

#### sed -n range across two files continues numbering
printf 'a1\na2\na3\n' > a.txt
printf 'b1\nb2\nb3\n' > b.txt
sed -n 2,5p a.txt b.txt

#### sed -n ranges on two files with ;
seq 1 10 | sed 's/^/x/' > x.txt
seq 1 10 | sed 's/^/y/' > y.txt
sed -n 1,2p x.txt; sed -n 9,10p y.txt

#### sed quoted ranges chained with &&
seq 1 20 | sed 's/^/log /' > app.log
sed -n '1,2p' app.log && sed -n '19,20p' app.log

#### sed -n on a missing file
# dialect: gnu
sed -n 1,5p missing.txt
echo "status=$?"

#### sed -n regex range
cat > server.py <<'EOF'
import http.server

class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.end_headers()

    def do_POST(self):
        self.send_response(405)
        self.end_headers()

def serve():
    pass
EOF
sed -n '/def do_GET/,/end_headers/p' server.py

#### sed -n regex range to end of file
printf '[a]\nx=1\n[b]\ny=2\nz=3\n' > conf.ini
sed -n '/^\[b\]/,$p' conf.ini

#### sed -n on a file without a trailing newline
# dialect: gnu
printf 'one\ntwo\nthree' > nonl.txt
sed -n 2,3p nonl.txt
echo "|end"

#### sed -n on a CRLF file keeps the carriage returns
printf 'a\r\nb\r\nc\r\n' > win.txt
sed -n 2,3p win.txt | od -c | sed -n 1p

#### sed -n with tabs and unicode
printf 'k\tv\n\xc3\xa9t\xc3\xa9\tsummer\nna\xc3\xafve\tword\n' > u.tsv
sed -n 2,3p u.tsv

#### nl -ba piped into sed range
cat > main.go <<'EOF'
package main

import "fmt"

func main() {
	fmt.Println("hello")
}
EOF
nl -ba main.go | sed -n '3,7p'

#### nl -ba numbers blank lines, plain nl does not
printf 'a\n\nb\n' > gaps.txt
nl -ba gaps.txt; nl gaps.txt

#### two nl -ba views with ;
seq 1 20 | sed 's/^/alpha /' > a.txt
seq 1 20 | sed 's/^/beta /' > b.txt
nl -ba a.txt | sed -n '2,3p'; nl -ba b.txt | sed -n '18,20p'

#### three nl -ba views with ;
seq 1 9 > one.txt; seq 10 19 > two.txt; seq 20 29 > three.txt
nl -ba one.txt | sed -n '1p'; nl -ba two.txt | sed -n '5p'; nl -ba three.txt | sed -n '10p'

#### nl -ba views joined with &&
seq 1 30 | sed 's/^/item /' > list.txt
nl -ba list.txt | sed -n '1,2p' && nl -ba list.txt | sed -n '29,30p'

#### cd dir && nl -ba of a large file
mkdir -p big
seq 1 2000 | sed 's/^/record /' > big/data.txt
cd big && nl -ba data.txt | sed -n '1500,1503p'

#### cat one file
printf 'line one\nline two\n' > README.md
cat README.md

#### cd dir && cat file
mkdir -p proj
printf '[package]\nname = "demo"\nversion = "0.1.0"\n' > proj/Cargo.toml
cd proj && cat Cargo.toml

#### cd dir && cat two and three files
mkdir -p proj
echo "A" > proj/a.txt; echo "B" > proj/b.txt; echo "C" > proj/c.txt
cd proj && cat a.txt b.txt; cat a.txt b.txt c.txt

#### cat files with printf separators
printf 'first file\n' > one.md
printf 'second file\n' > two.md
printf 'third file\n' > three.md
cat one.md && printf '\n---\n' && cat two.md && printf '\n---\n' && cat three.md

#### cat chained with &&
echo "left" > l.txt; echo "right" > r.txt
cat l.txt && cat r.txt

#### cat with a missing file continues and fails
echo "present" > ok.txt
cat ok.txt nope.txt 2>/dev/null
echo "status=$?"

#### cat -n numbers lines
printf 'x\n\ny\n' > f.txt
cat -n f.txt

#### cd dir && cat then sed range
mkdir -p d
printf 'k=1\nk=2\n' > d/env.txt
seq 1 20 > d/n.txt
cd d && cat env.txt; sed -n 1,3p n.txt

#### cat file piped to head
seq 1 100 | sed 's/^/entry /' > list.txt
cat list.txt | head -n 3

#### cat a large file into head
seq 1 200000 > huge.txt
cat huge.txt | head -5
echo "status=$?"

#### head -n and head -N
seq 1 50 > n.txt
head -n 3 n.txt; head -2 n.txt

#### head of several files prints headers
printf 'a1\na2\na3\n' > a.txt
printf 'b1\nb2\nb3\n' > b.txt
head -n 2 a.txt b.txt

#### head -n negative drops trailing lines
# dialect: gnu
seq 1 6 > n.txt
head -n -4 n.txt

#### tail -n and tail -N
seq 1 50 > n.txt
tail -n 2 n.txt; tail -3 n.txt

#### tail -n +N from a line to the end
seq 1 10 > n.txt
tail -n +8 n.txt

#### tail -n +N | head -n N gives a window
seq 1 100 | sed 's/^/L/' > n.txt
tail -n +40 n.txt | head -n 3

#### wc -l of one file
# dialect: gnu
seq 1 42 > n.txt
wc -l n.txt

#### wc -l of several files prints a total
# dialect: gnu
seq 1 3 > a.txt; seq 1 7 > b.txt
wc -l a.txt b.txt

#### wc -l from stdin
seq 1 9 > n.txt
wc -l < n.txt | tr -d ' '

#### python heredoc then sed range
mkdir -p bin proj && printf '#!/bin/sh\ncat >/dev/null\necho "patched"\n' > bin/python3 && chmod +x bin/python3 && export PATH="$PWD/bin:$PATH"
seq 1 20 | sed 's/^/code /' > proj/mod.py
cd proj && python3 - <<'EOF'
from pathlib import Path
p = Path("mod.py")
p.write_text(p.read_text())
EOF
sed -n 5,7p mod.py
