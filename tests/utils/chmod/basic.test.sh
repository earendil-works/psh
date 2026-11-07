# chmod: octal and symbolic modes, recursion.

#### octal modes
touch f
chmod 755 f
ls -l f | cut -c1-10
chmod 600 f
ls -l f | cut -c1-10
chmod 0 f
ls -l f | cut -c1-10

#### +x and friends
touch f
chmod +x f
ls -l f | cut -c1-10
chmod -x f
ls -l f | cut -c1-10
chmod u+x,g+w f
ls -l f | cut -c1-10

#### +w and = without who follow the umask
touch f
chmod 444 f
umask 022
chmod +w f
ls -l f | cut -c1-10
chmod =rw f
ls -l f | cut -c1-10

#### a, o, = and copying
touch f
chmod a=r f
ls -l f | cut -c1-10
chmod u=rwx,go= f
ls -l f | cut -c1-10
chmod g=u f
ls -l f | cut -c1-10
chmod o+r,o-r,o+x f
ls -l f | cut -c1-10

#### X only for directories and executables
mkdir d
touch d/f d/x
chmod 700 d/x
chmod -R a+rX d
ls -l d | cut -c1-10
ls -ld d | cut -c1-10

#### setuid, setgid and sticky
touch f
mkdir d
chmod u+s,g+s f
ls -l f | cut -c1-10
chmod +t d
ls -ld d | cut -c1-10
chmod 4755 f
ls -l f | cut -c1-10

#### -R changes a tree
mkdir -p d/e
touch d/f d/e/g
chmod -R go-rwx d
ls -ld d d/e | cut -c1-10
ls -l d/f d/e/g | cut -c1-10

#### -R does not follow symlinks inside
mkdir d
touch t
chmod 644 t
ln -s ../t d/l
chmod -R 700 d
ls -l t | cut -c1-10

#### a symlink operand is followed
touch t
ln -s t l
chmod 600 l
ls -l t | cut -c1-10

#### a missing file
chmod 644 nope
echo "status=$?"
chmod -f 644 nope
echo "status=$?"

#### an invalid mode
touch f
chmod bad f
echo "status=$?"
chmod 8 f
echo "status=$?"

#### -v and -c
## expect: gnu (D016: GNU's messages)
touch f
chmod 644 f
chmod -v 755 f
chmod -v 755 f
chmod -c 755 f
chmod -c 700 f

#### --reference
## expect: gnu (BSD chmod has no --reference)
touch f g
chmod 640 f
chmod --reference=f g
ls -l g | cut -c1-10

#### no operand
chmod
echo "status=$?"
chmod 755
echo "status=$?"
