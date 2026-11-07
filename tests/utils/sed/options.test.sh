# sed: options and in-place editing.

#### -e and -f
printf 's/a/b/\n' > script.sed
echo aa | sed -e 's/a/x/' -f script.sed

#### -f with a comment and two lines
printf '# comment\ns/a/b/\ns/b/c/\n' > script.sed
echo a | sed -f script.sed

#### --expression and --quiet
## expect: gnu (BSD sed has no long options)
echo a | sed --quiet --expression=p

#### -z
## expect: gnu (BSD sed has no -z)
printf 'a\0b\0' | sed -z 's/^/X/' | tr '\0' '\n'

#### -- ends the options
echo a > f
sed -- p f

#### options after the script
## expect: gnu (BSD sed takes options only first)
echo aa | sed 's/a+/X/' -E

#### -i edits in place, GNU form
## expect: gnu (D016: sed -i SCRIPT FILE is GNU's form; BSD takes SCRIPT as a suffix)
printf 'a\nb\n' > f
sed -i 's/a/X/' f
cat f; ls

#### -i with -e, GNU form
## expect: gnu (D016: an option after -i is not a suffix; BSD takes -e as the suffix)
printf 'a\n' > f
sed -i -e 's/a/X/' f
cat f; ls

#### -i '' is BSD's empty suffix
## expect: bsd (D016: sed -i '' is BSD's form)
printf 'a\n' > f
sed -i '' 's/a/X/' f
cat f; ls

#### -i '' -e
## expect: bsd (D016: sed -i '' is BSD's form)
printf 'a\n' > f
sed -i '' -e 's/a/X/' -e 's/X/Y/' f
cat f; ls

#### -i.bak attached suffix
printf 'a\n' > f
sed -i.bak 's/a/X/' f
cat f f.bak; ls

#### -i .bak is a BSD suffix
## expect: bsd (D016: a .-prefixed word followed by a script is BSD's suffix)
printf 'a\n' > f
sed -i .bak 's/a/X/' f
cat f f.bak; ls

#### -i .bak -e is a BSD suffix
## expect: bsd (D016: a .-prefixed word followed by a script is BSD's suffix)
printf 'a\n' > f
sed -i .bak -e 's/a/X/' f
cat f f.bak; ls

#### -Ei '' combined
## expect: bsd (D016: sed -i '' is BSD's form)
printf 'aa\n' > f
sed -Ei '' 's/a+/X/' f
cat f; ls

#### -i -n
## expect: gnu (D016: an option after -i is not a suffix)
printf 'a\nb\n' > f
sed -i -n 2p f
cat f; ls

#### --in-place=SUFFIX
## expect: gnu (BSD sed has no long options)
printf 'a\n' > f
sed --in-place=.orig 's/a/X/' f
cat f f.orig

#### -i with * in the suffix
## expect: gnu (BSD sed has no * in suffixes)
mkdir bak
printf 'a\n' > f
sed -i'bak/*.old' 's/a/X/' f
cat f bak/f.old

#### -i edits each file separately
printf 'a\nb\n' > x; printf 'c\nd\n' > y
sed -i.b '1d' x y
cat x y

#### -i and q keep what was printed
printf '1\n2\n3\n4\n' > f
sed -i.b 2q f
cat f

#### -i keeps the mode
printf 'a\n' > f
chmod 751 f
sed -i.b 's/a/b/' f
ls -l f | cut -c1-10

#### -i without files
## expect: gnu (GNU's message and status)
sed -i 's/a/b/'; echo $?

#### -i on a missing file goes on
## expect: gnu (GNU's message and status)
printf 'a\n' > f
sed -i 's/a/b/' nosuch f; echo $?
cat f

#### -i with w /dev/stdout
## expect: gnu (D016: sed -i SCRIPT FILE is GNU's form)
printf 'a\n' > f
sed -i 's/a/b/w /dev/stdout' f
cat f
