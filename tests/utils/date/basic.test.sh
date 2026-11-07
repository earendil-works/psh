# date: GNU and BSD flags (D016), fixed instants (the cases run with TZ=UTC).

#### the default format
touch -t 202311142213.20 f
date -u -r f
TZ=America/New_York date -r f

#### +FORMAT conversions
touch -t 202311142213.20 f
date -r f '+%Y-%m-%d %H:%M:%S %Z %z %j %u %w %a %A %b %B %e %C %y'
date -r f '+%D %F %T %R %r %c %x %X %U %W %V %G %g %h %I %p %s %%'

#### +FORMAT flags and widths (GNU)
## expect: gnu (BSD strftime has no widths or ^)
touch -t 202311142213.20 f
date -r f '+%-d %_m %05d %^a %10A|%-10A|%#Z'

#### GNU conversions: %N %:z %P %k %l %q
## expect: gnu (BSD date has no %N, %:z, %P, %q)
date -d @1700000000.5 '+%N %3N %:z %::z %:::z %P %k %l %q'

#### -d parses GNU date strings
## expect: gnu (BSD date has no -d)
date -d @1700000000
date -d '2024-01-15' +%s
date -d '2024-01-15 10:30:00' +%s
date -d '2024-01-15T10:30:00Z' +%s
date -d '2024-01-15 10:30:00 +0200' +%s
date -d '2024-01-15 10:30 PST' +%s
date -d 'Jan 15 2024' +%F
date -d '15 January 2024' +%F
date -d 'January 15, 2024 3pm' '+%F %T'
date -d '20240115' +%F

#### -d relative items
## expect: gnu (BSD date has no -d)
date -d '2024-01-31 +1 month' +%F
date -d '2024-01-15 1 day ago' +%F
date -d '2024-01-15 + 2 weeks' +%F
date -d '2024-03-12 12:00 next week' '+%F %T'
date -d '2024-03-10 tuesday' +%F
date -d '2024-01-15 10:00 3 hours ago' '+%F %T'

#### -d an invalid date
## expect: gnu (BSD date has no -d)
date -d garbage
echo "st=$?"

#### -I and --iso-8601
touch -t 202311142213.20 f
date -r f -I
date -r f -Iseconds
date -r f -Ihours
date -r f -Iminutes
date -r f -Ins

#### -R
touch -t 202311142213.20 f
date -r f -R
TZ=Asia/Kolkata date -r f -R

#### --rfc-3339 and --rfc-email
## expect: gnu (BSD date has no long options)
date -d @1700000000 --rfc-3339=date
date -d @1700000000 --rfc-3339=seconds
date -d @1700000000 --rfc-3339=ns
date -d @1700000000 --rfc-email
date -d @1700000000 --iso-8601=minutes

#### -u and TZ
touch -t 202407011000 summer
touch -t 202311142213.20 f
TZ=Europe/Berlin date -r summer '+%F %T %Z %z'
TZ=Europe/Berlin date -u -r summer '+%F %T %Z %z'
TZ=EST5EDT date -r f
TZ=UTC0 date -r f +%Z

#### -r FILE is the file's time
touch -t 202401151000 f
date -r f '+%F %T'
date -r nofile
echo "st=$?"

#### -r SECONDS (BSD, D016)
## expect: bsd (D016: digits that name no file are epoch seconds; GNU takes a file)
date -r 1700000000 +%s
date -r 0 +%F

#### -v adjusts (BSD)
## expect: bsd (GNU date has no -v)
date -j -r 1700000000 -v+1d +%F
date -j -r 1700000000 -v-1m -v+2H '+%F %T'
date -j -r 1700000000 -v1d +%F
date -j -r 1700000000 -v+1y -v-3w +%F
date -j -r 1706659200 -v+1m +%F
date -r 1700000000 -v+30M -v-10S +%T

#### -j -f parses with a format (BSD)
## expect: bsd (GNU date has no -j)
date -j -f '%Y-%m-%d %H:%M:%S' '2024-01-15 10:30:00' +%s
date -j -f '%Y-%m-%d' 2024-01-15 +%F
date -j -f %s 1700000000 +%F
date -j -f '%d/%b/%Y' 15/Jan/2024 +%F
date -j -f '%Y-%m-%d' 2024-13-45 +%F
echo "st=$?"

#### -j with a [[[mm]dd]HH]MM date (BSD)
## expect: bsd (GNU date has no -j)
date -j 0115103024 '+%F %T'
date -j 011510302024.05 '+%F %T'

#### errors
## expect: gnu (D016: GNU's messages)
touch f
date -d @1 -r f
echo "st=$?"
date +%F extra
echo "st=$?"
