# ls: names, sorting, the long format, recursion. Owners, groups, block counts and
# directory sizes differ by machine and file system; the cases cut them out.

#### names in byte order, one per line off a terminal
touch b a D c _x
ls

#### hidden names need -a or -A
touch .h x
ls
echo --
ls -a
echo --
ls -A

#### -1 and -d
mkdir d
touch d/f
ls -1 d
ls -d d

#### files first, then directories with headers
mkdir d e
touch f d/x e/y
ls e f d

#### a missing operand
## expect: gnu (D016: GNU's status 2 and message)
touch f
ls nope f
echo "status=$?"

#### -R
## expect: gnu (D016: GNU prints a header for a single directory too)
mkdir -p d/e/g
touch d/f d/e/h
ls -R d

#### -F and -p
mkdir d
touch f x
chmod +x x
ln -s f l
ls -F
echo --
ls -p

#### -t newest first, -r reverses
touch -t 202001010000 a
touch -t 202201010000 b
touch -t 202101010000 c
ls -t
echo --
ls -tr

#### -S largest first
printf 1 > a
printf 123 > b
printf 12 > c
ls -S
echo --
ls -Sr

#### the long format
## expect: gnu (macOS marks files with extended attributes with @)
printf 12345 > f
chmod 640 f
touch -t 202001020304 f
ls -l f | tr -s ' ' | cut -d' ' -f1,2,5-

#### the long format of a directory listing
mkdir d
printf 123 > d/a
ln -s a d/l
touch -t 202001020304 d/a
touch -h -t 202001020304 d/l
ls -l d | tail -n 2 | tr -s ' ' | cut -d' ' -f5-
ls -l d | tail -n 2 | cut -c1

#### -h sizes
## expect: gnu (D016: GNU rounds up; BSD rounds to nearest and prints 1B)
printf '%2048s' '' > a
printf '%1500000s' '' > b
printf x > c
ls -lh a b c | tr -s ' ' | cut -d' ' -f5,9

#### -n numeric ids
## expect: gnu (macOS marks files with extended attributes with @)
touch -t 202001020304 f
ls -n f | tr -s ' ' | cut -d' ' -f1,5-

#### --time-style
## expect: gnu (BSD ls has no --time-style)
touch -d '2020-01-02 03:04:05' f
ls -l --time-style=full-iso f | tr -s ' ' | cut -d' ' -f6-
ls -l --time-style=long-iso f | tr -s ' ' | cut -d' ' -f6-
ls -l --time-style=iso f | tr -s ' ' | cut -d' ' -f6-
ls -l --time-style=+%s f | tr -s ' ' | cut -d' ' -f6-
ls -l --full-time f | tr -s ' ' | cut -d' ' -f6-

#### -T shows full times (BSD)
## expect: bsd (D1152: -T is BSD's full time; GNU's -T sets a tab size)
touch -t 202001020304.05 f
ls -lT f | tr -s ' ' | cut -d' ' -f6-

#### -G is BSD's color: none off a terminal
## expect: bsd (D1152: -G is BSD's color; GNU's -G drops the group)
touch f
ls -G
echo $(ls -lG f | wc -w)

#### a symlink to a directory as an operand
mkdir d
touch d/x
ln -s d l
ls l
ls -d l
ls -l l | tr -s ' ' | cut -d' ' -f9-
ls -l l | cut -c1

#### -L follows symlinks
touch f
ln -s f l
ls -lL l | cut -c1

#### -i prints the inode
touch f
echo $(ls -i f | wc -w)

#### -s on an empty file
touch f
ls -s f

#### -C fills columns with tab stops
## expect: gnu (D016: GNU's column layout; BSD aligns with tabs only)
touch aaaaaaaaaa bbbbbbbbbbbbbbb cc dddddddddddddddddd e ffffffffffffff gggggggggggggggggggggg hh
ls -C
COLUMNS=40 ls -C
COLUMNS=40 ls -x

#### -m separates with commas
## expect: gnu (D016: GNU's line breaks)
touch aaaa bbbb cccc dddd
ls -m
COLUMNS=12 ls -m

#### -I and --hide
## expect: gnu (BSD ls has no -I or --hide)
touch a.c b.h c.c
ls -I '*.c'
ls --hide='*.h'
echo $(ls -a --hide='*.h' | wc -l)

#### -v and -X
## expect: gnu (BSD ls has no -v or -X)
touch f10 f9 f1 a.z b.a
ls -v
echo --
ls -X

#### --group-directories-first
## expect: gnu (BSD ls has no --group-directories-first)
mkdir b
touch a c
ls --group-directories-first
