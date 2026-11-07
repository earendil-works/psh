# awk: builtin functions.

#### length of a string, of $0 and of an array
echo 'hello world' | awk '{ print length($1), length(), length }'
awk 'BEGIN { a["x"]; a["y"]; a["z"]; print length(a) }'

#### length counts characters in a UTF-8 locale
## expect: gnu (macOS awk 20200816 counts bytes; gawk and current onetrueawk count characters)
awk 'BEGIN { print length("héllo") }'

#### substr
awk 'BEGIN { s = "hello"; print substr(s, 2, 3), substr(s, 3), substr(s, 0, 2), substr(s, -1, 3), "[" substr(s, 9) "]" }'

#### index
awk 'BEGIN { print index("hello", "ll"), index("hello", "z") }'

#### split with FS, a string and a regex
awk 'BEGIN { n = split("a b  c", p); print n, p[3] }'
awk 'BEGIN { n = split("a:b:c", p, ":"); print n, p[1] p[3] }'
awk 'BEGIN { n = split("a1b22c", p, /[0-9]+/); print n, p[2] }'

#### tolower and toupper
awk 'BEGIN { print tolower("MiXeD 1"), toupper("MiXeD 1") }'

#### sub and gsub with & and an escaped &
echo 'foo bar foo' | awk '{ n = gsub(/foo/, "[&]"); print n, $0 }'
echo 'aaa' | awk '{ sub(/a/, "b"); print }'
awk 'BEGIN { s = "x.y.z"; gsub(/\./, "\\&", s); print s }'

#### gsub on a field rebuilds the record
echo 'a-b c-d' | awk '{ gsub(/-/, "+", $2); print }'

#### match sets RSTART and RLENGTH
awk 'BEGIN { print match("foobar", /o+/), RSTART, RLENGTH; print match("x", /y/), RSTART, RLENGTH }'

#### sprintf and printf conversions
awk 'BEGIN { printf "%d|%5.2f|%-4s|%x|%o|%e|%c|%%\n", 42.9, 3.14159, "ab", 255, 8, 1234.5, 65 }'
awk 'BEGIN { s = sprintf("%03d-%s", 7, "x"); print s }'

#### printf with %*d and %.*f
awk 'BEGIN { printf "[%*d] [%-*d] [%.*f]\n", 5, 42, 4, 7, 2, 3.14159 }'

#### int and math functions
awk 'BEGIN { print int(3.9), int(-3.9), sqrt(16), exp(0), log(1), sin(0), cos(0), atan2(0, 1) }'

#### srand with a seed is repeatable
awk 'BEGIN { srand(42); a = rand(); srand(42); b = rand(); print (a == b), (a >= 0 && a < 1) }'

#### strftime with a time and UTC
## expect: gnu (strftime is gawk's; BSD awk has none)
awk 'BEGIN { print strftime("%Y-%m-%d %H:%M:%S", 0, 1); print strftime("%j %a %b", 86400 * 40, 1) }'

#### systime is near the shell's clock
## expect: gnu (systime is gawk's; BSD awk has none)
awk -v now="$(date +%s)" 'BEGIN { d = systime() - now; print (d >= -2 && d <= 2) }'

#### string to number conversions
awk 'BEGIN { print "3x" + 1, " 12 " + 0, ".5" * 2, "1e2" + 0, "abc" + 0 }'
