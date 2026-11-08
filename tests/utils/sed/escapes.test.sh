# sed: GNU's escapes in regexes, replacements and y (\xHH \dNNN \oNNN \cX \n \t ...).

#### \xHH matches a byte, even inside a UTF-8 char
## expect: gnu (BSD sed has no \x escapes)
export LC_ALL=C.UTF-8
printf 'echo \316\274 x\n' | sed 's/\xce//g' | cat -v
printf '\316\274a\n' | sed 's/\xce\xbc/mu/'

#### \xHH in the C locale
## expect: gnu (BSD sed has no \x escapes)
export LC_ALL=C
printf 'a\316\274b\n' | sed 's/\xce//' | cat -v

#### \xHH makes a char that is then special
## expect: gnu (BSD sed has no \x escapes)
echo 'a.b axb' | sed 's/a\x2eb/X/g'
echo 'a-b' | sed 's/[\x2d]/X/'

#### decimal, octal and control escapes
## expect: gnu (BSD sed has no \d \o \c escapes)
echo abc | sed 's/\d098/X/;s/\o143/Y/'
echo a | sed 's/a/\cA/' | cat -v

#### escapes in the replacement
## expect: gnu (BSD sed has no \x \t escapes in replacements)
echo a | sed 's/a/\x41\d066\o103/'
echo ab | sed 's/a/\t/' | cat -A
printf '\316\274\n' | sed 's/.*/[\xce\xbc]/'

#### escapes in addresses and y
## expect: gnu (BSD sed has no \x escapes)
printf 'a\nb\n' | sed -n '/\x62/p'
echo abc | sed 'y/\x61\o142/zy/'
printf 'a\tb\n' | sed 'y/\t/T/'

#### \n \t \r in regexes
## expect: gnu (BSD sed has no \t in regexes)
printf 'a\tb\n' | sed 's/\t/T/'
printf 'a\nb\n' | sed 'N;s/a\nb/joined/'
printf 'a\r\n' | sed 's/\r$//' | cat -A
