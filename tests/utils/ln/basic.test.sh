# ln: hard and symbolic links.

#### a symbolic link
touch t
ln -s t l
readlink l

#### a symlink target is stored as written, need not exist
ln -s some/where l
readlink l
[ -e l ] || echo dangling

#### a hard link
echo x > a
ln a b
[ a -ef b ] && echo same
cat b

#### a link in a directory
mkdir d
touch t
ln -s ../t d
readlink d/t

#### the name of the target in the current directory
mkdir d
touch d/t
ln -s d/t
readlink t

#### an existing link fails, -f replaces it
ln -s a l
ln -s b l
echo "status=$?"
ln -sf b l
readlink l

#### -sfn replaces a symlink to a directory
mkdir old new
ln -s old cur
ln -sfn new cur
readlink cur
ls old

#### without -n the link goes into the directory
mkdir old new
ln -s old cur
ln -sf new cur
readlink cur
readlink old/new

#### a hard link to a missing file fails
ln nope x
echo "status=$?"

#### a hard link to a directory fails
mkdir d
ln d x
echo "status=$?"

#### several targets need a directory
touch a b
ln -s a b c
echo "status=$?"

#### -v reports the link
## expect: gnu (D016: GNU's messages)
touch t
ln -sv t l
ln -v t h

#### -r makes the target relative
## expect: gnu (BSD ln has no -r)
mkdir -p a/b c
touch c/t
ln -sr c/t a/b/l
readlink a/b/l

#### -t and -T
## expect: gnu (BSD ln has no -t or -T)
mkdir d e
touch t
ln -s -t d ../t
readlink d/t
ln -sT t e/x
readlink e/x

#### -h is -n
## expect: bsd (GNU ln has no -h)
mkdir old new
ln -s old cur
ln -sfh new cur
readlink cur

#### -f does not replace a directory
mkdir d
ln -sfn t d
echo "status=$?"
ls

#### no operand
ln
echo "status=$?"
