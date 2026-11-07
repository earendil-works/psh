# Utility cases (L4)

Cases for psh's in-process utilities, one directory per utility (`head/basic.test.sh`),
in the Oils format (see `tests/agent/README.md`): each case runs as `SHELL -c CODE` with
an empty stdin in a fresh directory.

Every case has two recordings: GNU's (the Linux container, `tests/goldens/utils/`) and
BSD's (macOS with only `/usr/bin`, `/bin`, `/usr/sbin`, `/sbin` on PATH,
`tests/goldens/utils-bsd/`). A case says which one psh must match:

- `## expect: both` (the default): the two must agree, and psh matches them.
- `## expect: gnu` or `## expect: bsd`, with the reason: a flag only one dialect has, or
  the D016 rule that decides a conflict (`## expect: gnu (D016: wc widths)`).

Record with `scripts/record-goldens utils` (GNU, Docker) and `scripts/record-goldens
--bsd` (BSD, on macOS); add new cases to `expected-failures.txt` with `psh-spec --layer
utils --add-failures` only when psh does not pass them yet.
