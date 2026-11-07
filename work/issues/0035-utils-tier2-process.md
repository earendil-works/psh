# 0035: Tier 2 batch: process and host utilities

status: open
agent: -
branch: issue/0035-utils-tier2-process
depends-on: 0008, 0030
touches: crates/psh-utils/src/{ps,pgrep,uptime,nproc,id,uname,whoami,hostname,df}*, crates/psh-engine/src/host.rs (additive Host ops, announced), crates/psh-os/, tests/utils/
guide: utilities.md
decisions: D1350–D1359

## Goal
The process/host batch of Tier 2 (0021): `ps pgrep pkill uptime nproc id uname whoami
hostname df`. These are what agents use to inspect running servers and the machine
(`ps aux | grep`, `pgrep -f`, `pkill -f server`, `nproc`, `uname -a`, `df -h`).

## Acceptance criteria
- [ ] Per-OS process listing behind an additive Host op (Linux /proc, macOS
      libproc/sysctl, Windows Toolhelp/NtQuery as the windows agent later implements;
      wasm: the JS host's own jobs or empty). Decide `sysinfo` vs own code (D028) —
      prefer own code, it's small per OS.
- [ ] `ps`: BSD `ps aux`/`ps -axo …` and procps `-ef/-eo/-o/-p/-u` (both accepted, D016),
      `pgrep`/`pkill` with `-f -l -a -x -u -n -o -signal`; pkill signals through Host::kill
      (Linux numbering).
- [ ] `uptime nproc id uname whoami hostname df` with GNU and BSD flags; `df -h` human sizes.
- [ ] L4 cases (process listings are nondeterministic: test structure/format with
      processes the case starts itself), modes per D015/ARCH, classifier tables.

## Log

## Follow-ups

## Result
