# Upstream test suites for jq

Run by `cargo test -p psh-utils upstream_suites` (crates/psh-utils/src/jq/upstream.rs)
against psh's jq: each case is a program, an input line and the expected outputs, compared
as values the way jq's own `--run-tests` compares them. `expected-failures.txt` lists the
cases psh does not pass, each with its reason (a ratchet: a listed case that passes fails
the test too; `PSH_JQ_UPDATE=1` rewrites it, keeping the reasons).

- `jq-1.7.1/`: jq's `tests/*.test` at tag jq-1.7.1 (commit 71c2ab5), verbatim. jq is MIT
  licensed (`COPYING`, Copyright (C) 2012 Stephen Dolan); `man.test` and `manonig.test`
  are generated from jq's manual, whose text is under CC BY 3.0 (by the jq authors,
  https://creativecommons.org/licenses/by/3.0/).
- `jaq-3.1.2/`: jaq 3.1.2's tests (jaq-core, jaq-std and jaq-json `tests/*.rs`, commit
  f7fea92, MIT, by Michael Färber), converted to the same format by `convert.py` (the
  `give`/`gives`/`yields!` cases with literal arguments; 74 cases with Rust expressions
  are not converted). jaq's expectations are jaq's: where they differ from jq 1.7.1, psh
  gives jq's answer and the case is listed with that reason.
