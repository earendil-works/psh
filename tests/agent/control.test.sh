# Agent shape: control flow and grouping agents use: for, if/test, while read,
# subshells, groups, functions, background jobs; research/corpus.md §4.

#### for over words
for w in alpha beta; do echo "$w"; done

#### for over seq output
for i in $(seq 1 3); do printf '%s,' "$i"; done; echo

#### if with test -f
touch exists.txt
if [ -f exists.txt ]; then echo "file"; else echo "no file"; fi

#### if with test -d and else
if [ -d nodir ]; then echo dir; else echo "no dir"; fi

#### test -z and -n
e=""; f=x
[ -z "$e" ] && echo empty
[ -n "$f" ] && echo nonempty

#### numeric comparison
n=3
if [ "$n" -gt 2 ]; then echo big; fi

#### [[ ]] with pattern match
f=report.json
if [[ $f == *.json ]]; then echo json; fi

#### while read over a file
printf 'k1 v1\nk2 v2\n' > kv.txt
while read -r k v; do echo "$v<-$k"; done < kv.txt

#### while with a counter
i=0
while [ $i -lt 3 ]; do i=$((i+1)); done
echo "i=$i"

#### case statement
for f in a.rs b.py c.txt; do
  case "$f" in
    *.rs) echo "rust $f" ;;
    *.py) echo "python $f" ;;
    *) echo "other $f" ;;
  esac
done

#### function definition and call
greet() { echo "hello $1"; }
greet agent

#### function with local and return
f() { local x=inner; echo "$x"; return 3; }
f
echo "status=$?"

#### subshell isolates variables
x=outer
(x=inner; echo "$x")
echo "$x"

#### brace group shares the shell
x=1
{ x=2; }
echo "$x"

#### background job and wait
sleep 0.2 & pid=$!
wait "$pid"
echo "waited status=$?"

#### two background jobs, then wait
(echo first > bg1.txt) &
(echo second > bg2.txt) &
wait
cat bg1.txt bg2.txt

#### break and continue
for i in 1 2 3 4 5; do
  [ $i -eq 2 ] && continue
  [ $i -eq 4 ] && break
  echo "$i"
done

#### && || chain as if/else
true && echo yes || echo no
false && echo yes || echo no

#### line continuation
echo one \
  two \
  three

#### multiple statements on one line
echo a; echo b; echo c

#### comment after a command
echo visible # this is a comment

#### arrays are rare but used
arr=(x y z)
echo "${arr[1]} ${#arr[@]} ${arr[@]}"

#### mkdir -p then write and read
mkdir -p out/nested && echo data > out/nested/file && cat out/nested/file

#### source a file
echo 'SOURCED=yes' > env.sh
. ./env.sh && echo "$SOURCED"

#### trap on EXIT
trap 'echo cleanup' EXIT
echo working
