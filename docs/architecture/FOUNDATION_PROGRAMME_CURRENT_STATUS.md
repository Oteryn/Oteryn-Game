# Oteryn v2 Foundation Programme — Current Status

- Status: Canonical current execution-status overlay
- Date: 2026-08-10
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: accepted foundation progression and next ordered architecture gates
- FND-04 lifecycle closeout merge: `adb0882a5ddbe42944fe955f5effb78fd5495422`
- DUR-01 lifecycle closeout merge: `ef42fa47ab054ab8aa304c017307c1945f931b59`
- Active Stage-B delivery: Issue #135 (`ANL-01`)
- Current phase represented by this delivery candidate: `ANL-01 FINAL DELIVERY / CLOSEOUT_PENDING`

## 1. Authority of this overlay

This document answers what is accepted now and what may happen next. Detailed review/CI/repair evidence lives in accepted contracts, archived task records and merged PRs.

Older backlog/register prose that describes completed FND/DUR gates as live is historical execution narrative. Accepted contracts plus this overlay govern current progression. Stable decision IDs and future dependency requirements remain valid unless explicitly superseded.

`ANL-01 FINAL DELIVERY / CLOSEOUT_PENDING` is transition-safe: before delivery merge it means candidate validation is in progress; after delivery merge it means only separate lifecycle archive/ownership release remains. ANL-01 becomes `ACCEPTED AND LIFECYCLE-CLOSED` only through that closeout.

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
| `ANL-01` | `FINAL DELIVERY / CLOSEOUT_PENDING` | Issue #135; event/audit foundation candidate |
| `DUR-02` | `BLOCKED UNTIL ANL-01 CLOSEOUT` | DUR-01 satisfied; Persistence v1 needs accepted event/audit semantics |
| `DUR-03` | `BLOCKED ON ANL-01 + DUR-02 INTEGRATION` | ItemInstanceId satisfied; anti-duplication still needs persistence/evidence integration |
| `DUR-04` | `QUEUED / INDEPENDENT` | content/world/scripting architecture |
| `GAME-VISION-01` | `OPEN PRODUCT GATE` | blocks broad gameplay/content production |

## 3. Accepted baseline preserved

FND-02 retains TLS/protobuf gameplay protocol semantics, GameSession-scoped nonzero uint64 CommandId, server sequencing/revisions and reconciliation.

FND-03 retains one logical authoritative mutation owner per channel/instance, separate ownership generation, RuntimeExecutionOrdinal, bounded queues, fail-closed stale work and measured capacity requirements.

FND-04 remains accepted/closed with ownership-before-world admission, purpose-separated grant profiles, anti-rollback security evidence, PREPARE/COMMIT reconnect, healthy-binding non-preemption, ControlLossEpoch, exactly 4 seconds eligible defensive PvE re-entry protection and fail-closed recovery.

DUR-01 remains accepted/lifecycle-closed: UUIDv7 native durability uses PostgreSQL `uuid`, persisted CommandId preserves full uint64 via `numeric(20,0)`, ItemInstanceId is a game-owned UUIDv7 identity, legacy imports use stable source namespace identity, and internal IDs are not automatically public.

## 4. ANL-01 candidate foundation

Canonical candidate artifacts:

- `docs/architecture/ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_ANALYSIS.md`;
- `docs/architecture/ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md`;
- `docs/contracts/game-events/v1/foundation.proto`;
- `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json`;
- ANL-owned entries in `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`.

Candidate decisions:

