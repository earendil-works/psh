# Agent shape: commands that call a shell again: `bash -c '…'` and `sh -c '…' _ ARGS`
# (often under `xargs` or `find -exec`), `bash FILE`, `bash -n FILE` as a syntax check,
# options on the command line (`-e`, `-euc`, `-o pipefail`, `-x`, `-lc`). psh runs a
# bare `bash` or `sh` itself, as a new shell: only exported variables pass, `exit` ends
# only it, `$0` and `$@` come from its command line. These run as `bash -c CODE`; the
# nested shells are never the last command, so bash does not exec them in place.

#### bash -c with $0 and $@
bash -c 'echo "0=$0 #=$# @=$*"' name a 'b c'
bash -c 'echo "0=$0 #=$#"'
sh -c 'echo "0=$0 1=$1 2=$2"' _ x y
bash -c 'for a in "$@"; do echo "[$a]"; done' _ 'one two' three
echo done

#### only exported variables reach the nested shell
plain=1
export exported=2
f() { echo fn; }
alias ll='echo alias'
bash -c 'echo "plain=${plain-unset} exported=${exported-unset}"'
bash -c 'type f >/dev/null 2>&1 || echo "no function"'
bash -c 'alias ll >/dev/null 2>&1 || echo "no alias"'
exported=changed bash -c 'echo "prefix: $exported"'
echo "after: $exported"

#### options and traps do not leak into the nested shell
set -u
set -o pipefail
trap 'echo outer exit trap' EXIT
bash -c 'echo "flags=$-"; echo "pipefail: $(set -o | grep pipefail)"; echo "undefined=${nope-}"'
bash -c 'trap -p; echo "no traps above"'
echo end

#### exit, set -e and traps of the nested shell stay inside it
bash -c 'exit 3'
echo "status=$?"
bash -c 'set -e; false; echo not here'
echo "status=$?"
bash -c 'trap "echo inner exit trap" EXIT; echo body'
echo "outer goes on"
set -e
bash -c 'false' || echo "failed, outer goes on"
echo end

#### SHLVL, BASH_SUBSHELL, $$ and PPID of a nested shell
lvl=$SHLVL
bash -c 'echo $SHLVL' > lvl.txt
echo "SHLVL up by $(( $(cat lvl.txt) - lvl ))"
bash -c 'echo "BASH_SUBSHELL=$BASH_SUBSHELL"'
( bash -c 'echo "in a subshell: BASH_SUBSHELL=$BASH_SUBSHELL"'; true )
bash -c 'echo $PPID' > ppid.txt
[ "$(cat ppid.txt)" = "$$" ] && echo "PPID is the caller's \$\$"
bash -c 'echo $$' > pid.txt
[ "$(cat pid.txt)" != "$$" ] && echo "a pid of its own"
bash -c '[ "$$" = "$BASHPID" ] && echo "BASHPID is \$\$"'
rm -f lvl.txt ppid.txt pid.txt

#### nested bash -c inside bash -c
bash -c "bash -c 'echo \"deep \$0 \$1\"' inner x; echo middle \$0" outer
bash -c 'bash -c "bash -c \"echo three levels\"; echo two"; echo one'
x=$(bash -c 'echo "in \$(…)"'; true)
echo "$x"
echo done

#### command line options
bash -e -c 'false; echo not here'
echo "e: $?"
bash -euc 'echo "${undefined_var}"' 2>/dev/null
echo "eu: $?"
bash -o pipefail -c 'false | true; echo "pipefail: $?"'
bash -O extglob -c 'shopt -q extglob && echo extglob on'
bash -x -c 'echo traced' 2>&1
bash -lc 'echo login'
bash --norc --noprofile -c 'echo norc'
bash -c 'echo "flags: $-"'
bash -n -c 'echo not run'
echo "n: $?"

#### bash FILE and sh FILE
printf 'echo "script $0 #=$# 1=$1"\n' > s.sh
bash s.sh one two
sh ./s.sh x
printf 'exit 7\n' > e.sh
bash e.sh
echo "status=$?"
printf 'echo "args: $*"\nshift\necho "after shift: $*"\n' > args.sh
bash args.sh a b c

#### bash FILE errors
bash missing.sh
echo "missing: $?"
mkdir dir.sh
bash dir.sh
echo "dir: $?"
printf 'a\000b' > binary
bash binary
echo "binary: $?"

