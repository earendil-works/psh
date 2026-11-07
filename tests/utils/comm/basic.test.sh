# comm: three columns of two sorted files.

#### three columns
printf 'a\nb\nd\n' > f
printf 'b\nc\nd\ne\n' > g
comm f g

#### -12 common lines
printf 'a\nb\nd\n' > f
printf 'b\nc\nd\ne\n' > g
comm -12 f g

#### -23 only in the first
printf 'a\nb\nd\n' > f
printf 'b\nc\nd\ne\n' > g
comm -23 f g

#### -13 only in the second
printf 'a\nb\nd\n' > f
printf 'b\nc\nd\ne\n' > g
comm -13 f g

#### -3
printf 'a\nb\nd\n' > f
printf 'b\nc\nd\ne\n' > g
comm -3 f g

#### stdin as one file
printf 'a\nb\n' > f
printf 'b\nc\n' | comm f -

#### with process substitution and sort
comm -12 <(printf 'z\nx\ny\n' | sort) <(printf 'y\nw\nz\n' | sort)

#### --output-delimiter
## expect: gnu (BSD comm has no long options)
printf 'a\nb\n' > f
printf 'b\nc\n' > g
comm --output-delimiter=, f g

#### --total
## expect: gnu (BSD comm has no --total)
printf 'a\nb\nd\n' > f
printf 'b\nc\nd\ne\n' > g
comm --total f g

#### unsorted input is reported
## expect: gnu (BSD comm does not check the order)
printf 'b\na\n' > f
printf 'b\nc\n' > g
comm f g
echo "status=$?"

#### --nocheck-order
## expect: gnu (BSD comm has no long options)
printf 'b\na\n' > f
printf 'b\nc\n' > g
comm --nocheck-order f g
echo "status=$?"

#### a last line without a newline
printf 'a\nb' > f
printf 'b\n' > g
comm f g

#### -i ignores case
## expect: bsd (GNU comm has no -i)
printf 'A\nb\n' > f
printf 'a\nB\n' > g
comm -i f g

#### one operand is an error
printf 'a\n' > f
comm f
echo "status=$?"
