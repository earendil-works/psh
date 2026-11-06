# Agent shape: redirections: `2>&1` (15%), `/dev/null` (7.6%), to files (6.7%);
# research/corpus.md §4.

#### 2>&1 merges stderr into stdout
{ echo out; echo err >&2; } 2>&1

#### 2>/dev/null drops errors
ls missing-file 2>/dev/null
echo "status=$?"

#### >/dev/null 2>&1 silences everything
{ echo out; echo err >&2; } >/dev/null 2>&1
echo done

#### order matters: 2>&1 >/dev/null
{ echo out; echo err >&2; } 2>&1 >/dev/null

#### > writes a file
echo hello > out.txt
cat out.txt

#### >> appends
echo one > log.txt
echo two >> log.txt
cat log.txt

#### &> writes both streams to a file
{ echo out; echo err >&2; } &> both.txt
sort both.txt

#### >&2 writes to stderr
echo "to stderr" >&2
echo "to stdout"

#### < reads a file
printf 'x\ny\n' > in.txt
wc -l < in.txt

#### redirect to a file in a subdirectory that does not exist
echo hi > nodir/out.txt
echo "status=$?"

#### truncate with : >
echo data > t.txt
: > t.txt
wc -c < t.txt

#### exec redirection for the rest of the script
exec 3> fd3.txt
echo "via 3" >&3
exec 3>&-
cat fd3.txt

#### redirect a whole group
{ echo a; echo b; } > group.txt
cat group.txt

#### redirect a for loop
for i in 1 2 3; do echo "n$i"; done > loop.txt
tail -n 1 loop.txt

#### noclobber refuses to overwrite
echo first > keep.txt
set -o noclobber
echo second > keep.txt
echo "status=$?"
cat keep.txt

#### >| overrides noclobber
set -C
echo first > f.txt
echo second >| f.txt
cat f.txt

#### stderr of a missing command
nonexistent-cmd-xyz
echo "status=$?"

#### redirect stdin from /dev/null
cat < /dev/null
echo "status=$?"

#### tee to a file and stdout
echo teed | tee copy.txt
cat copy.txt
