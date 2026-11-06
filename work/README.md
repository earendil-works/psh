# Work tracking

All work is tracked in files in this directory, committed to git.

- `BOARD.md`: index of all issues by status, with dependencies and who works on them.
  Owned by the coordinator.
- `COORDINATOR.md`: the coordinator's checklist.
- `issues/NNNN-slug.md`: one file per issue. The assigned agent updates its status, log
  and result.
- `scratch/`: gitignored scratch space, one subdirectory per issue (`scratch/0002/`).
  Also holds `scratch/merge.lock` while `main` moves.

## Issue file format

```markdown
# NNNN: Title

status: open | in-progress | blocked | done
agent: <sub-agent name or ->
branch: issue/NNNN-slug
depends-on: NNNN, NNNN (or -)
touches: paths/or/areas, that/this/issue/may/change
guide: docs/guide pages this issue may affect, or -
decisions: D(1000+10·N)–D(1009+10·N), e.g. D1040–D1049 for 0004

## Goal
What and why, with links into ARCH.md / TESTING.md.

## Acceptance criteria
- [ ] concrete, checkable items

## Notes
Hints, pointers, constraints from the coordinator.

## Log
Appended by the agent while working (dated entries).

## Follow-ups
Work discovered but not done here.

## Result
Filled in when done: what landed, how to use it, known gaps.
```

## Lifecycle

1. The coordinator writes the issue (`status: open`) and adds it to the board.
2. The coordinator creates the branch `issue/NNNN-slug` from `main` and assigns an agent.
   Each agent has one permanent worktree, `.worktrees/<agent>`, which the coordinator
   switches to the issue's branch, so its warm `target/` is reused across issues.
3. The agent works, commits on the branch and merges with `scripts/merge-to-main.sh`
   (rebase, gate, fast-forward).
4. The coordinator reviews the result on `main`, updates the board, checks the pages in
   `guide:`, and deletes the merged branch (`git branch -d`).

## Parallel work

Issues run in parallel only when their `touches:` areas are disjoint and neither depends
on unmerged work of the other. When in doubt, run them one after another. The natural
areas (`research/hillclimb.md`): infra, parser, expansion, execution, process layer,
utilities (one agent per batch), Windows.

## Merging

`scripts/merge-to-main.sh` rebases the branch onto `main` and runs `scripts/gate.sh` in
the issue worktree, then takes `work/scratch/merge.lock` in the primary checkout just to
fast-forward `main`. If `main` moved while it gated, it rebases and gates again (D1010).
If the rebase conflicts, it stops: resolve, `git rebase --continue`, run it again.
`scripts/with-merge-lock.sh CMD...` runs a command under the same lock; the coordinator
commits on `main` through it. Each run appends a line to `work/scratch/merge-log.tsv`.

- `docs/decisions.md` and the ratchet files (`tests/**/expected-failures.txt`) are
  append-mostly; give them git's `union` merge driver in `.gitattributes` so appends from
  different branches never conflict. A stale ratchet line is harmless: the gate reports
  it and the next `--update-ratchet` rewrites the file.
- merge-to-main refuses an issue worktree with uncommitted or untracked files. If the
  primary checkout has uncommitted changes when it is about to fast-forward, it waits
  for them to be committed (up to 30 minutes), then lands on top (D1011).
