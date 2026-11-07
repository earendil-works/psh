# awk: errors and unsupported options.

#### a syntax error
## expect: gnu (D016: gawk exits 1 on a syntax error with its message, BSD awk 2)
awk 'BEGIN { x = }'; echo "status $?"

#### a missing input file
awk '{ print }' nonexistent; echo "status $?"

#### division by zero at run time
echo 0 | awk '{ print 1 / $1 }'; echo "status $?"

#### no program
awk 2>/dev/null; echo "status $?"
