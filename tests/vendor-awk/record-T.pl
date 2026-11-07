#!/usr/bin/perl
# Records which checks of onetrueawk's T.* scripts fail under AWK (gawk in the
# recording container): one line per `BAD` line a script prints, `script<TAB>line`.
# Called by record.sh; onetrueawk_t.rs runs the same scripts with psh (README.md).
use strict;
use warnings;
use File::Temp qw(tempdir);
use File::Copy qw(copy);

my ($dir, $shell, $awk) = @ARGV;
$ENV{LC_ALL} = 'C.UTF-8';
$ENV{TZ} = 'UTC';
$ENV{awk} = $awk;
my $src = "$dir/onetrueawk-T";
opendir(my $dh, $src) or die;
my @files = sort grep { !/^\./ } readdir $dh;
closedir $dh;
for my $t (grep { /^T\./ } @files) {
    my $tmp = tempdir(CLEANUP => 1);
    copy("$src/$_", "$tmp/$_") or die for @files;
    # The suite's ./echo (echo.c): its arguments, no escapes, -n.
    open(my $e, '>', "$tmp/echo") or die;
    print $e "#!/bin/sh\nn=1\nif [ \"\$1\" = -n ]; then n=0; shift; fi\nprintf '%s' \"\$*\"\n[ \$n = 1 ] && printf '\\n'\nexit 0\n";
    close $e;
    chmod 0755, "$tmp/echo";
    my $out = `cd '$tmp' && timeout 120 $shell $t 2>&1`;
    my %seen;
    for my $l (split /\n/, $out) {
        next unless $l =~ /BAD/;
        next if $seen{$l}++;
        print "$t\t$l\n";
    }
}
