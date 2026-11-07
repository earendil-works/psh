# timeout: GNU's (macOS has none; every case expects GNU).

#### a command that finishes in time
## expect: gnu (macOS has no timeout)
timeout 5 echo fast
echo "status=$?"

#### a command that times out
## expect: gnu (macOS has no timeout)
timeout 0.2 sleep 5
echo "status=$?"

#### the command's own status passes through
## expect: gnu (macOS has no timeout)
timeout 5 sh -c 'exit 3'
echo "status=$?"

#### KILL gives 137
## expect: gnu (macOS has no timeout)
timeout -s KILL 0.2 sleep 5
echo "status=$?"

#### --preserve-status
## expect: gnu (macOS has no timeout)
timeout --preserve-status 0.2 sleep 5
echo "status=$?"

#### a suffix and -k
## expect: gnu (macOS has no timeout)
timeout -k 1 0.003m sleep 5
echo "status=$?"

#### the whole pipeline inside sh is stopped
## expect: gnu (macOS has no timeout)
timeout 0.3 sh -c 'sleep 5 | sleep 5'
echo "status=$?"

#### timeout runs builtins' standalone forms
## expect: gnu (macOS has no timeout)
timeout 1 printf '%s\n' via-printf
timeout 1 true
echo "status=$?"

#### a command that does not exist
## expect: gnu (macOS has no timeout)
timeout 1 no-such-command-zz 2>/dev/null
echo "status=$?"

#### an invalid duration
## expect: gnu (macOS has no timeout)
timeout x sleep 1
echo "status=$?"

#### timeout inside a pipeline with head
## expect: gnu (macOS has no timeout)
timeout 2 yes | head -n 2
