# Batch review: catch most defects in one round

Read this before you start a Codex review (§5) or hand one to a subagent. It
explains how one review round finds most of the defects in a diff at once,
instead of 10 findings, then 5, then 3 over several rounds.

Why rounds shrink: one reviewer with one prompt stops after its first strong
findings, and a line cap cuts the rest. Each fix also adds new code that the
next round reviews. The rules below remove those causes. They do not change
the limits in §5: at most 2 reviews per PR, the review never edits code, and
the fix runs at effort `high`.

## 1. One round = several lens runs over the same head

A "review" in §5 is one round over one exact head. Inside that round, run
independent read-only Codex runs, each with one lens. Each run has a clean
context and does not see the other runs' output.

| Lens | Looks for |
|---|---|
| `concurrency` | lock order, a lock held across `.await` or I/O, foreground vs background holders, watchdogs that do not bound a hang, races between prepare and publish |
| `persistence` | fencing and generation checks, idempotency, transaction scope, an outcome mapped to success by `_ =>`, migration and SQL hazards |
| `protocol` | wire codes and the spec, capability gates, EOF or disconnect without a logged cause, every end path closing durable state |
| `authority` | durable value (gold, items, bounties), overflow and `i64::MAX`, checks done by the server and not the client, input bounds |
| `tests` | tests that pass without testing (skipped PostgreSQL route, no assertion on the outcome, a sleep instead of a wait), missing red case, CI job filters |

The lens list of a PR is fixed when its first round starts; the second
review (delta only) runs the subset of those lenses that had confirmed
findings. One whole-diff round on the integrated head replaces per-area review loops; per-area
loops on a package ran 8-9 rounds and did not converge, while the one
whole-diff round found the cross-cutting P0 the area loops missed (retro
2026-11-18).

Choose lenses by diff size and by what the diff touches:

- Up to about 400 changed lines: 2 lenses, the two that fit the diff best.
- Larger diffs, or any diff touching persistence, fencing, protocol or
  durable value: all 5 lenses.
- Diffs above about 1500 changed lines: also split into chunks of about
  400-500 lines by file group (one crate or module per chunk). Each chunk
  gets the lenses that fit it. Every chunk still gets the whole diff as
  read-only context, so cross-file defects are not lost.
- Every package PR, and every diff split into chunks, also gets two
  holistic runs in the same round. Each reviews the whole diff as one unit,
  not a chunk, and reports findings anywhere in it:
  - `whole-integration`: how the parts interact on the integrated head:
    shared state, duplicate or conflicting registrations, ordering of
    handlers and migrations, a migration against another member's code,
    contracts and fixtures against the code, generated files (quest tree,
    pins, registries) against their sources.
  - `whole-flows`: each player flow the diff changes, end to end (client
    command, dispatch, rules, durability, PostgreSQL and back), with the
    cheating-player cases: two clients at once, disconnect midway, a
    replayed frame, a restart during a write, values at their limits.
  Chunk runs alone are not a covered review of a package: per-chunk runs miss
  defects between members. The delta review (second round) reruns
  `whole-integration` over the fix commits together with the code they touch.

Each lens is one Codex Cloud task (read-only, model `gpt-6.2-astra`, reasoning
effort `high`), never `codex exec` on the PC. Name the task
`review-<tag>-<lens>`. The lens tasks of one round over one head are together
the one covered review of that head (`OWNER_FUNDED_AI_POLICY.md`, *Owner
review rule for every PR*), started by the review owner of `SKILL.md` §5.
The native `@codex review` comment route has no lenses: it is one run.

## 2. The prompt: all findings, no cap

Each lens prompt asks for every finding, with no line or count limit.
Use this text in the Codex Cloud review packet or a subagent prompt:

- Report EVERY real defect you find in your lens, P0 to P3. There is no limit
  on the number of findings. Do not stop after the first strong ones.
- For each finding, also list every other place in the diff with the same
  defect class (the pattern sweep).
- Format: `P0|P1|P2|P3 path:line - problem - concrete failure path`.
- Mark uncertain findings with `(uncertain)` instead of dropping them.
- End with `VERDICT: CLEAN` or `VERDICT: N findings`.
- Append `docs/agents/review/CODEX-CHECKLIST.md` and, when the PR fixes a CI
  failure, the failing log lines (test name, panic or assertion, the last 30
  lines of the job). A reviewer that sees the failure finds its cause in the
  diff much faster.

## 3. Merge, verify, sweep

1. Merge the lens outputs. Remove duplicates by `path:line` and defect class.
2. Give every finding a verdict (`SKILL.md` §6). Verify each P0/P1 against
   the code: trace a real caller or input to the failure. For the ones that
   are hard to call, a second check with the opposite brief helps: a fresh
   Claude subagent (not another paid Codex review) told "try to refute this
   finding; default to refuted when unsure". Keep the finding when the refute
   attempt fails.
3. Pattern sweep: for every confirmed finding, grep the whole tree for the
   same class (same call, same lock, same `_ =>`, same unchecked add, same
   literal that an environment knob overrides), and every test in any crate or
   language that asserts the changed value. Each hit is a candidate,
   verified as in step 2 before it joins the confirmed list; in a delta review
   a hit in unchanged code is confirmed only as a P0, otherwise it goes to the
   backlog. A class fixed at one site and left at another is the main cause of the
   next round: in the retro, rounds that did not converge found one sibling
   per round.
4. P2/P3 go to the grouped list (§4). They do not start a fix now.

## 4. One batch fix, one delta review, one CI cycle

1. Write all confirmed P0/P1 findings, sweep instances included, to one file
   and run one fix: on the PC `bash ~/oteryn-cache/codex-review.sh <worktree>
   <tag> --fix-confirmed <file>` (not in this repository), elsewhere one Codex
   Cloud fix packet with the same list (`SKILL.md` §5).
   Never one fix run per finding. The packet follows `SKILL.md` §7 (numbered
   items with a status each, sweep list, red test through the real caller,
   class guard, forbidden paths).
2. Review the fix diff yourself (§2-§3) and run the local checks.
3. Push once. One CI cycle covers the whole batch.
4. The second review allowed by §5 covers only the fix diff (the delta), with
   the lenses of the confirmed findings, under the same rules.

## 5. When to stop

The diff is done when every lens run of the last round finished and no
confirmed P0/P1 is open. A failed or empty lens run means "not reviewed". If
the second review still finds a
P0/P1, §5 applies: the design is wrong; stop and report to the owner. Never
start a third round.

## 6. For subagents and child sessions

A packet that delegates review or fixing points to this file and states:

- the head SHA, the lenses to run and, for big diffs, the chunks;
- "report every finding, no cap; sweep each class across the diff";
- the failing CI log lines, when there are any;
- "do not edit code in the review run; fixes go through `--fix-confirmed`
  after verification".

The merged, verified list with the sweep instances goes to a findings file
(path in the packet), not the raw lens logs; the final report (at most 15
lines, `CONTEXT_ROUTING.md`) gives the counts by severity and that path.
