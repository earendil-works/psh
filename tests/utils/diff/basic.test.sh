# diff: GNU diff's formats and options (macOS's diff agrees on most).

#### identical files
printf 'a\nb\n' > 1
cp 1 2
diff 1 2
echo "st=$?"

#### the normal format
printf 'a\nb\nc\nd\ne\n' > 1
printf 'a\nB\nc\nd\ne\nf\n' > 2
diff 1 2
echo "st=$?"
diff 2 1

#### insertions and deletions
printf 'a\nb\n' > 1
printf 'a\nb\nb\n' > 2
seq 1 20 > s1
seq 1 20 | sed '/^10$/d' > s3
diff 1 2
diff 2 1
diff s1 s3
printf 'a\nb\na\nb\n' > t1
printf 'a\nb\n' > t2
diff t1 t2

#### a missing newline at the end
printf 'x\ny' > 1
printf 'x\nz' > 2
printf 'x\ny\n' > 3
diff 1 2
diff 1 3

#### -u with labels
printf 'a\nb\nc\nd\ne\n' > 1
printf 'a\nB\nc\nd\ne\nf\n' > 2
diff -u --label L1 --label L2 1 2
echo "st=$?"

#### -U N and hunk merging
seq 1 20 > s1
seq 1 20 | sed 's/^5$/five/; s/^15$/fifteen/' > s2
diff -U0 --label a --label b s1 s2
diff -u --label a --label b s1 s2
diff -U1 --label a --label b s1 s2
diff -U5 --label a --label b s1 s2

#### -u against an empty file and without newlines
printf 'a\nb\n' > 1
: > e
printf 'x\ny' > 5
printf 'x\nz' > 6
diff -u --label x --label y e 1
diff -u --label x --label y 1 e
diff -u --label x --label y 5 6

#### -u headers carry the modification times
## expect: gnu (D016: BSD diff prints times without nanoseconds or zone)
printf '1\n' > a
printf '2\n' > b
touch -t 202401151000.30 a b
diff -u a b


#### -c
printf 'a\nb\nc\nd\ne\n' > 1
printf 'a\nB\nc\nd\ne\nf\n' > 2
seq 1 20 > s1
seq 1 20 | sed '/^10$/d' > s3
printf 'a\nb\n' > 4
printf 'a\nb\nb\n' > 3
diff -c --label L1 --label L2 1 2
diff -C1 --label a --label b s1 s3
diff -c --label a --label b 4 3

#### -q and -s
printf 'a\n' > 1
printf 'b\n' > 2
diff -q 1 2
echo "st=$?"
diff -q 1 1
echo "st=$?"
diff -s 1 1
diff --brief 1 2

#### missing files are trouble
printf 'a\n' > 1
diff nope 1
echo "st=$?"
diff -N nope 1
echo "st=$?"

#### -w -b -i
printf 'a b\n' > w1
printf 'a  b \n' > w2
printf 'ab\n' > w3
printf 'A B\n' > w4
diff -w w1 w3
echo "st=$?"
diff -b w1 w2
echo "st=$?"
diff -b w1 w3
echo "st=$?"
diff -i w1 w4
echo "st=$?"
diff --ignore-all-space w2 w3
echo "st=$?"

#### -B ignores blank-line changes
printf 'a\n\nb\n' > b1
printf 'a\nb\n\n\n' > b2
diff -B b1 b2
echo "st=$?"
diff b1 b2

#### -I ignores matching changes
printf 'a\n# x\nb\n' > i1
printf 'a\n# y\nb\n' > i2
printf 'a\n# y\nc\n' > i3
diff -I '^#' i1 i2
echo "st=$?"
diff -I '^#' i1 i3

#### --strip-trailing-cr
printf 'a\r\nb\r\n' > c1
printf 'a\nb\n' > c2
diff --strip-trailing-cr c1 c2
echo "st=$?"

#### binary files
printf 'a\0b' > bin1
printf 'a\0c' > bin2
diff bin1 bin2
echo "st=$?"
diff -q bin1 bin2

#### stdin as -
printf 'a\nb\n' > 1
printf 'a\nc\n' | diff 1 -
echo "st=$?"

#### a file against a directory
mkdir d
printf '1\n' > f
printf '2\n' > d/f
diff f d
echo "st=$?"
