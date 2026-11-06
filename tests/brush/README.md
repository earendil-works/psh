# brush test cases (L2)

Vendored from [brush](https://github.com/reubeno/brush) at commit
`943553433ae04f5b3bd55d2cf207e39eb9ef2af7`, under the MIT license (`LICENSE`).
Unmodified; `scripts/fetch-brush` reproduces the copy and `scripts/fetch-brush
--check` verifies it.

Only `brush-shell/tests/cases/**/*.yaml` is used, plus `brush-shell/tests/utils/`,
which a case sources. The expectations brush keeps in the files (`expected_stdout`,
`known_failure`, snapshots) are not our oracle: the goldens in `tests/goldens/brush/`,
recorded from bash 5.3, are.

`expected-failures.txt` (ours) is the ratchet for this layer.
