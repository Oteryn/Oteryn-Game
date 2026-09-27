# OTV2-20260927-dur03-native-one-item-audit-resource-evidence-513

```yaml
task_id: OTV2-20260927-dur03-native-one-item-audit-resource-evidence-513
title: DUR-03 native one-item audit schema codec and resource evidence
mode: AUDIT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/dur03-native-one-item-audit-evidence-513
issue: 162
pr: 1031
base_sha: e422fc9d2962f63ccb6f0af9ee80ec5f449c2a31
head_sha: 0e5bf0268b3efdad2622364637669ab544289e07
final_head_sha: 0e5bf0268b3efdad2622364637669ab544289e07
final_head_frozen_at: 2026-09-27T21:29:30Z
freeze_reference: "Issue #162 comment 5859991642"
owner: released after terminal lifecycle closeout
created_at: 2026-09-27T22:42:31+02:00
updated_at: 2026-09-28T00:13:39+02:00
completed_at: 2026-09-27T22:07:06Z
execution_policy: continuous_progress
owned_paths:
  - ADD docs/contracts/game-events/v1/native_one_item_transaction.proto
  - ADD apps/game-server/examples/dur03_native_one_item_audit.rs
  - ADD docs/agents/evidence/OTV2-20260927-dur03-native-one-item-audit-resource-evidence.json
  - ADD docs/agents/evidence/OTV2-20260927-dur03-native-one-item-audit-resource-evidence.md
  - ADD docs/agents/tasks/active/OTV2-20260927-dur03-native-one-item-audit-resource-evidence-513.md
public_contracts:
  - docs/contracts/game-events/v1/native_one_item_transaction.proto
depends_on:
  - issue #162 allocation comment 5859343636
  - DUR-03 sections 39.1-39.3
  - ANL-01 and RESOURCE_LIMITS_REGISTRY.json
  - aggregate issue #513
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome boundary

This B2 packet is an unregistered offline schema/codec/resource measurement for
the generic native one-item path. It is not a production event family, runtime
implementation, registry/profile admission, legal Content/loot/inventory
decision, Character readiness proof, PostgreSQL durability proof, playable
Combat result, or Reference parity result.

## Acceptance criteria

- [x] Additive closed candidate grammar represents complete separate one-item
  MINT and TRANSFER aggregates with exact membership.
- [x] MINT and TRANSFER use distinct EventId and TransactionId values and run
  through one shared receipt ledger, including ambiguous pickup replay.
- [x] Two typed definition/revision fixtures exercise one generic path.
- [x] Actual decoded native bindings are checked against independently supplied
  current facts, including exact native positions; stored expected bindings do
  not create current authority.
- [x] TRANSFER before/after state is bound to an independently supplied source
  ItemInstance identity/state; replacing both encoded IDs together rejects.
- [x] MINT carries and validates explicit pre-operation semantic absence instead
  of treating a default item state as proof that no item existed.
- [x] Map/content/room/runtime, death/output/revision and
  GameSession/CharacterLease/CommandRef fixtures are explicit and
  non-authoritative.
- [x] Literal payload and full-envelope goldens plus independent SHA-256 oracles
  cover both operations.
- [x] Exact retry identity/bytes, conflict, lost acknowledgement, ambiguous
  reconciliation, proven noncommit and idempotent replay are covered.
- [x] Payload/envelope/raw-clone/transient-construction-and-canonical-scratch/
  hash/decode/retained/retry dimensions use checked accounting and max/max+1
  rejection.
- [x] Normal, reverse and empty-PATH output is byte-identical.
- [x] Historical v1/v2 candidates are unchanged.
- [x] Canonical draft PR was bound, the repaired successor was frozen, and fresh
  exact-head independent review plus hosted CI passed.
- [x] Native exact-SHA Merge Queue completed with real aggregate `game-gate` and
  protected-main five-blob readback.
- [x] A separate lifecycle allocation moved this task from active to archive
  after source integration.

## Local evidence

- WSL Linux x86_64, Rust/Cargo 1.94 route.
- Focused example tests: 14 passed, 0 failed.
- `rustfmt --check`: PASS.
- Normal/reverse/absolute empty-PATH JSON: exact byte equality.
- JSON: 16,643 LF bytes, SHA-256
  `1558b339151612ecaa3a334a2863ffa7c54fc77c249ce00ac902a324f2157656`.
- Rust: 70,285 LF bytes, SHA-256
  `5da852d958799925acea1dd2fbed7bc781851ac7d8e7e12be04c416b540cb7a5`.
- Proto: 3,473 LF bytes, SHA-256
  `165beae9c29c3405002adf4d6cd2e6cf1cbdff752451c806162643f45eece983`.
- The inherited vendored Tokio missing-doc warning is outside the allocation.

## Protected integration and terminal disposition

- Allocation comment `5859343636` authorized exactly the five source additions.
- First frozen head `b8ed60c20042ed6403fae01f269493dbaa7dc683`
  was superseded after the exact-head review recorded three P2 findings in PR
  comment `5859942283`. The explicit AUTHORING return is Issue #162 comment
  `5859942448`; no old-head review or CI qualified the successor.
- Successor `0e5bf0268b3efdad2622364637669ab544289e07` was frozen under Issue #162
  comment `5859991642`. Fresh independent whole-diff review in PR comment
  `5860022136` found no P0-P3 issue.
- Exact-head pull-request Merge gate run `36352506400` and post-ready
  Architecture semantic audit run `36353448299` succeeded.
- Native exact-SHA merge-async UUID
  `fb0a55fb-72e3-4850-a984-8bb3993c2585` was read back as enqueued. Actual
  merge_group run `36353527506` used base
  `bab42d5c9900b05d9a7b4ff941df1fb60d2ea760`, head
  `1b903f64af569ef85c6fa54d5cd305de8e57a878`, one commit and exactly the five
  source blobs. Aggregate `game-gate` succeeded.
- PR #1031 merged at `2026-09-27T22:07:06Z`. Protected `main` read back as
  `1b903f64af569ef85c6fa54d5cd305de8e57a878`; all five protected blobs matched
  the frozen source. Terminal source proof is Issue #162 comment `5860279548`
  and aggregate #513 comment `5860279705`.
- The separate two-path archive allocation is Issue #162 comment `5860284061`.
  This closeout changes only the task location. Its own PR/head/Merge Queue proof
  remains control-plane output and is not self-referential evidence in this file.

The completed child remains **UNREGISTERED/OFFLINE** evidence. Aggregate #513,
Jira KAN-12 and playable Combat remain open. This archive grants no further
implementation, registration, runtime, SQL, production or external authority.

## Open downstream gates

B3 legal Content/Item/loot/inventory product admission, B4 registered
event/profile/resource binding, separate Character destination/global-revision
readiness, C physical PostgreSQL MINT then TRANSFER, D generic
death/loot/reward orchestration, E existing-room Server Seam/protocol/native
client composition, F restart/native-desktop qualification, and playable Combat
remain open.

R7 remains a separate XP/reference lifecycle under its own lease and branch.
No R7 path, lease, branch, manifest or implementation is modified by this task.
The existing native entry room is the later composition target; this task
creates no second room or harness.

## Context checkpoint

```yaml
last_progress: "PR #1031 merged through real Merge Queue and protected-main five-blob readback passed; record archived"
status: completed
branch: codex/dur03-native-one-item-audit-evidence-513
head_sha: 0e5bf0268b3efdad2622364637669ab544289e07
pr: 1031
final_head_sha: 0e5bf0268b3efdad2622364637669ab544289e07
final_head_frozen_at: 2026-09-27T21:29:30Z
ci_trigger_source: pull_request and ready_for_review
ci_check_generation: 36353448299
ci_checks_for_current_head: 2
ci_run_ids: [36352506400, 36353448299]
ci_job_ids: []
runner_assignment_state: success
merge_group_run_id: 36353527506
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: continue aggregate #513 through separately allocated B3/B4 and later production/playable gates
```
