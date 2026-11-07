# basename: the last component, suffixes, several names.

#### a path and a suffix
basename /usr/lib/libc.so .so
basename dir/file.txt .txt

#### trailing slashes and the root
basename a/b/
basename //
basename /

#### a suffix equal to the name stays
basename .c .c

#### -a takes several names
basename -a x/a y/b/ c

#### -s removes a suffix from every name
basename -s .c src/a.c b.c c.h

#### --suffix and --multiple
## expect: gnu (long options)
basename --suffix=.sh a.sh b.sh
basename --multiple p/q r

#### -z ends names with NUL
## expect: gnu (BSD basename has no -z)
basename -z a/b | tr '\0' '@'; echo

#### an extra operand
## expect: gnu (BSD basename treats a third operand differently)
basename a b c
echo "status=$?"

#### no operand
basename
echo "status=$?"
