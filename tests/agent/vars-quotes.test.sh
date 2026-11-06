# Agent shape: variables, parameter expansion with the common operators, quoting,
# env prefixes, command substitution; research/corpus.md §4.

#### $VAR in double quotes
name="a b"
echo "name: $name"

#### unquoted variable is word-split
v="1   2  3"
printf '[%s]' $v; echo

#### ${x:-default}
unset x
echo "${x:-fallback}"
x=set
echo "${x:-fallback}"

#### ${x:=default} assigns
unset y
: "${y:=assigned}"
echo "$y"

#### ${x#prefix} and ${x%suffix}
f=src/main.rs
echo "${f#src/}" "${f%.rs}" "${f##*/}" "${f%%/*}"

#### ${x/pattern/replacement}
s="a-b-c"
echo "${s/-/+}" "${s//-/+}"

#### ${#x} length
s=hello
echo "${#s}"

#### env prefix applies to one command
FOO=bar sh -c 'echo "FOO=$FOO"'
echo "after: ${FOO:-unset}"

#### export makes a variable visible to children
export ANSWER=42
sh -c 'echo $ANSWER'

#### command substitution with $( )
d=$(echo "  spaced  ")
echo "[$d]"

#### nested command substitution
echo "$(echo "$(echo nested)")"

#### arithmetic expansion
i=5
echo $((i * 2 + 1)) $((i / 2)) $((i % 3))

#### single quotes keep everything literal
echo '$HOME "x" \n'

#### double quotes with escaped characters
echo "a \"quoted\" \$dollar \\ backslash"

#### ANSI-C quoting
printf '%s\n' $'tab\there' $'nl\\n'

#### printf with format and arguments
printf '%s=%d\n' a 1 b 2

#### $? after a failing command
false
echo "status=$?"

#### set -u fails on an unset variable
set -u
echo "${UNSET_VAR_XYZ}"
echo "not reached"

#### special parameters in -c with arguments
sh -c 'echo "$0 $1 $#"' zero one

#### quotes inside command substitution inside quotes
echo "result: $(printf '%s' "a b")"

#### assignment of a multi-line value
v="line1
line2"
echo "$v" | wc -l

#### unset variable expands to nothing
echo "[${NOPE_NOT_SET}]"
