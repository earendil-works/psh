# diff -y (side by side) and --color.

#### -y
printf 'a\nb\nc\nd\ne\n' > 1
printf 'a\nB\nc\nd\ne\nf\n' > 2
diff -y 1 2
echo "st=$?"
diff -y -W 40 1 2
diff -y --suppress-common-lines 1 2

#### -y widths
printf 'b\nxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\nc\n' > 1
printf 'B\nyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy\nc\n' > 2
for w in 20 21 25 30 39 40 41 60 80 100; do diff -y -W $w 1 2; done

#### --left-column (GNU)
## expect: gnu (BSD diff has no --left-column)
printf 'a\nb\nc\n' > 1
printf 'a\nB\nc\n' > 2
diff -y --left-column -W 30 1 2

#### --color=always
## expect: gnu (BSD diff has no --color)
printf 'a\nb\nc\nd\ne\n' > 1
printf 'a\nB\nc\nd\ne\nf\n' > 2
diff --color=always -u --label a --label b 1 2
diff --color=always 1 2
diff --color=never 1 2
diff --color=always -c --label x --label y 1 2

#### -p shows the function
printf 'int f()\n{\n  a;\n  b;\n  c;\n  d;\n  e;\n}\n' > 1
printf 'int f()\n{\n  a;\n  b;\n  c;\n  d;\n  E;\n}\n' > 2
diff -p -u --label a --label b 1 2
diff -F '^int' -u --label a --label b 1 2
