# 0026: Arrays and the rest of word expansion

status: open
agent: -
branch: issue/0026-arrays-expansion
depends-on: 0005
touches: crates/psh-engine/src/{expand.rs,state.rs,arith.rs}, crates/psh-engine/src/builtins/{declare*,unset*,shift*,mapfile*} (new modules where possible), tests/**/expected-failures.txt
guide: engine.md (follow-up)
decisions: D1260–D1269

## Goal
Indexed and associative arrays and the remaining parameter expansions: the biggest
block of failing cases after 0005 (arrays ~510 cases), and the core of the variable
model everything else uses. ARCH.md "Engine internals other work builds on" applies:
arrays extend `Var`'s value; `Fields` stays the expansion model.

## Acceptance criteria
- [ ] Indexed and associative arrays: `a=(…)`, `a[i]=`, `a+=(…)`, `${a[i]}`, `${a[@]}`,
      `${a[*]}`, `${#a[@]}`, `${!a[@]}`, negative indices, sparse arrays, `"$@"`-like
      quoting of `"${a[@]}"`, arrays in arithmetic (via `ArithEnv`), `unset a[i]`.
- [ ] `declare`/`typeset`/`local` with `-a -A -i -r -x -g -n -l -u -p -f -F` (namerefs; `declare -f` prints functions with `psh_syntax::print_function_bash`, restoring bytes for non-UTF-8 sources),
      `readonly -a`, `export` of arrays (not exported, as bash), `mapfile`/`readarray`.
- [ ] Remaining parameter expansions: `${x:o:l}`, `${x/p/r}` (`//`, `/#`, `/%`),
      `${x^^}` / `${x,,}` and friends, `${!prefix*}`, `${!name}` indirection,
      `${x@Q}` / `@E` / `@P` (where non-interactive) / `@A` / `@a`, on arrays too.
- [ ] `$*`/`$@` and IFS edge cases, `set --` with arrays, `shift`.
- [ ] Ratchets updated; Result records scores and which hit-list areas moved.

## Notes
- Other agents work in parallel on `[[ ]]`/regex (0027), shell options/errexit (0028)
  and builtins (0029). Keep your changes in expand.rs/state.rs and new builtin modules;
  announce changes to shared signatures (`Var`, `Fields`, builtin fn type) to the
  coordinator before landing them.

## Log

## Follow-ups

## Result
