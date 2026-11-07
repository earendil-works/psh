# 0018: Utilities batch G: jq

status: done
agent: procs
branch: issue/0018-utils-jq
depends-on: 0008 (jaq allowed as a dependency, D028)
touches: crates/psh-utils/src/jq*, tests/utils/jq/, Cargo.toml (jaq)
guide: utilities.md (not written yet), utilities reference (not written yet)
decisions: D1180–D1189

## Goal
`jq` (S) on jaq (MIT), with jaq's known differences from jq fixed where agents hit them.

## Acceptance criteria
- [x] Flags: `-r -c -s -n -e -S -j -a -R --arg --argjson --slurpfile --rawfile --args
      --jsonargs --tab --indent -f`.
- [x] jq behavior where jaq differs: `-s` over several files gives one array; `?//`;
      `. |= empty`; number formatting; error text. Each difference left in place is
      listed with a reason.
- [x] Upstream test suites (owner request): jq's own `tests/*.test` (jq is MIT; check each
      file's license and the man-page-derived tests) and jaq's tests (MIT) vendored at
      pinned commits with attribution, run through a small runner (program, input,
      expected outputs) against psh's `jq`; own ratchet with reasons; pass counts in the
      Result.
- [x] Each utility registered with its mode; unsupported flags fall through or fail
      clearly (tested per utility).
- [x] L4 cases in `tests/utils/<name>/` for every flag listed above, recorded against
      GNU and BSD, with the D016 rule named where they conflict.
- [x] Attribution headers and `NOTICE` entries for ported code.

## Notes
- jaq is a dependency (D028). Copying it instead is decided here only if its size
  demands it; measure and report the size it adds.
- Goldens come from real jq 1.7+ in the recording container (add it there).

## Log
- Dependencies (D1180): jaq-json and jaq-std pull no serde, but bring jaq's value model
  and messages plus num-bigint, indexmap, hifijson, jiff, regex-bites, base64,
  urlencoding. psh depends on jaq-core only (+ dyn-clone, once_cell, typed-arena) and
  libm; `crates/psh-utils/src/jq/` has jq 1.7's values (`val.rs`, `num.rs`, `map.rs`),
  builtins (`defs.jq` from jq's builtin.jq, natives in `funs.rs`, dates in `time.rs`),
  JSON reader/printer (`json.rs`, hand-written: jq's texts and leniencies; the exception
  to D029) and the CLI (`mod.rs`, a blocking utility). Size: +1.2 MiB stripped native,
  +687 KB wasm (+210 KB gzipped). deps-allowlist updated.
- Oracle: jq 1.7.1, already in the recording image (Debian `jq=1.7.1-6+deb13u4`, pinned
  by 0031) and macOS's /usr/bin/jq (1.7.1-apple) for the BSD goldens: no docker change.
- jaq's differences fixed (D1181): `?//` (a source rewrite), `$__loc__`, unknown
  `@format`s, surrogate escapes; `-s` over several files; `.[] |= empty`; jq's number
  output (`1.000`, `1E+1000`, `1e+17`, `1e-05`); error texts; exit statuses and input
  positions (D1182).
- Upstream suites (`tests/utils/jq/upstream/`, README there, NOTICE): jq 1.7.1's
  tests/*.test verbatim (MIT; man.test/manonig.test CC BY 3.0), jaq 3.1.2's tests
  converted by `convert.py`. Runner `jq/upstream.rs` (a unit test) with its own ratchet.
  Pass counts: jq 704/738 (jq.test 415/447, man 224/224, manonig 17/17, onig 38/40,
  optional 3/3, base64 7/7); jaq 442/534 (core 256/277, json 81/109, std 105/148), and of
  jaq's 92 failures 82 are cases where jaq's expectation is jaq's own and psh gives jq
  1.7.1's answer (checked by running jq on each).
- L4: tests/utils/jq/basic.test.sh, 44 cases (every listed flag, errors, formats, dates,
  regexes, colors), GNU and BSD goldens agree; also pass on wasm.
- Touched outside `touches:`: Cargo.toml of psh-utils (deps), scripts/deps-allowlist.txt,
  psh-classify's jq entry (`-L` takes a value), NOTICE, TESTING.md (the suite), docs.

## Follow-ups
- Modules: `import`/`include`/`-L`/`modulemeta` (jaq-core has a loader; wire a search
  path through SyncCtx).
- `|= empty` on several paths at once deletes path by path (jaq-core's update); jq
  collects the paths first. Needs an update hook in jaq-core or our own `_modify`.
- A pure CPU loop (`last(range(1e12))`) is not stopped by a cancel on the thread backend
  (jaq has no hook); output-producing programs are.
- `localtime`/`strflocaltime` use the zone offset of now for every instant (DST).
- Regex: no look-around (Oniguruma has it); captures of empty repetitions differ.

## Result
`jq` is a psh utility (mode S) on every host, wasm included: jq 1.7.1's command line
(`-r -j -a -c -s -n -e -S -R -C -M -f -L --tab --indent --arg --argjson --slurpfile
--rawfile --args --jsonargs --seq --stream --raw-output0 --unbuffered`), builtins, output
and messages. Unknown options fall through to the real jq, else fail with "unsupported
option". Upstream: jq's suites 704/738, jaq's 442/534 (82 of the rest are jaq-specific
expectations where psh matches jq). Known gaps: see Follow-ups and the ratchet
(`tests/utils/jq/upstream/expected-failures.txt`).
