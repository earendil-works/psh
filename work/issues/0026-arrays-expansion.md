# 0026: Arrays and the rest of word expansion

status: in-progress
agent: engine
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

### 2026-11-07 part 1
- Variable model (D1260, announced to the coordinator first): `vars.rs` (`Var`, `Value`,
  `Assoc`), `arrays.rs` (element and compound assignment, negative indices, unset of
  elements); accessors on `Vp` (`var`, `set_var`, `assign`, `is_set`, `resolve`, …).
- Associative arrays in bash's exact iteration order (D1261), derived black-box: FNV-1,
  1024 buckets, head insertion, ×4 growth past 2× entries; verified up to 9000 keys.
- `a=(…)`, `a+=(…)`, `a[i]=v`, `h=([k]=v)`, `h=(k v k v)`, `${a[i]}`, `${a[@]}`,
  `${a[*]}`, `${#a[@]}`, `${#a[i]}`, `${!a[@]}`, `${a[@]:o:l}`, `${@:o:l}`, `${x:o:l}`,
  `${!x}` incl. `x='a[1]'`/`'a[@]'`, arrays in arithmetic, `unset 'a[i]'`, `test -v a[i]`.
- `declare`/`typeset`/`local`/`export`/`readonly` in `builtins/declare.rs` (D1262):
  `-a -A -i -l -u -n -r -x -g -t -p -f -F`, `declare -p` in bash's format (`$'…'` for
  control chars, assoc keys quoted when needed), `declare -f` through
  `psh_syntax::print_function_bash` (restoring bytes), namerefs with validation, cycle
  and depth warnings, `for` over a nameref.
- 2 accidental passes added to the brush ratchet with reasons (declare/24 needs
  `enable`, special_parameters/29 needs `$_`).
- Scores: oils 1449/2511 (was 1111), brush 1445/3171 (was 1066), agent 116/144.

## Follow-ups

## Result
