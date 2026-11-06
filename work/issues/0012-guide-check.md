# 0012: Guide check tool

status: done
agent: shapes
branch: issue/0012-guide-check
depends-on: 0001
touches: crates/psh-docs/ (or scripts/), scripts/gate.sh (one leg), docs/guide/README.md (the Tooling section)
guide: README.md
decisions: D1120–D1129

## Goal
Enforce the guide's writing rules (`docs/guide/README.md`) in the gate, so the guide
stays self-contained and true without relying on reviewers.

## Acceptance criteria
- [x] `check` over the pages in `SUMMARY.md`: links to pages, sections and repository
      files resolve; no mentions or links of ARCH.md, TESTING.md, docs/*.md, work/,
      research/ or issue numbers; no decision IDs; common history phrases ("no longer",
      "used to", "was replaced", "originally"); `pi` only on `pi.md`; goal links point
      to existing sections of `goals.md`; no bare goal numbers. Code blocks are exempt.
- [x] A gate leg, cached or cheap enough to run on every merge.
- [x] Zero dependencies (Markdown handled by a small hand parser for links, headings
      and code spans).

## Notes
- djs's `djs-docs check` (`~/Development/djs/crates/djs-docs`, read-only) is the model;
  site building and generated regions are out of scope for now.

## Log
- 2026-11-06: `crates/psh-docs` (zero dependencies), `psh-docs check [--root DIR]`.
  Hand line scanner: fences (``` and ~~~, length-matched), code spans (n-backtick
  runs), inline links with balanced parens, reference definitions, headings → mdBook
  ids (`Bash's behavior` → `bashs-behavior`, duplicates `-1`). Unit tests for ids,
  links, the self-containment, decision, history and pi rules, code spans and fences,
  SUMMARY parsing.
- Gate leg `guide` (`cargo run -q --locked -p psh-docs -- check`, 600 s bound; < 1 s
  once built). Also listed in TESTING.md's gate list (outside `touches:`, one line).
- The current guide (introduction.md, goals.md) passes: no violations to fix or hand
  over.
- docs/guide/README.md: the rules paragraph now names the tool; new Tooling section.

## Follow-ups
- Building the site (mdBook-compatible output) and generated regions, as djs-docs does,
  are out of scope here.
- Not checked: an aim called a goal without being on goals.md, "Not built yet" notes
  against the code, and whether a goal link's text matches the goal's name.

## Result
- `crates/psh-docs`: `cargo run -p psh-docs -- check` reports
  `docs/guide/PAGE:LINE: message` for broken page/section/file links, mentions or links
  of ARCH.md, TESTING.md, docs/*.md outside the guide, `work/`, `research/` and
  non-guide `.md` names, issue numbers, decision IDs, bare goal numbers (`G2`,
  "goal 3"), history phrases, and `pi` outside pi.md. Code blocks exempt; inline code
  held to the word rules only.
- Gate leg `guide`; decision D1120; docs/guide/README.md Tooling section.
