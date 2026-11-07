# find: GNU's expression language with BSD's extra options (D016). Directory order is
# not stable across file systems, so cases sort what they print unless one entry per
# directory is the point.

#### no path means .
## expect: gnu (D016: find without a path searches .)
mkdir -p a/b; touch a/x.txt a/b/y.txt z.txt
find | sort

#### -name, -iname and -type
mkdir -p a/b; touch a/x.txt a/b/y.TXT z.txt
find . -name '*.txt' | sort; echo --; find . -iname '*.txt' -type f | sort; echo --; find . -type d | sort

#### -name matches leading dots and the start path's own name
mkdir -p d; touch d/.hidden d/vis
find d -name '*' | sort; find d/ -name d

#### -path and -ipath
mkdir -p src/lib test; touch src/lib/a.rs src/b.rs test/c.rs
find . -path './src/*' | sort; echo --; find . -ipath '*LIB*' | sort

#### -maxdepth and -mindepth
mkdir -p a/b/c; touch a/f a/b/g a/b/c/h
find . -maxdepth 2 | sort; echo --; find . -mindepth 2 -maxdepth 3 | sort

#### -maxdepth 0 is only the start paths
mkdir a b
find a b nosuch -maxdepth 0; echo status=$?

#### a missing start path is an error, the others are walked
mkdir a; touch a/f
find a nosuch | sort; echo status=$?

#### -o, -a, ! and parentheses
touch a.c b.h c.txt d.c
find . \( -name '*.c' -o -name '*.h' \) ! -name 'd*' | sort; echo --; find . -name '*.c' -a -not -name a.c

#### -prune
mkdir -p keep skip/deep; touch keep/k skip/s skip/deep/d
find . -name skip -prune -o -type f -print | sort

#### -prune with the default action prints the pruned directory
mkdir -p node_modules/x src; touch node_modules/x/m src/s
find . -name node_modules -prune | sort

#### -print0
mkdir d; touch 'd/a b' d/c
find d -type f -print0 | sort -z | tr '\0' '|'; echo

#### -exec with ;
mkdir d; touch d/one d/two
find d -type f -exec echo got {} \; | sort

#### -exec replaces {} inside words
touch f
find . -name f -exec echo '<{}>' '{}.bak' \;

#### -exec status is the test's value
touch yes no
find . -type f -exec test {} = ./yes \; -print

#### -exec with + collects the files
mkdir d; touch d/a d/b d/c
find d -type f -exec echo batch {} + | tr ' ' '\n' | sort | tr '\n' ' '; echo

#### -exec runs utilities and standalone builtins of the shell
## expect: gnu (D016: wc widths)
mkdir d; printf 'x\ny\n' > d/f
find d -type f -exec wc -l {} \; ; find d -type f -exec printf '[%s]\n' {} \;

#### -execdir runs in the file's directory
## expect: gnu (BSD -execdir passes bare names)
mkdir -p a/b; touch a/b/f
find a -name f -execdir sh -c 'echo "$1 in ${PWD##*/}"' _ {} \;

#### -execdir with +
## expect: gnu (BSD -execdir passes bare names)
mkdir -p a/b; touch a/b/f a/b/g
find a/b -type f -execdir echo {} +

#### -delete removes files and directories, depth first
mkdir -p tree/sub; touch tree/sub/f tree/g keep
find tree -delete; ls

#### -delete with a test
mkdir d; touch d/a.o d/b.o d/c.c
find d -name '*.o' -delete; ls d

#### -quit stops after the first match
mkdir d; touch d/f
find d -type f -print -quit

#### -empty
mkdir -p e full; touch full/x empty.txt; echo hi > data.txt
find . -empty | sort

#### -size
## expect: gnu (BSD rounds -size k differently)
printf '%01000d' 0 > k1; printf '%03000d' 0 > k3; touch zero
find . -type f -size +1k | sort; echo --; find . -type f -size -2k | sort; echo --; find . -type f -size 1000c; echo --; find . -size -1M -type f

#### -mtime and -mmin
touch new; touch -t 202001010000 old
find . -type f -mtime +30; echo --; find . -type f -mtime -1; echo --; find . -type f -mmin -5; echo --; find . -type f -mmin +5

