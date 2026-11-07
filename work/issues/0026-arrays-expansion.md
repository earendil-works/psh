# 0026: Arrays and the rest of word expansion

status: done
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
- [x] Indexed and associative arrays: `a=(…)`, `a[i]=`, `a+=(…)`, `${a[i]}`, `${a[@]}`,
      `${a[*]}`, `${#a[@]}`, `${!a[@]}`, negative indices, sparse arrays, `"$@"`-like
      quoting of `"${a[@]}"`, arrays in arithmetic (via `ArithEnv`), `unset a[i]`.
- [x] `declare`/`typeset`/`local` with `-a -A -i -r -x -g -n -l -u -p -f -F` (namerefs; `declare -f` prints functions with `psh_syntax::print_function_bash`, restoring bytes for non-UTF-8 sources),
      `readonly -a`, `export` of arrays (not exported, as bash), `mapfile`/`readarray`.
- [x] Remaining parameter expansions: `${x:o:l}`, `${x/p/r}` (`//`, `/#`, `/%`),
      `${x^^}` / `${x,,}` and friends, `${!prefix*}`, `${!name}` indirection,
      `${x@Q}` / `@E` / `@P` (where non-interactive) / `@A` / `@a`, on arrays too.
- [x] `$*`/`$@` and IFS edge cases, `set --` with arrays, `shift`.
- [x] Ratchets updated; Result records scores and which hit-list areas moved.

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

### 2026-11-07 part 2
- `mapfile`/`readarray` (all options incl. -C/-c through eval's `run_text`), process
  substitution `<(…)`/`>(…)` as `/dev/fd/N` (63 down, closed when the command completes;
  externals get the fd through psh-os).
- declare: `+`/`-` attribute rules (+ wins), `-c -t -I`, `-p` filters, declared-only
  arrays (`declare -a x` shows no `=()`), quoted compound assignments re-parsed
  (`declare -a a='(1 2)'`), `set`-style quoting for `declare`/`set` listings, bare
  subscripted names, `-p` errors as written, `local` listing, `+a` "cannot destroy".
- Namerefs to elements (`declare -n r='a[2]'`, also through `unset`, arithmetic,
  `+=`), chains in `${!ref}`, self references refused.
- `${x@A}`, `@a`, `@K`, `@k`, `@P`; one-char case mappings (ß stays); backslashes in
  double-quoted operands (`"${u-\z}"`); assoc slices number from 1 (bash).
- Scores: oils 1651/2511, brush 1789/3171, agent 121/144.

### 2026-11-07 part 3
- Dynamic variables (D1263): `RANDOM SRANDOM SECONDS EPOCHSECONDS EPOCHREALTIME BASHPID
  BASH_SUBSHELL`, computed when read through `Host::now`; `$_` (last word of the last
  command, set on completion; externals get `_` = their path, not through `exec`);
  `OPTIND=1` at start. `BASH_VERSION`/`BASH_VERSINFO` as bash 5.3.9 plus `PSH_VERSION`
  (D1264, coordinator decision); oils/builtin-special/7–9 ratcheted (`set -o posix`).
- Operand words of `${x-…}`: unquoted literal text splits; single quotes are literal in
  double quotes; backquotes in operands unescape as outside double quotes; tilde after
  `=` in assignment-like words (also command arguments) and after `:`.
- Null tests of `"$*"` (joined) and empty `$@`; empty elements of unquoted `$@` stay
  words when IFS is non-whitespace (not the last); `${!p*}`/`${!a[*]}` with empty IFS.
- `$(< file)` read by the shell; backslash-newline in backquotes.
- Scores: oils 1996/2511, brush 2424/3171, agent 133/144.

## Follow-ups
- `read -a` (0029's read.rs) can now use `vp.assign_array(name, items, false)`.
- `test -v 'a[i+1]'` needs arithmetic in `test.rs` (0027's area): `Vp::is_set` handles
  literal subscripts only.

## Result
Landed in three parts (4361a04, 14616ca, and the last on top).
- Variable model (D1260): `Value::{Unset, Str, Indexed, Assoc}` with attributes
  (`-i -l -u -c -n -r -t -x`), used through `Vp` accessors (`var`, `set_var`, `assign`,
  `assign_with`, `assign_elem`, `assign_array`, `get_elem`, `unset_elem`, `is_set`,
  `resolve`, `target`); associative arrays in bash's exact order (D1261).
- Arrays end to end (assignment forms, `${a[@]}`/`${a[*]}`/`${#…}`/`${!a[@]}`, slices,
  negative indices, arithmetic, `unset`, sparse), namerefs (chains, elements, cycles),
  declaration builtins with `DeclArg`s (D1262), `mapfile`/`readarray`, process
  substitution, all `${x@…}` transforms, substrings, dynamic variables and `$_` (D1263),
  `BASH_VERSION` (D1264), IFS/`$*`/`$@` edge cases, operand quoting, `$(< file)`.
- Scores (macOS, at merge): L1 oils 2019/2511 active (731 after 0005, 1111 at the
  start of 0026, other issues landing in parallel), L2 brush 2450/3171, L3 agent 133/144. Hit-list
  areas moved: arrays (oils array* now 9 left), declare (72→32 brush), nameref (69→23),
  mapfile (40→few), word-split (13→1), var-op-* and var-sub-quote mostly done.
- Left: host info variables (UID, EUID, PPID, HOSTNAME, OSTYPE, MACHTYPE, GROUPS; a host
  op, procs), the call stack arrays (FUNCNAME, BASH_SOURCE, BASH_LINENO, BASH_ARGV;
  brush callstack 37 cases), `~user`, `${x//~/…}` tilde in patterns, `set -u` cases
  (0028), `[[ -v ]]` arithmetic subscripts (0027), `printf -v a[k]` and `read -a`
  (0029).
