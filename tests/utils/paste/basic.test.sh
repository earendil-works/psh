# paste: columns, -s, -d.

#### two files side by side
printf '1\n2\n3\n' > a
printf 'x\ny\n' > b
paste a b

#### -d with one delimiter
printf '1\n2\n' > a
printf 'x\ny\n' > b
paste -d, a b a

#### -d cycles through the list
printf '1\n2\n' > a
paste -d ':-' a a a

#### -s joins lines
printf '1\n2\n3\n' | paste -sd+ -

#### summing with bc's input form
## expect: gnu (BSD paste needs a file operand)
printf '4\n5\n6\n' | paste -sd+

#### -s with several delimiters and files
printf '1\n2\n3\n' > a
printf 'x\ny\n' > b
paste -s -d '+-' a b

#### -s with the default tab
## expect: gnu (BSD paste needs a file operand)
printf 'a\nb\n' | paste -s

#### - - reads stdin in turns
printf '1\n2\n3\n4\n5\n' | paste - -

#### a last line without a newline
## expect: gnu (BSD paste -s does not end with a newline)
printf 'x\ny' > b
paste -s -d, b

#### the \0 delimiter is empty
printf '1\n2\n' > a
printf 'x\ny\n' > b
paste -d '\0' a b

#### \n and \t in the list
printf '1\n2\n' > a
paste -d '\t\n' a a a

#### -s of an empty input
## expect: gnu (BSD paste -s prints nothing)
paste -s /dev/null | wc -l

#### a missing file
paste missing
echo "status=$?"
