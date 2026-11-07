# rg: ripgrep 15.2.0 in-process. The oracle is the upstream rg 15.2.0 release binary in
# the recording container (BSD has no rg, so every case expects GNU's recording). Not a
# terminal: no headings, no line numbers by default. Real rg's order across files is
# arbitrary with several threads (psh's is path-sorted), so cases with output from
# several files sort it or use --sort path. `t` builds a small project: a git
# repository (a .git directory is enough for ignore rules) with ignored, hidden and
# nested files.

#### one file: lines, no file name
## expect: gnu (rg is not a BSD tool)
printf 'alpha\nbeta\nalphabet\n' > f.txt
rg alpha f.txt

#### -n, line numbers
## expect: gnu (rg is not a BSD tool)
printf 'alpha\nbeta\nalphabet\n' > f.txt
rg -n alp f.txt; rg alp -n f.txt

#### a directory: file names, ./ stripped only for the implicit path
## expect: gnu (rg is not a BSD tool)
mkdir -p src/a && printf 'x foo\n' > src/a/one.rs && printf 'foo\nbar\n' > src/two.py
rg --sort path foo; rg --sort path foo .; rg --sort path foo src; rg -n --sort path foo ./src

#### the gitignore, hidden files, -u, -uu, -uuu
## expect: gnu (rg is not a BSD tool)
mkdir .git sub .hid && printf '*.log\nbuild/\n' > .gitignore
printf 'foo\n' > a.txt; printf 'foo\n' > b.log; printf 'foo\n' > .hid/h.txt
mkdir build && printf 'foo\n' > build/out.txt; printf 'fo\0o foo\n' > sub/bin.dat
rg --sort path foo; echo ---; rg --sort path -u foo; echo ---; rg --sort path -uu foo; echo ---; rg --sort path -uuu foo

#### --hidden and --no-ignore
## expect: gnu (rg is not a BSD tool)
mkdir .git .hid && printf '*.log\n' > .gitignore && printf 'foo\n' > a.log && printf 'foo\n' > .hid/x
rg --sort path --hidden foo; echo ---; rg --sort path --no-ignore foo; echo ---; rg --sort path --no-ignore --hidden foo

#### ignore files of parent directories
## expect: gnu (rg is not a BSD tool)
mkdir -p .git sub/deep && printf 'skip.txt\n' > .gitignore
printf 'foo\n' > sub/deep/skip.txt; printf 'foo\n' > sub/deep/keep.txt
cd sub && rg foo; rg --no-ignore-parent foo | sort

#### .ignore and .rgignore, negations, nested rules
## expect: gnu (rg is not a BSD tool)
mkdir -p .git a/b && printf '*.txt\n!keep.txt\n' > .gitignore && printf 'x.md\n' > a/.ignore && printf 'y.md\n' > a/b/.rgignore
for f in one.txt keep.txt a/x.md a/y.md a/b/y.md a/b/z.md; do printf 'foo\n' > $f; done
rg --sort path foo; echo ---; rg --sort path --no-ignore-dot foo; echo ---; rg --sort path --no-ignore-vcs foo

#### gitignore rules apply only in a repository
## expect: gnu (rg is not a BSD tool)
printf '*.txt\n' > .gitignore && printf 'foo\n' > a.txt
rg foo; echo $?; rg --no-require-git foo; echo $?

#### -g globs, negation, braces, --iglob
## expect: gnu (rg is not a BSD tool)
mkdir -p src/x && for f in a.rs b.py c.md src/d.rs src/x/E.RS; do printf 'foo\n' > $f; done
rg --sort path -g '*.rs' foo; echo ---; rg --sort path -g '!*.rs' foo; echo ---
rg --sort path -g '*.{py,md}' foo; echo ---; rg --sort path --glob 'src/**' foo; echo ---; rg --sort path --iglob '*.rs' foo

#### -t, -T and --type-add
## expect: gnu (rg is not a BSD tool)
for f in a.rs b.py c.md d.js; do printf 'foo\n' > $f; done
rg --sort path -t rust foo; rg --sort path --type py foo; echo ---; rg --sort path -T rust -T py foo; echo ---
rg --sort path --type-add 'web:*.{js,md}' -t web foo

#### --type-list starts with the defaults
## expect: gnu (rg is not a BSD tool)
rg --type-list | head -5; rg --type-list | grep -c .

#### case: sensitive by default, -i, -S
## expect: gnu (rg is not a BSD tool)
printf 'Foo\nfoo\nFOO\n' > f
rg foo f; echo ---; rg -i foo f; echo ---; rg -S foo f; echo ---; rg -S Foo f; echo ---; rg --smart-case -i FOO f

#### -C, -A, -B and the context separator
## expect: gnu (rg is not a BSD tool)
seq 1 20 > n
rg -C1 '^(5|9|15)$' n; echo ---; rg -A2 -n '^3$' n; echo ---; rg -B1 '^1[05]$' n

#### context in several files
## expect: gnu (rg is not a BSD tool)
mkdir d && seq 1 5 > d/a && seq 3 7 > d/b
rg --sort path -C1 '^4$' d

#### -l, -c, --count-matches, --files-without-match
## expect: gnu (rg is not a BSD tool)
mkdir d && printf 'foo foo\nbar\n' > d/a && printf 'bar\n' > d/b && printf 'foo\n' > d/c
rg --sort path -l foo d; rg --sort path -c foo d; rg --sort path --count-matches foo d; rg --sort path --files-without-match foo d

