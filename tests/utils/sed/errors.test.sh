# sed: GNU's error messages and statuses.

#### unterminated s
## expect: gnu (GNU's messages)
echo | sed 's/a/b'; echo $?

#### unknown command
## expect: gnu (GNU's messages)
echo | sed 'k'; echo $?

#### unknown option to s
## expect: gnu (GNU's messages)
echo | sed 's/a/b/k'; echo $?

#### missing command
## expect: gnu (GNU's messages)
echo | sed '1'; echo $?

#### unmatched braces
## expect: gnu (GNU's messages)
echo | sed '{p'; echo $?
echo | sed 'p;}'; echo $?

#### extra characters
## expect: gnu (GNU's messages)
echo | sed 'p x'; echo $?

#### bad regex
## expect: gnu (GNU's messages)
echo | sed 's/\(/x/'; echo $?
echo | sed -E 's/(/x/'; echo $?

#### invalid reference
## expect: gnu (GNU's messages)
echo | sed 's/a/\1/'; echo $?

#### missing label
## expect: gnu (GNU's messages)
echo | sed 'b foo'; echo $?

#### errors in -f and later -e
## expect: gnu (GNU's messages)
printf 'p\nk\n' > s.sed
echo | sed -f s.sed; echo $?
echo | sed -e p -e 'k'; echo $?

#### addresses where none are allowed
## expect: gnu (GNU's messages)
seq 2 | sed '1# foo'; echo $?
seq 2 | sed '1,2q'; echo $?
seq 2 | sed -n '1!!p'; echo $?

#### y lengths
## expect: gnu (GNU's messages)
echo | sed 'y/ab/c/'; echo $?

#### missing input file
## expect: gnu (GNU's messages)
echo a > f
sed p nosuch f; echo $?

#### no previous regex
## expect: gnu (GNU's messages)
echo a | sed 's//x/'; echo $?

#### no script
## expect: gnu (GNU's usage text is longer; the status matters)
sed 2>/dev/null; echo $?
