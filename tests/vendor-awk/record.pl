#!/usr/bin/perl
# Records the expected results of the vendored awk suites (README.md here): runs every
# case under AWK (gawk in the recording container, /usr/bin/awk on macOS for the BSD
# column) and prints one line per case: id, exit status and stdout (escaped: \\ \n \t
# \xNN). Called by record.sh.
use strict;
use warnings;
use File::Temp qw(tempdir);
use File::Copy qw(copy);
use POSIX qw(:sys_wait_h);

my ($dir, $awk) = @ARGV;
$ENV{LC_ALL} = 'C.UTF-8';
$ENV{TZ} = 'UTC';
delete $ENV{AWKPATH};

sub esc {
    my ($s) = @_;
    $s =~ s/([\\\n\t]|[^\x20-\x7e])/
        $1 eq "\\" ? "\\\\" : $1 eq "\n" ? "\\n" : $1 eq "\t" ? "\\t" : sprintf("\\x%02x", ord $1)/ge;
    return $s;
}

sub unesc {
    my ($s) = @_;
    $s =~ s/\\(\\|n|t|x([0-9a-f]{2}))/
        $1 eq "\\" ? "\\" : $1 eq "n" ? "\n" : $1 eq "t" ? "\t" : chr(hex $2)/ge;
    return $s;
}

# Runs ARGS in CWD with STDIN; returns (status, stdout).
sub run {
    my ($cwd, $stdin, @args) = @_;
    open(my $in, '>:raw', "$cwd/.stdin") or die;
    print $in $stdin;
    close $in;
    my $pid = fork();
    die "fork" unless defined $pid;
    if ($pid == 0) {
        chdir $cwd or die;
        open(STDIN, '<', "$cwd/.stdin") or die;
        open(STDOUT, '>', "$cwd/.stdout") or die;
        open(STDERR, '>', '/dev/null') or die;
        exec { $args[0] } @args or exit 127;
    }
    my $status;
    eval {
        local $SIG{ALRM} = sub { die "timeout\n" };
        alarm 10;
        waitpid($pid, 0);
        alarm 0;
        $status = $? & 127 ? 128 + ($? & 127) : $? >> 8;
    };
    if ($@) {
        kill 'KILL', $pid;
        waitpid($pid, 0);
        $status = 'timeout';
    }
    open(my $out, '<:raw', "$cwd/.stdout") or die;
    local $/;
    my $data = <$out> // '';
    close $out;
    return ($status, $data);
}

# onetrueawk: t.* read test.data, p.* test.countries, in a directory with the suite.
my $ota = "$dir/onetrueawk";
opendir(my $dh, $ota) or die;
my @progs = sort grep { /^[tp]\./ } readdir $dh;
closedir $dh;
for my $p (@progs) {
    my $tmp = tempdir(CLEANUP => 1);
    for my $f (@progs, 'test.data', 'test.countries') {
        copy("$ota/$f", "$tmp/$f") or die;
    }
    my $input = $p =~ /^t\./ ? 'test.data' : 'test.countries';
    my ($st, $out) = run($tmp, '', $awk, '-f', $p, $input);
    print "onetrueawk/$p\t$st\t", esc($out), "\n";
}

# goawk: `awk -- SRC -` with the case's stdin, in an empty directory.
open(my $cases, '<:raw', "$dir/goawk/cases.txt") or die;
my $n = 0;
while (my $line = <$cases>) {
    chomp $line;
    my ($src, $stdin) = split /\t/, $line, 2;
    my $tmp = tempdir(CLEANUP => 1);
    my ($st, $out) = run($tmp, unesc($stdin // ''), $awk, '--', unesc($src), '-');
    print "goawk/$n\t$st\t", esc($out), "\n";
    $n++;
}