#### -o, -r, -w, -x
## expect: gnu (rg is not a BSD tool)
printf 'foo foobar barfoo\nfoo\n' > f
rg -o 'foo\w*' f; echo ---; rg -r X foo f; echo ---; rg -w foo f; echo ---; rg -x foo f; echo ---; rg -o -r '<$0>' 'f(o+)' f

#### -F, several -e, -v, -m
## expect: gnu (rg is not a BSD tool)
printf 'a.b\naxb\nc\nd\n' > f
rg -F a.b f; echo ---; rg -e c -e d f; echo ---; rg -v a f; echo ---; rg -m1 a f

#### -f reads patterns from a file and from stdin
## expect: gnu (rg is not a BSD tool)
printf 'one\ntwo\nthree\n' > f && printf 'one\nthree\n' > pats
rg -f pats f; echo two | rg -f - f

#### stdin: piped, and - among paths
## expect: gnu (rg is not a BSD tool)
printf 'foo\n' > a
printf 'xfoo\nbar\n' | rg foo; printf 'yfoo\n' | rg -j1 -n foo - a

#### exit statuses and errors
## expect: gnu (rg is not a BSD tool)
printf 'foo\n' > a
rg foo a >/dev/null; echo $?; rg nope a; echo $?; rg foo missing a; echo $?
rg -q foo missing a; echo $?; rg -s foo missing; echo $?; rg --no-messages foo missing; echo $?

#### a syntax error
## expect: gnu (rg is not a BSD tool)
printf 'foo\n' > a
rg 'fo(' a; echo $?; rg '\p{Nope}' a; echo $?

#### unknown flags and missing values
## expect: gnu (rg is not a BSD tool)
rg --frobnicate x; echo $?; rg -e; echo $?; rg; echo $?

#### --files, -g with --files, --max-depth
## expect: gnu (rg is not a BSD tool)
mkdir -p a/b/c && touch top.txt a/one.rs a/b/two.rs a/b/c/three.rs
rg --files --sort path; echo ---; rg --files --sort path -g '*.rs' a; echo ---; rg --files --sort path --max-depth 2

#### --vimgrep, --column, -H, -I, --heading, -0
## expect: gnu (rg is not a BSD tool)
mkdir d && printf 'a foo\nfoo b\n' > d/x
rg --vimgrep foo d; rg --column foo d/x; rg -H foo d/x; rg -I foo d; rg --heading foo d; rg -l -0 foo d | cat -v

#### multiline -U
## expect: gnu (rg is not a BSD tool)
printf 'start\nmiddle\nend\n' > f
rg -U 'start\nmid' f; echo ---; rg -U -o '(?s)start.*end' f; rg 'a\nb' f; echo $?

#### binary files
## expect: gnu (rg is not a BSD tool)
mkdir d && printf 'foo\0bar\nfoo\n' > d/bin && printf 'foo\n' > d/txt
rg --sort path foo d; echo ---; rg foo d/bin; echo ---; rg -a foo d/bin | cat -v; echo ---; rg --sort path --binary foo d

#### symbolic links: -L follows them
## expect: gnu (rg is not a BSD tool)
mkdir -p real d && printf 'foo\n' > real/f && ln -s ../real d/link && ln -s ../real/f d/flink
rg --sort path foo d; echo ---; rg --sort path -L foo d

#### --sort and --sortr
## expect: gnu (rg is not a BSD tool)
mkdir -p d/b && for f in d/c d/a d/b/x; do printf 'foo\n' > $f; done
rg --sort path -l foo d; rg --sortr path -l foo d; rg --sort none -l foo d | sort

#### --trim, --max-columns, --passthru, --null-data
## expect: gnu (rg is not a BSD tool)
printf '   indented foo\nfoo and a very long line indeed\nbar\n' > f
rg --trim foo f; rg -M 10 foo f; rg -M 10 --max-columns-preview foo f; rg --passthru foo f
printf 'a foo\0b\0' | rg --null-data foo | cat -v

#### colors when asked
## expect: gnu (rg is not a BSD tool)
mkdir d && printf 'a foo b\n' > d/x
rg --color always foo d | cat -v; rg --color always --colors 'match:fg:blue' foo d/x | cat -v

#### RIPGREP_CONFIG_PATH
## expect: gnu (rg is not a BSD tool)
printf -- '# defaults\n--line-number\n--smart-case\n' > rc && printf 'Foo\nfoo\n' > f
RIPGREP_CONFIG_PATH=rc rg foo f; echo ---; RIPGREP_CONFIG_PATH=rc rg --no-config foo f; echo ---; RIPGREP_CONFIG_PATH=missing rg foo f

#### a closed pipe ends the search with status 0
## expect: gnu (rg is not a BSD tool)
mkdir -p big && for i in $(seq 1 200); do seq 1 2000 > big/f$i; done
rg -n 1 big | head -1 | grep -c :; echo ${PIPESTATUS[@]}; rg . big/f1 | head -2; echo ${PIPESTATUS[0]}

#### --help and -h, but for the version line
## expect: gnu (rg is not a BSD tool)
rg --help | tail -n +2 | cksum; rg -h | tail -n +2 | cksum; rg --help | wc -l; rg -h | head -1 | cut -d' ' -f1-2

#### --version
## expect: gnu (rg is not a BSD tool)
rg --version | head -1 | cut -d' ' -f1-2; rg -V | cut -d' ' -f1-2
