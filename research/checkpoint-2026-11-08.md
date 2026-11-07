# Checkpoint 2026-11-08: size and direction

All agents stopped at main `1178f382`.

## Binary size

The 11.9 MB `target/release/psh` is a debug-friendly release profile: no LTO, no strip,
16 codegen units, unwinding. The same code with a shipping profile:

| Profile | Size | gzip -9 |
|---|---:|---:|
| current `release` (opt 3, no LTO, not stripped) | 11.9 MB | – |
| + strip | 9.8 MB | – |
| + fat LTO, 1 codegen unit, panic=abort, strip | 6.4 MB | 3.1 MB |
| + opt-level=s | **5.0 MB** | **2.5 MB** |
| wasm (`psh.wasm`, current wasm profile) | 4.3 MB | 1.5 MB |
| parser + classifier wasm | 0.39 MB | 0.15 MB |

(`panic=abort` would break the catch_unwind isolation of blocking utilities, D1130; without
it the optimized build is a little larger.)

## Where the code goes (cargo-bloat, .text 6.3 MB of the unoptimized build)

| Part | .text | Share |
|---|---:|---:|
| std | 1.1 MB | 18% |
| Shell core: engine 1.0, parser 0.19, classifier 0.11, psh-os 0.08, psh-regex 0.03, argument/deser | ~1.5 MB | 24% |
| regex stack (regex-automata 0.44, regex-syntax 0.24, aho-corasick 0.13), shared by rg/grep/fd/jq | 0.8 MB | 13% |
| ripgrep family: rg CLI 0.19 + its generic instantiations 0.21, grep 0.12, fd 0.12, ignore 0.11, globset 0.09, grep-printer/searcher/regex 0.14, encoding_rs 0.03 | ~1.0 MB | 16% |
| jq: own builtins 0.23 + generics 0.28 + jaq-core 0.11 + libm 0.04 | ~0.66 MB | 10% |
| awk | ~0.26 MB | 4% |
| everything else in psh-utils (sed, find, diff, sort, cp, date, checksums, ~45 more) | ~0.9 MB | 14% |

So the owner's guess is right: jq and the rg/fd/grep family with the regex stack are
together about 2.5 MB of code, more than the whole shell core. Notable waste: grep-searcher's
core loops are monomorphized once per sink × matcher reference (rg's standard, summary and
JSON sinks, plus grep's own sink), about a dozen 5–9 KB copies.

## Options

1. **Shipping profile** (free): `[profile.dist]` with fat LTO, codegen-units=1, strip,
   opt-level=s → ~5 MB. Keep `release` for development.
2. **Cargo features per heavy group** in psh-utils: `search` (rg, fd, grep on the ripgrep
   crates + regex), `jq`, `awk`; a `core` set (coreutils, sed on psh-regex, find, diff) is
   always on. pi-env and the browser choose. Estimated core-only optimized binary: ~2.5–3 MB.
3. **Fewer instantiations**: one sink type (dyn) for rg/grep, `&RegexMatcher` vs
   `&&RegexMatcher` unified → maybe −150 KB.
4. **regex features**: drop `perf-dfa-full`/`perf-onepass` → smaller regex-automata at some
   speed cost; measure.
5. **Reconsider scope**: rg/fd/jq at full CLI fidelity are the expensive part; F mode (use
   the real program when installed) for rg/fd/jq would keep them as fallbacks, but they would
   still be linked unless feature-gated.

## Direction

- Bash compatibility is strong (Linux: Oils 92%, brush 88%, agent cases 701/701), the
  parser/classifier are small and complete, the process layers work on Unix and Windows.
- The utility work grew fastest and is now most of the binary. Tier 2 (tar, gzip, tree, …)
  would add more.
- Not started: pi-env integration, the classifier in Pi's UI/permissions — the reason this
  project exists.
- Proposal: stop adding utilities; do (1) and (2) now; then pi-env integration.
