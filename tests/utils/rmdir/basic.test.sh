# rmdir: empty directories only.

#### an empty directory
mkdir d
rmdir d
[ -e d ] || echo gone

#### a directory with entries fails
mkdir d
touch d/f
rmdir d
echo "status=$?"

#### a file fails
touch f
rmdir f
echo "status=$?"

#### a missing name fails
rmdir nope
echo "status=$?"

#### -p removes the parents
mkdir -p a/b/c
rmdir -p a/b/c
[ -e a ] || echo gone

#### -p stops at a non-empty parent
mkdir -p a/b/c
touch a/f
rmdir -p a/b/c
echo "status=$?"
[ -d a ] && [ ! -e a/b ] && echo kept

#### -v reports each directory
## expect: gnu (D016: GNU's messages)
mkdir -p a/b
rmdir -pv a/b

#### --ignore-fail-on-non-empty
## expect: gnu (BSD rmdir has no --ignore-fail-on-non-empty)
mkdir -p d/e
rmdir --ignore-fail-on-non-empty d
echo "status=$?"
