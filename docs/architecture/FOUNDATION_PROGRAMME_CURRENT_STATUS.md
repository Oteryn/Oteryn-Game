# Oteryn v2 Foundation Programme — Current Status

- Status: Canonical current execution-status overlay
- Date: 2026-08-10
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: accepted foundation progression and next ordered architecture gates
- FND-04 lifecycle closeout merge: `adb0882a5ddbe42944fe955f5effb78fd5495422`
- DUR-01 lifecycle closeout merge: `ef42fa47ab054ab8aa304c017307c1945f931b59`
- ANL-01 delivery merge: `af2fa495c1126080ffc1d0717b7d0ef54f6b29ca`
- Current phase represented by this closeout candidate: `ANL-01 ACCEPTED AND LIFECYCLE-CLOSED / NEXT DUR-02`

## 1. Authority of this overlay

This document answers what is accepted now and what may happen next. Detailed review/CI/repair evidence lives in accepted contracts, archived task records and merged PRs.

Older backlog/register prose that describes completed FND/DUR/ANL gates as live is historical execution narrative. Accepted contracts plus this overlay govern current progression. Stable decision IDs and future dependency requirements remain valid unless explicitly superseded.

No status row implies runtime implementation or production activation.

## 2. Foundation and Stage-B progression

| Gate | Current status | Canonical evidence / note |
|---|---|---|
| `FND-01` | `ACCEPTED AND APPLIED` | workspace/dependency contract + canonical Rust cutover |
| `VSL-02` | `ACCEPTED AND COMPLETE` | client migration/cutover complete |
| `FND-ID-01` | `ACCEPTED` | semantic identity contract |
| `FND-02` | `ACCEPTED` | `protocol-oteryn` v1 architecture; implementation separately gated |
| `FND-03` | `ACCEPTED` | authoritative runtime execution architecture |
| `FND-04A/B/C` | `ACCEPTED AND LIFECYCLE-CLOSED` | admission + reconnect/recovery + integration |
| `FND-04` overall | `ACCEPTED AND CLOSED` | programme #112 complete |
| `DUR-01` | `ACCEPTED AND LIFECYCLE-CLOSED` | durable representation + ItemInstanceId; closeout merge `ef42fa47ab054ab8aa304c017307c1945f931b59` |
| `ANL-01` | `ACCEPTED AND LIFECYCLE-CLOSED` | event/audit foundation; delivery PR #141 merge `af2fa495c1126080ffc1d0717b7d0ef54f6b29ca`; lifecycle ownership released by this closeout |
| `DUR-02` | `NEXT DIRECT PERSISTENCE GATE` | DUR-01 + ANL-01 semantic prerequisites satisfied; Persistence v1 remains architecture-only until separately authorized implementation |
| `DUR-03` | `BLOCKED ON DUR-02 INTEGRATION` | ItemInstanceId and ANL-01 evidence semantics satisfied; anti-duplication still needs accepted persistence transaction boundaries |
| `DUR-04` | `QUEUED / INDEPENDENT` | content/world/scripting architecture |
| `GAME-VISION-01` | `OPEN PRODUCT GATE` | blocks broad gameplay/content production |

## 3. Accepted baseline preserved

FND-02 retains TLS/protobuf gameplay protocol semantics, GameSession-scoped nonzero uint64 CommandId, server sequencing/revisions and reconciliation.

FND-03 retains one logical authoritative mutation owner per channel/instance, separate ownership generation, owner-scoped RuntimeExecutionOrdinal, bounded queues, fail-closed stale work and measured capacity requirements.

FND-04 remains accepted/closed with ownership-before-world admission, purpose-separated grant profiles, anti-rollback security evidence, PREPARE/COMMIT reconnect, healthy-binding non-preemption, ControlLossEpoch, exactly 4 seconds eligible defensive PvE re-entry protection and fail-closed recovery.

DUR-01 remains accepted/lifecycle-closed: UUIDv7 native durability uses PostgreSQL `uuid`, persisted CommandId preserves full uint64 via `numeric(20,0)`, ItemInstanceId is a game-owned UUIDv7 identity, legacy imports use stable source namespace identity, and internal IDs are not automatically public.

## 4. Accepted ANL-01 foundation

Canonical artifacts:

- `docs/architecture/ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_ANALYSIS.md`;
- `docs/architecture/ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md`;
- `docs/contracts/game-events/v1/foundation.proto`;
- `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json`;
- ANL-owned entries in `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`.

Accepted decisions include:

