# grep, egrep, fgrep in-process: GNU grep 3.11 (the recording container) and BSD grep
# (macOS). Where the two disagree psh follows GNU (D016), named per case. GNU's
# recursion order is the directory's, psh's is sorted by name: recursive cases sort.

#### lines, line numbers, file names
printf 'alpha\nbeta\nalphabet\n' > f
grep alp f; grep -n alp f; grep -H alp f; cp f g; grep beta f g; grep -h beta f g

#### -c, -l, -L, -q and the exit status
printf 'a\nb\na\n' > f; printf 'x\n' > g
grep -c a f g; grep -l a f g; grep -L a f g; echo "L=$?"
grep -q a f; echo "q=$?"; grep -q zz f; echo "q=$?"; grep zz f; echo "none=$?"

#### -v, -i, -w, -x
printf 'foo bar\nfoobar\nFOO\nfoo_x\nx foo\n' > f
grep -v foo f; echo --; grep -i foo f; echo --; grep -w foo f; echo --; grep -x FOO f; grep -ix foo f

#### -E, -F and the BRE operators
printf 'ab\naab\na+b\na|b\n(a)\n' > f
grep 'a\+b' f; echo --; grep 'a+b' f; echo --; grep -E 'a+b' f; echo --; grep -F 'a|b' f
echo --; grep -E '^(ab|\(a\))$' f; echo --; grep 'a\|(' f

#### egrep and fgrep
printf 'ab\na.b\n' > f
egrep 'a.b|ab' f; fgrep a.b f

#### brackets, classes and anchors
printf 'a1\nb 2\nC\n\n' > f
grep '[[:digit:]]' f; grep -c '[[:space:]]' f; grep '^[[:upper:]]$' f; grep -c '^$' f; grep '[^a-z0-9 ]' f

#### back-references
printf 'abab\nabcd\nxyzzy\n' > f
grep '\(ab\)\1' f; grep -E '(z)\1' f; grep -o '\(.\)\1' f

#### multiple -e, a newline in a pattern, -f
## expect: gnu (D016: GNU reads no file when -f gives no pattern)
printf 'one\ntwo\nthree\n' > f; printf 'one\nthree\n' > pats
grep -e one -e two f; echo --; grep 'one
three' f; echo --; grep -f pats f; echo --; grep -c -f /dev/null f; echo "st=$?"

#### an empty pattern matches every line
printf 'a\n\nb\n' > f
grep -c '' f; grep -c -e a -e '' f; printf '\n' > p; grep -cf p f

#### -o: every match, leftmost-longest
printf 'xabcdy ab\none two\n' > f
grep -o 'ab[a-z]*' f; grep -oE 'ab|abcd' f; grep -o '[a-z]*' f; grep -ob 'o' f

#### -o with -w, -i and several patterns
printf 'foobar foo foo_ FOO\n' > f
grep -ow 'foo\|foobar' f; grep -oi foo f; grep -o -e foo -e foob f

#### context: -A, -B, -C, -NUM and group separators
## expect: gnu (D016: GNU separates groups also with -A0)
printf '1a\n2\n3\n4a\n5\n6\n7\n8a\n9\n' > f
grep -A1 a f; echo ==; grep -B1 4a f; echo ==; grep -C1 -n 4a f; echo ==; grep -1 8a f; echo ==; grep -A0 a f

#### context across files
printf '1\n2a\n3\n' > f; cp f g
grep -C1 a f g

#### --group-separator, --no-group-separator
## expect: gnu (BSD grep has no group separator options)
printf '1a\n2\n3a\n' > f
grep --group-separator=XX -A0 a f; grep --no-group-separator -A0 a f

#### -m, also with context and -c
printf 'a1\nb\na2\nc\na3\n' > f
grep -m1 a f; grep -m2 -A1 a f; grep -c -m2 a f; grep -v -m1 a f

#### -b byte offsets
printf 'ab\ncab\n' > f
grep -b ab f; grep -bo b f

#### -Z after file names
## expect: gnu (D016: BSD's -Z is decompression)
printf 'a\n' > f; cp f g
grep -Z a f g | od -c | sed 's/  */ /g; s/ $//'; grep -lZ a f g | od -c | sed 's/  */ /g; s/ $//'

#### -z: NUL-terminated lines
printf 'one\0two\0one two\0' | grep -z one | od -c | sed 's/  */ /g; s/ $//'

#### stdin, -, --label
printf 'x\n' > f
echo x | grep x; echo x | grep -H x; echo x | grep x - f; echo x | grep --label=IN -c x - f

#### -s and errors: a missing file, a directory
printf 'x\n' > f; mkdir d
grep x nope f; echo "st=$?"; grep -s x nope; echo "st=$?"; grep x d; echo "st=$?"; grep -d skip x d f; echo "st=$?"

#### -q succeeds despite an error once a line is selected
printf 'x\n' > f
grep -q x nope f; echo "st=$?"; grep -q x f nope; echo "st=$?"; grep -sq y nope f; echo "st=$?"

