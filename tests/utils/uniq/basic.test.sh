# uniq: adjacent duplicates, counts, keys.

#### adjacent duplicates collapse
printf 'a\na\nb\na\n' | uniq

#### -c counts
## expect: gnu (D016: uniq -c widths)
printf 'a\na\nb\nc\nc\nc\n' | uniq -c

#### the frequency idiom
## expect: gnu (D016: uniq -c widths)
printf 'x\ny\nx\nz\nx\ny\n' | sort | uniq -c | sort -rn

#### -d only repeated
printf 'a\na\nb\nc\nc\n' | uniq -d

#### -u only unique
printf 'a\na\nb\nc\nc\n' | uniq -u

#### -c with -d
## expect: gnu (D016: uniq -c widths)
printf 'a\na\nb\n' | uniq -cd

#### -D all repeated lines
printf 'a\na\nb\nc\nc\n' | uniq -D

#### --all-repeated=separate
## expect: gnu (BSD uniq has no long options)
printf 'a\na\nb\nc\nc\n' | uniq --all-repeated=separate

#### --group
## expect: gnu (BSD uniq has no --group)
printf 'a\na\nb\nc\n' | uniq --group

#### -f skips fields
printf '1 a\n2 a\n3 b\n' | uniq -f1

#### -s skips characters
printf 'xa\nya\nzb\n' | uniq -s1

#### -w compares the first characters
## expect: gnu (BSD uniq has no -w)
printf 'ab1\nab2\nac\n' | uniq -w2

#### -i ignores case
printf 'A\na\nB\n' | uniq -i

#### input and output files
printf 'a\na\nb\n' > in
uniq in out
cat out

#### a last line without a newline
printf 'a\na' | uniq

#### -z
## expect: gnu (BSD uniq has no -z)
printf 'a\0a\0b\0' | uniq -z | tr '\0' '|'; echo

#### empty input
uniq < /dev/null
echo "status=$?"

#### a missing file
uniq missing
echo "status=$?"

#### -c with -D is an error
## expect: gnu (BSD uniq allows it)
printf 'a\n' | uniq -c -D
echo "status=$?"
