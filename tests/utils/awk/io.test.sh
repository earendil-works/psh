# awk: getline, output redirection, system and pipes.

#### getline from a file
printf 'one\ntwo\n' > f
awk 'BEGIN { while ((getline line < "f") > 0) n++; print n, line }'

#### getline from a missing file is -1
awk 'BEGIN { print (getline x < "nonexistent") }'

#### plain getline reads the next record
printf '1\n2\n3\n4\n' | awk '{ getline; print }'

#### getline var from the main input
printf 'a\nb\n' | awk 'NR == 1 { getline x; print $0, x, NR }'

#### "cmd" | getline through the shell
awk 'BEGIN { "echo hello; echo world" | getline a; "echo hello; echo world" | getline b; print a, b }'
awk 'BEGIN { while (("printf \"x\\ny\\n\"" | getline l) > 0) print "got", l }'

#### cmd | getline sets $0 and NF
awk 'BEGIN { "echo a b c" | getline; print NF, $2 }'

#### close lets a command run again
awk 'BEGIN { c = "echo once"; c | getline x; close(c); c | getline y; print x, y }'

#### system runs a command and returns its status
awk 'BEGIN { r = system("echo from-system; exit 3"); print "status", r }'

#### system output is in order with print
awk 'BEGIN { print "before"; system("echo during"); print "after" }'

#### print to a file, append and read back
awk 'BEGIN { print "a" > "out"; print "b" > "out"; close("out"); print "c" >> "out" }'
cat out

#### print to a pipe
printf 'b\na\nc\n' | awk '{ print | "sort" }'

#### print to /dev/stderr
awk 'BEGIN { print "to stderr" > "/dev/stderr"; print "to stdout" }' 2>err
echo "stdout done"
cat err

#### printf to /dev/stderr
awk 'BEGIN { printf "e%d\n", 1 > "/dev/stderr" }' 2>&1

#### close of a pipe returns the command's status
## expect: gnu (gawk's close returns the exit status, BSD awk 0)
awk 'BEGIN { print "x" | "cat >/dev/null; exit 2"; print close("cat >/dev/null; exit 2") }'

#### fflush
awk 'BEGIN { printf "a"; fflush(); system("printf b"); print "" }'

#### input files and stdin with -
printf 'from stdin\n' | awk '{ print FILENAME ": " $0 }' -
