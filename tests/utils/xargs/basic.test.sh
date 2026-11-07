# xargs: GNU's, with BSD's rule for empty input (D1170: a command never runs without
# arguments read; -r is accepted and changes nothing).

#### blanks, newlines, quotes and backslashes split and group items
printf 'a b\nc "d e" f\\ g\n' | xargs printf '[%s]\n'

#### the default command is echo
printf 'one\ntwo\n' | xargs

#### -n1 runs one command per item
printf 'a b\nc\n' | xargs -n1 echo x

#### -n2 with -x
## expect: gnu (BSD -n with -x splits differently)
printf 'a b c d e\n' | xargs -n2 -x echo

#### -0 splits on NUL only
printf 'a\0b c\0' | xargs -0 -n1 printf '<%s>\n'

#### --null is -0
printf 'x\0y\0' | xargs --null echo

#### -d splits on one byte
## expect: gnu (BSD xargs has no -d)
printf 'a,b,c' | xargs -d, -n1 echo d

#### -d with an escape
## expect: gnu (BSD xargs has no -d)
printf 'a\tb\tc\n' | xargs -d '\t' printf '<%s>\n'

#### -I replaces in every argument, one line per command
printf 'l1 a\n  l2 b\n' | xargs -I{} echo "[{}]" "<{}>"

#### -I attached and -i
## expect: gnu (BSD xargs has no -i)
printf 'p\nq\n' | xargs -IX echo X-X; printf 'r\n' | xargs -i echo '{}!'

#### -L2 takes two lines per command
printf 'a\nb\nc\n' | xargs -L2 echo z

#### -L1: a trailing blank continues the line
## expect: gnu (BSD -L does not join lines ending in a blank)
printf 'a \nb\nc\n' | xargs -L1 echo L

#### -t shows each command on stderr
printf 'a b' | xargs -t echo 2>&1

#### -a reads a file
## expect: gnu (BSD xargs has no -a)
printf 'f1\nf2\n' > list; xargs -a list echo got

#### -E stops at a word
printf 'a\nEND\nb\n' | xargs -E END echo

#### -P runs commands in parallel; the status is 123 when one failed
## expect: gnu (D016: GNU statuses; BSD xargs exits 1)
printf '0\n3\n0\n' | xargs -P3 -n1 sh -c 'exit $0'; echo status=$?

#### a command that exits 255 stops xargs with 124
## expect: gnu (D016: GNU statuses; BSD xargs exits 1)
printf 'x\ny\n' | xargs -n1 sh -c 'echo $0; exit 255'; echo status=$?

#### a failing command gives 123
## expect: gnu (D016: GNU statuses; BSD xargs exits 1)
printf 'a\n' | xargs false; echo status=$?

#### empty input runs nothing (D1170)
## expect: bsd (D1170: no command without arguments)
true | xargs echo ran; echo status=$?; echo | xargs echo ran; echo status=$?

#### -r is accepted
true | xargs -r echo ran; echo status=$?

#### an unmatched quote is an error
## expect: gnu (D016: GNU message)
printf "it's\n" | xargs echo; echo status=$?

#### commands get /dev/null as stdin
printf 'a\n' | xargs sh -c 'cat; echo done $0'

#### xargs runs shell builtins' standalone forms and utilities
printf 'b\na\n' | xargs printf '%s;'; echo; printf '3\n' | xargs seq

#### -s limits the command line
## expect: gnu (BSD counts the command line differently)
printf 'aaa bbb ccc ddd\n' | xargs -s 12 echo
