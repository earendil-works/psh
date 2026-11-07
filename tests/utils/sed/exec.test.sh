# sed: the e command and the e flag of s, which run shell commands.

#### e with a command prints its output first
## expect: gnu (BSD sed has no e)
printf 'a\nb\n' | sed '1e echo hi'

#### e runs the pattern space
## expect: gnu (BSD sed has no e)
printf 'echo x\nb\n' | sed '1e'
printf 'printf "1\\n2\\n"\n' | sed e

#### the e flag of s
## expect: gnu (BSD sed has no e flag)
echo a | sed 's/a/echo hi/e'
echo b | sed 's/a/echo hi/e'

#### e with -n and i
## expect: gnu (BSD sed has no e)
printf 'a\nb\n' | sed -n '1e echo hi'
printf 'a\n' | sed -e 'i\' -e 'ins' -e 'e echo hi'

#### e takes the rest of the line
## expect: gnu (BSD sed has no e)
printf 'a\n' | sed '1e echo hi; echo there'

#### e sees exported variables, and its status does not matter
## expect: gnu (BSD sed has no e)
printf 'a\n' | X=5 sed 'e echo $X'
printf 'a\n' | sed '1e false'; echo $?

#### e when editing in place
## expect: gnu (BSD sed has no e)
printf 'a\n' > f
sed -i '1e echo hi' f
cat f
