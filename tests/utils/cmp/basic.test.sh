# cmp: GNU's output (D016: BSD says "char" where GNU says "byte").

#### equal files
printf 'hello\nworld\n' > a
cp a b
cmp a b
echo "st=$?"

#### the first difference
## expect: gnu (D016: GNU says byte, BSD char)
printf 'hello\nworld\n' > a
printf 'hello\nwOrld\n' > b
cmp a b
echo "st=$?"

#### -s says nothing
printf 'hello\nworld\n' > a
printf 'hello\nwOrld\n' > b
cmp -s a b
echo "st=$?"
cmp --quiet a a
echo "st=$?"
cmp -s a nope
echo "st=$?"

#### EOF on the shorter file
## expect: gnu (D016: GNU's EOF message)
printf 'hello\nworld\n' > a
printf 'hello\n' > c
: > e
cmp a c
echo "st=$?"
cmp c a
echo "st=$?"
cmp e a
echo "st=$?"
cmp -l a c
echo "st=$?"

#### -l lists every differing byte
## expect: gnu (D016: GNU pads offsets to the longest possible one)
printf 'hello\nworld\n' > a
printf 'hellO\nwOrlD\n' > d
cmp -l a d
echo "st=$?"

#### -b prints the differing bytes
## expect: gnu (D016: GNU says byte, BSD char)
printf 'hello\nworld\n' > a
printf 'hello\nwOrld\n' > b
cmp -b a b
printf '\001\200\377a' > x
printf 'bbbb' > y
cmp -b x y

#### -lb
## expect: gnu (D016: GNU's columns)
printf 'hello\nworld\n' > a
printf 'hellO\nwOrlD\n' > d
cmp -lb a d
printf '\001\200\377a' > x
printf 'bbbb' > y
cmp -l --print-bytes x y

#### -n limits the bytes compared
## expect: gnu (D016: GNU says byte, BSD char)
printf 'hello\nworld\n' > a
printf 'hello\nwOrld\n' > b
cmp -n 7 a b
echo "st=$?"
cmp --bytes=8 a b
echo "st=$?"

#### -i and operands skip initial bytes
## expect: gnu (D016: GNU says byte, BSD char)
printf 'hello\nworld\n' > a
printf 'hello\nwOrld\n' > b
cmp -i 2 a b
cmp -i 1:1 a b
cmp a b 1 1
cmp --ignore-initial=1:2 a b
echo "st=$?"
cmp -i 1k a b
echo "st=$?"

#### stdin as - or the missing operand
## expect: gnu (D016: GNU names stdin -)
printf 'hello\nworld\n' > a
printf 'hello\nwOrld\n' > b
cmp a - < b
cat b | cmp a
echo "st=$?"

#### a missing file is trouble
printf 'x' > a
cmp nope a
echo "st=$?"
