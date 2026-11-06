# Agent shape: commands that start with `cd X && ...` or `cd X; ...` (46% of agent
# commands; research/corpus.md §4, §9.6).

#### cd into a subdirectory and list it
mkdir -p proj/src && touch proj/src/a.rs proj/src/b.rs
cd proj && ls src

#### cd into the directory we are already in
mkdir -p work && cd work
cd "$PWD" && pwd | sed "s|.*/||"

#### cd with a relative path, then a pipeline
mkdir -p a/b && printf 'one\ntwo\nthree\n' > a/b/f.txt
cd a/b && cat f.txt | head -n 2

#### failing cd stops the && chain
cd does-not-exist && echo "should not print"
echo "status=$?"

#### failing cd does not stop a ; chain
cd does-not-exist; echo "still here"; pwd | sed "s|.*/||"

#### cd in a subshell does not leak
mkdir -p inner
(cd inner && pwd | sed "s|.*/||")
pwd | sed "s|.*/||"

#### cd - returns to the previous directory
mkdir -p x y
cd x && cd ../y && cd - >/dev/null && pwd | sed "s|.*/||"

#### cd then a here-doc fed to a shell
mkdir -p d && cd d && sh <<'EOF'
echo "in $(basename "$PWD")"
EOF

#### cd then sed -n range read
mkdir -p repo && seq 1 50 > repo/lines.txt
cd repo && sed -n '10,13p' lines.txt

#### two cd chains with ;
mkdir -p p q && echo P > p/f && echo Q > q/f
cd p && cat f; cd ../q && cat f

#### cd to an absolute path from $PWD
mkdir -p abs/dir
cd "$PWD/abs/dir" && basename "$(pwd)"

#### cd into a path with spaces
mkdir -p "with space/sub"
cd "with space" && ls

#### cd with a trailing slash and ..
mkdir -p m/n
cd m/n/ && cd .. && basename "$PWD"

#### cd to HOME with no argument
cd && test "$PWD" = "$HOME" && echo home

#### cd X && git-like status line with ||
mkdir -p r
cd r && (ls missing 2>/dev/null || echo "nothing here")

#### cd chain with output limiting and 2>&1
mkdir -p logs && printf 'l1\nl2\nl3\nl4\n' > logs/out.log
cd logs && cat out.log missing.log 2>&1 | tail -n 3