- common game-event interchange uses protobuf/proto3, independent from broker/database/warehouse product;
- EventId, OperationId, TransactionId and CorrelationId are strongly typed UUIDv7 identities with separate owners/lifecycles;
- causation is a typed `CausationRef` to an existing Event/Command/Operation/Transaction identity rather than a newly minted causation UUID;
- AnalyticsActorId is purpose/domain + epoch scoped UUIDv7 pseudonymous identity with protected audited mapping to operational identity;
- only `BEST_EFFORT_TELEMETRY` and `DURABLE_AUDIT` are game-event durability classes; operational observability remains separate;
- durable mutation evidence commits atomically with the owning mutation under later DUR-02/DUR-03 physical semantics;
- publication is at-least-once after commit, EventId-stable and consumer-idempotent; replay never replays gameplay mutation;
- no global event total order: use CommandRef, owner-local RuntimeExecutionOrdinal, TransactionId + transaction_event_ordinal, CausationRef and domain revisions according to their scopes;
- event type IDs and protobuf fields are stable/non-reused with explicit schema revisions;
- privacy classes separate internal non-personal, pseudonymous analytical, restricted player-linked and security-sensitive data;
- every production event family must bind an accepted finite retention/purpose/access profile; ordinary unlimited retention is forbidden;
- raw player IDs cannot silently leak into pseudonymous families;
- high-cardinality event/player/item/session identities remain forbidden as ordinary Prometheus labels;
- hard event/queue/batch/replay/query/export ceilings are security boundaries, not throughput promises.

This candidate creates no event table, outbox, broker, runtime collector, detector or production collection.

## 5. Failure integration

ANL-01 candidate semantically closes:

- `FS-ANALYTICS-TELEMETRY-OVERFLOW`;
- `FS-AUDIT-OUTBOX-BACKLOG` at evidence/publication semantics, with physical backlog DUR-02-owned;
- `FS-EVENT-DUPLICATE-DELIVERY`;
- `FS-EVENT-OUT-OF-ORDER`;
- `FS-AUDIT-MUTATION-MISMATCH` at atomic semantic boundary, with physical proof DUR-02/DUR-03-owned;
- `FS-ANALYTICS-PRIVACY-POLICY`;
- `FS-DB-OUTBOX-BOUNDARY` at semantic boundary.

Detector false positives remain ANL-03 and investigation mutation resistance remains ANL-04 implementation evidence. ADR-0006 already prohibits automatic sanctions/mutations.

## 6. Security/privacy boundary

- event identity/correlation never grants gameplay authority;
- no credentials/proofs/private keys in events;
- pseudonymization never falls back to raw identity;
- restricted mapping access is audited;
- production collection is blocked if purpose/privacy/retention/access profile is unresolved;
- durable audit never silently downgrades to best effort;
- committed durable backlog is not discarded to satisfy in-memory capacity;
- investigation/AI remains read-only and cannot ban, balance, rollback, deploy or mutate authoritative state.

## 7. Runtime/implementation status

ANL-01 architecture does **not** authorize:

- runtime event collector implementation;
- PostgreSQL table/outbox/checkpoint/migration work;
- transaction isolation/locking/RPO/RTO;
- broker/stream/warehouse/lake/dashboard selection or deployment;
- item/currency transaction implementation;
- balance/security detector implementation;
- investigation/AI write access;
- Platform writes;
- production analytics collection.

## 8. Next ordered architecture work

After ANL-01 delivery **and lifecycle closeout**:

1. `DUR-02 — Persistence v1` becomes the next direct persistence gate and consumes accepted DUR-01 + ANL-01.
2. `DUR-03 — Item Transaction and Anti-Duplication Invariants` consumes ItemInstanceId plus DUR-02 atomic transactions and ANL-01 durable audit semantics.
3. `DUR-04 — Content, World Detail and Scripting` remains independent durable-content architecture.
4. `GAME-VISION-01` still blocks broad gameplay/content production.
5. ANL-02/03/04 refine consumers/detectors/investigation after the foundation.

`PROD-ENTITLEMENTS-01` remains independently blocked by open P1 Oteryn-Platform#944 and is not part of ANL-01.

## 9. Concise current rule

```text
FND-01 .. FND-04
-> accepted / closed as applicable

DUR-01
-> ACCEPTED AND LIFECYCLE-CLOSED

ANL-01
-> FINAL DELIVERY / CLOSEOUT_PENDING
-> event/audit architecture only

DUR-02
-> waits for accepted + lifecycle-closed ANL-01

DUR-03
-> waits for ANL-01 + DUR-02 integration

runtime / production analytics
-> still not authorized
```
