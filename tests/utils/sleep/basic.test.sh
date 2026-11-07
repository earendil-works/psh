# sleep: fractions and GNU suffixes.

#### a fraction
sleep 0.1
echo done

#### several operands add up
## expect: gnu (BSD sleep takes one operand)
sleep 0.05 0.05
echo done

#### a suffix
## expect: gnu (BSD sleep has no suffixes)
sleep 0.001m
echo done

#### an invalid interval
## expect: gnu (D016: GNU's messages)
sleep x
echo "status=$?"

#### no operand
## expect: gnu (D016: GNU's messages)
sleep
echo "status=$?"
