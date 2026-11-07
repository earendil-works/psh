# printenv: values of variables.

#### one name
export FOO=bar
printenv FOO
echo "st=$?"
printenv NOPE
echo "st=$?"

#### several names (GNU)
## expect: gnu (BSD printenv takes one name)
export FOO=bar BAZ=qux
printenv FOO BAZ
echo "st=$?"
printenv FOO NOPE BAZ
echo "st=$?"

#### a name with = is never set
export FOO=bar
printenv FOO=bar
echo "st=$?"

#### only exported variables
FOO=local
printenv FOO
echo "st=$?"
BAR=prefix printenv BAR

#### the whole environment
export FOO=bar
printenv | grep '^FOO='

#### -0 ends values with NUL (GNU)
## expect: gnu (BSD printenv has no -0)
export FOO=bar
printenv -0 FOO | tr '\0' @
