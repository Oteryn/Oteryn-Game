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
updated_at: 2026-11-10T17:46:53Z
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

## Architecture and source of truth

- PROVEN: the bound META policy is Oteryn/Oteryn at
  `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`, version 3.1.0.
- PROVEN: `protocol-oteryn`, distinct WorldId/ChannelId, current node and
  session-generation fences, and the accepted S17 contracts remain unchanged.
- PROVEN: the lead is the only publisher; workers return disjoint edits or
  read-only reviews. API publication creates a new one-parent candidate with
  an expected-head, non-force ref update, followed by exact readback and freeze.

## High-risk authority/recovery qualification

Cast Join/Renew retry only `DurabilityError::RootUnavailable`; every attempt
proves the current node incarnation and reads the durable ban. The retry window
is one second; the existing SQL-pass budget remains unchanged. SQL/authority
failures and persisted bans remain final. No cached ban, weaker proof, protocol
field or durable write is introduced.

The independent inputs are the recovered node proof, persistent ban row, opaque
VerifiedAdmission and monotonic clock. Actual elapsed time advances `now` before
existing Join/Renew checks, including removal of an expired existing observer.
The actual-root regression holds a semantic pass, requires Pending, releases it,
and checks Join/Renew, persisted-ban refusal, expiry and missing-table refusal.
On746 it passes in7.165s (job129980996909); the original live observer journey
passes in25.297s (job129980997020). The persistent-ban regression retains restart,
idempotency and current-node checks. Extracted proofs do not replace database runs.

The sibling inspection covered both operations and their direct test caller.
Recovery changes only add closed diagnostics and propagate the same failures in
the same order; no recovery acceptance or mutation predicate changes.

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

PROVEN: on `746ee8a124d68d1f2b06b3933f82cc5ea985eae7`, game-gate,
Clippy, all 10,904 unit tests and 23/24 PostgreSQL partitions pass. PG15 job
129980996991 has one failing library case out of 198 (all 43 E2E cases pass):
`target_move_real_dispatch_walk_turn_follow_and_cancel_on_postgres`, at
`target_move.rs:309`, observes `(6,3)` instead of `(6,4)` after the target returns
from another floor. The fixture sleeps 350 ms without confirming that FOLLOW
observed the target loss. Production removes the pending walk once that loss
is observed; the original log does not identify the exact tick/relocation order.

A small harness with unchanged production poll.rs and PendingWalk/tick, plus a
modeled owner, reproduces the same position failure: target returns at 350 ms,
owner completes at 600 ms, and the queued FOLLOW moves. A neutral retained-command
replay barrier passes 4/4 cases, including a 1400 ms delayed validity read and a
mutant which proves broken floor cancellation still fails the position assertion.
This is a scheduling proof, not a PostgreSQL execution of the repaired fixture.

The bounded repair replaces five unacknowledged waits in the existing fixture
with replay ACK, the same 350 ms wait, then two replay ACKs. The helper verifies
DuplicateReplay and the retained result; it never re-executes FOLLOW. The five
observations cover target logout twice, floor loss/return, and death. Assertions,
command numbering, delays and timeout budgets are preserved. Stop/manual-step
observations already have cancellation ACKs. No production hook or logic changes.
The separate unproven death-cell read/marker gap is collected with P2/P3, not
silently repaired here.

## Validation

Focused evidence before the next freeze:

- Rust1.99.1 formatting/parser checks for the changed Rust files: pass.
- `git diff --check`: pass.
- `python3 tools/qualification/node_boot/test_diagnostics.py`: pass, 2/2.
- `bash -n tools/qualification/node_boot/run.sh`: pass.
- TARGET-MOVE scheduling proof: old sequence fails with y=3/y=4; neutral replay
  barrier passes 4/4, including delayed validation and a broken-cancel mutant.
- Exact cast retry helper: baseline held-resource failure, fixed 3/3 pass.
- Existing highscore selection test with exact production tables/functions:
  baseline reproduces Some(16)/Some(17), corrected test 1/1 pass.
- Unmodified production logging validator in an isolated small runner:
  baseline unregistered event, registered event pass.
- `python tools/agents/validate_governance.py`: pass on this authoring batch.
- Governance framework tests:129/129 passed on746; the validator code is unchanged.

Component/integration/E2E: GitHub owns full crate and database/Node execution.
On455c, all10,904unit tests, Clippy and24/24PG partitions pass; its missing logging
registration was repaired in746. The exact746 results and its single blocking
fixture failure are recorded above. Neither predecessor qualifies the upcoming
fixture-only successor. No heavy local build or database test was run.

## Self-review

The lead inspected the fixture diff: only five waits and a neutral replay helper
change; existing assertions, command numbering and budgets remain. Prior repairs
have exact remote-tree readback. Peer inspection confirmed this bounded delta.

## Independent review

Three Codex rounds have now completed. The owner-approved additional final
review completed on `746ee8a124d68d1f2b06b3933f82cc5ea985eae7` with no findings
(receipt 6773134030; sole trigger 6772999885). The additional authorization is
used; no duplicate or fourth review is requested.

The follow-up changes only fixture synchronization and this record. It changes
no production/server logic, persistence, session fence, protocol or durable value.
Under OWNER_FUNDED_AI_POLICY.md, unchanged-risk movement does not authorize
another paid review. The existing receipt covers the unchanged production delta;
it is not represented as a Codex review of the successor SHA. Focused independent
source inspection and fresh exact-head CI still apply to the fixture repair.

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

The exact published head, run/job IDs, review authorization, next action and
ownership are maintained in PR comment 6767805546. Current next action: finish
and inspect the bounded TARGET-MOVE fixture repair, publish one successor, freeze
its exact SHA and requalify it. No owner permission question is pending.
