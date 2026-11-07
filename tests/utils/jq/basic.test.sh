# jq: jq 1.7.1's behavior, output and messages (the recording container's jq and macOS's
# /usr/bin/jq are both 1.7.1).

#### pretty output by default, -c compact
echo '{"a":1,"b":[1,2,{"c":null}],"d":{}}' | jq .; echo '{"a":[1,{}]}' | jq -c .

#### field access, iteration, pipes
echo '{"items":[{"name":"x","n":1},{"name":"y","n":2}]}' | jq -c '.items[] | {name, double: (.n * 2)}'

#### -r raw strings, -j joined
echo '["a","b",1,null]' | jq -r '.[]'; echo '["a","b"]' | jq -j '.[]'; echo

#### -n null input and -e exit status
jq -n '1+1'; jq -n -e 'false'; echo "status=$?"; jq -n -e 'null'; echo "status=$?"; jq -n -e 'empty'; echo "status=$?"; jq -ne '1'; echo "status=$?"

#### -s slurps every input into one array, also over several files
printf '1 2\n3\n' | jq -c -s .; echo '{"a":1}' > a.json; echo '{"b":2}' > b.json; jq -s -c . a.json b.json

#### several files without -s, input_filename
echo '{"a":1}' > a.json; echo '[2]' > b.json; jq -c '[., input_filename]' a.json b.json

#### -S sorts keys, --tab and --indent
echo '{"b":1,"a":{"d":1,"c":2}}' | jq -S -c .; echo '{"a":[1]}' | jq --tab .; echo '{"a":[1]}' | jq --indent 4 .; echo '{"a":[1]}' | jq --indent 0 .

#### -a escapes non-ASCII
echo '"é😀"' | jq -a .

#### -R raw input lines, -R -s whole text
printf 'one\ntwo\n' | jq -R .; printf 'one\ntwo\n' | jq -Rs .

#### --arg and --argjson, $ARGS
jq -n -c --arg name x --argjson v '{"k":[1]}' '{name: $name, v: $v, named: $ARGS.named}'

#### --args and --jsonargs
jq -n -c '$ARGS.positional' --args a b c; jq -n -c '$ARGS.positional' --jsonargs 1 '{"x":2}' null

#### --slurpfile and --rawfile
printf '1 2\n' > nums.json; printf 'raw text\n' > t.txt; jq -n -c --slurpfile n nums.json --rawfile t t.txt '[$n, $t]'

#### -f reads the filter from a file
printf '.a | . + 1\n' > prog.jq; echo '{"a":41}' | jq -f prog.jq

#### options combine and may follow the filter
echo '{"a":"x"}' | jq '.a' -r; echo '[1,2]' | jq -cr '.'

#### numbers keep their literals until arithmetic
echo '[1.000, 100000000000000000000, 1e1000, 0.1, 3.0]' | jq -c .; echo '[1.000, 3.0]' | jq -c 'map(. + 0)'; jq -n -c '[0.1 + 0.2, 1e17 + 0, 1/3, 1e-5 + 0, 100 / 4]'

#### key order is insertion order; keys sorts
jq -n -c '{b: 1, a: 2} | ., keys, (. + {c: 3, b: 4})'

#### runtime errors: jq's messages and status 5
echo '{"a":1}' | jq '.a.b'; echo "status=$?"; echo 'null' | jq '.[]'; echo "status=$?"; echo '{"a":"x"}' | jq '.a + 1'; echo "status=$?"

#### an error in one input does not stop the others
printf '1\n"x"\n2\n' | jq '. + 1'; echo "status=$?"

#### error with a non-string value
jq -n '{"a":1} | error'; echo "status=$?"

#### parse errors in the input
printf '{"a":1,}\n' | jq .; echo "status=$?"; printf '[1,2\n' | jq .; echo "status=$?"; printf '1\nx\n' | jq .; echo "status=$?"

#### compile errors: status 3
jq -n 'nosuchfn'; echo "status=$?"

#### a missing file: status 2, the others still read
echo 1 > one.json; jq . one.json nosuch.json; echo "status=$?"

#### usage errors: status 2
jq --indent 9 . </dev/null; echo "status=$?"; jq -n --arg x; echo "status=$?"

#### try, catch, //, ?
echo '[1,"a",null]' | jq -c '[.[] | try (. + 1) catch "bad"], [.[] | . // "d"], [.[] | tonumber?]'