#### -newer
touch -t 202001010000 ref; touch -t 202101010000 after; touch -t 201901010000 before
find . -type f -newer ref

#### -newermt
touch -t 202001010000 old; touch -t 202201010000 new
find . -type f -newermt '2021-01-01'; find . -type f ! -newermt '2021-06-01 12:00'

#### -perm
touch a b c; chmod 644 a; chmod 755 b; chmod 600 c
find . -type f -perm 644; echo --; find . -type f -perm -u=x; echo --; find . -type f -perm -044 | sort

#### -perm /mode is any of the bits
## expect: gnu (BSD find has no /mode)
touch a b c; chmod 600 a; chmod 640 b; chmod 604 c
find . -type f -perm /044 | sort

#### -type with a list
## expect: gnu (BSD -type takes one letter)
mkdir d; touch f; ln -s f l
find . -type f,l | sort

#### symbolic links: -P by default, -L follows
mkdir -p real/inside; touch real/inside/f; ln -s real link
find link | sort; echo --; find -L link | sort; echo --; find -L . -type l; echo --; find . -type l

#### -H follows only the start paths
mkdir -p real; touch real/f; ln -s real link; ln -s real/f real/lf
find -H link | sort

#### a dangling link with -L stays a link
ln -s nowhere broken
find -L . -type l

#### -L detects loops
## expect: gnu (D016: GNU message)
mkdir -p loop/d; ln -s .. loop/d/up
find -L loop | sort; echo status=$?

#### -regex matches the whole path
## expect: gnu (D016: GNU's default regex syntax)
mkdir d; touch d/a.js d/b.ts d/c.rs
find d -regex '.*\.\(js\|ts\)' | sort; echo --; find d -regex 'c\.rs'

#### -regextype selects ERE
## expect: gnu (BSD find has no -regextype)
mkdir d; touch d/a1 d/a22 d/b
find d -regextype posix-extended -regex '.*/a[0-9]+' | sort

#### BSD's -E selects ERE
## expect: bsd (GNU find has no -E)
mkdir d; touch d/a1 d/a22 d/b
find -E d -regex '.*/a[0-9]{2}'

#### -printf
## expect: gnu (BSD find has no -printf)
mkdir -p a/b; printf abc > a/f; touch -t 202403021000.05 a/f
find a -printf '%p|%f|%h|%P|%H|%d|%y\n' | sort; find a/f -printf '%s %m %M %TY-%Tm-%Td %TH:%TM:%TS\n'

#### -printf widths and escapes
## expect: gnu (BSD find has no -printf)
touch file
find file -printf '[%10p][%-6f][%.2p]\t%03d\101\n'

#### -printf %l and %Y
## expect: gnu (BSD find has no -printf)
touch f; ln -s f link; ln -s nowhere broken
find . -type l -printf '%p -> %l %Y\n' | sort

#### -depth lists directories after their contents
mkdir -p a/b; touch a/b/f
find a -depth

#### ! -name with -type
mkdir -p d/sub; touch d/a d/b
find d -mindepth 1 ! -type d | sort

#### options after tests apply everywhere
## expect: gnu (BSD applies -maxdepth too; GNU may warn)
mkdir -p a/b; touch a/b/f
find . -type f -maxdepth 1; echo status=$?

#### bad arguments
## expect: gnu (D016: GNU messages)
find . -name; echo status=$?; find . -type q; echo status=$?; find . -maxdepth x; echo status=$?

#### operators with missing operands
## expect: gnu (D016: GNU messages)
find . -o -name x; echo status=$?; find . \( -name x; echo status=$?; find . \( \); echo status=$?; find . -name x \); echo status=$?

#### a word after the expression
## expect: gnu (D016: GNU messages)
touch a b
find . -name a b; echo status=$?

#### find with xargs
mkdir d; touch d/a d/b
find d -type f -print0 | xargs -0 -n1 echo rm | sort

#### timeout stops find and the command it runs
## expect: gnu (macOS has no timeout)
touch f
timeout 0.5 find . -type f -exec sleep 10 \; ; echo status=$?