#### pattern errors and usage
## expect: gnu (D016: GNU's messages and status 2)
grep 'a\{1' /dev/null; echo "st=$?"; grep -E '(' /dev/null; echo "st=$?"; grep '[:space:]' /dev/null; echo "st=$?"
grep; echo "st=$?"; grep -A x a /dev/null; echo "st=$?"; grep -e; echo "st=$?"

#### ERE: a leading repetition operator, a literal brace
## expect: gnu (D016: GNU's ERE extensions and warnings)
printf 'a\n*a\nx{1\n' > f
grep -E '*a' f; echo --; grep -E 'x{1' f

#### binary files
## expect: gnu (D016: GNU 3.11 reports a binary match on stderr)
printf 'text a\nbin\0ary a\n' > b; printf 'a\n' > t
grep a b; echo "st=$?"; grep -c a b; grep -l a b; grep -a a b | od -c | sed 's/  */ /g; s/ $//'; grep -I a b t; echo "st=$?"

#### -r: hidden files searched, .gitignore not read, ./ only when given
## expect: gnu (D016: GNU leaves ./ out of the implicit path)
mkdir -p d/sub d/.hid && printf 'hit\n' > d/a.txt && printf 'hit\n' > d/sub/b.rs
printf 'hit\n' > d/.hid/c && printf '*.txt\n' > d/.gitignore
grep -r hit d | sort; echo --; cd d; grep -r hit . | sort; echo --; grep -rl hit | sort

#### -r: one file operand has no name
## expect: gnu (D016: GNU names no file for one file operand, and no ./ for the implicit path)
mkdir d && printf 'hit\n' > d/f
grep -r hit d/f; grep -rc hit d; cd d && grep -r hit

#### --include, --exclude, --exclude-dir
mkdir -p d/sub d/skip && for f in d/a.txt d/b.rs d/sub/c.txt d/skip/e.txt; do echo hit > $f; done
grep -r --include='*.txt' hit d | sort; echo --; grep -r --exclude='*.txt' hit d | sort
echo --; grep -r --exclude-dir=skip hit d | sort; echo --; grep -rl --exclude-dir=sub --exclude-dir=skip --include='*.rs' hit d

#### --include and --exclude: the last matching glob wins
## expect: gnu (D016: BSD's --exclude also wins over a later --include)
mkdir d && for f in d/a.txt d/b.txt d/c.rs; do echo hit > $f; done
grep -r --include='*.txt' --exclude='a*' hit d | sort; echo --; grep -r --exclude='a*' --include='*.txt' hit d | sort

#### --include on command-line files, braces are literal
printf 'hit\n' > a.txt; printf 'hit\n' > b.rs
grep --include='*.txt' hit a.txt b.rs; grep --exclude='*.txt' hit a.txt b.rs; grep -c --include='*.{txt,rs}' hit a.txt b.rs; echo "st=$?"

#### -r: symbolic links followed with -R only
## expect: gnu (D016: BSD's -r and -R follow no links found while recursing)
mkdir -p d/real && printf 'hit\n' > d/real/f && ln -s real d/link && ln -s real/f d/flink
grep -r hit d | sort; echo --; grep -R hit d | sort; echo --; grep -r hit d/link

#### --color=always
## expect: gnu (D016: GNU's GREP_COLORS escapes)
printf '1a\n2\n3a\n' > f; cp f g
grep --color=always a f | od -c | sed 's/  */ /g; s/ $//'
grep --color=always -n -A1 a f g | od -c | sed 's/  */ /g; s/ $//'

#### --color: -o, -c, -l and -v context
## expect: gnu (D016: GNU's GREP_COLORS escapes)
printf '1a\n2\n' > f; cp f g
grep --color=always -o a f | od -c | sed 's/  */ /g; s/ $//'; grep --color=always -c a f g | od -c | sed 's/  */ /g; s/ $//'
grep --color=always -l a f g | od -c | sed 's/  */ /g; s/ $//'; grep --color=always -v -A1 a f | od -c | sed 's/  */ /g; s/ $//'

#### GREP_COLORS
## expect: gnu (BSD grep has no GREP_COLORS)
printf '1a\n2\n' > f
GREP_COLORS='mt=1;34:fn=33:ne' grep --color=always -H a f | od -c | sed 's/  */ /g; s/ $//'
GREP_COLORS='sl=1:cx=2' grep --color=always -A1 a f | od -c | sed 's/  */ /g; s/ $//'

#### --color=auto and never are plain when not a terminal
printf 'a\n' > f
grep --color=auto a f; grep --color=never a f; grep --colour a f

#### -P with what Rust's regex takes
## expect: gnu (BSD grep has no -P)
printf 'a1\nb\tc\n' > f
grep -P 'a\d' f; grep -cP '\t' f; grep -oP '[a-z](?:1)' f
