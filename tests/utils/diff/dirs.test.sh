# diff on directories: -r, -N, -x, -q.

#### the tree
mkdir -p a/sub b/sub a/only b/bonly
printf '1\n' > a/f
printf '2\n' > b/f
printf 's\n' > a/sub/x
printf 't\n' > b/sub/x
printf 'same\n' > a/same
cp a/same b/same
printf 'o\n' > a/onlyfile
printf 'p\n' > b/bfile
diff a b
echo "st=$?"

#### -r recurses
mkdir -p a/sub b/sub
printf '1\n' > a/f
printf '2\n' > b/f
printf 's\n' > a/sub/x
printf 't\n' > b/sub/x
printf 'q\n' > a/sub/gone
printf 'p\n' > b/bfile
diff -r a b
echo "st=$?"

#### -ru and -rq
## expect: gnu (D016: BSD diff prints times without nanoseconds or zone)
mkdir -p a/sub b/sub
printf '1\n' > a/f
printf '2\n' > b/f
printf 's\n' > a/sub/x
printf 't\n' > b/sub/x
touch -t 202401151000 a/f b/f a/sub/x b/sub/x
diff -ru a b
diff -rq a b

#### -rN compares absent files as empty
mkdir -p a/sub b/sub
printf '1\n' > a/f
printf '2\n' > b/f
printf 'q\n' > a/sub/gone
printf 'p\n' > b/bfile
diff -rN a b

#### -x excludes names
## expect: gnu (D016: GNU quotes the options it echoes)
mkdir -p a/sub b/sub
printf '1\n' > a/f
printf '2\n' > b/f
printf 's\n' > a/sub/x
printf 't\n' > b/sub/x
printf 'z\n' > a/skip.log
printf 'y\n' > b/skip.log
diff -r -x '*.log' -x sub a b
diff -r --exclude=sub --exclude='f*' a b
echo "st=$?"

#### identical trees
mkdir -p a/s b/s
printf '1\n' > a/s/f
cp a/s/f b/s/f
diff -r a b
echo "st=$?"
