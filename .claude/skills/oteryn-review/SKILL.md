---
name: oteryn-review
description: Reviewing an Oteryn-Game diff before it lands (a returned Codex or subagent result, your own branch before ready, a wave's batch diff) and answering review findings from Codex, Claude review or a person. Load before landing or marking a change ready, when asked for a review, and when review comments arrive.
---

# Reviewing changes in Oteryn-Game

A review reads code, not reports. The worker's summary, the PR body and a
green check are claims; the diff and the merged tree are the evidence.

## 1. Gather the whole input

- The diff against the exact base: `git diff <base>...<head>` (the base SHA
  from the packet or the PR). When the output is truncated, page through it
  until every changed line has been seen, and list the files.
- The task card or packet, the parity table and the claimed numbers.

## 2. Look beyond the diff summary

- Files outside the packet's owned paths, and commits nobody asked for (a
  "raise timeout" or "add margin" commit is a lead, not noise).
- Changes to guards, budgets, baselines, golden counts, pins, digests or
  limits: find who moved each number and why.
- Edits to test helpers, fixtures or assertions: a helper change can keep a
  suite green while hiding a regression; `#[ignore]`, `cfg(any())` or a
  looser assertion blocks landing (step 4).
- Migrations: new number free on `main` and open PRs
  (`oteryn-dev` `scripts/next-free-numbers.py`), grants present, no edit to a
  merged file.

## 3. Check every item for every file

Use `docs/agents/review/CODEX-CHECKLIST.md`: every class in it is one Codex
has flagged on Oteryn PRs (unbounded reads, role privileges, lock before
mutate, receipts and idempotency, deadlines, capability gates, reconnect
races, end paths, wiring). Add these, which it does not spell out:

- input from a client (protocol frames, chat, item ids, positions): bounds,
  arithmetic overflow and indexing on untrusted values, size limits;
- a lock or guard held across `.await`; a refused path that does not roll back;
- a hand-edited generated file (pin, digest, derived registry);
- behaviour against the reference: each parity-table row matches its source.

For each possible finding, confirm it is real: it is not handled elsewhere in
the diff, no existing test covers it, and the surrounding code agrees.

## 4. Audit, then findings

First note which files you read, each checklist class with clean or not, and
what you could not verify and why. Then write findings with the severities
of root `AGENTS.md`, *Review guidelines*:

- `P0` (security, data loss, broken `main`) and `P1` (wrong behaviour a real
  caller reaches) block landing and name the concrete path: the input or
  state, the caller that reaches it, the wrong result. Without a reachable
  caller it is at most `P2`.
- A skipped, ignored, loosened or deleted test, or a hand-edited generated
  file, breaks a repository rule (root `AGENTS.md`, *Repository boundaries*)
  and blocks landing whatever its severity label.
- `P2` and lower never block. P0/P1 are fixed at once; P2/P3 are not fixed
  per PR: they are collected (PR, `file:line`, finding) and fixed together in
  one grouped PR at the end (owner 2026-11-09).
- Each finding: `file:line`, the defect class with every instance of it in
  the diff, the evidence it is real, and a concrete fix. Style points,
  praise and a retelling of the diff are left out.
- Nothing significant found: say so in one line; a review with no findings is
  a valid result.

## 5. Independent review by Codex

Owner decision 2026-11-09 (replaces the 2026-10-29 trial and its "at most 5 a
day" limit; `docs/agents/OWNER_FUNDED_AI_POLICY.md` authorizes it): every PR
gets an independent Codex review before it merges. The review runs in parallel
with CI and never blocks or delays it.

- When: once per PR, after its first push, over the whole diff against `main`.
  A second review only when a substantial logic change (server code,
  persistence, session fencing, protocol, durable value) lands after the first
  one, and then only over the new diff. At most 2 reviews per PR. If the second
  review still finds a P0/P1, the design of the change is wrong: stop and
  report to the owner instead of a third round.
- How: the coordinator (the review-dispatch owner) starts it with `codex exec`
  in a read-only sandbox, single agent, model gpt-6.1-sol, reasoning effort high (an
  `@codex review` PR comment has gone unanswered since 2026-10-20, so it is not
  used), with the diff, `CODEX-CHECKLIST.md`, the risky areas of this diff
  (locks, retries, commit and fencing, tests that can pass without testing) and
  the instruction to report findings only, at most 30 lines. Code Codex wrote is
  reviewed by a new Codex run with a clean context, never as a continuation of
  the run that wrote it.
- Its output is findings to verify with step 3, not a verdict; a failed or
  empty run means "not reviewed".
- Fixing confirmed findings (owner decision 2026-11-11): the review run never
  edits code. After you verify each finding (§6), write only the confirmed ones,
  one per line as `P1|P2 path:line - problem - failure path`, to a file and hand
  them to one code-writing Codex run at reasoning effort `medium`: on the PC
  `bash ~/oteryn-cache/codex-review.sh <worktree> <tag> --fix-confirmed <file>`
  (it runs `codex-task.sh` as `fix-<tag>`); elsewhere a Codex Cloud packet with
  the same list. Per finding it writes a red test first, reports `NOT
  REPRODUCED` without a code change when the test cannot fail, then makes the
  smallest fix and reruns the test green; it changes no existing assertion,
  timeout or budget and nothing outside the listed findings. Nothing is
  committed: you review that diff (§2-§3), then commit and push it.
- Fixes made for a review do not get another review unless they meet the
  second-review rule above.
- Merge: CI green and no open P0/P1 finding from the review.

## 6. Receiving findings (Codex, Claude review, people)

- Verify each finding against the code: trace a real caller or input to the failure.
- Real and in proportion: a P0/P1 is fixed in the next push and answered in
  one line with the fix; a P2/P3 is answered in one line and goes to the
  grouped P2/P3 backlog (step 4). Not real or not worth its code: reply with the evidence (the path
  you traced, the test that covers it). A P1 is never left silent.
- No performative agreement ("great catch"); actions and evidence only.
- Collect all fixes of one review round into one push.

## Sources

Adapted, not copied verbatim: getsentry/skills `find-bugs` (Apache-2.0: full
diff, attack surface, check every item, pre-conclusion audit, no invented
issues); steipete/agent-scripts `codex-first` coordination notes (MIT:
verify merged code, unrequested commits, moved limits, test-helper edits);
obra/superpowers `receiving-code-review` (MIT); the cross-model review idea of
trailofbits/skills `second-opinion` (described, not copied); Oteryn's
`CODEX-CHECKLIST.md` and `claude-review.yml` (P1/P2 format).
