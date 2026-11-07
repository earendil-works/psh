# cp: files, trees, attributes, links.

#### copy a file
echo data > a
cp a b
cat b

#### overwrite keeps the destination's mode
echo new > a
echo old > b
chmod 600 b
cp a b
cat b
ls -l b | cut -c1-10

#### a new file gets the source's mode less the umask
echo x > a
chmod 755 a
umask 027
cp a b
ls -l b | cut -c1-10

#### into a directory
mkdir d
touch a b
cp a b d
ls d

#### a directory needs -R
mkdir d
cp d e
echo "status=$?"
[ -e e ] || echo none

#### -R copies a tree
mkdir -p d/e
echo f > d/e/f
echo g > d/g
cp -R d x
ls -R x
cat x/e/f

#### -r is -R
mkdir -p d/e
cp -r d x
ls x

#### -R into an existing directory
mkdir -p d/e dst
cp -R d dst
ls dst dst/d

#### -R src/ copies the directory itself
## expect: gnu (D016: cp -R src/ copies the directory, as GNU; BSD copies the contents)
mkdir -p src/e dst
touch src/f
cp -R src/ dst
ls dst

#### -R src/. copies the contents in both dialects
mkdir -p src/e dst
touch src/f
cp -R src/. dst
ls dst

#### into itself
## expect: gnu (BSD cp copies one level and succeeds)
mkdir d
cp -R d d/x
echo "status=$?"

#### the same file
touch a
cp a a
echo "status=$?"

#### a missing source
cp nope x
echo "status=$?"

#### several sources need a directory
touch a b c
cp a b c
echo "status=$?"

#### -n does not overwrite
echo a > a
echo b > b
cp -n a b
cat b

#### -f replaces an unwritable destination
echo new > a
echo old > b
chmod 444 b
cp -f a b
cat b

#### -p keeps mode and times
echo x > a
chmod 640 a
touch -t 200001010000 a
cp -p a b
ls -l b | cut -c1-10
[ a -nt b ] || [ a -ot b ] || echo same-time

#### -a keeps symlinks and times in a tree
mkdir d
echo x > d/f
ln -s f d/l
touch -t 200001010000 d/f
cp -a d e
readlink e/l
[ d/f -nt e/f ] || [ d/f -ot e/f ] || echo same-time

#### -R copies symlinks as symlinks
mkdir d
touch d/f
ln -s f d/l
cp -R d e
readlink e/l

#### a symlink operand is followed without -R
echo x > t
ln -s t l
cp l c
[ -L c ] || echo regular
cat c

#### -P copies the link itself
## expect: gnu (BSD cp -P needs -R)
echo x > t
ln -s t l
cp -P l c
readlink c

#### -L follows links inside a tree
mkdir d
echo x > d/t
ln -s t d/l
cp -RL d e
[ -L e/l ] || echo regular
cat e/l

#### -v reports each copy
## expect: gnu (D016: GNU's messages)
mkdir d
touch d/f a
cp -v a b
cp -Rv d e

#### -t, -T and --parents
## expect: gnu (BSD cp has no -t, -T or --parents)
mkdir -p d x/y
touch a x/y/z
cp -t d a
cp --parents x/y/z d
ls -R d
mkdir e f
cp -RT e f
ls

#### -l and -s make links
## expect: gnu (BSD cp has no -l or -s)
echo x > a
cp -l a hard
cp -s a soft
readlink soft
[ a -ef hard ] && echo same

#### -u keeps a newer destination
echo old > a
touch -t 200001010000 a
echo new > b
cp -u a b
cat b

#### -c (BSD clone) copies
## expect: bsd (GNU -c preserves the SELinux context and fails without SELinux)
echo x > a
cp -c a b
cat b

#### no operand
cp
echo "status=$?"
cp a
echo "status=$?"
