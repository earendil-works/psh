# realpath: physical paths; GNU's modes and relative output.

#### files and directories
mkdir -p d/sub
touch d/f
cd d
realpath f sub . ..

#### symlinks are resolved
mkdir -p d/sub
ln -s d/sub l
realpath l l/..

#### a missing last component is fine
## expect: gnu (D016: BSD realpath needs the file to exist)
realpath missing

#### a missing directory fails
realpath missing/x
echo "status=$?"

#### a file used as a directory fails
## expect: gnu (D016: BSD realpath accepts f/)
touch f
realpath f/
echo "status=$?"
realpath f/x
echo "status=$?"

#### -e needs every component
## expect: gnu (BSD realpath has no -e)
touch f
realpath -e f missing
echo "status=$?"

#### -m needs none
## expect: gnu (BSD realpath has no -m)
touch f
realpath -m f/x/../y nope/z

#### -s keeps symlinks
## expect: gnu (BSD realpath has no -s)
mkdir -p d/sub
ln -s d/sub l
realpath -s l l/..

#### --relative-to
## expect: gnu (BSD realpath has no --relative-to)
mkdir -p a/b c
touch c/f
realpath --relative-to=a/b c/f
realpath --relative-to=. c/f a
realpath --relative-to=c c

#### --relative-base
## expect: gnu (BSD realpath has no --relative-base)
mkdir -p a/b
realpath --relative-base=. a/b /
realpath --relative-base=a --relative-to=a/b a /

#### -q is quiet
touch f
realpath -q nope/x f
echo "status=$?"

#### -z ends names with NUL
## expect: gnu (BSD realpath has no -z)
mkdir d
realpath -z --relative-to=. d d | tr '\0' '@'; echo

#### no operand means the working directory
## expect: bsd (GNU realpath needs an operand; psh takes BSD's .)
mkdir d
cd d
realpath
