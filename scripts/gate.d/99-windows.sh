# timeout: 3000
# grace: 120
# inputs: crates/ Cargo.toml Cargo.lock .cargo/ tests/ scripts/windows-gate.sh scripts/windows-gate.ps1 scripts/merge-lock.sh
# The Rust legs on the Parallels Windows VM (D1070). Always last: it is the slowest, and
# a failure on macOS is cheaper to find first. The grace above gives it time after a
# TERM to stop its runner in the VM and suspend it.
#
# PSH_GATE_WINDOWS=0 skips it when the VM is broken: for the coordinator and the owner
# only, never to get green.
if [ "${PSH_GATE_WINDOWS:-1}" = 0 ]; then
  echo "gate: !!! WINDOWS LEG SKIPPED (PSH_GATE_WINDOWS=0): this commit is untested on Windows !!!" >&2
  # Nothing was checked: never record this as a pass in the gate cache.
  touch "${PSH_GATE_NOCACHE:-/dev/null}"
  source scripts/merge-lock.sh
  log="$(primary_checkout)/work/scratch/windows-gate/skipped.tsv"
  mkdir -p "$(dirname "$log")"
  printf '%s\t%s\t%s\n' "$(date '+%F %T')" "$(basename "$(pwd -P)")" "$(git rev-parse --short HEAD)" >>"$log"
  exit 0
fi
scripts/windows-gate.sh
