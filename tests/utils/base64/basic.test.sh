# base64: GNU and BSD flags (D016), RFC 4648 encoding.

#### encode stdin
printf 'hello\n' > a
base64 < a
printf 'foobar' | base64

#### encode a file operand (GNU); empty input is no output
## expect: gnu (BSD base64 takes no operand and ends empty output with a newline)
printf 'hello\n' > a
base64 a
printf '' | base64 | wc -c

#### the RFC 4648 vectors
for s in f fo foo foob fooba foobar; do printf %s "$s" | base64; done

#### long input wraps at 76 columns
## expect: gnu (D016: GNU wraps at 76, BSD not at all)
seq 1 40 | base64

#### -w N wraps at N (GNU)
## expect: gnu (BSD base64 has no -w)
seq 1 30 | base64 -w 20
seq 1 30 | base64 --wrap=30

#### -w 0 writes one line without a newline (GNU)
## expect: gnu (BSD base64 has no -w)
seq 1 30 | base64 -w 0
echo

#### -b N breaks lines (BSD)
## expect: bsd (GNU base64 has no -b)
seq 1 30 | base64 -b 20

#### -d decodes
echo aGVsbG8K | base64 -d
echo aGVsbG8K | base64 --decode
printf 'aGVs\nbG8K\n' | base64 -d

#### -D decodes (BSD)
## expect: bsd (GNU base64 has no -D)
echo aGk= | base64 -D

#### decoding round trip
seq 1 100 > f
base64 < f | base64 -d | cmp - f && echo same

#### invalid input fails after what decoded
## expect: gnu (D016: GNU's decoder and messages)
echo 'aGVs bG8K' | base64 -d
echo " st=$?"
echo 'aGV!' | base64 -d
echo " st=$?"

#### unpadded input decodes
## expect: gnu (D016: GNU's decoder)
echo aGk | base64 -d
echo " st=$?"
echo 'aGk=aGk=' | base64 -d
echo " st=$?"

#### -i alone ignores garbage (GNU)
## expect: gnu (BSD -i takes a file)
echo 'aGV!sbG8K' | base64 -d -i
echo "st=$?"

#### -i FILE reads the file (BSD, D016)
printf 'hello\n' > a
base64 -i a
base64 -i a | base64 -d -i -

#### -o FILE writes the file (BSD)
## expect: bsd (GNU base64 has no -o)
printf 'hello\n' > a
base64 -i a -o out
echo "st=$?"
cat out
base64 -d -i out -o back
cat back

#### a missing file
## expect: gnu (D016: GNU's messages)
base64 nope
echo "st=$?"
