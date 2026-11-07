# sed: regular expressions and replacements.

#### BRE groups and back-references
echo 'hello world' | sed 's/\(hello\) \(world\)/\2 \1/'
echo abab | sed -n '/\(ab\)\1/p'

#### ERE with -E and -r
echo 'hello world' | sed -E 's/(hello) (world)/\2 \1/'
echo aaa | sed -r 's/a+/X/'

#### & and escaped &
echo ab | sed 's/a/[&][\&]/'

#### GNU BRE extensions
## expect: gnu (BSD sed's BREs have no \+ \? \|)
echo aaa | sed 's/a\+/X/'
echo cat | sed 's/dog\|cat/X/'
echo ab | sed 's/x\?b/X/'

#### GNU escapes \w \s \b \< \>
## expect: gnu (BSD sed has no \w \s \b \< \>)
echo 'a b' | sed 's/\s/_/;s/\w/W/'
echo 'ab ab' | sed 's/\<a/X/g'
echo 'cat concat' | sed 's/\bcat\b/X/g'

#### intervals
echo aaa | sed 's/a\{2\}/X/'
echo aaa | sed -E 's/a{2}/X/'

#### case conversion
## expect: gnu (BSD sed has no \U \L \u \l \E)
echo 'hello world' | sed -E 's/(\w+) (\w+)/\u\1 \U\2\E!/'
echo 'foo bar' | sed 's/[a-z]\+/\u&/g'
echo 'Hello' | sed 's/.*/\L&/'

#### newline and tab in the replacement
## expect: gnu (BSD sed has no \n or \t in replacements)
echo a.b | sed 's/\./\n/'
echo ab | sed 's/a/\t/' | od -c | head -1

#### escaped newline in the replacement
echo ab | sed 's/a/x\
y/'

#### newline in the pattern space
printf 'a\nb\n' | sed 'N;s/\n/,/'

#### other delimiters
echo 'a/b' | sed 's|/|_|'
echo 'a|b' | sed 's|a\|b|X|'
printf 'a/b\nc\n' | sed -n '\,a/b,p'

#### empty matches with g
echo abc | sed 's/x*/-/g'
echo baaac | sed 's/a*/x/g'
echo hello | sed 's/l*/X/g'

#### anchors only at the start in g
echo aaa | sed 's/^a/X/g'

#### the empty regex reuses the last one
echo abc | sed '/b/s//X/'

#### I flag
## expect: gnu (BSD sed has no I flag)
echo ABC | sed 's/b/x/I'
printf 'ABC\nx\n' | sed -n '/b/Ip'

#### M flag
## expect: gnu (BSD sed has no M flag)
printf 'a\nb\n' | sed 'N;s/^/>/Mg'

#### bracket expressions
echo 'a]b' | sed 's/[]]/X/'
echo 'a b' | sed 's/[[:space:]]/X/'
echo 'a/b' | sed 's/[/]/X/'

#### dot is a char in UTF-8
LC_ALL=C.UTF-8 ; export LC_ALL
echo 'é' | sed 's/./X/g'
