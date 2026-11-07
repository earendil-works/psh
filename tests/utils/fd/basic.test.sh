# fd: fd 10.5.1 in-process. The oracle is the upstream fd 10.5.1 release binary in the
# recording container (BSD has no fd, so every case expects GNU's recording). Not a
# terminal: no colors unless asked, and `./` is stripped only for the implicit search
# path. Real fd prints in path order when its search ends within 100 ms (always here),
# but runs -x commands and orders -X arguments arbitrarily, so those cases sort or have
# one result. `t` builds a small project: a git repository (a .git directory is
# enough for ignore rules) with ignored, hidden and nested files.

#### the implicit search path: ./ stripped, directories with a trailing /
## expect: gnu (fd is not a BSD tool)
mkdir -p src/sub docs && for f in a.txt b.rs src/main.rs src/sub/deep.rs docs/README.md; do printf 'x\n' > $f; done
fd; echo ---; fd .; echo ---; fd . .; echo ---; fd . src

#### a leading dash keeps its ./
## expect: gnu (fd is not a BSD tool)
printf 'x\n' > ./-rf && printf 'x\n' > b
fd; fd -x echo | sort

#### the pattern: a regex on the file name, smart case
## expect: gnu (fd is not a BSD tool)
mkdir -p src && for f in Main.rs main.c lib.rs README.md src/util.RS; do printf 'x\n' > $f; done
fd rs; echo ---; fd RS; echo ---; fd -s rs; echo ---; fd -i MAIN; echo ---; fd '^[a-z]+\.rs$'; echo ---; fd '(?i)readme'

#### -g globs, -F, --exact, --and
## expect: gnu (fd is not a BSD tool)
mkdir -p a/b && for f in x.rs y.py a/z.rs a/b/w.rs 'a.b'; do printf 'x\n' > $f; done
fd -g '*.rs'; echo ---; fd -g '*.{rs,py}'; echo ---; fd -g '**/b/*'; echo ---; fd -g -p '**/b/*'
echo ---; fd -F .; echo ---; fd --exact z.rs; echo ---; fd --and rs '^[wz]'; echo ---; fd -g --regex '\.py$'

#### -p matches the full path
## expect: gnu (fd is not a BSD tool)
mkdir -p src/sub && printf 'x\n' > src/sub/m.rs && printf 'x\n' > m.rs
fd -p 'src/.*rs$'; echo ---; fd --full-path 'sub/m'; echo ---; fd -p -g '**/src/**/*.rs'

#### a pattern with a slash, or a leading dot without -H
## expect: gnu (fd is not a BSD tool)
mkdir -p src && printf 'x\n' > src/main && printf 'x\n' > .env
fd src/main; echo $?; fd '^\.env'; echo $?; fd -H '^\.env'

#### the gitignore, hidden files, -H, -I, -u
## expect: gnu (fd is not a BSD tool)
mkdir -p .git .hid build && printf '*.log\nbuild/\n' > .gitignore
for f in a.txt b.log .hid/h.txt build/out.txt .env; do printf 'x\n' > $f; done
fd; echo ---; fd -H; echo ---; fd -I; echo ---; fd -u; echo ---; fd -HI --no-hidden; echo ---; fd -u --ignore

#### .ignore, .fdignore, .git/info/exclude, parent directories, --no-ignore-vcs
## expect: gnu (fd is not a BSD tool)
mkdir -p .git/info p/a/b p/c && printf 'secret*\n' > .git/info/exclude && printf '*.tmp\n!keep.tmp\n' > .gitignore
printf 'b/\n' > p/a/.fdignore && printf '*.md\n' > p/c/.ignore
for f in secret.txt p/x.tmp p/keep.tmp p/a/one.rs p/a/b/two.rs p/c/R.md p/c/main.c; do printf 'x\n' > $f; done
fd; echo ---; fd --no-ignore-vcs; echo ---; cd p/a && fd; echo ---; fd --no-ignore-parent . ..

#### gitignore rules apply only in a repository
## expect: gnu (fd is not a BSD tool)
printf '*.txt\n' > .gitignore && printf 'x\n' > a.txt && printf 'x\n' > b.rs
fd; echo ---; fd --no-require-git

#### --ignore-file, and a missing one
## expect: gnu (fd is not a BSD tool)
printf '*.c\n' > myignore && printf 'x\n' > a.c && printf 'x\n' > b.rs
fd --ignore-file myignore; echo ---; fd --ignore-file nope; echo $?

#### -t types
## expect: gnu (fd is not a BSD tool)
mkdir -p d emptydir && printf 'x\n' > d/f && : > empty && printf '#!/bin/sh\n' > run && chmod +x run && ln -s d/f link && ln -s nowhere dangling
fd -tf; echo ---; fd -td; echo ---; fd -tl; echo ---; fd -tx; echo ---; fd -te; echo ---; fd -te -tf; echo ---; fd --type=executable --type directory

#### -L follows links; broken links are kept
## expect: gnu (fd is not a BSD tool)
mkdir -p real && printf 'x\n' > real/f && ln -s real linkdir && ln -s nowhere dangling
fd; echo ---; fd -L; echo ---; fd -L -tl; echo ---; fd -L --no-follow -tl; echo ---; fd . linkdir

#### -e extensions
## expect: gnu (fd is not a BSD tool)
mkdir -p d.rs && for f in a.rs b.RS c.txt d.tar.gz noext; do printf 'x\n' > $f; done
fd -e rs; echo ---; fd -e .TXT; echo ---; fd -e rs -e txt -tf; echo ---; fd -e gz; echo ---; fd -e tar.gz