- common `oteryn-game-events` interchange uses protobuf/proto3, independent from broker/database/warehouse product;
- EventId, OperationId, TransactionId and CorrelationId are strongly typed UUIDv7 identities with distinct owners/lifecycles;
- immediate causation is typed `CausationRef` to Event/Command/Operation/Transaction rather than a separately minted causation UUID;
- AnalyticsActorId is purpose/domain + epoch scoped pseudonymous UUIDv7 and the same operational actor receives a fresh pseudonym each epoch;
- only `BEST_EFFORT_TELEMETRY` and `DURABLE_AUDIT` are game-event durability classes; operational observability remains separate;
- same EventId fixes all semantic envelope values plus exact payload bytes across retry/redelivery; protobuf decode/re-serialize is not treated as canonical semantic byte identity;
- `RuntimeOrderRef` binds RuntimeExecutionOrdinal to scope ownership generation plus explicit channel/instance scope;
- `TransactionEventRef` atomically carries TransactionId + ordinal + event count, allowing deterministic complete-set/gap/duplicate validation;
- no global event total order is invented: command, runtime, transaction, causation and domain revision scopes remain separate;
- mandatory durable mutation evidence commits atomically with the owning mutation under downstream DUR-02/DUR-03 physical mechanics;
- durable publication is at-least-once, EventId-stable and consumer-idempotent; replay never replays gameplay mutation;
- event type/schema IDs are stable/non-reused with explicit compatibility rules;
- privacy classes separate internal non-personal, pseudonymous analytical, restricted player-linked and security-sensitive data;
- every production event family requires an accepted purpose/privacy/access profile with finite ordinary retention; ordinary unlimited retention is forbidden;
- raw player identities cannot silently leak into pseudonymous families;
- high-cardinality event/player/item/session identities are not ordinary Prometheus labels;
- ANL event/queue/batch/replay/query/export limits are absolute security ceilings, not throughput promises;
- committed durable audit backlog is never discarded merely to satisfy in-memory capacity.

ANL-01 acceptance creates no event table, outbox implementation, broker, runtime collector, detector, warehouse or production collection.

## 5. ANL-01 delivery evidence

- final PR #141 head: `b398d8866ad8a8abb74ffc8f9801252573993924`;
- Agent Governance `31390651358`: PASS;
- Dependency Review `31390651373`: PASS;
- CodeQL `31390651366`: PASS;
- terminal architecture/security/privacy/data-integrity review `4896985694`: PASS, zero material findings;
- unresolved material review threads: 0;
- repair budget used: `2/3`;
- squash delivery merge: `af2fa495c1126080ffc1d0717b7d0ef54f6b29ca`;
- runtime/component/browser E2E: `NOT_APPLICABLE`.

## 6. Failure and privacy integration

ANL-01 semantically closes telemetry overflow, durable audit backlog/publication, duplicate delivery, out-of-order events, mutation/audit mismatch, privacy-policy and DB/outbox boundary scenarios at its owning layer. Physical PostgreSQL proofs remain DUR-02/DUR-03-owned. Detector false positives remain ANL-03 and investigation mutation resistance remains ANL-04 implementation evidence.

Game Intelligence remains observational/investigative. It cannot autonomously ban, sanction, mutate gameplay/database state, balance, rollback or deploy.

Production collection fails closed when an event family lacks accepted purpose/privacy/finite-retention/access policy. Pseudonymization never falls back to raw identity, and privileged pseudonym mapping access is audited.

## 7. Runtime/implementation status

Accepted FND/DUR-01/ANL-01 architecture does **not** authorize:

- runtime event collector implementation;
- PostgreSQL table/outbox/checkpoint/migration implementation;
- transaction isolation/locking/retry/RPO/RTO implementation;
- item/currency transaction implementation;
- broker/stream/warehouse/lake/dashboard selection or deployment;
- balance/security detector implementation;
- investigation/AI write authority;
- Platform migrations/writes;
- production analytics collection;
- gameplay runtime/deployment/traffic activation.

## 8. Next ordered architecture work

The immediate dependency chain is now:

1. `DUR-02 — Persistence v1` — next direct persistence architecture gate; consumes accepted DUR-01 + ANL-01 to freeze schema/migrations/transactions/fencing/checkpoints/outbox/backup/restore/RPO/RTO semantics.
2. `DUR-03 — Item Transaction and Anti-Duplication Invariants` — consumes ItemInstanceId, accepted DUR-02 atomic transaction boundaries and ANL-01 durable evidence semantics.
3. `DUR-04 — Content, World Detail and Scripting` — independent durable-content architecture.
4. `GAME-VISION-01` — still required before broad gameplay/content production.
5. `ANL-02/03/04` — downstream analytical consumers, integrity/security detection and read-only investigation.

`PROD-ENTITLEMENTS-01` remains independently blocked by open P1 `Oteryn-Platform#944`; ANL-01 does not change that dependency.

## 9. Concise current rule

```text
FND-01 .. FND-04
-> accepted / closed as applicable

DUR-01
-> ACCEPTED AND LIFECYCLE-CLOSED

ANL-01
-> ACCEPTED AND LIFECYCLE-CLOSED
-> event/audit architecture only

DUR-02
-> NEXT DIRECT PERSISTENCE GATE

DUR-03
-> waits for DUR-02 integration

runtime / production analytics
-> still not authorized
```
