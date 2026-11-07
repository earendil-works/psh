# mv: renames, moves into directories, overwrites.

#### rename a file
echo a > a
mv a b
ls
cat b

#### replace an existing file
echo new > a
echo old > b
mv a b
cat b
ls

#### move into a directory
mkdir d
touch a b
mv a b d
ls d

#### rename a directory
mkdir -p d/e
touch d/e/f
mv d x
ls x x/e

#### a directory into an existing directory
mkdir a b
touch a/f
mv a b
ls b b/a

#### several sources need a directory
touch a b c
mv a b c
echo "status=$?"

#### a missing source
mv nope x
echo "status=$?"

#### into itself
mkdir -p d/e
mv d d/e
echo "status=$?"
[ -d d/e ] && echo kept

#### a directory over a file and a file over a directory
mkdir d
touch f
mv d f
echo "status=$?"
mkdir -p e/f
mv f e
echo "status=$?"

#### the same file
## expect: gnu (BSD mv renames a file onto itself and succeeds)
touch a
mv a ./a
echo "status=$?"

#### -n does not overwrite
echo a > a
echo b > b
mv -n a b
cat b
ls

#### -f overwrites
echo a > a
echo b > b
mv -f a b
cat b

#### -v reports the rename
## expect: gnu (D016: GNU's messages)
touch a
mv -v a b

#### -t and -T
## expect: gnu (BSD mv has no -t or -T)
mkdir d
touch a b
mv -t d a b
ls d
mkdir e f
mv -T e f
ls

#### -b keeps a backup
## expect: gnu (BSD mv has no -b)
echo a > a
echo b > b
mv -b a b
cat b b~

#### -u keeps a newer destination
## expect: gnu (BSD mv has no -u)
echo old > a
touch -t 200001010000 a
echo new > b
mv -u a b
cat b
ls

#### a symlink is moved, not its target
touch t
ln -s t l
mv l m
ls
readlink m

#### a trailing slash on a directory
mkdir d
mv d/ e
ls

#### no operand
## expect: gnu (D016: GNU's usage status; BSD exits 64)
mv
echo "status=$?"
mv a
echo "status=$?"
