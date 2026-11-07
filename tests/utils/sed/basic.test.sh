# sed: the commands, addresses and the cycle.

#### substitute, global, nth
printf 'aaa\nbab\n' | sed 's/a/X/'
printf 'aaa\n' | sed 's/a/X/g'
printf 'aaa\n' | sed 's/a/X/2'

#### nth and g together
## expect: gnu (BSD sed rejects a number with g)
echo aaaaa | sed 's/a/X/3g'

#### p and -n
printf 'a\nb\n' | sed -n 's/a/X/p'
printf 'a\nb\n' | sed -n '2p'

#### line and last addresses
seq 5 | sed -n '3p;$p'

#### ranges
seq 6 | sed -n '2,4p'
seq 6 | sed -n '4,2p'
seq 6 | sed -n '/2/,/4/p'
printf 'ab\nc\nb\n' | sed -n '/a/,/b/p'

#### range ends again and restarts
printf 'a\nb\na\nb\n' | sed -n '/a/,/b/p'

#### GNU range forms
## expect: gnu (BSD sed has no 0,/re/, addr,+N, addr,~N or first~step)
printf 'ab\nc\nb\n' | sed -n '0,/b/p'
seq 6 | sed -n '/2/,+2p'
seq 10 | sed -n '5,~4p'
seq 10 | sed -n '0~3p'

#### negation
seq 3 | sed -n '2!p'
seq 3 | sed '1 ! d'

#### delete and print
seq 4 | sed '2d;3p'

#### blocks
seq 5 | sed -n '2,4{/3/d;p}'

#### append, insert, change (POSIX form)
seq 2 | sed '1a\
after
2i\
before
1c\
changed'

#### one-line a, i, c
## expect: gnu (BSD sed needs a backslash after a, i and c)
seq 3 | sed -e '1a after' -e '2i before' -e '3c changed'

#### a with leading blanks kept
## expect: gnu (BSD sed rejects text after a\ on the same line)
seq 1 | sed 'a\   x'

#### a text continued
seq 1 | sed 'a\
foo\
bar'

#### a, i, c from several -e
## expect: gnu (BSD sed does not join -e pieces into one text)
seq 1 | sed -e 'i\' -e 'x' -e p

#### c over a range prints once
seq 4 | sed '2,3c\
X'

#### c with negation
## expect: gnu (BSD sed needs a backslash after c)
seq 3 | sed '$!c X'

#### append queue flushed by n and N
## expect: gnu (BSD sed needs a backslash after a)
seq 3 | sed '1{a foo
n}'

#### r reads a file after the cycle
echo inc > inc
seq 2 | sed 'r inc'
seq 2 | sed 'r nosuch'

#### R reads one line at a time
## expect: gnu (BSD sed has no R)
printf 'x\ny\n' > inc
seq 3 | sed 'R inc'

#### w writes a file
seq 3 | sed -n '2w out'
cat out

#### w to /dev/stdout
seq 2 | sed 'w /dev/stdout'

#### hold space
printf 'a\nb\nc\n' | sed -n '1!G;h;$p'
echo a | sed x

#### N, P, D
printf '1\n2\n3\n' | sed '$!N;P;D'
printf '1\n2\n3\n' | sed '$!N;s/\n/+/'

#### N on the last line prints
## expect: gnu (BSD sed quits without printing)
printf '1\n2\n3\n' | sed 'N;s/\n/-/'

#### n
seq 3 | sed 'n;d'
seq 3 | sed -n 'n;p'

#### labels and branches
printf 'a\nb\nc\n' | sed ':a;N;$!ba;s/\n/ /g'

#### t and T
printf 'ab\nb\n' | sed -e 's/a/X/;ta' -e 's/$/ no/;b' -e ':a' -e 's/$/ yes/'

#### T branches without a substitution
## expect: gnu (BSD sed has no T)
printf 'ab\nb\n' | sed 's/a/X/;Ta;s/$/ yes/;b;:a;s/$/ no/'

#### = prints the line number
printf 'a\nb\n' | sed -n '$='

#### y transliterates
echo hello | sed 'y/el/ip/'
printf 'a\tb\n' | sed 'y/\t/X/'

#### l shows the pattern space
printf 'a\tb\001\\\n' | sed -n l

#### l wraps
## expect: gnu (BSD sed wraps l at a different width and has no l N)
echo aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa | sed -n l
echo aaaaaaaaaaaaaaaaaaaaaaaaaaa | sed -n 'l 10'
echo aaaaaaaaaaaaaaaaaaaaaaaaaaa | sed -n -l 8 l

#### q and Q with exit codes
## expect: gnu (BSD sed has no exit code for q and no Q)
seq 3 | sed '2q5'; echo $?
seq 3 | sed '2Q'; echo $?

#### q
seq 5 | sed 2q

#### z and F
## expect: gnu (BSD sed has no z or F)
echo a | sed 'z;s/^$/empty/'
echo a > f; sed F f

#### a last line without a newline stays so
printf 'a\nb' | sed p
printf 'a' | sed 's/a/b/'

#### #n on the first line
seq 2 | sed '#n
2p'

#### comments
seq 2 | sed -n '# a comment
1p # another'

#### several files are one stream
printf 'a\n' > x; printf 'b\n' > y
sed -n '$p;1p' x y

#### -s separates files
## expect: gnu (BSD sed has no -s)
printf 'a\nb\n' > x; printf 'c\nd\n' > y
sed -s -n '$p;1p' x y

#### stdin as -
echo a > f
echo b | sed p f -
