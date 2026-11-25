---
name: oteryn-review
description: Reviewing an Oteryn-Game diff before it lands (a returned Codex or subagent result, your own branch before ready, a wave's batch diff) and answering review findings from Codex, Claude review or a person. Load before landing or marking a change ready, when asked for a review, and when review comments arrive.
---

# Reviewing changes in Oteryn-Game

A review reads code, not reports. The worker's summary, the PR body and a
green check are claims; the diff and the merged tree are the evidence.

## 0. Gate before any review is requested

Review is for what a command cannot find. The review retro of 2026-11-18
(5 sessions) found that most findings were exact golden counts, stale digests
or pins, missing predecessor pins, test-only code in a non-test build and
first-entry paths nobody ran; each of these a command catches. So before a
review is requested on a head, the head has:

- the local checks that `oteryn-dev` `references/local-checks.md` selects for
  its changed paths green, run with the exact commands given there (content
  and world changes: the generators and `pin-check` as in
  `references/content-world.md`, with `git status --porcelain` empty
  afterwards, since `--resolve` stages what it regenerates);
- when the change has a runtime caller, that caller run: the PG case or
  node-boot row the packet names, red before the fix and green after, or,
  while it is red on `main` for an unrelated reason, the same rows compared
  against `main`. A PG case counts only with proof it ran against PostgreSQL
  (a route-less case returns `Ok(())` without running). Where the session
  cannot run it (no PostgreSQL route), say so: the coordinator runs it when it
  has a route, otherwise CI (`heavy-tests`) does;
- the head that will merge: a train PR is reviewed on its integrated train
  head, and a unit that lands only through the train PR gets no review of its own. A
  PR that merges on its own, a train unit merged through its own PR included,
  is reviewed after its first push (§5);
- the author's self-review at effort high (owner decision 2026-11-25): the
  whole diff against `docs/agents/review/CODEX-CHECKLIST.md` and, when it
  touches a subsystem, that subsystem's invariant file
  (`docs/architecture/subsystems/<id>.md`, consumers and guard tests
  included; until a lane's file exists, the lane's owning contracts in
  `docs/contracts/` and its guard tests), with the consumer map of the packet checked against the diff;
- the pre-push list of `oteryn-coding` (*Before you push*): own PostgreSQL
  cases proven to reach assertions, every changed literal grepped and each
  real consumer in the owned paths updated (the others named),
  full tests of changed crates after any main-merge.

A head that fails the gate goes back to its author, not to review.

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
Then grep the root cause across the whole tree, not only the diff: a class
fixed in one place and left in another is the next round's finding.

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

- When: once per PR, after its first push, over the whole diff against `main`,
  at effort high. Every content train PR and every PR with code in a risk
  class (`oteryn-coding`, *Risk classes*) gets this review even with the
  `heavy-tests` label (owner decision 2026-11-25); only a data-only
  `heavy-tests` PR keeps the automatic skip. A second review only when a substantial logic change (server code,
  persistence, session fencing, protocol, durable value) lands after the first
  one, and then only over the new diff. At most 2 reviews per PR. If the second
  review still finds a P0/P1, the design of the change is wrong: stop and
  report to the owner instead of a third round.
- Who (`docs/agents/OWNER_FUNDED_AI_POLICY.md`, *Single review-dispatch
  owner*): under an active coordinator, the coordinator, unless the packet
  names the lane (a Claude session, child session or PC lane) as the review
  owner of the heads it pushes (owner decision 2026-11-11); a lane not so named
  returns a review packet instead. Without a coordinator, the exact task
  owner. Before either route (Cloud task or PR comment), the review owner
  reads the live PR state and does not start a second review of the same
  head. It runs in Codex
  Cloud (owner rule 2026-11-17), never with `codex exec` on the PC. The owner puts
  "Codex review <head SHA>: N findings, M fixed" in its report and, when it
  opens the PR, in one PR comment; a head that already has that line is not
  reviewed again. A session without the PC still runs it in Codex Cloud;
  "not reviewed, no PC" is not an accepted result. A Codex lane never reviews its
  own code.
- How: the review owner starts it from its own chat session in Codex Cloud,
  read-only, model gpt-6.2-sol at reasoning effort high (owner decision
  2026-11-25, calibration), never through the PC; the gpt-6.2-astra runs at effort high that the
  run-selection table (`references/batch-review.md` §1) names run on the same head,
  whose P0/P1 count only after a traced REAL verdict (§6). Never at effort
  medium. From a session without Codex Cloud, one native `@codex review` PR
  comment naming the exact head. By default the round is one holistic run
  over the whole diff and the callers of what it changes, told to report
  every finding with no cap, every other instance of the same class, a
  quoted caller for each P0/P1 and a refute attempt before its output; a
  diff above about 1500 lines is also split into chunks. The holistic run,
  chunking of big diffs, the prompt, the merge, verify and sweep steps and the
  stop rule are in `references/batch-review.md`; read it before you start a
  review or delegate one. Code Codex wrote is
  reviewed by a new Codex run with a clean context, never as a continuation of
  the run that wrote it.
- Its output is findings to verify with step 3, not a verdict; a failed or
  empty run means "not reviewed".
- Fixing confirmed findings (owner decision 2026-11-11): the review run never
  edits code. The review owner (§5 *Who*) verifies each finding (§6), writes
  only the confirmed P0/P1, one per line as `P0|P1 path:line - problem -
  failure path`, to a file and hands
  them, with every instance the pattern sweep found, to one code-writing
  Codex run, model gpt-6.2-sol (one run for the whole batch, never one per
  finding), at reasoning effort `high` when a P0/P1 is in a risk class and
  `medium` for any other fix (owner decision 2026-11-25), as one Codex Cloud
  packet with the same list, launched from the review owner's chat session
  (owner rule 2026-11-25: never through the PC); when another session holds
  the branch as its writer, the list goes to that writer instead. Per finding it writes a red test first, reports `NOT
  REPRODUCED` without a code change when the test cannot fail, then makes the
  smallest fix and reruns the test green; it changes no existing assertion,
  timeout or budget and nothing outside the listed findings. Nothing is
  committed: the review owner reviews that diff (§2-§3), then commits and
  pushes it.
- Fixes made for a review do not get another review unless they meet the
  second-review rule above.
- Merge: CI green and no open P0/P1 finding from the review.

## 6. Receiving findings (Codex, Claude review, people)

- Give every finding one verdict before any fix: `REAL-P0..P3`, `BY DESIGN`,
  `DUPLICATE`, `FALSE` or `OUT OF SCOPE`, with `file:line`. A `REAL-P0/P1`
  names the reachable caller you traced; a `FALSE` names the trace that
  disproved it; a P2/P3 may be speculative (§4). A finding that needs a change
  outside the branch's owned paths (engine, client, another lane) is `OUT OF
  SCOPE` and keeps its severity: a P0/P1 goes as a BLOCKER to the owning lane,
  a P2/P3 to that lane's backlog; neither goes into this fix run.
- Verify each finding against the code: trace a real caller or input to the failure.
- Real and in proportion: a P0/P1 is fixed in the next push and answered in
  one line with the fix; a P2/P3 is answered in one line and goes to the
  grouped P2/P3 backlog (step 4). Not real or not worth its code: reply with the evidence (the path
  you traced, the test that covers it). A P1 is never left silent.
- No performative agreement ("great catch"); actions and evidence only.
- Name the class of every `REAL-P0/P1` (missed caller, data and test
  coupling, drift after a main-merge, swallowed error, and so on) and check
  whether a guard test already covers it; if not, the fix adds one (§7).
- Collect all fixes of one review round into one push.

## 7. The fix packet

The fix run (§5) gets one packet for the whole round:

- the findings numbered, and a required status per number in the report:
  `FIXED <test>`, `NOT REPRODUCED` or `BLOCKED <why>`. Lanes handed a list
  tend to stop after the first item; the numbered status list makes a skipped
  item visible;
- the sweep list: every verified instance of each class across the tree, with
  `file:line`, including sibling tests in other crates or languages (a reward
  count asserted in Rust and in Python). Instances inside the packet's owned
  paths are in scope; the others are `OUT OF SCOPE` (§6). An exact count or
  digest that follows changed data is not the fix run's to edit (it changes
  no existing assertion): the review owner updates or regenerates it in its
  own commit and names it in the report;
- a red test first, through the real caller (PG case, dispatch path or node-boot
  row), not a unit test of the patched helper only;
- a guard test for the class of every P0/P1 (owner decision 2026-11-25: a
  recurring P0/P1 class becomes a CI guard test, not a note in a skill): one
  choke-point function and a source-scan, table or invariant test over every
  variant. Stale data (counts, digests, pins) is guarded by the generator
  check, and tests derive expected values from data or regenerated goldens,
  not from hard-coded counts, coordinates or versions;
- the forbidden paths: a fix diff outside the packet's owned paths is
  rejected whole. `.github/**`, `tools/repository/**` and
  `.github/repository-policy.json` also need the owner's approval of that
  path for this change, even inside an allocation.

## 8. When review does not converge

Patching stops when any of these holds (P2/P3 never trigger it):

- a round finds more P0/P1 than the round before;
- the same P0/P1 class in the same module or flow comes back in a second PR
  or round;
- a P0/P1 is a regression or a second site of an earlier P0/P1 fix.

Then stop and report to the owner with a proposed rule the code can check (a
transition table with one test per state and event, a single transaction for
a durable value, one shared source for a policy plus a parity test). It is
implemented after the owner agrees, as its own PR with its own review, never
as a further push to a PR whose reviews are used up. This is the §5 rule ("a
P0/P1 after the second review means the design is wrong") applied earlier.

## 9. Two kinds of review: per-PR review and the owner's review loop

§0-§8 describe the **per-PR review**: at most two per PR (§5). An
**owner-requested review loop** reviews a whole area or product of merged code
in rounds until it is clean, under its own authorization
(`docs/agents/OWNER_FUNDED_AI_POLICY.md`, *Owner review loop*). Its rounds run
on `main` and are never counted as, or used in place of, the review of an open
PR; each fix PR it produces gets its own per-PR review. Only the coordinator
starts a round, for a loop the owner asked for. The procedure (charter, cuts
by state, flow and boundary, findings ledger, stop bar, budget) is
`references/review-loop.md`; read it only when running a loop.

## Sources

Adapted, not copied verbatim: getsentry/skills `find-bugs` (Apache-2.0: full
diff, attack surface, check every item, pre-conclusion audit, no invented
issues); steipete/agent-scripts `codex-first` coordination notes (MIT:
verify merged code, unrequested commits, moved limits, test-helper edits);
obra/superpowers `receiving-code-review` (MIT); the cross-model review idea of
trailofbits/skills `second-opinion` (described, not copied); Oteryn's
`CODEX-CHECKLIST.md` and `claude-review.yml` (P1/P2 format).