#### the destructuring alternative ?//
echo '[[1,2],{"a":3},4]' | jq -c '.[] as [$a, $b] ?// {a: $a} ?// $a | [$a, $b]'

#### updates: |=, +=, del, |= empty
echo '{"a":[1,2,3],"b":{"c":1}}' | jq -c '.a |= map(. * 10) | .b.c += 1, del(.a[0]), (.a[] |= empty), (.b |= with_entries(.value |= tostring))'

#### paths, getpath, setpath, to_entries
echo '{"a":{"b":[1,2]}}' | jq -c '[paths], getpath(["a","b",1]), setpath(["a","x"]; true), to_entries'

#### reduce, foreach, limit, first, range
jq -n -c '[reduce range(5) as $i (0; . + $i), [foreach (1,2,3) as $x (0; . + $x)], [limit(2; range(10))], first(range(3;9)), [range(0;10;3)]]'

#### strings: split, join, test, sub, gsub, ascii case, ltrimstr
echo '"Hello, World"' | jq -c '[split(", "), (split(", ") | join("-")), test("world"; "i"), sub("o"; "0"), gsub("o"; "0"), ascii_downcase, ltrimstr("Hello"), (. / ",")]'

#### regex: match, capture, scan, splits
echo '"a1b22c333"' | jq -c '[match("[0-9]+"; "g") | .string], (capture("(?<l>[a-z])(?<d>[0-9]+)")), [scan("[a-z]")], [splits("[0-9]+")]'

#### formats: @csv @tsv @sh @json @base64 @base64d @uri @html @text
echo '[1,"a,b","c\"d",null,true]' | jq -r '@csv, @tsv, @json, @text'; echo '"it'\''s a <b>&</b>"' | jq -r '@sh, @base64, (@base64|@base64d), @uri, @html'

#### string interpolation and tostring, tojson, fromjson
echo '{"n":1.50,"s":"x"}' | jq -c '"n=\(.n) s=\(.s)", (.n|tostring), tojson, ("[1, 2]" | fromjson)'

#### sort_by, group_by, unique_by, min_by, max_by
echo '[{"k":2,"v":"b"},{"k":1,"v":"a"},{"k":2,"v":"c"}]' | jq -c 'sort_by(.k), group_by(.k), unique_by(.k), min_by(.k), max_by(.k)'

#### length, keys, has, in, contains, inside, indices
echo '{"a":[1,2,1],"s":"abca"}' | jq -c '[(.a|length), (.s|length), keys, has("a"), ("a"|in({"a":1})), (.a|contains([2])), (.a|indices(1)), (.s|indices("a"))]'

#### type, select, map_values, with_entries, add, any, all, flatten
echo '[1,"a",[2,[3]],null,{"x":true}]' | jq -c '[.[]|type], [.[]|numbers], (map(select(type=="array"))|flatten), ([1,2,3]|add), ([true,false]|any,all)'

#### env and $ENV
FOO=bar jq -n -r 'env.FOO, $ENV.FOO'

#### dates: todate, fromdate, strftime, gmtime, mktime, strptime
jq -n -c '0|todate, ("2015-03-05T23:51:47Z"|fromdate), (1425599507|strftime("%A, %B %d, %Y")), (1425599507|gmtime), (1425599507|gmtime|mktime), ("10:15 2015-03-05"|strptime("%H:%M %Y-%m-%d"))'

#### --seq and --stream
jq -n --seq '1,[2]' | tr '\036' R; echo '{"a":[1,2]}' | jq -c --stream .

#### input and inputs
printf '1 2 3 4\n' | jq -c '[., input]'; printf '1 2 3\n' | jq -n -c '[inputs]'

#### debug and stderr go to stderr
echo '[1]' | jq -c 'debug | .[0] | stderr | . + 1' 2>err; echo; cat err; echo

#### halt and halt_error
jq -n '"bye\n" | halt_error(3)'; echo "status=$?"; jq -n '1, halt, 2'; echo "status=$?"

#### --raw-output0
jq -n --raw-output0 '"a","b"' | tr '\0' N; echo

#### a large file of objects
seq 1 2000 | jq -c -s 'map({n: ., sq: (. * .)}) | map(select(.n % 500 == 0))'

#### -C colors, -M plain
echo '{"a":[1,"x",null,true,{"b":false}],"c":{}}' | jq -C . | tr '\033' E; echo '{"a":1}' | jq -C -c . | tr '\033' E; echo '[1]' | jq -M .
