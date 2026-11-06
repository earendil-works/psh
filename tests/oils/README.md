# Oils spec tests (L1)

Vendored from [Oils](https://github.com/oils-for-unix/oils) at commit
`d114978ca467143f171a835cbcbe8ab6ccc009a7`, under the Apache License 2.0
(`LICENSE.txt`). Unmodified; `scripts/fetch-oils` reproduces the copy and
`scripts/fetch-oils --check` verifies it.

- `spec/*.test.sh`: Oils' SUITE-compat set (the OSH spec files minus
  `strict-options`).
- `spec/bin/`, `spec/testdata/`: helpers and data the cases use. The Python helpers
  (`argv.py`, `printenv.py`, ...) are kept for reference only: psh-spec runs its own
  built-in equivalents, so neither recording nor testing needs Python.

The `## stdout:` / `## BUG bash` expectations in the files are Oils' and are not our
oracle: the goldens in `tests/goldens/oils/`, recorded from bash 5.3, are.

`expected-failures.txt` (ours) is the ratchet for this layer.
