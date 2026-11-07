# readlink: a link's target, and GNU's canonicalizing modes.

#### a symlink's target as stored
ln -s some/where l
readlink l

#### not a symlink: status 1, no message
touch f
readlink f
echo "status=$?"
readlink missing
echo "status=$?"

#### -f resolves every link
mkdir -p d/sub
ln -s d/sub l
ln -s ../l d/back
readlink -f l
readlink -f d/back/../sub

#### -f with a missing last component
## expect: gnu (D016: macOS readlink -f needs the file to exist)
readlink -f missing

#### -f with a missing directory fails
readlink -f missing/x
echo "status=$?"

#### -e needs every component
## expect: gnu (BSD readlink has no -e)
ln -s nowhere dangling
readlink -e dangling
echo "status=$?"
touch f
readlink -e f

#### -m needs none
## expect: gnu (BSD readlink has no -m)
readlink -m a/../b/c
readlink -m /x/./y/

#### -n drops the newline
ln -s target l
readlink -n l; echo '|'

#### -n with several names is ignored
## expect: gnu (BSD readlink -n honors it with several names)
ln -s t1 a
ln -s t2 b
readlink -n a b

#### -v reports why
## expect: gnu (BSD readlink has no -v)
touch f
readlink -v f
echo "status=$?"

#### a link chain resolves to its end
touch end
ln -s end one
ln -s one two
readlink two
readlink -f two
