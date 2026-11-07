# awk: fields, patterns, variables, the common one-liners.

#### print a field
echo 'a b c' | awk '{ print $2 }'

#### NF, NR and $NF
printf 'a b c\nd e\n' | awk '{ print NR, NF, $NF }'

#### sum a column in END
printf '1 x\n2 y\n3 z\n' | awk '{ s += $1 } END { print s, NR }'

#### a pattern without an action prints the record
printf 'apple\nbanana\ncherry\n' | awk '/an/'
printf '1\n5\n10\n' | awk '$1 > 4'

#### a range pattern
printf 'a\nstart\nb\nend\nc\n' | awk '/start/,/end/'

#### assigning a field rebuilds the record with OFS
echo 'a b c' | awk -v OFS=- '{ $2 = "X"; print; print NF }'

#### NF assignment truncates and extends
echo 'a b c d' | awk '{ NF = 2; print; NF = 4; print }' OFS=,

#### -F with one character, a tab and a regex
echo 'a:b:c' | awk -F: '{ print $3 $1 }'
printf 'a\tb c\td\n' | awk -F'\t' '{ print $2 }'
echo 'a1b22c333d' | awk -F'[0-9]+' '{ print $1, $2, $3, $4 }'

#### -F with several characters is a regex
echo 'a::b::c' | awk -F'::' '{ print $2, NF }'
echo 'a, b,c' | awk -F', *' '{ print $3 }'

#### -F '[][]' splits on brackets
echo 'x[1]y[2]' | awk -F'[][]' '{ print $2, $4, NF }'

#### -F t is a tab
## expect: gnu (D016: GNU on conflict; gawk takes -Ft as the letter t, BSD awk as a tab)
printf 'a\tb\n' | awk -Ft '{ print $2 }'

#### FS set in BEGIN
echo 'a|b|c' | awk 'BEGIN { FS = "|" } { print $2 }'

#### -v assigns before BEGIN, with escapes
awk -v x=5 -v 'msg=a\tb' 'BEGIN { print x * 2; print msg }'

#### var=value operands are assigned when reached
printf 'l1\n' > f1
printf 'l2\n' > f2
awk '{ print v, $0 }' v=1 f1 v=2 f2

#### -f reads the program from a file
printf '{ print toupper($1) }\n' > prog.awk
echo 'abc def' | awk -f prog.awk

#### -- ends the options
echo x | awk -- '{ print "got", $0 }'

#### uninitialized variables are 0 and ""
awk 'BEGIN { print x + 0, "[" x "]", length(x) }'

#### string and number comparisons
awk 'BEGIN { print (10 < 9), ("10" < "9"), (2 == 2.0), ("a" < "b") }'
echo '10 9' | awk '{ print ($1 > $2) }'

#### increment, compound assignment, exponent, modulo
awk 'BEGIN { x = 5; x++; x += 2; y = x ^ 2; print x, y, y % 7, -2 ^ 2 }'

#### the ternary operator and logical operators
awk 'BEGIN { a = 1; b = 0; print (a && !b) ? "yes" : "no", (a || b), !a }'

#### string concatenation and number output format
## expect: gnu (D016: gawk prints an integral 1e30 with all its digits, BSD with %.30g)
awk 'BEGIN { x = 0.1 + 0.2; print x, x "" , 1e6, 1e30, 100/3 }'

#### OFMT and CONVFMT
awk 'BEGIN { OFMT = "%.2f"; CONVFMT = "%.3f"; x = 3.14159; print x; print x "" }'

#### exit status and END after exit
echo a | awk '{ exit 3 } END { print "end" }'; echo "status $?"
awk 'BEGIN { exit }'; echo "status $?"

#### next skips the rest of the actions
printf '1\n2\n3\n' | awk '$1 == 2 { next } { print }'

#### while, do, for and break/continue
awk 'BEGIN { i = 0; while (i < 3) i++; do i--; while (i > 1); for (j = 0; j < 5; j++) { if (j == 1) continue; if (j == 3) break; print i, j } }'

#### functions with locals and recursion
awk 'function fact(n) { return n <= 1 ? 1 : n * fact(n - 1) } function f(a,   loc) { loc = a * 2; return loc } BEGIN { print fact(10), f(4), loc }'

#### arrays passed by reference
awk 'function fill(arr, n,   i) { for (i = 1; i <= n; i++) arr[i] = i * i } BEGIN { fill(sq, 4); print sq[3], length(sq) }'

#### RS as a paragraph separator
printf 'a b\nc\n\n\nd e\n' | awk 'BEGIN { RS = "" } { print NR ": " $1 "-" $NF " (" NF ")" }'

#### RS as one character and as a regex
## expect: gnu (RT is gawk's; BSD awk has no RT)
printf 'a;b;c' | awk -v RS=';' '{ print NR, $0 }'
printf 'a12b345c' | awk -v RS='[0-9]+' '{ print NR, $0, RT }'

#### ORS and OFS
printf 'a b\nc d\n' | awk -v ORS=';' -v OFS=: '{ print $1, $2 } END { printf "\n" }'

#### FILENAME and FNR across files
printf 'x\ny\n' > a
printf 'z\n' > b
awk '{ print FILENAME, FNR, NR }' a b

#### ENVIRON
FOO=bar awk 'BEGIN { print ENVIRON["FOO"] }'

#### ARGV and ARGC
awk 'BEGIN { for (i = 0; i < ARGC; i++) print i, ARGV[i] }' one two
