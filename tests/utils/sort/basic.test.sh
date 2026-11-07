# sort: orders, keys, unique, output. Collation is byte order (D1160); the cases run
# with LC_ALL=C.UTF-8, where GNU and BSD sort by bytes too.

#### default byte order
printf 'b\nB\na\nA\n_x\n-y\n10\n9\n' | sort

#### -r
printf 'b\na\nc\n' | sort -r

#### -n
printf '10\n9\n-1\n1.5\n0\n100\n' | sort -n

#### -n with non-numbers and ties
printf 'x\n-1\n1a\n\n 3\n1.5\n-0\n0\n1,000\n' | sort -n

#### -rn, the frequency idiom
## expect: gnu (D016: uniq -c widths)
printf 'x\ny\nx\nz\nx\ny\n' | sort | uniq -c | sort -rn

#### -u
printf 'b\na\nb\na\nc\n' | sort -u

#### -u with -n keeps the first of equal numbers
printf '1\n01\n2\n1.0\n' | sort -nu

#### -k field
printf 'a 3\nb 1\nc 2\n' | sort -k2

#### -k with -n
printf 'x 10\ny 9\nz 100\n' | sort -k2 -n

#### -k with its own options
printf 'x 10\ny 9\nz 100\n' | sort -k2,2nr

#### -t and -k
printf 'c:3\na:1\nb:2\n' | sort -t: -k2

#### -t with a numeric key
printf 'u:x:1000\nr:x:0\nd:x:1\n' | sort -t: -k3,3n

#### several keys
printf 'b 1\na 2\na 1\nb 2\n' | sort -k1,1 -k2,2nr

#### a key with a character offset
printf 'x1b\ny1a\nz0c\n' | sort -k1.3

#### a key ending in the middle of a field
printf 'ab 2\naa 1\nab 1\n' | sort -k1,1.1 -k2,2n

#### keys without -t include leading blanks
printf 'a b\na  a\n' | sort -k2

#### -b ignores them
printf 'x  2\ny 1\nz   3\n' | sort -b -k2

#### a key's own options replace the global ones
printf '1 b\n10 a\n2 c\n' | sort -n -k1b

#### -r reverses the last resort too
printf 'b 2\na 2\nc 1\n' | sort -r -k2,2n

#### -s keeps input order of equal keys
printf 'b 1\na 1\nc 0\n' | sort -s -k2,2n

#### -f folds case
printf 'B\na\nb\nA\n' | sort -f

#### -d dictionary order
printf 'b-1\na_2\n#c\n' | sort -d

#### -h human sizes
printf '1K\n2M\n500\n1G\n1.5K\n' | sort -h

#### du-style -h with a key
printf '4.0K\tsrc\n1.2M\ttarget\n512\tREADME\n' | sort -rh

#### -g general numbers
printf '1e3\n5\n-inf\n0.5\n' | sort -g

#### -M months
printf 'Mar\njan\nFeb\nDec\n' | sort -M

#### -V versions
printf 'v1.10\nv1.2\nv1.9\nv1.2.1\n' | sort -V

#### -V with file names
printf 'file.tar.gz\nfile1.txt\nfile.txt\na-1.2~rc\na-1.2\n' | sort -V

#### -R shuffles but keeps all lines
printf '1\n2\n3\n4\n5\n' | sort -R | sort -n

#### -z
printf 'b\0a\0c\0' | sort -z | tr '\0' '\n'

#### -o writes a file, which may be an input
printf 'b\na\n' > f
sort -o f f
cat f

#### several files
printf 'b\nd\n' > f
printf 'a\nc' > g
sort f g

#### -m merges sorted files
printf 'a\nc\n' > f
printf 'b\nd\n' > g
sort -m f g

#### -c finds disorder
printf 'a\nc\nb\n' | sort -c
echo "status=$?"

#### -C is silent
printf 'a\nc\nb\n' | sort -C
echo "status=$?"
printf 'a\nb\n' | sort -C
echo "status=$?"

#### --sort=version
## expect: gnu (BSD sort has no --sort=version)
printf 'v10\nv9\n' | sort --sort=version

#### a last line without a newline
printf 'b\na' | sort

#### empty input
sort < /dev/null
echo "status=$?"

#### a missing file
sort missing
echo "status=$?"

#### a bad key
sort -k0 < /dev/null
echo "status=$?"

#### a multi-character separator
sort -t ab < /dev/null
echo "status=$?"

#### sort | uniq -c | sort -rn | head, as agents count
## expect: gnu (D016: uniq -c widths)
printf 'GET /a\nGET /b\nGET /a\nPOST /a\nGET /a\n' | cut -d' ' -f2 | sort | uniq -c | sort -rn | head -n 2

#### -S and --parallel are accepted
printf 'b\na\n' | sort -S 1M --parallel=2
