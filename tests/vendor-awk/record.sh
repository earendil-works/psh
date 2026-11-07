#!/usr/bin/env bash
# Records the expected results of the vendored awk suites (README.md here):
#   tests/vendor-awk/record.sh        gawk in the recording container -> expected.gnu.txt
#   tests/vendor-awk/record.sh --bsd  macOS's /usr/bin/awk            -> expected.bsd.txt
#   tests/vendor-awk/record.sh --T    onetrueawk's T.* under gawk     -> expected.T.gnu.txt
# Re-record only when the cases change, in a commit of its own.
set -euo pipefail
cd "$(dirname "$0")"
here=$(pwd)
if [ "${1:-}" = --bsd ]; then
  [ "$(uname -s)" = Darwin ] || { echo "record.sh --bsd: run this on macOS" >&2; exit 2; }
  PATH=/usr/bin:/bin:/usr/sbin:/sbin perl record.pl "$here" /usr/bin/awk > expected.bsd.txt.new
  mv expected.bsd.txt.new expected.bsd.txt
  exit
fi
tag=$(git -C ../.. hash-object docker/Dockerfile | cut -c1-12)
if [ "${1:-}" = --T ]; then
  docker run --rm -v "$here:/v:ro" "psh-goldens:$tag" \
    env PATH=/usr/local/bin:/usr/bin:/bin perl /v/record-T.pl /v bash gawk > expected.T.gnu.txt.new
  mv expected.T.gnu.txt.new expected.T.gnu.txt
  exit
fi
docker run --rm -v "$here:/v:ro" "psh-goldens:$tag" \
  env PATH=/usr/local/bin:/usr/bin:/bin perl /v/record.pl /v gawk > expected.gnu.txt.new
mv expected.gnu.txt.new expected.gnu.txt
