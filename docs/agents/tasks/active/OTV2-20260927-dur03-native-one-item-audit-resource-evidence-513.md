# OTV2-20260927-dur03-native-one-item-audit-resource-evidence-513

```yaml
task_id: OTV2-20260927-dur03-native-one-item-audit-resource-evidence-513
title: DUR-03 native one-item audit schema codec and resource evidence
mode: AUDIT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/dur03-native-one-item-audit-evidence-513
issue: 162
pr: 1031
base_sha: e422fc9d2962f63ccb6f0af9ee80ec5f449c2a31
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: bounded Luna author with root OTV2_WORK_DELIVERY_COORDINATOR control plane
created_at: 2026-09-27T22:42:31+02:00
updated_at: 2026-09-27T23:28:05+02:00
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
- [ ] Create the canonical draft PR, bind its number here, freeze the successor
  exact head, and obtain fresh exact-head independent review and hosted CI.
- [ ] Integrate only through native exact-SHA Merge Queue and verify the actual
  terminal merge group plus protected-main five-blob readback.
- [ ] Archive this task through a separate lifecycle allocation after source
  integration.

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
