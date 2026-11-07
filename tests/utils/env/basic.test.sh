# env: print and change the environment, run commands through the shell's dispatch.

#### a variable for one command
A=1 env | grep '^A='
env A=2 sh -c 'echo $A'
echo "after: ${A:-unset}"

#### -i starts empty
env -i B=2 env

#### -u removes a variable
export X=1
env -u X sh -c 'echo "${X:-gone}"'

#### a lone - is -i
env - C=3 env

#### env runs standalone builtins
env printf '%s\n' via-printf
env true
echo "status=$?"

#### env does not run functions
## expect: gnu (D016: GNU's messages)
f() { echo fn; }
env f
echo "status=$?"

#### env runs utilities
printf 'a\nb\n' | env head -n 1

#### PATH changes the lookup of the command
env PATH=/nonexistent ls 2>/dev/null
echo "status=$?"
