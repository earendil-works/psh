# Coordinator checklist

Notes for the coordinator (the main agent). Many come from djs, where they were learned
the hard way.

## Editing main
- Commit coordination edits (issues, board, docs) right away; never leave the primary
  checkout dirty. Once `scripts/with-merge-lock.sh` exists, make the edit **inside** the
  lock: `scripts/with-merge-lock.sh bash -c '<edit>; git add -A; git commit -qm ...'`.
  Editing first and then waiting for the lock leaves `main` dirty while an agent's merge
  holds it, and its fast-forward fails after a full gate.

## Writing and assigning an issue
1. Every issue has `depends-on:`, `touches:`, `guide:` (pages under `docs/guide/` it may
   affect, or `-`) and `decisions:` (its ID range, D(1000 + 10·N)…+9). Forgetting the
   range made two agents in djs use the same ID.
2. Write the issue file, update `BOARD.md`, commit.
3. Switch the agent's permanent worktree to the new branch, **in its own tool call**:
   `git -C .worktrees/<agent> switch -c issue/NNNN-slug main` (worktree clean, old
   branch merged; check `git status --short` first, since `git switch` carries
   uncommitted edits along), or create it once with `git worktree add`.
4. Configure the agent with the **absolute** worktree path, in a separate call (it races
   the worktree creation otherwise).
5. Only then send the assignment, again in a separate call: the agent reacts at once.

## When an agent reports done
- Check `git log main`, that the issue's `## Result` is filled in, and that the `guide:`
  pages, ARCH.md and TESTING.md still read true. Larger doc updates become issues.
- Before removing a worktree or switching it: the agent is idle, `git status --short` is
  clean, the branch is merged (`git branch -d`, never `-D`).
- Update the board (and the scores in it), relay follow-ups that belong to other areas,
  commit.

## Health checks (whenever an agent has been quiet)
- An agent that is idle but hasn't reported may have lost its final message; read its
  last answer.
- `ps -Ao pid,etime,command | grep -v grep | grep -E "psh|bash|cargo"`: anything running
  for more than ~15 minutes is probably a hang. Kill it and tell the agent. Look for
  leftover child processes of spec runs, too.
- `git -C .worktrees/<x> log --oneline main..HEAD` and `status --short`: no commits for a
  long time means a nudge ("commit often").

## Cutting issues
- Once the scoreboard exists (0002), cut language and utility issues from the top of
  `scripts/status --hitlist`, one area per issue, with disjoint `touches:`.
- Moving cases between skip / later / active is the coordinator's call; record why in
  `tests/spec.toml`.
- Keep `research/` as it is: it is the record of the investigation. Current design
  lives in ARCH.md, TESTING.md and the decision log.

## Parallelism
- Parallel only when `touches:` are disjoint, or the overlap is small, additive and
  announced to both agents.
- Expected hot files: the AST types in `psh-syntax` (parser vs everything else), the
  `Host` trait (engine vs `psh-os` vs utilities), `docs/decisions.md` (append-only).

## Pushing
- There is no remote yet. Sub-agents never push.