#### depth: -d, --min-depth, --exact-depth
## expect: gnu (fd is not a BSD tool)
mkdir -p a/b/c && touch top a/one a/b/two a/b/c/three
fd -d1; echo ---; fd --max-depth=2; echo ---; fd --min-depth 3; echo ---; fd --exact-depth 2; echo ---; fd -d0; echo $?

#### -E excludes, --prune, --ignore-contain
## expect: gnu (fd is not a BSD tool)
mkdir -p node_modules/p src/gen && for f in node_modules/p/i.js src/a.rs src/gen/b.rs src/gen/.skip; do printf 'x\n' > $f; done
fd -E node_modules; echo ---; fd -E '*.rs' -E node_modules; echo ---; fd -E 'src/gen'; echo ---; fd --prune -td
echo ---; fd --prune src; echo ---; fd --ignore-contain .skip -H

#### -S sizes
## expect: gnu (fd is not a BSD tool)
mkdir d && : > zero && printf 'abc' > three && head -c 2000 /dev/zero > big
fd -S -1b; echo ---; fd -S +3b; echo ---; fd -S 3b; echo ---; fd -S +1k; echo ---; fd -S -2ki -tf

#### --changed-within/--changed-before with a timestamp
## expect: gnu (fd is not a BSD tool)
printf 'x\n' > a
fd --changed-within @1; echo ---; fd --changed-before @1; echo $?

#### -0, --print0
## expect: gnu (fd is not a BSD tool)
mkdir d && printf 'x\n' > d/a && printf 'x\n' > b
fd -0 | tr '\0' '@'; echo; fd --print0 . d | tr '\0' '@'

#### --format placeholders
## expect: gnu (fd is not a BSD tool)
mkdir -p src/sub && printf 'x\n' > src/sub/m.tar.gz && printf 'x\n' > top.rs && printf 'x\n' > .hid
fd --format '{} {/} {//} {.} {/.}' -tf; fd -H --format '{{{/.}}}' hid; fd --format x -tf

#### -x runs a command per result, placeholders, several commands
## expect: gnu (fd is not a BSD tool)
mkdir -p src && printf 'x\n' > src/a.rs && printf 'x\n' > b.rs
fd rs -x echo | sort; fd rs -x echo {/} {//} {.} {/.} | sort; fd rs -x echo one {} \; -x echo two {/} | sort
fd -0 b.rs -x echo | tr '\0' '@'

#### -X runs one command with all results
## expect: gnu (fd is not a BSD tool)
mkdir -p src && printf 'x\n' > src/a.rs
fd rs -X echo; fd rs -X echo -- {/} end; fd -tf -X wc -l; fd zzz -X echo nothing; echo $?

#### command statuses and missing commands
## expect: gnu (fd is not a BSD tool)
printf 'x\n' > a && printf 'x\n' > b
fd -x false; echo $?; fd -X false; echo $?; fd a -x nonexist; echo $?; fd -X nonexist; echo $?; fd -x true; echo $?

#### -C, --base-directory: results and commands relative to it
## expect: gnu (fd is not a BSD tool)
mkdir -p proj/src && printf 'x\n' > proj/src/m.rs && printf 'x\n' > proj/t.rs
fd -C proj; echo ---; fd -C proj rs src; echo ---; fd --base-directory proj -x cat {} | sort; echo ---; fd -C nope; echo $?

#### --search-path, several paths
## expect: gnu (fd is not a BSD tool)
mkdir -p a b && printf 'x\n' > a/1 && printf 'x\n' > b/2
fd --search-path b --search-path a; echo ---; fd . b a; echo ---; fd x nope a; echo $?; fd x nope; echo $?

#### --path-separator, --strip-cwd-prefix, -a
## expect: gnu (fd is not a BSD tool)
mkdir -p a/b && printf 'x\n' > a/b/c
fd --path-separator '#'; echo ---; fd --strip-cwd-prefix=never; echo ---; fd --strip-cwd-prefix -x echo | sort; echo ---
fd -a c | sed "s|$(pwd -P)|PWD|"

#### -q, exit statuses
## expect: gnu (fd is not a BSD tool)
printf 'x\n' > a
fd -q a; echo $?; fd -q zzz; echo $?; fd zzz; echo $?; fd --has-results a; echo $?

#### colors when asked
## expect: gnu (fd is not a BSD tool)
mkdir -p src emptydir && printf 'x\n' > src/a.rs && printf 'x\n' > b.txt && printf '#!/bin/sh\n' > run && chmod +x run && ln -s b.txt link && ln -s nowhere dangling
fd --color=always | cat -v; echo ---; LS_COLORS='di=01;34:*.rs=91:ln=4' fd -c always | cat -v; echo ---; NO_COLOR=1 fd -c always b.txt | cat -v; fd -c auto b.txt

#### errors: patterns, values, conflicts
## expect: gnu (fd is not a BSD tool)
printf 'x\n' > a
fd '('; echo $?; fd -g '['; echo $?; fd -E '['; echo $?; fd -t z; echo $?; fd -S 1; echo $?; fd -d x; echo $?
fd -g -F x; echo $?; fd --exact-depth 1 -d 2; echo $?; fd -X echo {} {}; echo $?; fd -X {}; echo $?; fd -x; echo $?; fd --and; echo $?

#### a closed pipe ends the walk
## expect: gnu (fd is not a BSD tool)
mkdir -p big && for i in $(seq 1 300); do : > big/f$i; done
fd . big | head -2

#### --version, -h, --help
## expect: gnu (fd is not a BSD tool)
fd --version; fd -V; fd -h | cksum; fd --help | cksum; fd --help | tail -1
