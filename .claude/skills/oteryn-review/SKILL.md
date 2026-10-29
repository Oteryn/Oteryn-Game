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
- `P2` and lower never block: fixed in the same push when cheap, otherwise
  in the module's next batch.
- Each finding: `file:line`, the defect class with every instance of it in
  the diff, the evidence it is real, and a concrete fix. Style points,
  praise and a retelling of the diff are left out.
- Nothing significant found: say so in one line; a review with no findings is
  a valid result.

## 5. Independent review by Codex

Codex review spends owner-funded quota. `docs/agents/OWNER_FUNDED_AI_POLICY.md`
covers exactly one review of an exact head, and only when the bound META
review policy or the task contract requires it; an optional second opinion
needs the owner's authorization for that use and is skipped without it, not
turned into a question.

When a review is covered: only the coordinator (the review-dispatch owner)
starts it, once per frozen head, after the local checks pass. On a PR that is
one `@codex review` comment; outside a PR, `codex exec` in a read-only
sandbox at reasoning effort medium, with the diff, `CODEX-CHECKLIST.md` and
the instruction to report findings only. Its output is findings to verify
with step 3, not a verdict; a failed or empty run means "not reviewed".

## 6. Receiving findings (Codex, Claude review, people)

- Verify each finding against the code: trace a real caller or input to the failure.
- Real and in proportion: fix it in the next push and reply in one line with
  the fix. Not real or not worth its code: reply with the evidence (the path
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
