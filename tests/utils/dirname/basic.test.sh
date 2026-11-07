# dirname: everything but the last component.

#### the usual paths
dirname /usr/lib/libc.so
dirname dir/file
dirname file

#### trailing and repeated slashes
dirname /a//b//
dirname a/
dirname //
dirname /x

#### several names
dirname a/b c/d/e f

#### -z ends names with NUL
## expect: gnu (BSD dirname has no -z)
dirname -z a/b c/d | tr '\0' '@'; echo

#### no operand
dirname
echo "status=$?"
