# mkdir: directories, parents, modes.

#### one and several directories
mkdir a
mkdir b c
ls -d a b c

#### an existing name fails
mkdir a
touch f
mkdir a f
echo "status=$?"

#### a missing parent fails
mkdir x/y
echo "status=$?"

#### -p makes the parents and accepts existing directories
mkdir -p a/b/c
mkdir -p a/b
echo "status=$?"
[ -d a/b/c ] && echo made

#### -p through dot and dot-dot
mkdir -p ./d//e/../f
[ -d d/e ] && [ -d d/f ] && echo made

#### -p over a file fails
touch f
mkdir -p f
echo "status=$?"
mkdir -p f/x
echo "status=$?"

#### -v reports each directory
## expect: gnu (D016: GNU's messages; BSD -v prints the bare name)
mkdir -pv a/b
mkdir -v c

#### -m sets the mode exactly
mkdir -m 700 private
mkdir -m 555 ro
ls -ld private ro | cut -c1-10

#### -m takes a symbolic mode from a=rwx
mkdir -m u=rwx,g=rx,o= m1
mkdir -m go-w m2
ls -ld m1 m2 | cut -c1-10

#### the umask applies without -m
umask 077
mkdir a
umask 022
mkdir b
ls -ld a b | cut -c1-10

#### an invalid mode
mkdir -m bad z
echo "status=$?"
[ -e z ] || echo none

#### --parents and --mode
## expect: gnu (long options)
mkdir --parents --mode=750 a/b
ls -ld a/b | cut -c1-10

#### no operand
## expect: gnu (D016: GNU's usage status; BSD exits 64)
mkdir
echo "status=$?"
