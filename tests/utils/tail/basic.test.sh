# tail: last lines and bytes, from a line, the old forms, BSD's -r.

#### default ten lines
seq 1 20 | tail

#### -n and -N
seq 1 20 > f
tail -n 3 f
tail -2 f

#### +N and -n +N start at a line
seq 1 5 > f
tail -n +4 f
tail +4 f

#### -c bytes
printf 'abcdef' | tail -c 3; echo

#### -c +N
printf 'abcdef' | tail -c +3; echo

#### a last line without a newline
printf 'a\nb' | tail -n 1; echo

#### several files get headers
printf 'a\nb\n' > f
printf 'c\n' > g
tail -n 1 f g

#### -r reverses
## expect: bsd (GNU tail has no -r; tac does it there)
printf 'a\nb\nc\n' > f
tail -r f

#### -n with -r
## expect: bsd (GNU tail has no -r)
seq 1 5 > f
tail -n 2 -r f

#### a missing file
tail -n 1 missing
echo "status=$?"

#### -f on a file that does not grow, stopped by timeout
## expect: gnu (macOS has no timeout)
printf 'a\nb\n' > f
timeout 0.3 tail -f f
echo "status=$?"
