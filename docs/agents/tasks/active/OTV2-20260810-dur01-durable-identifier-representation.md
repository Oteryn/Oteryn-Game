# OTV2-20260810-dur01-durable-identifier-representation

```yaml
task_id: OTV2-20260810-dur01-durable-identifier-representation
title: DUR-01 durable identifier representation
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/dur01-durable-identifier-representation
issue: 111
pr: 133
trusted_base_sha: adb0882a5ddbe42944fe955f5effb78fd5495422
owner: GPT-5.6 Sol architecture continuation session
created_at: 2026-08-10T13:56:00+02:00
updated_at: 2026-08-10T14:20:00+02:00
repair_cycles_for_current_gate: 2
max_repair_cycles_for_current_gate: 3
blocker: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260810-dur01-durable-identifier-representation.md
  - docs/architecture/DUR-01_DURABLE_IDENTIFIER_REPRESENTATION_ANALYSIS.md
  - docs/architecture/DUR-01_DURABLE_IDENTIFIER_REPRESENTATION_CONTRACT.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
```

## Goal

Close the Stage-B `DUR-01` architecture gate after accepted/lifecycle-closed FND-04. Freeze only the durable/database identity representation required before persistence and item/economy contracts can safely design authoritative durable state.

## Delivery

- Issue: #111
- PR: #133
- trusted base: `main@adb0882a5ddbe42944fe955f5effb78fd5495422`
- exact final head will be recorded on immutable PR review evidence after the final self-review; no self-referential SHA commit.

## Normative accepted inputs

- `FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`;
- ADR-0004 PostgreSQL and separate Platform/game data ownership;
- ADR-0006 Game Intelligence analytics/audit separation;
- accepted FND-02 protocol semantics;
- accepted FND-03 runtime/fencing semantics;
- accepted/lifecycle-closed FND-04 A/B/C contracts;
- Platform ADR 0028: canonical native `AccountId` is Platform-issued strongly typed UUIDv7, full 128 bits;
- Platform ADR 0029: canonical native `WorldId`/`ChannelId` are Platform-issued UUIDv7 and ChannelId remains WorldId-scoped semantically.

These inputs are consumed, not reopened.

## Owned decisions

DUR-01 owns:

- exact PostgreSQL physical representation of accepted durable UUID identities;
- lossless physical persistence of FND-02 `CommandId` when durable state requires it, without changing its semantics/scope;
- Rust/persistence strong-typing and canonicalization obligations;
- nil/absence rules;
- scoped composite persistence rules;
- identity versus revision/generation/order separation;
- minimum new durable-domain identity needed by DUR-03 (`ItemInstanceId`);
- internal foreign-key and cross-database boundary rules;
- legacy/import anti-corruption mapping rules;
- privacy/public-reference boundary for UUIDv7 exposure;
- representation evolution and migration compatibility;
- deterministic representation evidence required before implementation claims.

## Explicit exclusions

This task does **not** authorize or select:

- concrete table layout, migration SQL/tooling, transaction isolation, row/advisory locks, retry policy, outbox/checkpoint/RPO/RTO (`DUR-02`);
- item movement/split/merge/conservation/anti-dup transaction semantics (`DUR-03`);
- event/audit envelope IDs such as EventId/OperationId/TransactionId/CorrelationId/CausationId/AnalyticsActorId (`ANL-01`, with DUR integration where required);
- protocol wire/CommandId semantic changes (`FND-02`);
- runtime-local handles/fencing (`FND-03`);
- session/admission/lease semantics (`FND-04`);
- Platform database migration or Platform repository writes;
- application/runtime implementation, PostgreSQL schema creation, deployment or production mutation.

## Repair history

### Cycle 1 — stable legacy identity key versus snapshot provenance

Initial candidate used `(source_system, source_revision_or_namespace, source_entity_kind, legacy_identifier)` as the conceptual legacy mapping key. That allowed a later snapshot/revision of the same legacy entity to appear as a new key and potentially receive a second native identity.

Repair: stable mapping key is `(source_system, source_namespace, source_entity_kind, legacy_identifier)`. Source revision/snapshot/hash, import-run identity and migration classification are provenance, not mapping identity. A later snapshot of the same stable source key must reuse the same native identity; revision changes alone cannot mint a second identity.

### Cycle 2 — full-range CommandId persistence

Review found that the initial candidate separated CommandId semantically from UUID identity but failed to freeze its physical persistence when DUR-02 needs durable deduplication/recovery. FND-02 defines CommandId as non-zero `uint64`; PostgreSQL `bigint` is signed and cannot preserve the full legal range.

Repair: persisted CommandId uses PostgreSQL `numeric(20,0)` constrained to `1..18446744073709551615`, and command identity always remains `(GameSessionId, CommandId)`. Signed bigint, floating point, textual canonical storage, offset/xor remapping, truncation and globally unique CommandId semantics are rejected. Boundary fixtures around `INT64_MAX`/`UINT64_MAX` are mandatory for a later persistence implementation.

## Final acceptance plan

1. Final self-review against the exact four-path candidate; max repair cycles 3, current usage `2/3`.
2. If a third material issue exists, repair it once and freeze; any later material finding blocks this gate.
3. Exact-head Agent Governance, Dependency Review and CodeQL must PASS.
4. Zero unresolved material review threads and terminal exact-head review with zero material findings are required.
5. Squash merge only on unchanged accepted head.
6. Separate lifecycle closeout/archive closes Issue #111 and releases ownership.

Runtime/component/browser E2E is `NOT_APPLICABLE` for this architecture-only gate; later physical persistence implementation requires the database integration evidence defined by the contract.