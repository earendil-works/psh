# tee: copies stdin to stdout and files.

#### to stdout and a file
echo hello | tee f
cat f

#### several files
printf 'a\nb\n' | tee f g > /dev/null
cat f g

#### -a appends
echo one > f
echo two | tee -a f
cat f

#### without -a the file is truncated
echo old > f
echo new | tee f > /dev/null
cat f

#### a file that cannot be opened does not stop the rest
echo x | tee nodir/f g
echo "status=$?"
cat g

#### - is a file name
echo x | tee - > /dev/null
cat ./-

#### no input, no files
tee < /dev/null
echo "status=$?"

#### -i is accepted
echo x | tee -i f
cat f

#### -p and --output-error
## expect: gnu (BSD tee has no -p)
echo x | tee -p f
echo y | tee --output-error=warn g
cat f g

#### in the middle of a pipeline
seq 1 3 | tee f | wc -l | tr -d ' '
cat f
