# nl: numbering lines.

#### default numbers non-empty lines
## expect: gnu (D016: GNU indents unnumbered lines with spaces only)
printf 'a\n\nb\n' | nl

#### -ba numbers every line
printf 'a\n\nb\n' | nl -ba

#### a file, as agents read code
printf 'fn main() {\n    x();\n}\n' > f.rs
nl -ba f.rs

#### -n ln and rz
nl -nln -ba <<'X'
a
b
X
printf 'a\nb\n' | nl -nrz -w3

#### -w and -s
printf 'a\nb\n' | nl -w2 -s': '

#### -v and -i
printf 'a\nb\nc\n' | nl -v10 -i5

#### -bn numbers nothing
## expect: gnu (D016: GNU indents unnumbered lines with spaces only)
printf 'a\nb\n' | nl -bn

#### -l joins blank lines
## expect: gnu (D016: GNU indents unnumbered lines with spaces only)
printf '\n\n\n\nx\n' | nl -ba -l2

#### sections
## expect: gnu (D016: GNU prints section delimiters as empty lines)
printf 'a\n\\:\\:\\:\nh\n\\:\\:\nb\n\\:\nf\n' | nl -ha -fa

#### -p does not renumber at sections
## expect: gnu (D016: GNU prints section delimiters as empty lines)
printf 'a\n\\:\\:\nb\n' | nl -p

#### a last line without a newline
## expect: gnu (BSD nl does not add the newline)
printf 'a\nb' | nl

#### several files continue the numbering
## expect: gnu (BSD nl takes one file)
printf 'a\n' > f
printf 'b\n' > g
nl f g

#### an invalid style
nl -b x < /dev/null
echo "status=$?"

#### a missing file
nl missing
echo "status=$?"
