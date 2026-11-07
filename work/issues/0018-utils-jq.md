# 0018: Utilities batch G: jq

status: in-progress
agent: procs
branch: issue/0018-utils-jq
depends-on: 0008 (jaq allowed as a dependency, D028)
touches: crates/psh-utils/src/jq*, tests/utils/jq/, Cargo.toml (jaq)
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
- [ ] Upstream test suites (owner request): jq's own `tests/*.test` (jq is MIT; check each
      file's license and the man-page-derived tests) and jaq's tests (MIT) vendored at
      pinned commits with attribution, run through a small runner (program, input,
      expected outputs) against psh's `jq`; own ratchet with reasons; pass counts in the
      Result.
- [ ] Each utility registered with its mode; unsupported flags fall through or fail
      clearly (tested per utility).
- [ ] L4 cases in `tests/utils/<name>/` for every flag listed above, recorded against
      GNU and BSD, with the D016 rule named where they conflict.
- [ ] Attribution headers and `NOTICE` entries for ported code.

## Notes
- jaq is a dependency (D028). Copying it instead is decided here only if its size
  demands it; measure and report the size it adds.
- Goldens come from real jq 1.7+ in the recording container (add it there).

## Log

## Follow-ups

## Result
