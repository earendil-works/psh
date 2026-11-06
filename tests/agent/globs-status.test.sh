# Agent shape: globs (6.4%; an unmatched glob stays literal in bash, unlike zsh),
# status handling (`|| true`, `; echo "exit=$?"`, set -e); research/corpus.md §4, §9.

#### glob matches files
touch a.txt b.txt c.log
echo *.txt

#### unmatched glob stays literal
echo *.nomatch

#### unmatched glob passed to ls fails
ls *.nomatch
echo "status=$?"

#### quoted glob is not expanded
touch x.txt
echo "*.txt"

#### for loop over a glob
touch one.md two.md
for f in *.md; do echo "file: $f"; done

#### for loop over an unmatched glob runs once
for f in *.zzz; do echo "got: $f"; done

#### nullglob drops unmatched globs
shopt -s nullglob
echo start *.zzz end

#### glob in a subdirectory
mkdir -p dir/sub && touch dir/sub/k.rs dir/sub/l.rs
echo dir/*/*.rs

#### rm -f with a glob
touch tmp1.o tmp2.o keep.c
rm -f *.o
ls

#### || true hides a failure
false || true
echo "status=$?"

#### || echo on failure
ls missing 2>/dev/null || echo "missing"

#### ; echo exit status idiom
grep -q zzz /dev/null; echo "exit=$?"

#### set -e stops at the first failure
set -e
echo before
false
echo after

#### set -e does not stop in a condition
set -e
if false; then echo yes; fi
false || echo handled
echo end

#### set -euo pipefail
set -euo pipefail
echo ok | grep -q ok
echo passed

#### exit with a status
sh -c 'exit 7'
echo "status=$?"

#### command not found is 127
definitely-not-a-command-zz 2>/dev/null
echo "status=$?"

#### negation with !
! false
echo "status=$?"

#### timeout kills a slow command
timeout 1 sleep 5
echo "status=$?"

#### timeout lets a fast command finish
timeout 5 echo fast
echo "status=$?"
