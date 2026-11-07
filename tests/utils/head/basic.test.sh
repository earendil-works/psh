# head: lines, bytes, several files, the old -N form.

#### default ten lines
seq 1 20 | head

#### -n and -N
seq 1 20 > f
head -n 3 f
head -2 f

#### -c bytes
printf 'abcdef' | head -c 3; echo

#### several files get headers
printf 'a\nb\n' > f
printf 'c\n' > g
head -n 1 f g

#### -q suppresses headers
## expect: gnu (BSD head has no -q)
printf 'a\nb\n' > f
printf 'c\n' > g
head -q -n 1 f g

#### -v prints a header for one file
## expect: gnu (BSD head has no -v)
printf 'a\n' > f
head -v f

#### all but the last lines
## expect: gnu (BSD head has no negative counts)
seq 1 5 | head -n -2

#### all but the last bytes
## expect: gnu (BSD head has no negative counts)
printf 'abcdef' | head -c -2; echo

#### head ends the producer
yes | head -n 3

#### an endless loop into head
i=0; while :; do echo $i; i=$((i+1)); done | head -n 2

#### a missing file
head -n 1 missing
echo "status=$?"

#### counts with suffixes
## expect: gnu (BSD head has no suffixes)
seq 1 2000 | head -c 1K | wc -c

#### an invalid count
## expect: gnu (D016: GNU's messages)
head -n x /dev/null
echo "status=$?"
