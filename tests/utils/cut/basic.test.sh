# cut: bytes, characters, fields.

#### one field with a delimiter
echo 'a:b:c' | cut -d: -f2

#### several fields and ranges
echo 'a:b:c:d:e' | cut -d: -f1,3-4
echo 'a:b:c:d:e' | cut -d: -f3-
echo 'a:b:c:d:e' | cut -d: -f-2

#### tab is the default delimiter
printf 'a\tb\tc\n' | cut -f2

#### lines without the delimiter print whole
printf 'a:b\nnodelim\n' | cut -d: -f2

#### -s drops lines without the delimiter
printf 'a:b\nnodelim\n' | cut -s -d: -f2

#### a field past the end is empty
echo 'a:b' | cut -d: -f5

#### fields in list order are printed in input order
echo 'a:b:c' | cut -d: -f3,1

#### characters
echo 'abcdefgh' | cut -c2-4
echo 'abcdefgh' | cut -c-3
echo 'abcdefgh' | cut -c6-
echo 'abcdefgh' | cut -c1,3,5

#### bytes
echo 'abcdef' | cut -b1-3

#### several lines and files
printf 'k1=v1\nk2=v2\n' > f
cut -d= -f1 f
cut -d= -f2 f - < f

#### the last line without a newline
printf 'a:b' | cut -d: -f1

#### /etc/passwd style
printf 'root:x:0:0:root:/root:/bin/bash\nbin:x:1:1::/bin:/sbin/nologin\n' | cut -d: -f1,7

#### a space delimiter
echo 'one two three' | cut -d ' ' -f2

#### --complement
## expect: gnu (BSD cut has no long options)
echo 'a:b:c' | cut -d: --complement -f2

#### --output-delimiter
## expect: gnu (BSD cut has no long options)
echo 'a:b:c' | cut -d: -f1,3 --output-delimiter=' '
echo 'abcdef' | cut -c1-2,4-5 --output-delimiter=:

#### -w splits on blank runs
## expect: bsd (GNU cut has no -w)
printf 'a   b\tc\n' | cut -w -f2,3

#### -c counts characters on UTF-8
## expect: bsd (D1161: cut -c counts characters; GNU counts bytes)
echo 'héllo wörld' | LC_ALL=en_US.UTF-8 cut -c1-4

#### -b counts bytes
## expect: gnu (BSD cut -b on UTF-8 differs)
printf 'h\303\251llo\n' | cut -b1-3 | wc -c

#### a missing file
echo x > f
cut -c1 f missing
echo "status=$?"

#### no list
echo x | cut
echo "status=$?"

#### field zero
## expect: gnu (BSD's message differs)
echo x | cut -f0
echo "status=$?"

#### a decreasing range
## expect: gnu (BSD cut accepts a decreasing range)
echo x | cut -f3-1
echo "status=$?"

#### a delimiter with -c
echo x | cut -c1 -d:
echo "status=$?"

#### a long delimiter
echo x | cut -d ab -f1
echo "status=$?"
