# 0012: Guide check tool

status: in-progress
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
- [ ] `check` over the pages in `SUMMARY.md`: links to pages, sections and repository
      files resolve; no mentions or links of ARCH.md, TESTING.md, docs/*.md, work/,
      research/ or issue numbers; no decision IDs; common history phrases ("no longer",
      "used to", "was replaced", "originally"); `pi` only on `pi.md`; goal links point
      to existing sections of `goals.md`; no bare goal numbers. Code blocks are exempt.
- [ ] A gate leg, cached or cheap enough to run on every merge.
- [ ] Zero dependencies (Markdown handled by a small hand parser for links, headings
      and code spans).

## Notes
- djs's `djs-docs check` (`~/Development/djs/crates/djs-docs`, read-only) is the model;
  site building and generated regions are out of scope for now.

## Log

## Follow-ups

## Result
