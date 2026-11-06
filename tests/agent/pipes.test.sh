# Agent shape: pipelines and output limiting (`| head` alone is in 32% of commands;
# research/corpus.md §4, T5b).

#### pipe into head
seq 1 100 | head -n 5

#### pipe into head with the old -N form
seq 1 100 | head -3

#### pipe into tail
seq 1 100 | tail -n 4

#### head of a long stream ends the producer (SIGPIPE)
yes | head -n 2
echo "status=$?"

#### seq into head, status of the pipeline
seq 1 1000000 | head -n 1
echo "status=${PIPESTATUS[*]}"

#### 2>&1 into head
{ echo out; echo err >&2; echo out2; } 2>&1 | head -n 2

#### grep -n into head
printf 'alpha\nbeta\nalphabet\ngamma\n' > words.txt
grep -n alpha words.txt | head -n 5

#### nl -ba piped to sed -n
printf 'a\nb\nc\nd\ne\n' > f.txt
nl -ba f.txt | sed -n '2,4p'

#### sort | uniq -c | sort -rn
printf 'b\na\nb\nc\nb\na\n' | sort | uniq -c | sort -rn

#### wc -l of a pipeline
printf 'x\ny\nz\n' | wc -l

#### pipe status without pipefail
false | true
echo "status=$?"

#### pipe status with pipefail
set -o pipefail
false | true
echo "status=$?"

#### PIPESTATUS of a three-stage pipeline
true | false | true
echo "${PIPESTATUS[@]}"

#### cut -c to truncate lines
printf 'a very long line of text\nshort\n' | cut -c1-10

#### tr and sort
echo "c b a" | tr ' ' '\n' | sort

#### pipeline into while read
printf 'one\ntwo\n' | while read -r line; do echo "got $line"; done

#### variable set in a pipeline's while loop is lost
n=0
printf 'a\nb\n' | while read -r l; do n=$((n+1)); done
echo "n=$n"

#### head -c bytes
printf 'abcdefgh' | head -c 3; echo

#### tail -n +N
seq 1 6 | tail -n +4

#### xargs with echo
printf 'a\nb\nc\n' | xargs echo

#### pipe a here-string
grep -c o <<< "foo boo"

#### command substitution of a pipeline
n=$(seq 1 10 | wc -l | tr -d ' ')
echo "lines: $n"

#### |& pipes stderr too
{ echo out; echo err >&2; } |& sort

#### grep with no match in a pipe, then || true
printf 'a\nb\n' | grep zzz || true
echo "status=$?"
