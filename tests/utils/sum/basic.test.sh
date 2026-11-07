# The checksum utilities: GNU names (md5sum, sha*sum), BSD md5, perl shasum.

#### GNU names print HASH  FILE
printf 'hello\n' > a
printf 'x' > b
: > e
md5sum a b
sha1sum a
sha224sum a
sha256sum a e
sha384sum a
sha512sum e

#### stdin is -
printf 'x' > b
echo hi | md5sum
sha256sum - < b

#### -b marks binary, --tag writes BSD lines
printf 'hello\n' > a
md5sum -b a
md5sum --tag a
sha256sum --tag a

#### -z ends lines with NUL
printf 'hello\n' > a
md5sum -z a | tr '\0' @

#### a missing file or a directory
## expect: gnu (D016: GNU's messages quote names)
printf 'hello\n' > a
mkdir d
md5sum nope a d
echo "st=$?"
md5sum 'no such'
echo "st=$?"

#### a name with a newline is escaped
## expect: gnu (BSD md5sum does not escape names)
printf 'a\nb' > "$(printf 'n\nl')"
md5sum n*
md5sum n* | md5sum -c
echo "st=$?"
# The case's directory is recorded; a newline would break its listing.
rm n*

#### -c verifies
printf 'hello\n' > a
printf 'x' > b
md5sum a b > sums
md5sum -c sums
echo "st=$?"
printf 'changed' > a
md5sum -c sums
echo "st=$?"

#### -c --quiet and --status
printf 'hello\n' > a
printf 'x' > b
md5sum a b > sums
md5sum -c --quiet sums
echo "st=$?"
printf 'changed' > a
md5sum -c --quiet sums
echo "st=$?"
md5sum --status -c sums
echo "st=$?"

#### -c reports bad lines
## expect: gnu (D016: BSD md5sum warns about blank lines and comments)
printf 'hello\n' > a
md5sum a > sums
printf 'garbage\n\n# comment\n' >> sums
md5sum -c sums
echo "st=$?"
md5sum -c --strict sums
echo "st=$?"
md5sum -c -w sums
echo "st=$?"
echo junk > j
md5sum -c j
echo "st=$?"

#### -c a missing file
## expect: gnu (D016: GNU's FAILED open or read)
printf 'hello\n' > a
printf 'x' > b
md5sum a b > sums
rm b
md5sum -c sums
echo "st=$?"
md5sum -c --ignore-missing sums
echo "st=$?"

#### -c from stdin, both line formats
## expect: gnu (BSD md5sum -c does not read stdin)
printf 'hello\n' > a
md5sum --tag a | md5sum -c
md5sum a | md5sum -c -
echo 'b1946ac92492d2347c6235b4d2611184 a' | md5sum -c
printf 'b1946ac92492d2347c6235b4d2611184  a\r\n' | sha1sum -c
echo "st=$?"
: | md5sum -c
echo "st=$?"

#### -c counts failures
## expect: gnu (BSD md5sum -c does not read stdin)
printf 'hello\n' > a
printf 'x' > b
printf '%s  a\n%s  b\n' 00000000000000000000000000000000 00000000000000000000000000000000 | md5sum -c
echo "st=$?"

#### options that need -c
## expect: gnu (D016: GNU's usage errors)
printf 'hello\n' > a
md5sum --quiet a
echo "st=$?"
md5sum --tag -c a
echo "st=$?"

#### sha256sum -c
printf 'hello\n' > a
sha256sum a > s
sha256sum -c s
sha512sum a > s
sha512sum -c s

#### md5 (BSD)
## expect: bsd (GNU has no md5)
printf 'hello\n' > a
md5 a
md5 -r a
md5 -q a
echo hi | md5
md5 -s abc
md5 nope
echo "st=$?"

#### md5 -c compares with a hash (BSD)
## expect: bsd (GNU has no md5)
printf 'hello' > a
md5 -c 5d41402abc4b2a76b9719d911017c592 a
echo "st=$?"
md5 -c 0000 a
echo "st=$?"

#### md5 -p passes stdin through (BSD)
## expect: bsd (GNU has no md5)
echo hi | md5 -p

#### shasum
printf 'hello\n' > a
shasum a
shasum -a 1 a
shasum -a 224 a
shasum -a 256 a
shasum -a 384 a
shasum -a 512 -b a
shasum -a 512224 a
shasum -a 512256 a
shasum --tag a
echo hi | shasum

#### shasum -c takes the algorithm from the hash
printf 'hello\n' > a
shasum a > s1
shasum -c s1
echo "st=$?"
sha256sum a | shasum -c
echo "st=$?"
shasum -a 512 a | shasum -a 1 -c
echo "st=$?"

#### shasum -c failures
printf 'hello\n' > a
printf '%s  a\n' 0000000000000000000000000000000000000000 | shasum -c
echo "st=$?"
printf '%s  zz\n' 0000000000000000000000000000000000000000 | shasum -c
echo "st=$?"
printf 'junk\n' | shasum -c
echo "st=$?"

#### shasum errors
mkdir d
shasum nope
echo "st=$?"
shasum d
echo "st=$?"
shasum -a 7 d
echo "st=$?"
