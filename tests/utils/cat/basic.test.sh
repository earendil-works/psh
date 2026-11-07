# cat: plain copies, numbering, squeezing, showing what is invisible.

#### copy a file and stdin
printf 'one\ntwo\n' > f
echo mid | cat f - f

#### a file without a final newline
printf 'x\ny' > f
cat f f; echo

#### -n numbers lines across files
## expect: gnu (BSD cat numbers each file from 1)
printf 'a\nb\n' > f
printf 'c' > g
cat -n f g; echo

#### -b numbers non-blank lines
printf 'a\n\nb\n' | cat -b

#### -s squeezes blank lines
printf 'a\n\n\n\nb\n\n' | cat -s

#### -e shows line ends and control characters
printf 'a\tb\001\r\n' | cat -e

#### -t shows tabs and control characters
printf 'a\tb\001\n' | cat -t

#### -v shows control and high bytes
## expect: gnu (BSD cat -v prints invalid UTF-8 bytes as they are in a UTF-8 locale)
printf 'x\001\177\200\377\n' | cat -v

#### -A shows everything
## expect: gnu (BSD cat has no -A)
printf 'a\tb\001\200\r\n' | cat -A

#### -E and -T
## expect: gnu (BSD cat has no -E or -T)
printf 'a\tb\n' | cat -E -T

#### --number and --squeeze-blank
## expect: gnu (long options)
printf 'a\n\n\nb\n' | cat --number --squeeze-blank

#### a missing file is reported, the rest is printed
printf 'ok\n' > f
cat missing f
echo "status=$?"

#### a directory
mkdir d
cat d
echo "status=$?"

#### cat to a closed pipe
yes | cat | head -n 2
