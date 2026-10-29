---
name: oteryn-codex-packet
description: Write a delegation packet for a Codex (cloud or local) or subagent implementation task in Oteryn-Game, and review what comes back before it reaches a branch. Use when handing a mechanic, slice, fix or audit to another agent, or when integrating a returned diff.
---

# Delegating work and taking it back

A delegated agent knows only its packet. Most rework in this repository came
from packets that left out the base, the claimed numbers or the definition of
done, and from returned diffs that were pushed without review.

## The packet

Write it as one self-contained file. Include, in this order:

1. **Task:** id, one-sentence goal, and whether the result must be DONE or a
   named PARTIAL slice (`oteryn-mechanic`, *What "done" means*).
2. **Base:** the exact `main` SHA to start from and the branch name.
3. **Claimed numbers:** migration, capability, command and offer order,
   checked against `main` and open PRs at writing time.
4. **Owned paths:** the files and directories it may write. Everything else is
   read-only; shared registration files are off limits (use the mounted
   directories and per-entry fragments).
5. **Scope and excluded scope:** what to build and what to leave alone, with
   the reference rules (tibia.com page, Canary/Crystal file and line) it must
   match.
6. **Validation:** the exact commands from `oteryn-build-test` for those
   paths, and which tests must fail before the change.
7. **Report:** at most 15 lines: result, head SHA or diff name, commands run
   with pass/fail counts, open questions with options and a recommendation,
   what is still PARTIAL.

Keep the packet short: point at repository files (`AGENTS.md`, task card,
README of a directory) instead of pasting them. Keep reasoning effort at
`medium` or lower unless the owner says otherwise.

## Limits to state in every packet

- No Docker, no new third-party dependencies, no network access beyond what
  the task names.
- Never weaken, skip or delete a test; never hand-edit pins or digests; never
  edit a merged migration.
- Never read credentials, tokens or `.env` files; never print a database URL.
- Do not commit notes, logs, validation JSON, `__pycache__`, evidence scratch
  directories or `*-FINAL.md` working files.
- Do not open PRs, push to other branches, merge or label; return the result.

## Taking the result back

Before a returned diff goes onto a branch:

1. Apply it to a worktree on the packet's base, then merge current
   `origin/main`.
2. Read the full diff. Reject or trim: files outside the owned paths, stray
   notes/logs, weakened assertions, `#[ignore]`, hand-edited digests,
   renumbered migrations of other mechanics, unrelated refactors.
3. Check the claimed numbers are still free on `main`.
4. Run the validation yourself; the agent's own report is a claim, not
   evidence.
5. Push one commit with the result, then open or update the PR
   (`oteryn-mechanic`, *The PR*).

A task that died without a result (capacity error, timeout) is restarted from
the same packet, not silently dropped. Track every delegated task until its
result is merged, rejected or explicitly parked.
