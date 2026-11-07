# touch: creating files and setting times.

#### creates missing files
touch a b
[ -f a ] && [ -f b ] && [ ! -s a ] && echo made

#### keeps the contents of existing files
echo data > f
touch f
cat f

#### -c does not create
touch -c nope
echo "status=$?"
[ -e nope ] || echo none

#### a missing directory fails
touch x/y
echo "status=$?"

#### the umask applies to new files
umask 077
touch a
ls -l a | cut -c1-10

#### -t sets a time
touch -t 202401010000 old
touch -t 202401010000.01 newer
[ newer -nt old ] && echo newer

#### -t with a two-digit year
touch -t 2401010000 a
touch -t 202401010000 b
[ a -nt b ] || [ a -ot b ] || echo same

#### -r copies another file's times
touch -t 200001010000 ref
touch f
touch -r ref f
[ f -nt ref ] || [ f -ot ref ] || echo same

#### -d with an ISO date
## expect: gnu (BSD touch -d takes only YYYY-MM-DDThh:mm:SS)
touch -d '2024-01-02 03:04:05' a
touch -t 202401020304.05 b
[ a -nt b ] || [ a -ot b ] || echo same

#### -d with the BSD form
touch -d 2024-01-02T03:04:05 a
touch -t 202401020304.05 b
[ a -nt b ] || [ a -ot b ] || echo same

#### -d with a relative date
## expect: gnu (BSD touch -d takes only YYYY-MM-DDThh:mm:SS)
touch -d '2 days ago' a
touch -d yesterday b
touch c
[ a -ot b ] && [ b -ot c ] && echo ordered

#### -d with epoch seconds
## expect: gnu (BSD touch -d takes only YYYY-MM-DDThh:mm:SS)
touch -d @0 a
touch -t 197001010000 b
[ a -nt b ] || [ a -ot b ] || echo same

#### -m changes only the modification time
touch -t 200001010000 a
touch -m -t 201001010000 a
touch -t 201001010000 b
[ a -nt b ] || [ a -ot b ] || echo same

#### an invalid -t stamp
touch -t 99 f
echo "status=$?"

#### an invalid -d date
## expect: gnu (BSD touch -d rejects this differently)
touch -d 'not a date' f
echo "status=$?"

#### no operand
touch
echo "status=$?"