#### bash -n checks a script without running it
printf 'echo would run\nif true; then\n  echo fine\nfi\n' > good.sh
printf 'echo would run\nif true; then\n  echo missing fi\n' > bad.sh
bash -n good.sh
echo "good: $?"
bash -n bad.sh
echo "bad: $?"
bash bad.sh
echo "running bad: $?"
for f in good.sh bad.sh; do bash -n "$f" 2>/dev/null && echo "$f ok" || echo "$f broken"; done

#### xargs runs sh -c with each argument
printf 'a\nb c\n' | xargs -I{} sh -c 'echo "[$1]"' _ {}
printf '1\n2\n3\n' | xargs -n1 bash -c 'echo $(( $1 * 2 ))' _
printf 'x y z\n' | xargs sh -c 'echo "$# args: $*"' sh
echo done

#### find -exec runs bash -c on each file
mkdir -p d/sub
touch d/a.txt d/sub/b.txt d/c.log
find d -name '*.txt' -exec bash -c 'echo "found $(basename "$1")"' _ {} \; | sort
find d -name '*.txt' -exec sh -c 'for f; do echo "batch $f"; done' sh {} + | sort
find d -type f -exec bash -c '[ "${1##*.}" = log ] && echo "log: $1"' _ {} \;
echo done

#### wrappers: env, timeout, nohup, command, time
env FOO=bar bash -c 'echo "FOO=$FOO"'
env -i PATH="$PATH" bash -c 'echo "HOME=${HOME-unset}"'
timeout 5 bash -c 'echo in timeout'
echo "timeout ok: $?"
timeout 1 bash -c 'sleep 5; echo not here'
echo "timed out: $?"
nohup sh -c 'echo nohup out' > out.txt 2>/dev/null
cat out.txt
command bash -c 'echo through command'
{ time bash -c 'echo timed'; } 2>/dev/null
command -v bash >/dev/null && echo "bash found"
command -v sh >/dev/null && echo "sh found"

#### exec runs the nested shell in place
( exec bash -c 'echo exec ok; exit 4' )
echo "status=$?"
( exec -a renamed bash -c 'echo "0=$0"' )
bash -c 'echo $SHLVL > before.txt; exec bash -c "echo \$SHLVL > after.txt"'
[ "$(cat before.txt)" = "$(cat after.txt)" ] && echo "exec keeps SHLVL"
bash -c 'exec sh -c "echo exec in nested \$0"'
rm before.txt after.txt

#### descriptors, pipes and the working directory are shared
bash -c 'echo piped' | tr a-z A-Z
echo input | bash -c 'read -r x; echo "read $x"'
printf 'l1\nl2\n' > in.txt
bash -c 'wc -l' < in.txt | tr -d ' '
bash -c 'echo to3 >&3' 3>&1
exec 4>out4.txt
bash -c 'echo via4 >&4'
exec 4>&-
cat out4.txt
bash -c 'echo err >&2' 2>&1 | sed 's/^/stderr: /'
mkdir -p sub
cd sub
bash -c 'basename "$PWD"; basename "$(pwd)"'
bash -c 'cd /; echo "cd inside"'
basename "$PWD"

#### a signal to a nested shell runs its trap
bash -c 'trap "echo got TERM; exit 0" TERM; echo ready > ready; while :; do sleep 0.05; done' &
while [ ! -s ready ]; do sleep 0.05; done
kill $!
wait $!
echo "status=$?"
rm ready
sh -c 'echo $$ > pidfile; trap "echo got USR1" USR1; while [ ! -e go ]; do sleep 0.05; done' &
while [ ! -s pidfile ]; do sleep 0.05; done
kill -USR1 "$(cat pidfile)"
sleep 0.3
touch go
wait
echo "done"
rm -f pidfile go

#### kill $$ in a nested shell ends only it
bash -c 'kill $$; echo not here' 2>/dev/null
echo "status=$?"
bash -c 'trap "echo trapped; exit 5" TERM; kill $$; echo not here'
echo "status=$?"
bash -c 'trap "" TERM; kill $$; echo ignored'
echo end

#### a path runs the program
/bin/sh -c 'echo "0=$0"'
echo "status=$?"

#### options psh lacks run the real bash
bash --rcfile /dev/null -c 'echo rcfile'
echo "status=$?"
