# du: sizes. Allocated blocks differ by file system, so most cases use files whose
# usage is the same everywhere, or apparent sizes.

#### an empty file
touch f
du f

#### a file of one block
printf x > f
du -k f

#### -h
printf x > f
du -h f

#### -c totals
## expect: gnu (D016: GNU's 1 KiB units; BSD counts 512-byte blocks)
printf x > a
printf y > b
du -c a b

#### apparent sizes
## expect: gnu (BSD du has no --apparent-size; psh takes both)
mkdir -p d/e
printf '%3000s' '' > d/a
printf '%5000s' '' > d/e/b
du --apparent-size d
du --apparent-size -a -b d

#### BSD -A
## expect: bsd (BSD's -A is GNU's --apparent-size; GNU du has no -A)
printf '%3000s' '' > f
du -A -k f

#### -s, -d and --max-depth
## expect: gnu (BSD du counts directory blocks differently)
mkdir -p d/e/f
printf '%2000s' '' > d/e/f/x
du -b -s d
du -b -d 1 d
du -b --max-depth=0 d

#### --exclude
## expect: gnu (BSD du has no --exclude)
mkdir -p d/e
printf '%100s' '' > d/a
printf '%200s' '' > d/e/b
du -a -b --exclude='*b' d
du -b --exclude=e d

#### hard links count once
## expect: gnu (BSD du has no -b)
mkdir d
printf '%100s' '' > d/a
ln d/a d/b
du -s -b d
du -s -b -l d

#### a missing file
du nope
echo "status=$?"
