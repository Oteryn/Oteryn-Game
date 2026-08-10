# Oteryn v2 Foundation Programme — Current Status

- Status: Canonical current execution-status overlay
- Date: 2026-08-10
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: accepted foundation progression and next ordered architecture gates
- FND-04 lifecycle closeout merge: `adb0882a5ddbe42944fe955f5effb78fd5495422`
- DUR-01 lifecycle closeout merge: `ef42fa47ab054ab8aa304c017307c1945f931b59`
- ANL-01 delivery merge: `af2fa495c1126080ffc1d0717b7d0ef54f6b29ca`
- Current phase: `ANL-01 ACCEPTED + LIFECYCLE_CLOSED + NOT_STARTED runtime / DUR-02 discovery next / product-channel refinements planned`

## 1. Authority of this overlay

This document answers what is accepted now and what may happen next. Detailed review/CI/repair evidence lives in accepted contracts, archived task records and merged PRs.

Older backlog/register prose that describes completed FND/DUR/ANL gates as live is historical execution narrative. Accepted contracts plus this overlay govern current progression. Stable decision IDs and future dependency requirements remain valid unless explicitly superseded.

`docs/architecture/ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-10.md` adds owner-accepted programme ordering and product/operations refinements. `docs/architecture/ADR-0014-dual-gameplay-transport-tcp-default-quic-opt-in.md` accepts the long-term dual-transport strategy but explicitly preserves TCP transport profile `1` as the only currently registered/authoritative gameplay transport. QUIC player admission remains blocked until `PROTOCOL_OTERYN_V1_REGISTRY.json` and both FND-04 fresh/recovery grant profiles are reconciled by a later accepted delivery. The application protocol and all security/sequencing/fencing semantics remain one `protocol-oteryn` contract.

`ARCHITECTURE_STATUS_MODEL.md` is normative for current status presentation. Every row below separates `DecisionStatus`, `DeliveryStatus` and `ImplementationStatus`; one axis never implies another. `PLANNED` means the gate is registered for future work but no active task/PR currently owns its delivery; `OPEN` is reserved for a concrete active delivery or unresolved lifecycle record.

## 2. Foundation and Stage-B progression

