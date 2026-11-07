# tr: translate, delete, squeeze; ranges, classes, escapes.

#### lower to upper with ranges
echo 'hello world' | tr a-z A-Z

#### classes
echo 'Hello World 42' | tr '[:upper:]' '[:lower:]'
echo 'Hello World 42' | tr '[:lower:]' '[:upper:]'

#### delete
echo 'a1b2c3' | tr -d '0-9'

#### delete a class
echo 'a1 b2, c3!' | tr -d '[:punct:]'

#### delete carriage returns
printf 'a\r\nb\r\n' | tr -d '\r'

#### newlines to spaces
printf 'a\nb\nc\n' | tr '\n' ' '; echo

#### squeeze
echo 'a    b     c' | tr -s ' '

#### squeeze while translating
echo 'aabbccdd' | tr -s 'a-c' 'x'

#### complement and delete
echo 'abc-123_XYZ!' | tr -cd '[:alnum:]'; echo

#### complement and squeeze into newlines
echo 'one, two;  three' | tr -cs '[:alpha:]' '\n'

#### complement translate
echo 'ab1cd2' | tr -c 'a-z\n' '_'

#### octal escapes
echo 'a:b:c' | tr ':' '\011'
printf 'a\000b\n' | tr '\000' '-'

#### backslash escapes
printf 'a\tb\n' | tr '\t' ','

#### set2 shorter than set1 is padded with its last character
echo 'abcde' | tr 'a-e' 'xy'

#### -t truncates set1
## expect: gnu (BSD tr has no -t)
echo 'abcde' | tr -t 'a-e' 'xy'

#### repeat construct
echo 'abcdef' | tr 'a-f' '[x*3]y'
echo 'abcdef' | tr 'a-f' 'z[-*]'

#### brackets are literal outside constructs
echo '[a]' | tr '[a-z]' '[A-Z]'

#### -d and -s together
echo 'aa11bb22' | tr -ds '0-9' 'a-z'

#### squeeze alone with complement
echo 'a..b,,c' | tr -cs 'a-z' '#'

#### delete the space class
printf ' a \t b\n' | tr -d '[:space:]'; echo

#### digits to letters
echo 2024 | tr 0-9 a-j

#### reversed range
## expect: gnu (BSD tr accepts a reversed range)
echo x | tr z-a x
echo "status=$?"

#### missing operand
tr a < /dev/null
echo "status=$?"

#### invalid class
echo x | tr '[:nope:]' x
echo "status=$?"

#### delete with two sets is an error
echo x | tr -d a b
echo "status=$?"

#### UTF-8 characters in a set are characters
## expect: bsd (D1162: a non-ASCII set works on characters; GNU works on its bytes)
echo 'café' | LC_ALL=en_US.UTF-8 tr 'é' 'e'

#### ASCII sets leave UTF-8 text intact
echo 'héllo wörld' | tr a-z A-Z
echo 'héllo' | tr -d 'l'
