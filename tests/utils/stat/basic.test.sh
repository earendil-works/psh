# stat: GNU's -c and BSD's -f formats, chosen per D016.

#### GNU -c
## expect: gnu (BSD stat has no -c)
printf 12345 > f
chmod 640 f
stat -c '%s %n %a %A %F' f

#### GNU -c times
## expect: gnu (BSD stat has no -c)
touch -d '2024-01-02 03:04:05' f
stat -c '%Y %y' f
stat -c %.3Y f

#### GNU %N and -L for symlinks
## expect: gnu (BSD stat has no -c)
touch f
ln -s f l
stat -c '%N %F' l
stat -L -c '%n %F' l

#### GNU --printf and --format
## expect: gnu (BSD stat has no --printf)
printf abc > f
stat --printf '%n\t%s\n' f
stat --format='%n=%s' f

#### GNU file types
## expect: gnu (BSD stat has no -c)
mkdir d
touch e
printf x > f
stat -c %F d e f /dev/null

#### GNU widths
## expect: gnu (BSD stat has no -c)
printf 123456 > f
chmod 644 f
stat -c '[%10s|%-4n|%05a]' f

#### GNU -t terse
## expect: gnu (D016: -t without a BSD format is GNU's terse output)
printf 12 > f
stat -t f | cut -d' ' -f1,2

#### GNU default output
## expect: gnu (D016: the default output is GNU's)
printf 12 > f
stat f | head -n 1
stat f | sed -n 2p | tr -s ' \t' ' ' | cut -d' ' -f2,3

#### BSD -f
## expect: bsd (D016: -f with a % is BSD's format; GNU's -f is file-system status)
printf 12345 > f
chmod 640 f
stat -f '%z %N %Lp %Sp %p' f

#### BSD -f times and -t
## expect: bsd (D016: -f with a % is BSD's format)
touch -t 202401020304.05 f
stat -f %m f
stat -f %Sm -t '%Y-%m-%d %H:%M:%S' f
stat -t %F -f %Sm f

#### BSD file types and links
## expect: bsd (D016: -f with a % is BSD's format)
mkdir d
touch f
ln -s f l
stat -f '%N %HT%SY' f d l
stat -f '%N%T' d l

#### BSD widths
## expect: bsd (D016: -f with a % is BSD's format)
printf 123456 > f
chmod 644 f
stat -f '[%10z|%-4N|%05Lp]' f

#### a missing file
stat -c %s nope
echo "status=$?"
