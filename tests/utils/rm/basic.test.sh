# rm: files, trees, and the safety rules.

#### files
touch a b
rm a b
ls

#### a missing file fails, -f is quiet
rm nope
echo "status=$?"
rm -f nope
echo "status=$?"

#### a directory needs -r or -d
mkdir d
rm d
echo "status=$?"
rm -d d
echo "status=$?"
[ -e d ] || echo gone

#### -d fails on a non-empty directory
mkdir d
touch d/f
rm -d d
echo "status=$?"

#### -r removes a tree
mkdir -p d/e/f
touch d/x d/e/y d/e/f/z
rm -r d
[ -e d ] || echo gone

#### -rf on a mix of existing and missing names
mkdir -p d/e
touch f
rm -rf d f nope
echo "status=$?"
ls

#### -R is -r
mkdir -p d/e
rm -R d
[ -e d ] || echo gone

#### symlinks are removed, not followed
mkdir -p keep/inner
touch keep/inner/f
ln -s keep link
ln -s keep/inner/f flink
rm link flink
mkdir d
ln -s ../keep d/back
rm -r d
ls keep/inner

#### a symlink to a directory with a trailing slash
## expect: gnu (BSD rm removes the link itself)
mkdir real
ln -s real link
rm -r link/
echo "status=$?"
[ -L link ] && echo link-kept

#### . and .. are refused
mkdir d
rm -r .
echo "status=$?"
rm -rf d/..
echo "status=$?"
[ -d d ] && echo kept

#### / is refused
## expect: gnu (D016: GNU's failsafe message and status)
rm -rf /
echo "status=$?"

#### -v reports each name
## expect: gnu (D016: GNU's messages; BSD -v prints the bare name)
mkdir -p d/e
touch f d/e/g
rm -v f
rm -rv d

#### write-protected files go without prompting when stdin is not a terminal
touch f
chmod 444 f
rm f < /dev/null
[ -e f ] || echo gone

#### a file in a read-only directory cannot be removed
mkdir d
touch d/f
chmod 555 d
rm -f d/f
echo "status=$?"
chmod 755 d

#### no operand
rm
echo "status=$?"
rm -f
echo "status=$?"

#### --
touch -- -f
rm -- -f
ls
