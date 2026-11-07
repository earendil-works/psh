# env -S and -C.

#### -S splits a string into words
env -S 'echo a  b'
env -S"echo 'x y' \"z\tw\""
FOO=bar env -S 'printf %s\n ${FOO}'
env -S 'A=1 sh -c "echo \$A"'
env -S 'echo a\_b\c ignored'
env -S 'echo a #comment'

#### -S words can be options
env -S'-i B=2 env'

#### -C runs the command in a directory
mkdir sub
touch sub/file
env -C sub sh -c 'basename "$(pwd -P)"'
env -C sub ls

#### -C errors
## expect: gnu (D016: GNU's messages)
env -C nodir pwd
echo "st=$?"
env -C .
echo "st=$?"
