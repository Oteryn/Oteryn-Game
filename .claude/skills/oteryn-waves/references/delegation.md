# Delegating work and taking it back

A delegated agent knows only its packet. Most rework in this repository came
from packets that left out the base, the claimed numbers or the definition of
done, and from returned diffs that were pushed without review.

## How much to delegate at once

Send no more delegated tasks than results you can review and land in the
same day; unreviewed results pile up as open PRs that are later closed
unmerged. A delegated result lands on a branch; it becomes a PR only when it
is DONE or an `a` slice (`oteryn-dev` `references/mechanic.md`), and parked
work stays on its branch with a line in the tracking ledger, not as an open
PR.

## The packet

Write it as one self-contained file. Include, in this order:

1. **Task:** id, one-sentence goal, and whether the result must be DONE or a
   named PARTIAL slice (`oteryn-dev` `references/mechanic.md`, *What "done"
   means*).
2. **Base:** the exact `main` SHA to start from and the branch name. The
   worker's first step is to check that its `HEAD` equals that SHA, and to
   stop and report on a mismatch rather than build on another tree.
3. **Claimed numbers:** migration, capability, command and offer order,
   checked against `main` and open PRs at writing time.
4. **Owned paths:** the files and directories it may write. Everything else is
   read-only; shared registration files are off limits (use the mounted
   directories and per-entry fragments).
5. **Scope and excluded scope:** what to build and what to leave alone, with
   the reference rules (manual note, TibiaData record, Canary/Crystal file
   and line) it must match.
6. **Success check:** the test or command that is red on the base and must be
   green on the result, named exactly (`oteryn-coding`, *Every task has a
   check*), plus the commands from `oteryn-dev` `references/local-checks.md`
   for the owned paths. It must run where the worker runs: a PostgreSQL check
   only when the packet gives a database route (a PC lane's own database);
   otherwise a non-PostgreSQL test, with the PostgreSQL case run by the
   coordinator.
7. **Report:** at most 15 lines: result, head SHA or diff name, commands run
   with pass/fail counts, open questions with options and a recommendation,
   what is still PARTIAL.

Keep the packet short and decided: write it for an engineer who has never
seen the repository, so it names the files, the function and type names, the
values taken from the source and the test for each step, and leaves open only
what any competent engineer would choose the same way. Point at repository
files (`AGENTS.md`, task card, README of a directory) instead of pasting them.
State what to do rather than what to avoid; a prohibition belongs only in the
limits below. Keep Codex reasoning effort at
`medium` or lower unless the owner says otherwise; repository subagents use
the model and effort set in their own `.claude/agents/*.md` definition.

## Limits to paste into every packet

The receiving agent cannot see this skill, so paste these lines verbatim:

- No Docker, no new third-party dependencies, no network access beyond what
  the task names; use only the database route this packet gives.
- Never weaken, skip or delete a test; never hand-edit pins or digests; never
  edit a merged migration.
- Never read credentials, tokens or `.env` files; never print a database URL.
- Do not commit notes, logs, validation JSON, `__pycache__`, evidence scratch
  directories or `*-FINAL.md` working files.
- Codex and child sessions: push only the packet's own branch, and only when
  the packet says so; never open PRs, merge or label. Repository subagents (`.claude/agents/`) push
  their own packet branch and report as their definition says, and never
  merge or label.

## Taking the result back

Before a returned diff goes onto a branch:

1. Apply it to a worktree on the packet's base and check it against current
   `main` with `git merge-tree --write-tree origin/main HEAD`; merge only on
   a conflict.
2. Read the full diff. Reject or trim: files outside the owned paths, stray
   notes/logs, weakened assertions, `#[ignore]`, hand-edited digests,
   renumbered migrations of other mechanics, unrelated refactors.
3. Check the claimed numbers are still free on `main`.
4. Run the success check and the validation yourself; the agent's own report
   is a claim, not evidence (`oteryn-verify`).
5. Review it (`oteryn-review`), push one commit with the result, then open or
   update the PR (`oteryn-dev` `references/mechanic.md`, *The PR*).

A task that died without a result (capacity error, timeout) is restarted from
the same packet, not silently dropped. Track every delegated task until its
result is merged, rejected or explicitly parked.
