# Agent shape: here-docs (13% of commands; 99.7% with a quoted delimiter, 22% are
# `cat <<'EOF' > file`); research/corpus.md §4.

#### cat with a quoted delimiter writes a file
cat <<'EOF' > config.txt
name = "demo"
path = $HOME/not-expanded
EOF
cat config.txt

#### quoted delimiter keeps $ and backticks literal
cat <<'EOF'
$PATH `date` $(echo no) \n
EOF

#### unquoted delimiter expands
name=world
cat <<EOF
hello $name $(echo sub) $((1+2))
EOF

#### double-quoted delimiter does not expand
x=1
cat <<"END"
x is $x
END

#### <<- strips leading tabs
cat <<-EOF
	indented with a tab
		two tabs
	EOF

#### here-doc fed to sh
sh <<'EOF'
for i in 1 2; do echo "i=$i"; done
EOF

#### here-doc fed to bash with arguments
bash -s -- a b <<'EOF'
echo "args: $1 $2 $#"
EOF

#### here-doc into a pipeline
cat <<'EOF' | sort
pear
apple
fig
EOF

#### here-doc to a while loop
while read -r k v; do echo "$k=$v"; done <<'EOF'
a 1
b 2
EOF

#### two here-docs in one command list
cat <<'A'; cat <<'B'
first
A
second
B

#### here-doc in a command substitution
v=$(cat <<'EOF'
inside
EOF
)
echo "v=$v"

#### here-doc delimiter text inside the body
cat <<'EOF'
not EOF yet
 EOF with a space
EOF

#### here-doc with an empty body
cat <<'EOF'
EOF
echo after

#### here-doc appended to a file
echo start > notes.md
cat <<'EOF' >> notes.md
- item
EOF
cat notes.md

#### here-doc larger than a pipe buffer
body=$(i=0; while [ $i -lt 10000 ]; do echo "line $i"; i=$((i+1)); done)
wc -l <<EOF
$body
EOF

#### here-string
read -r a b <<< "x y"
echo "$b $a"

#### here-doc then && chain
cat <<'EOF' > s.sh && sh s.sh
echo from-script
EOF

#### unterminated here-doc warns
cat <<'EOF'
no end marker
