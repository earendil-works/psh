# wc: counts, and GNU's widths (D016).

#### one count from stdin
## expect: gnu (D016: wc widths)
printf 'a b\nc\n' | wc -l

#### all counts from stdin
## expect: gnu (D016: wc widths)
printf 'a b\nc\n' | wc

#### one count of a file
## expect: gnu (D016: wc widths)
printf 'a b\nc\n' > f
wc -l f

#### all counts of a file
## expect: gnu (D016: wc widths)
printf 'a b\nc\n' > f
wc f

#### several files and a total
## expect: gnu (D016: wc widths)
printf 'a b\nc\n' > f
printf 'hello world foo\n' > g
wc f g

#### stdin redirected from a file
## expect: gnu (D016: wc widths)
printf 'one\ntwo\n' > f
wc -l < f

#### words and bytes
## expect: gnu (D016: wc widths)
printf '  lead  and   trail  \n' | wc -w -c

#### characters in UTF-8
## expect: gnu (D016: wc widths)
printf 'h\303\251llo\n' | LC_ALL=C.UTF-8 wc -m

#### longest line
## expect: gnu (BSD wc's -L differs)
printf 'a\tb\nlonger line\n' | wc -L

#### a number with nothing else, as agents use it
## expect: gnu (D016: wc widths)
n=$(seq 1 42 | wc -l)
echo "[$n]"

#### a missing file
## expect: gnu (D016: wc widths)
printf 'x\n' > f
wc -l f missing
echo "status=$?"
