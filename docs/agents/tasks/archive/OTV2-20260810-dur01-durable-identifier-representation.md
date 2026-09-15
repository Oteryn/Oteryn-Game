# OTV2-20260810-dur01-durable-identifier-representation

```yaml
task_id: OTV2-20260810-dur01-durable-identifier-representation
title: DUR-01 durable identifier representation
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
delivery_issue: 111
delivery_pr: 133
closeout_pr: 134
trusted_base_sha: adb0882a5ddbe42944fe955f5effb78fd5495422
final_delivery_head_sha: 1a431b4e0667a4a218ea15ed7152a7e979a1b072
delivery_merge_sha: bc172d7244f823425feb84c46c7d04be1f942eed
terminal_review_id: 4896597698
repair_cycles_used: 2
max_repair_cycles: 3
runtime_e2e: NOT_APPLICABLE
```

## Completion summary

DUR-01 is accepted as an architecture-only durable representation contract. It does not implement PostgreSQL schema/runtime behavior.

Canonical delivered artifacts:

- `docs/architecture/DUR-01_DURABLE_IDENTIFIER_REPRESENTATION_ANALYSIS.md`;
- `docs/architecture/DUR-01_DURABLE_IDENTIFIER_REPRESENTATION_CONTRACT.md`;
- transition-safe `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`.

## Accepted decisions

- native UUIDv7-backed durable identities use PostgreSQL native `uuid`, preserving all 128 bits;
- Platform canonical AccountId/WorldId/ChannelId are consumed literally under Platform ADR 0028/0029 and are never game-side re-keyed from local integer/Canary identities;
- semantic strong types remain distinct even though the DB scalar is shared;
- nil UUID is invalid and semantic absence uses NULL/typed absence;
- scoped references preserve WorldId + component semantics;
- UUIDv7 ordering is not authority, causality, fencing or business chronology;
- persisted FND-02 CommandId uses PostgreSQL `numeric(20,0)` with complete non-zero uint64 range and remains scoped by GameSessionId;
- `ItemInstanceId` is a game-owned global UUIDv7 identity for one concrete item-instance lifecycle;
- generic catch-all entity identity is rejected;
- event/audit IDs remain ANL-01-owned;
- no cross-database Platform/game FK is authority proof;
- legacy imports use stable source namespace identity mapping with snapshot/revision/import-run as provenance;
- internal UUIDv7 identifiers are not automatically public identifiers;
- future representation changes require explicit lossless migration and compatibility evidence.

## Repair history

### Cycle 1 — stable legacy mapping identity

The initial mapping key could include source revision and therefore risk creating a second native identity for the same legacy entity in a later snapshot. The final contract separates stable source namespace identity from mutable revision/snapshot provenance.

### Cycle 2 — full-range CommandId persistence

The initial candidate did not freeze the durable physical scalar for FND-02 CommandId. PostgreSQL `bigint` cannot represent full uint64. Final contract uses `numeric(20,0)` over `1..18446744073709551615` and preserves `(GameSessionId, CommandId)` identity.

No third repair cycle was consumed; final review found zero material defects.

## Exact-head acceptance evidence

Final head: `1a431b4e0667a4a218ea15ed7152a7e979a1b072`.

- Agent Governance run `31387230160`: PASS;
- Dependency Review run `31387230532`: PASS;
- CodeQL run `31387230664`: PASS;
- terminal architecture/security/data-integrity review `4896597698`: PASS, zero material findings;
- unresolved review threads: 0;
- delivery PR #133 squash merge: `bc172d7244f823425feb84c46c7d04be1f942eed`.

## Explicitly not implemented

This delivery did not create or authorize:

- PostgreSQL schema/table/migration code;
- ORM/query/persistence implementation;
- transaction/locking/retry/outbox/checkpoint/RPO/RTO policy;
- item movement/conservation/anti-duplication code;
- protocol/runtime/admission changes;
- Platform writes/migrations;
- deployment or production traffic.

## Downstream handoff

- `ANL-01` is the next missing semantic gate and must define event/audit identifiers/envelope before DUR-02/DUR-03 finalize atomic audit/outbox evidence.
- `DUR-02` consumes DUR-01 representation for persistence v1.
- `DUR-03` consumes `ItemInstanceId` for item transaction/conservation design.
- `PROD-ENTITLEMENTS-01` remains independently blocked by `Oteryn-Platform#944`.

Lifecycle ownership is released only when closeout PR #134 carrying this archive and the final current-status transition merges.