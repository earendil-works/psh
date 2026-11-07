# seq: integers, steps, decimals, -w -s -f.

#### last only
seq 3

#### first and last
seq 2 4

#### an increment
seq 1 3 10

#### counting down
seq 5 -2 0

#### negative numbers
seq -2 1

#### nothing when first is past last
## expect: gnu (BSD seq counts down: seq 0 prints 1 and 0)
seq 0
seq 5 1
echo "status=$?"

#### -w pads with zeros
seq -w 8 10

#### -w with negative numbers
seq -w -3 1

#### -s separator
## expect: gnu (BSD seq ends with the separator, not a newline)
seq -s, 1 5

#### -s with a space, as agents join numbers
## expect: gnu (BSD seq ends with the separator, not a newline)
echo "[$(seq -s ' ' 3)]"

#### -f format
seq -f 'line%g' 3

#### -f with width and precision
seq -f '%05.1f' 1 2

#### -f %e
seq -f '%e' 2

#### -f with %%
seq -f '%g%%' 2

#### decimal steps
## expect: gnu (D016: GNU decimals; BSD seq prints %g)
seq 0 0.25 1

#### decimals follow the operands
## expect: gnu (BSD seq prints %g: 1 1.5 2)
seq 1 0.5 2

#### a decimal step that does not land exactly on the end
## expect: gnu (D016: GNU decimals; BSD seq prints %g)
seq 0 0.1 0.3

#### the end is not reached exactly
## expect: gnu (D016: GNU decimals; BSD seq prints %g)
seq 1 0.3 2

#### -w with decimals
seq -w 1 0.5 10

#### large integers stay exact
## expect: gnu (BSD seq prints large numbers in %g)
seq 99999999999999999999 100000000000000000001

#### in a for loop
for i in $(seq 1 3); do echo "n$i"; done

#### piped into head
seq 1000000 | head -n 2

#### zero increment
seq 1 0 3
echo "status=$?"

#### invalid argument
## expect: gnu (BSD seq exits 2)
seq x
echo "status=$?"

#### too many operands
## expect: gnu (BSD seq's usage differs)
seq 1 2 3 4
echo "status=$?"

#### -f with -w
## expect: gnu (BSD seq allows -f with -w)
seq -w -f %g 2
echo "status=$?"

#### a format without a directive
seq -f abc 2
echo "status=$?"