| Gate | DecisionStatus | DeliveryStatus | ImplementationStatus | Canonical evidence / note |
|---|---|---|---|---|
| `FND-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `PROVEN` | workspace/dependency contract is applied and machine-enforced; canonical Rust cutover evidence exists |
| `VSL-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `PROVEN` | native client migration/cutover completed and source/destination lifecycle closed |
| `FND-ID-01` | `ACCEPTED` | `OPEN` | `NOT_STARTED` | semantic identity architecture is accepted, but legacy FND-ID task records still remain under `docs/agents/tasks/active/`; lifecycle cleanup/ownership reconciliation must complete before this row can become `LIFECYCLE_CLOSED`; no separate runtime implementation claim |
| `FND-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | `protocol-oteryn` v1 architecture accepted and its task lifecycle is archived; current registered transport remains TCP profile `1`; production protocol adapter remains separately gated |
| `NET-TRANSPORT-01` | `ACCEPTED` | `IN_REVIEW` | `NOT_STARTED` | dual-transport strategy delivery PR #145; TCP profile `1` remains current, QUIC activation blocked pending profile/grant/evidence gates |
| `FND-03` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | authoritative runtime execution architecture accepted and its task lifecycle is archived; GameNode runtime implementation not claimed |
| `FND-04A/B/C` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | admission + reconnect/recovery + integration architecture accepted; runtime implementation separately gated |
| `FND-04` overall | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | programme #112 architecture lifecycle complete; no gameplay admission runtime claimed |
| `DUR-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | durable representation + ItemInstanceId accepted; physical PostgreSQL implementation is downstream |
| `ANL-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | event/audit foundation accepted; no runtime event collector/outbox/broker/warehouse implementation |
| `GAME-VISION-01` | `PROPOSED` | `PLANNED` | `NOT_STARTED` | near-term product gate before broad gameplay/content and product-sensitive persistence semantics |
| `GAME-CHANNEL-01` | `PROPOSED` | `PLANNED` | `NOT_STARTED` | social/economic/PvP/UX channel policy required before multichannel becomes a product feature |
| `GAME-CHAR-01` | `PROPOSED` | `PLANNED` | `NOT_STARTED` | blocks final character-bearing DUR-02 semantics |
| `GAME-ITEM-01` | `PROPOSED` | `PLANNED` | `NOT_STARTED` | blocks final DUR-03 item transaction semantics |
| `SIM-DETERMINISM-01` | `PROPOSED` | `PLANNED` | `NOT_STARTED` | required before broad combat/AI formula freeze |
| `DUR-02` | `PROPOSED` | `PLANNED` | `NOT_STARTED` | persistence discovery may start from DUR-01 + ANL-01; final character-bearing schema also requires GAME-CHAR-01 |
| `DUR-03` | `PROPOSED` | `PLANNED` | `NOT_STARTED` | waits for accepted DUR-02 + GAME-ITEM-01 + ANL-01 evidence semantics |
| `DUR-04` | `PROPOSED` | `PLANNED` | `NOT_STARTED` | content/world/scripting architecture; minimum headless schema/validator/compiler/bundle/loader precedes full Studio |

The `ImplementationStatus` column describes only the scope of the named gate. `NOT_STARTED` does not mean adjacent repository code is absent; it means this overlay does not claim the gate's production/runtime behavior as implemented. `PROVEN` is used only where named migration/workspace evidence already exists.

## 3. Accepted baseline preserved

FND-02 retains one `protocol-oteryn` application protocol, TLS/protobuf gameplay semantics, GameSession-scoped nonzero uint64 CommandId, server sequencing/revisions, reconciliation, bounded inputs and fail-closed compatibility/security behavior. The current accepted registry contains TCP+TLS 1.3 transport profile `1` only. ADR-0014 refines future transport direction, not current admission compatibility: QUIC remains blocked until a stable QUIC transport profile and matching FND-04 fresh/recovery grant semantics are accepted. FND-02's measured-benefit prerequisite remains binding. Any future QUIC adapter must preserve identical application/security authority and visible ordering.

FND-03 retains one logical authoritative mutation owner per channel/instance, separate ownership generation, owner-scoped RuntimeExecutionOrdinal, bounded queues, fail-closed stale work and measured capacity requirements. The first GameNode implementation should remain a domain-modular monolith until real deployment/security/data/failure boundaries justify separation.

FND-04 remains accepted/lifecycle-closed with ownership-before-world admission, purpose-separated grant profiles, anti-rollback security evidence, PREPARE/COMMIT reconnect, healthy-binding non-preemption, ControlLossEpoch, exactly 4 seconds eligible defensive PvE re-entry protection and fail-closed recovery. Any future change to the four-second value requires explicit game-design evidence and superseding policy rather than hidden configuration drift.

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

## 6. Failure, privacy and operator integration

ANL-01 semantically closes telemetry overflow, durable audit backlog/publication, duplicate delivery, out-of-order events, mutation/audit mismatch, privacy-policy and DB/outbox boundary scenarios at its owning layer. Physical PostgreSQL proofs remain DUR-02/DUR-03-owned. Detector false positives remain ANL-03 and investigation mutation resistance remains ANL-04 implementation evidence.

Game Intelligence remains observational/investigative. It cannot autonomously ban, sanction, mutate gameplay/database state, balance, rollback or deploy.

Production collection fails closed when an event family lacks accepted purpose/privacy/finite-retention/access policy. Pseudonymization never falls back to raw identity, and privileged pseudonym mapping access is audited.

Before external alpha, operator/GM mutations must use typed, RBAC-controlled, idempotent and audited commands rather than ad-hoc raw SQL. High-risk identity/economy operations may require dual control. Compensation for confirmed server-caused incidents must use the same audited domain mechanisms.

## 7. Runtime/implementation status

Accepted FND/DUR-01/ANL-01/NET-TRANSPORT-01 architecture does **not** authorize:

- TCP or QUIC gameplay adapter/listener implementation;
- a QUIC transport profile, QUIC admission/recovery, functional player QUIC option, QUIC library selection, 0-RTT or DATAGRAM activation;
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

The native client therefore remains legitimately pre-native-protocol until a separately authorized implementation task proves the transport/session/runtime path.

## 8. Next ordered architecture and proof work

The immediate programme is refined to avoid freezing persistence/gameplay from technical schemas before product semantics:

1. `GAME-VISION-01` minimum launch/product baseline and `GAME-CHANNEL-01` channel semantics may proceed in parallel with bounded persistence discovery.
2. `GAME-CHAR-01` — accept character lifecycle/progression semantics before final character-bearing `DUR-02` schema.
3. `DUR-02 — Persistence v1` — discovery may start now from accepted DUR-01 + ANL-01; final character schema waits for GAME-CHAR-01.
4. `GAME-ITEM-01` — accept item model/equipment/container/transform semantics.
5. `DUR-03 — Item Transaction and Anti-Duplication Invariants` — consumes accepted DUR-02 + GAME-ITEM-01 + ANL-01 evidence semantics.
6. `DUR-04` minimum headless content path — schema -> validator -> deterministic compiler -> bundle -> loader; full Studio remains downstream.
7. `SIM-DETERMINISM-01` — freeze authoritative arithmetic/replay requirements before broad combat/AI implementation.
8. `NET-TRANSPORT-02` (or an equivalent bounded successor) — register QUIC transport profile, reconcile FND-04 fresh/recovery transport bindings and prove FND-02 ordered-lane/snapshot semantics before functional player QUIC admission.
9. Implement the umbrella `VSL-01` as ordered real-boundary slices: admission, movement, combat, persistence, recovery, then multichannel.
10. Establish minimal admin/security/SRE readiness before external alpha.

`PROD-ENTITLEMENTS-01` remains independently blocked by open P1 `Oteryn-Platform#944`; these refinements do not change that dependency.

## 9. Vertical-slice execution rule

The broad proof is decomposed as:

```text
VSL-ADMISSION-01
-> VSL-MOVE-01
-> VSL-COMBAT-01
-> VSL-PERSISTENCE-01
-> VSL-RECOVERY-01
-> VSL-MULTICHANNEL-01
```

Each slice must cross the real boundaries it claims. A mock that bypasses Gateway, transport, GameNode authority or PostgreSQL cannot be the terminal proof for that boundary.

## 10. Concise current rule

```text
accepted foundation architecture
!= implemented runtime
!= proven production system

TCP + TLS 1.3 profile 1
-> current registered/default gameplay transport
-> safe baseline

QUIC v1 + TLS 1.3
-> accepted future player-opt-in target
-> admission/recovery BLOCKED until registry + FND-04 profile reconciliation + ordering/resource evidence
-> FND-02 measured-benefit gate retained

DUR-02
-> discovery may start
-> final character-bearing schema waits for GAME-CHAR-01

DUR-03
-> waits for accepted DUR-02 + GAME-ITEM-01

GAME-VISION-01 + GAME-CHANNEL-01 + SIM-DETERMINISM-01
-> shape product/multichannel/simulation before broad implementation

runtime / production activation
-> still separately unauthorized
```
