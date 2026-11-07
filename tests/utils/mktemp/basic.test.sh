# mktemp: random names (checked by shape, then removed), exclusive creation, modes.

#### a file from a template here
f=$(mktemp foo.XXXXXX)
case $f in foo.??????) echo shape;; esac
[ -f "$f" ] && [ ! -s "$f" ] && echo file
ls -l "$f" | cut -c1-10
rm "$f"

#### a directory
d=$(mktemp -d tmpd.XXXXXXXX)
[ -d "$d" ] && echo dir
ls -ld "$d" | cut -c1-10
rmdir "$d"

#### -p puts it in a directory
mkdir t
f=$(mktemp -p "$PWD/t" x.XXXX)
case $f in "$PWD"/t/x.????) echo shape;; esac
set -- t/*
echo $#
rm -r t

#### the default template in TMPDIR
## expect: gnu (macOS mktemp prefers its per-user directory over TMPDIR)
mkdir t
export TMPDIR=$PWD/t
f=$(mktemp)
case $f in "$PWD"/t/tmp.??????????) echo shape;; esac
rm -r t

#### -t with a template (GNU)
## expect: gnu (D016: an argument with XXX is GNU's template)
mkdir t
export TMPDIR=$PWD/t
f=$(mktemp -t foo.XXXX)
case $f in "$PWD"/t/foo.????) echo shape;; esac
rm -r t

#### -t with a prefix (BSD)
## expect: bsd (D016: an argument without XXX is BSD's prefix)
f=$(mktemp -t foo)
case ${f##*/} in foo.??????????) echo shape;; esac
rm -f "$f"

#### -u does not create
f=$(mktemp -u foo.XXXXXX)
[ -e "$f" ] || echo none

#### every name differs
a=$(mktemp a.XXXXXXXX)
b=$(mktemp a.XXXXXXXX)
[ "$a" != "$b" ] && echo differ
set -- a.*
echo $#
rm "$a" "$b"

#### too few X's
## expect: gnu (D016: GNU's template rules; BSD uses the name as it is)
mktemp x
echo "status=$?"

#### a missing directory
mktemp -q nope/x.XXXX
echo "status=$?"

#### --suffix
## expect: gnu (BSD mktemp has no --suffix)
f=$(mktemp --suffix=.txt s.XXXX)
case $f in s.????.txt) echo shape;; esac
rm "$f"

#### several templates
## expect: bsd (GNU mktemp takes one template)
set -- $(mktemp a.XXXX b.XXXX)
echo $#
rm "$@"
