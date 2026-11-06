# Writing the guide

The guide explains psh to a human reader: what a user or an embedder of psh sees and can
do, and the architecture that was chosen, how it works and, where it helps, why it beats
the obvious alternative. It is not a second ARCH.md: ARCH.md, TESTING.md, the decision
log and `research/` stay the detailed reference for people changing the code, and the
guide says what it needs in its own words.

This file is about the guide; it is not a chapter, so the rules below do not apply to it.

## The rules

These hold for every page listed in `SUMMARY.md` (code blocks are exempt; inline code is
held to the word rules). `psh-docs check` enforces them in the gate (see Tooling below);
what it cannot see, such as a goal called a goal without being on `goals.md`, is on the
writer and the reviewer.

- **Self-contained.** A reader of the guide has the guide and nothing else. Never mention
  or link ARCH.md, TESTING.md, docs/*.md (the decision log included), `research/`,
  `work/`, or issue numbers (`0004`). Write what the page needs, or what was found:
  "in a corpus of about 295,000 agent commands, 47% use a pipe", not "see corpus.md".
- **No decisions.** No decision IDs (`D005`) and no links to the decision log. Explain the
  reason in the page's own words.
- **No history.** Describe psh as it is: no "used to", "was replaced", "originally",
  earlier designs or migration stories. Comparisons with other shells are fine where they
  explain a choice ("unlike a shell built on an async runtime, ...").
- **Built or not.** psh is young. A page may describe designed behavior that is not built
  yet, marked with a `> **Not built yet.** ...` note at the start of the section. When it
  lands, the note goes and the text is checked against the code. Measured numbers come
  only from things that exist.
- **pi on one page.** pi, the agent psh was built for, is named only on `pi.md`: how pi
  and its pi-env daemon use psh, and how another agent host could. Everywhere else,
  describe hosts in general ("an agent host", "the embedder"). This applies to the whole
  word `pi`, not to `pipe` or `pipeline`.
- **Goals by name, and only goals.md's.** Every goal a page refers to or relies on is a
  section of `goals.md`, which says what it means for a user, why psh chose it, and what
  it costs. Refer to one by its name and link its section
  (`[portable](goals.md#portable)`), never by number. An aim that is not on `goals.md`
  is not called a goal elsewhere. A new goal goes onto `goals.md` first.
- **Source as source.** Pages may point at code, as relative links to repository files
  (`[crates/psh-syntax/src/lexer.rs](../../crates/psh-syntax/src/lexer.rs)`). A link to
  a path that does not exist is an error, so link code only once it exists.

`SUMMARY.md` lists the pages in reading order (mdBook's format). A `- [Title]()` entry is
a chapter not written yet.

## How a page is written

- Start with what the reader wants to know, then an `> **In short.** ...` paragraph: the
  whole page in three sentences.
- Pages about using psh (the language, utilities, Windows, the command line, embedding,
  classification) say what a user sees and can do, with examples. Pages about how it
  works go from the problem to the design. Show real commands and real output, not
  paraphrases.
- Give a design choice its cost where it helps: `> **Trade-off.** X over Y. Why ... The
  cost: ...`.
- End with "Where to look" (source links), once there is source to link.
- Plain, direct sentences, in the present tense. No marketing.
- When psh deliberately behaves differently from bash, say so where the behavior is
  described, with the reason.

## Tooling

`cargo run -p psh-docs -- check` checks every page in `SUMMARY.md`; the gate runs it. It
reports `docs/guide/PAGE:LINE: message` for:

- a link to a page that is not in `SUMMARY.md`, to a section (`#id`, as mdBook makes ids
  from headings) that does not exist, or to a repository file that does not exist;
- a mention or link of ARCH.md, TESTING.md, docs/*.md outside the guide, `work/` or
  `research/`, and any `.md` name that is not a guide page;
- issue numbers (`0004`), decision IDs (`D005`), bare goal numbers (`G2`, "goal 3");
- history phrases ("no longer", "used to", "was replaced", "originally", ...); "is used
  to" is fine;
- `pi` as a whole word on any page but `pi.md`.

Fenced code blocks are skipped; inline code is held to the word rules but not to the
history phrases. A false positive is fixed in the tool, not worked around in the text.
