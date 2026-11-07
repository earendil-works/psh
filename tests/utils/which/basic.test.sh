# which: PATH hits (utilities report themselves; see the unit tests).

#### the first hit and -a
mkdir bin bin2
printf '#!/bin/sh\n' > bin/tool
chmod +x bin/tool
cp bin/tool bin2/tool
PATH=$PWD/bin:$PWD/bin2:$PATH
which tool | sed "s|$PWD|.|"
echo "st=$?"
which -a tool | sed "s|$PWD|.|"

#### not found is status 1
mkdir bin
printf 'x' > bin/noexec
PATH=$PWD/bin:$PATH
which noexec
echo "st=$?"
which nothere
echo "st=$?"

#### several names, one missing
mkdir bin
printf '#!/bin/sh\n' > bin/tool
chmod +x bin/tool
PATH=$PWD/bin:$PATH
which tool nothere | sed "s|$PWD|.|"
echo "st=${PIPESTATUS[0]}"

#### -s only sets the status
mkdir bin
printf '#!/bin/sh\n' > bin/tool
chmod +x bin/tool
PATH=$PWD/bin:$PATH
which -s tool
echo "st=$?"
which -s nothere
echo "st=$?"

#### a path is checked as it is
mkdir bin
printf '#!/bin/sh\n' > bin/tool
chmod +x bin/tool
printf 'x' > bin/plain
which ./bin/tool
echo "st=$?"
which bin/tool
echo "st=$?"
which bin/plain
echo "st=$?"
