# 0018: Utilities batch G: jq

status: open
agent: -
branch: issue/0018-utils-jq
depends-on: 0008; owner/coordinator decision on jaq (dependency or copy)
touches: crates/psh-utils/src/jq*, tests/utils/jq/, Cargo.toml if jaq becomes a dependency
guide: utilities.md (not written yet), utilities reference (not written yet)
decisions: D1180–D1189

## Goal
`jq` (S) on jaq (MIT), with jaq's known differences from jq fixed where agents hit them.

## Acceptance criteria
- [ ] Flags: `-r -c -s -n -e -S -j -a -R --arg --argjson --slurpfile --rawfile --args
      --jsonargs --tab --indent -f`.
- [ ] jq behavior where jaq differs: `-s` over several files gives one array; `?//`;
      `. |= empty`; number formatting; error text. Each difference left in place is
      listed with a reason.
- [ ] jaq's `jq-tests.jq` runs; failures listed.
- [ ] Each utility registered with its mode; unsupported flags fall through or fail
      clearly (tested per utility).
- [ ] L4 cases in `tests/utils/<name>/` for every flag listed above, recorded against
      GNU and BSD, with the D016 rule named where they conflict.
- [ ] Attribution headers and `NOTICE` entries for ported code.

## Notes
- Goldens come from real jq 1.7+ in the recording container (add it there).

## Log

## Follow-ups

## Result
