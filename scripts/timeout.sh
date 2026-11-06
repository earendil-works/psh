#!/usr/bin/env bash
# Portable timeout (macOS has no `timeout`):
#
#   scripts/timeout.sh DURATION command [args...]
#
# DURATION is seconds, fractions allowed, with an optional suffix s, m, h or d (like GNU
# timeout). The command runs in its own process group under `nice -n 10`; when the time
# is up the whole group gets TERM, and KILL once it has not emptied after a grace period
# (PSH_TIMEOUT_GRACE seconds, default 5). So the grandchildren a test or a shell spawns
# do not survive as orphans. Processes that move into a process group of their own are
# out of reach, as with GNU timeout.
#
# Exit status: 124 on timeout (like GNU timeout); otherwise the command's status, or
# 128+N when a signal N killed it; 126/127 when it cannot be run; 125 on a usage error.
# INT, TERM and HUP sent to this script are forwarded to the group the same way (status
# 128+N).
#
# Not niced: the gate and the merge scripts, or anything run with PSH_GATE=1 (gate.sh sets
# it). Everything else yields to them, since one merge at a time is the bottleneck, and a
# niced process can never lower its niceness again.
#
# Needs only bash 3.2 and perl 5 (perl-base suffices), present on macOS and Linux.
set -euo pipefail

if [ $# -lt 2 ]; then
  echo "usage: scripts/timeout.sh DURATION command [args...]" >&2
  exit 125
fi
duration=$1
shift

nice_cmd="nice -n 10"
case "$(basename "$1")" in
  merge-to-main.sh | gate.sh | with-merge-lock.sh) nice_cmd= ;;
esac
if [ "${PSH_GATE:-}" = 1 ]; then nice_cmd=; fi

# $nice_cmd is unquoted on purpose: empty, or the words `nice -n 10`.
# shellcheck disable=SC2086
exec perl -e '
  use strict;
  use warnings;
  use POSIX qw(WNOHANG ceil);
  # Time::HiRes gives fractional alarms. Debian perl-base lacks it; then a fractional
  # duration is rounded up to whole seconds.
  my $hires = eval { require Time::HiRes; 1 };
  sub set_alarm { $hires ? Time::HiRes::alarm($_[0]) : alarm(ceil($_[0])) }

  my $spec = shift @ARGV;
  my %unit = ("" => 1, s => 1, m => 60, h => 3600, d => 86400);
  my ($n, $u) = $spec =~ /^(\d+(?:\.\d*)?|\.\d+)([smhd]?)$/
    or do { print STDERR "timeout.sh: invalid duration: $spec\n"; exit 125 };
  my $secs = $n * $unit{$u};
  my $grace = $ENV{PSH_TIMEOUT_GRACE} // 5;

  my $pid = fork() // do { print STDERR "timeout.sh: fork: $!\n"; exit 125 };
  if ($pid == 0) {
    setpgrp(0, 0);
    { no warnings "exec"; exec { $ARGV[0] } @ARGV; }
    print STDERR "timeout.sh: $ARGV[0]: $!\n";
    POSIX::_exit($!{ENOENT} ? 127 : 126);
  }
  # Also from the parent, so the group exists before any signal is sent to it (fails
  # harmlessly once the child has exec-ed and set it itself).
  setpgrp($pid, $pid);

  # TERM the group, wait for it to empty (reaping the leader so it does not linger as a
  # zombie member), then KILL whatever is left.
  my $reaped = 0;
  sub kill_group {
    kill "TERM", -$pid;
    my $end = time + $grace;
    while (time < $end) {
      $reaped = 1 if !$reaped && waitpid($pid, WNOHANG) == $pid;
      last if $reaped && !kill(0, -$pid);
      select(undef, undef, undef, 0.05);
    }
    kill "KILL", -$pid;
    waitpid($pid, 0) unless $reaped;
  }
  $SIG{ALRM} = sub { kill_group(); exit 124 };
  $SIG{INT}  = sub { alarm 0; kill_group(); exit 130 };
  $SIG{HUP}  = sub { alarm 0; kill_group(); exit 129 };
  $SIG{TERM} = sub { alarm 0; kill_group(); exit 143 };
  set_alarm($secs) if $secs > 0;
  while (waitpid($pid, 0) != $pid) {}
  my $st = $?;
  alarm 0;
  exit($st & 127 ? 128 + ($st & 127) : $st >> 8);
' "$duration" $nice_cmd "$@"
