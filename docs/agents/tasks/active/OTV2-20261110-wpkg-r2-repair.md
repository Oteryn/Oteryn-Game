# OTV2-20261110-wpkg-r2-repair

```yaml
task_id: OTV2-20261110-wpkg-r2-repair
title: WPKG-R2 collector repair and grouped follow-up
mode: REPAIR
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy-wpkg2
pr: 3042
base_sha: 4f0b6c62a789489fca6c37dd36d5917d096f328e
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: PR3042 repair lead; publication and review-dispatch owner in comment 6767805546
created_at: 2026-11-10T15:59:04Z
updated_at: 2026-11-10T22:32:22Z
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/dispatch/src/gameplay_transport/connection/serve/
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc7_hydra.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/target_move.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/s17_audit_busy.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/s17_client_watch.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/s17_persistent_bans.rs
  - crates/gameplay/durability/src/durability/character_authority.rs
  - crates/gameplay/social/src/cast/service.rs
  - crates/client-ui/tests/highscores.rs
  - tools/qualification/node_boot/run.sh
  - tools/qualification/node_boot/test_diagnostics.py
  - docs/contracts/logging/events/durability.json
  - .github/workflows/merge-gate.yml
  - tools/repository/test_classify_pr_test_lanes.py
  - tools/repository/validate_repository_policy_core.py
  - docs/agents/tasks/active/OTV2-20261110-wpkg-r2-repair.md
public_contracts:
  - docs/contracts/protocol-oteryn/v1/cast/cast-v1.md
  - docs/contracts/S17_CAST_CONSENT_PERSISTENCE.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Repair the remaining reproducible failures of collector PR #3042, retain strict
authority and content assertions, and qualify one exact published candidate.
The owner requested frequent coherent pushes, subagents, light local checks,
database validation on GitHub, and one grouped P2/P3 follow-up at the end.
This record covers the repair continuation. Live PR/check evidence supplies
the final SHA and qualification state; this file does not supersede it.

## Current continuation: repeated CI metadata-count failure

The owner resumed the same repair task. Direct exact-head readback of
`268b9bbf2ff9bc6f62dc44b5dd66364b0e3d4d2d` now confirms all 24 PostgreSQL
partitions and heavy-gate pass in run42868636377. The original merge gate
42868635891 passes with Clippy and 10,904 unit tests. PG15 passes all 43 E2E
and 198 library tests, including the repaired TARGET-MOVE case.

The metadata companion42869075398 failed scope job130039738079 with
`pull request file count changed during scope classification`. One retry,
after stable exact head/base and 2507-file readback, failed the same condition
in job130172723589. There is no further retry of that unchanged code.

After explicitly returning to AUTHORING, this successor repairs only count
handling in the scope job. Both live metadata reads still require valid integer
counts. A valid count-only change invalidates transported file evidence while
all state/repository/head/base fences remain mandatory. The protected-base
classifier reconstructs the immutable Git diff; if unavailable it selects FULL.
No required job, PostgreSQL partition, assertion or identity check is suppressed.
The scope's canonical policy digest is regenerated from its actual job text.

The executable regression reproduces the original failure before the change.
It then checks count drift in both directions and across the 300-file boundary,
strict refusal of malformed counts and real identity changes, clearing stale
records, actual Git recovery of a server change, and FULL fallback without Git
objects. Independent source review found no P0/P1 in this bounded delta.
The exact successor's check and Codex receipts belong in control comment
6767805546 after publication; predecessor results do not qualify its new SHA.

Node130018822709 remains red with a visibility disconnect and replacement
revision-chain refusal. Existing repairs remain owned by PR3076 and PR3093;
the Node exception is not evidence of a runtime pass. Current main c255c6b8
also has a separate Exiva fixture readiness failure; this patch does not claim
to fix it. The 22-file P2/P3 batch stays separate from this CI repair.

## Architecture and source of truth

- PROVEN: the bound META policy is Oteryn/Oteryn at
  `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`, version 3.1.0.
- PROVEN: `protocol-oteryn`, distinct WorldId/ChannelId, current node and
  session-generation fences, and the accepted S17 contracts remain unchanged.
- PROVEN: the lead is the only publisher; workers return disjoint edits or
  read-only reviews. API publication creates a new one-parent candidate with
  an expected-head, non-force ref update, followed by exact readback and freeze.

## High-risk authority/recovery qualification

Earlier repairs preserve current node/session-generation fences and the accepted
S17 contracts. Cast Join/Renew retries only RootUnavailable within one second;
each attempt rechecks node authority and persistent bans, and elapsed time still
reduces admission lifetime. SQL/authority failures remain final. The actual-root
regression covers held resources, bans, expiry and missing storage. The recovery
change only adds closed, sanitized diagnostic labels; it changes no predicate.
Full historical evidence and exact earlier job IDs remain in comment6767805546.

## Acceptance criteria

- [ ] Required `game-gate` and `heavy-gate`, including all 24 PostgreSQL
  partitions, pass on the unchanged final PR head.
- [ ] The approved final Codex review has an exact-head receipt and no open
  concrete P0/P1 finding; existing resolved findings stay resolved.
- [ ] The grouped P2/P3 follow-up is recorded with explicit dispositions.
- [ ] Landing, if eligible, satisfies main-health and post-merge requirements.

## Reference parity

NOT_APPLICABLE: this is a repair of existing code and qualification fixtures,
not a new mechanic or change to reference gameplay values.

## Excluded scope

No workflow/protection weakening, production deployment, external-repository
mutation, duplicate Windows fix or broad appearance dependency import.
Node boot remains informational under the current workflow and the owner
exception in PR comment 6745293631; diagnostics are not a claim of full Node
qualification. The separate Windows repair is PR #3094.

## Implementation / findings

- PROVEN: `c5c16989f` heap-pins the remaining native quest route futures without
  changing their predicates, calls or assertions. Trader passed in 70.506 s on
  predecessor `2aa8b76b8`; all 194 library cases in its partition passed.
- PROVEN: `2aa8b76b8` rotates six periodic refresh classes while preserving
  input/probe/server-message/chase precedence. The exact production poll proof
  is baseline-red/fixed-green; the real slow-vitals/chat regression passed in CI.
- PROVEN: `455c0c243` repairs the one-shot cast-ban holder path, uses monotonic
  fixture clocks, preserves failed observer server results and clears the Hydra
  fixture's retained ground cache across its simulated reset epoch.
- DERIVED: the original observer log hid its inner error. Holder contention
  can explain it, but that attribution is not proven. The same journey passes
  on746 as recorded above.
- PROVEN: closed inbound/recovery diagnostic labels preserve behavior and are
  sanitized. The missing event registration found by the Rust policy job is
  repaired using the exact PR3076 registry entry from `96b4cd55`.
- PROVEN: main `4f0b6c62` added category 16 to the highscore map but left old
  selection assertions. Updating indices 13, 14 and 15 retains reserved-ID coverage.
- PROVEN: the Windows repair in PR #3094 passed its real installer job
  129953705514 on `b028bef4`; all 24 PostgreSQL partitions of that main also pass.
  Its failing leaf jobs are the known highscore assertion and Node admission.
  Main later advanced through PR #3090 to PR #3091 at `43f48d1f`; its new full
  run42862387522 is not yet qualified. Read-only composition is conflict-free,
  and source/light checks preserve our cast and diagnostic behavior. The Node
  exception is not evidence of successful full-main qualification.

## TARGET-MOVE fixture repair after exact-head validation

Commit268b9bbf replaces five unacknowledged FOLLOW observations with retained
DuplicateReplay barriers, preserving all assertions, IDs, 350ms delays and timeouts.
The isolated production poll/tick scheduling proof reproduced the y3/y4 failure;
the barrier passed4/4 cases and detected a deliberately broken cancellation.
Exact268 PG15 subsequently passed43/43E2E and198/198library cases, including
TARGET-MOVE. The separate unproven death-cell/marker gap remains in the backlog.

## Validation

Current CI-scope repair: the real workflow regression is baseline-red/fixed-green.
All12 existing routing test functions pass. The repository policy validator passes
(23files,68workflows); the scope pin is computed from the actual job text. Focused
applicability and PR-gate unit groups each pass10/10. Native PowerShell canaries
are left to CI because this executor has no pwsh. Governance validation passes; the final whitespace check runs before publication.

Exact268 predecessor evidence: Clippy,10904units,24/24PG andheavy-gate pass.
Its companion scope retry still fails; the new successor needs its own complete
required checks. No local Rust workspace build or database execution is used.
Earlier diagnostics, cast, highscore and scheduling proofs remain in control
comment6767805546 and the preceding revisions of this task record.

## Self-review

The lead and an independent worker inspected the complete CI repair. Count drift
cannot authorize stale narrowed lanes: transported records are discarded, exact
Git recovery is verified, and missing evidence selects FULL. Real identity drift
still fails. The canonical scope digest matches the modified job.

## Independent review

Three historical Codex rounds completed; the latest clean receipt6773134030
reviews746ee8a124. Commit268 only adjusted the fixture and record; its independent
source review and exact-head CI are recorded separately from that receipt.

The owner's current task explicitly requests Codex Review for P0/P1 detection.
This material CI-control delta will receive one fresh read-only review after
publication, scoped to the new delta. The lead alone dispatches it after fresh
same-head deduplication. A failed CLI initialization is not a completed review.
No duplicate same-head invocation or automatic review loop is planned.

## PR and closeout

PR: https://github.com/Oteryn/Oteryn-Game/pull/3042.
The one Rashid P2 thread is fixed and resolved. Remaining P2/P3 are consolidated
in comment 6766071958 for the requested grouped follow-up after primary work.
The precise 22-file mechanical P2/P3 scope is prepared, with unproven suggestions
explicitly deferred. No merge or ownership release is claimed. Keep this record active while exact
qualification, the external main-health blocker or the grouped follow-up remain;
archive it in the final authoring change of the task's last PR. Do not move a
frozen head merely to copy CI/review status into this file.

## Context checkpoint

Live comment6767805546 binds the exact published head, run/job IDs, review receipt,
next action and sole publisher. Publish the bounded CI repair, freeze its exact
remote SHA, obtain one scoped Codex review and inspect its required runner checks.
Keep the22-file P2/P3 batch separate; do not conflate the owned Node fixes in3076
and3093 with this repair. No owner permission question is pending.
