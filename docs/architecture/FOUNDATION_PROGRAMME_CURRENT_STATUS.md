# Oteryn v2 Foundation Programme — Current Status

- Status: Canonical current execution-status overlay
- Date: 2026-08-11
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: accepted foundation progression and next ordered architecture gates
- FND-04 lifecycle closeout merge: `adb0882a5ddbe42944fe955f5effb78fd5495422`
- DUR-01 lifecycle closeout merge: `ef42fa47ab054ab8aa304c017307c1945f931b59`
- ANL-01 delivery merge: `af2fa495c1126080ffc1d0717b7d0ef54f6b29ca`
- Dual-transport architecture closeout merge: `05544969baf58c3a40354f366438d759bfd159e5`
- Platform entitlement producer-remediation merge: `blakinio/Oteryn-Platform@afaa6d1d8340e44b1152b62d6d27e5fd1649804a`
- Current phase: `foundation architecture accepted/lifecycle-closed where delivered + runtime largely NOT_STARTED / GAME-VISION minimum, GAME-CHAR Stage A and first Reference target accepted; GAME-CHAR Stage B, GAME-CHANNEL and bounded DUR-02 discovery next`

## 1. Authority of this overlay

This document answers what is accepted now and what may happen next. Detailed review/CI/repair evidence lives in accepted contracts, archived task records and merged PRs.

Older backlog/register prose that describes completed FND/DUR/ANL gates as live is historical execution narrative. Accepted contracts plus this overlay govern current progression. Stable decision IDs and future dependency requirements remain valid unless explicitly superseded.

`docs/architecture/ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-10.md` adds owner-accepted programme ordering and product/operations refinements. `docs/architecture/ADR-0014-dual-gameplay-transport-tcp-default-quic-opt-in.md` accepts the long-term dual-transport strategy but explicitly preserves TCP transport profile `1` as the only currently registered gameplay transport profile. QUIC player admission remains blocked until `PROTOCOL_OTERYN_V1_REGISTRY.json` and both FND-04 fresh/recovery grant profiles are reconciled by a later accepted delivery. The application protocol and all security/sequencing/fencing semantics remain one `protocol-oteryn` contract.

`ARCHITECTURE_STATUS_MODEL.md` is normative for current status presentation. Every row below separates `DecisionStatus`, `DeliveryStatus` and `ImplementationStatus`; one axis never implies another. `PLANNED` means the gate is registered for future work but no active task/PR currently owns its delivery; `OPEN` is reserved for a concrete active delivery or unresolved lifecycle record.

## 2. Foundation and Stage-B progression

| Gate | DecisionStatus | DeliveryStatus | ImplementationStatus | Canonical evidence / note |
|---|---|---|---|---|
| `FND-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `PROVEN` | applied 19-member workspace/dependency cutover proven by PR #50 exact final head `5092f868a42d545f47a98c0b9723210570cd9d45`, squash merge `78988f72a80cc904aa9176ae850c50d4efa0b0f0`; exact-head Agent Governance `31095853261`, Dependency Review `31095853437`, CodeQL `31095853606`, Rust workspace `31095853343` and adversarial migration audit `31095053578` all PASS |
| `VSL-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `PROVEN` | native-client cutover/migration proven by the same PR #50 exact head `5092f868a42d545f47a98c0b9723210570cd9d45` and merge `78988f72a80cc904aa9176ae850c50d4efa0b0f0`; Linux/Windows build, tests, pre-native fail-closed, supply-chain and named exact-head checks recorded PASS in PR #50 |
| `FND-ID-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | semantic identity architecture is accepted; PR #147 merge `81db47966d76709a0e44dfbf1bc3979f38a24ffa` archived the remaining stale merged FND-ID support records and post-merge lifecycle normalization releases its cleanup owner; no runtime identity implementation is claimed |
| `FND-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | `protocol-oteryn` v1 architecture accepted and its task lifecycle is archived; TCP profile `1` is registered architecturally, but production gameplay protocol/transport runtime remains separately gated |
| `NET-TRANSPORT-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | final successor PR #149 exact head `641de04b1397cb910f6f26e7dd1594babb8ad1ac` passed exact-head self-review, Agent Governance, Dependency Review, CodeQL and required independent review; squash merge `05544969baf58c3a40354f366438d759bfd159e5`; no gameplay transport adapter/listener or player mode is runtime-available |
| `FND-03` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | authoritative runtime execution architecture accepted and its task lifecycle is archived; GameNode runtime implementation not claimed |
| `FND-04A/B/C` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | admission + reconnect/recovery + integration architecture accepted; runtime implementation separately gated |
| `FND-04` overall | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | programme #112 architecture lifecycle complete; no gameplay admission runtime claimed |
| `DUR-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | durable representation + ItemInstanceId accepted; physical PostgreSQL implementation is downstream |
| `ANL-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | event/audit foundation accepted; no runtime event collector/outbox/broker/warehouse implementation |
| `GAME-VISION-01` | `ACCEPTED` | `OPEN` | `NOT_STARTED` | minimum product-vision baseline remains accepted; owner also accepted `GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md`, selecting Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance boundary as the immutable first Reference target with explicit evidence classification/provenance rules; this accepted-target delivery is currently active and authorizes no runtime |
| `GAME-CHANNEL-01` | `PROPOSED` | `PLANNED` | `NOT_STARTED` | social/economic/PvP/UX channel policy required before multichannel becomes a product feature |
| `GAME-CHAR-01` | `PROPOSED` | `PLANNED` | `NOT_STARTED` | Stage A baseline-neutral semantics are owner-accepted; the exact first Reference target prerequisite is now satisfied, so Stage B is unblocked for paper-only evidence reconciliation but remains unaccepted; final character-bearing DUR-02 schema still waits for full GAME-CHAR acceptance; no runtime authority |
| `GAME-ITEM-01` | `PROPOSED` | `PLANNED` | `NOT_STARTED` | blocks final DUR-03 item transaction semantics; Reference-sensitive item work uses the accepted 2026-07-28 target unless explicitly superseded/scoped |
| `SIM-DETERMINISM-01` | `PROPOSED` | `PLANNED` | `NOT_STARTED` | required before broad combat/AI formula freeze |
| `DUR-02` | `PROPOSED` | `PLANNED` | `NOT_STARTED` | bounded persistence discovery may consume DUR-01 + ANL-01 + accepted GAME-CHAR Stage-A invariants and use the selected Reference target only as question/compatibility context; final character-bearing schema still requires accepted Stage B/full GAME-CHAR closure |
| `DUR-03` | `PROPOSED` | `PLANNED` | `NOT_STARTED` | waits for accepted DUR-02 + GAME-ITEM-01 + ANL-01 evidence semantics |
| `DUR-04` | `PROPOSED` | `PLANNED` | `NOT_STARTED` | content/world/scripting architecture; minimum headless schema/validator/compiler/bundle/loader precedes full Studio |
| `PROD-ENTITLEMENTS-01` | `PROPOSED` | `PLANNED` | `NOT_STARTED` | Platform producer security prerequisite #944 is satisfied by PR #968 merge `afaa6d1d8340e44b1152b62d6d27e5fd1649804a`; Oteryn-v2 consumer/enforcement contract remains unaccepted and Premium/VIP/game-consumed entitlement activation remains unauthorized |

The `ImplementationStatus` column describes only the scope of the named gate. `NOT_STARTED` does not mean adjacent repository code is absent; it means this overlay does not claim the gate's production/runtime behavior as implemented. `PROVEN` is used only where exact revision and named evidence are present in the row or directly named delivery record.

## 3. Accepted baseline preserved

FND-02 retains one `protocol-oteryn` application protocol, TLS/protobuf gameplay semantics, GameSession-scoped nonzero uint64 CommandId, server sequencing/revisions, reconciliation, bounded inputs and fail-closed compatibility/security behavior. The current accepted registry contains TCP+TLS 1.3 transport profile `1` only as architecture/compatibility registration; **no gameplay transport adapter/listener is implemented or authorized by these architecture documents**. ADR-0014 refines future transport direction, not current admission compatibility: QUIC remains blocked until a stable QUIC transport profile and matching FND-04 fresh/recovery grant semantics are accepted. FND-02's measured-benefit prerequisite remains binding. Any future QUIC adapter must preserve identical application/security authority and visible ordering.

FND-03 retains one logical authoritative mutation owner per channel/instance, separate ownership generation, owner-scoped RuntimeExecutionOrdinal, bounded queues, fail-closed stale work and measured capacity requirements. ADR-0009 remains binding that one `GameNode` is one game-server process. ADR-0015 leaves internal module/crate decomposition unfrozen; a domain-modular monolith is only the preferred nonbinding starting hypothesis until measured deployment/security/data/failure evidence justifies a distinct adjacent service boundary or a later explicit superseding decision.

FND-04 remains accepted/lifecycle-closed with ownership-before-world admission, purpose-separated grant profiles, anti-rollback security evidence, PREPARE/COMMIT reconnect, healthy-binding non-preemption, ControlLossEpoch, exactly 4 seconds eligible defensive PvE re-entry protection and fail-closed recovery. Any future change to the four-second value requires explicit game-design evidence and superseding policy rather than hidden configuration drift.

DUR-01 remains accepted/lifecycle-closed: UUIDv7 native durability uses PostgreSQL `uuid`, persisted CommandId preserves full uint64 via `numeric(20,0)`, ItemInstanceId is a game-owned UUIDv7 identity, legacy imports use stable source namespace identity, and internal IDs are not automatically public.

`GAME-VISION-01` minimum product semantics are accepted in `GAME-VISION-01_MINIMUM_OWNER_BASELINE.md`. The accepted loop is player-goal driven across preparation, risk/activity, committed progress/value and recovery/planning, with persistent character/equipment/exploration/social/prestige horizons. Reference uses mechanical source/sink parity rather than historical market-state parity, conservation precedes tuning, and intentional differences remain explicit/versioned/measurable. Success is measured by Reference correctness, interaction quality, progress/value trust, core-loop health, economy health and product/operational health. Numeric targets remain milestone-owned.

`GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md` now fixes the first Reference external behavior cut to **Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance change boundary**. The target is immutable; later Global changes are candidate evidence for a later explicit Reference revision rather than silent mutation. Target selection remains separate from evidence completeness: individual mechanics may be `PROVEN`, `OBSERVED`, `DERIVED`, `UNKNOWN`, `CONFLICT` or `DECLARED_DIFFERENCE`. Official public sources are primary but not assumed exhaustive; controlled black-box observation may provide target evidence; community sources are corroborative/discovery inputs; Canary/crystalserver/other OTS remain hypothesis/inventory inputs rather than proof of Global behavior. Patch-note/search absence is not evidence of no change. Security/integrity/legal/provenance constraints override defect compatibility.

`GAME-CHAR-01_STAGE_A_OWNER_BASELINE.md` is binding for the baseline-neutral partial scope only. It freezes the bounded Character aggregate, minimal ACTIVE/DELETION_SCHEDULED/RETIRED lifecycle, atomic idempotent creation, a durable Character revision/fence distinct from FND-04 session/lease fencing, conservative quiescence for terminal retirement/world/account transfer, progression-fact/derived-value separation, idempotent death-consequence boundary, explicit ruleset/profile migration, ruleset-owned vocation/build state, opt-in offline progression, Character Authority naming/quota authority boundaries, transfer-as-capability, and the exact limits on what bounded DUR-02 discovery may consume before Stage B. It does not select any Reference formula or physical schema and does not close the overall GAME-CHAR gate.

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

Accepted FND/DUR-01/ANL-01/NET-TRANSPORT-01/GAME-VISION-01, the selected first Reference target and the partial GAME-CHAR Stage-A architecture do **not** authorize:

- TCP or QUIC gameplay adapter/listener implementation;
- any currently functional gameplay transport client mode;
- a QUIC transport profile, QUIC admission/recovery, functional player QUIC option, QUIC library selection, 0-RTT or DATAGRAM activation;
- runtime event collector implementation;
- PostgreSQL table/outbox/checkpoint/migration implementation;
- transaction isolation/locking/retry/RPO/RTO implementation;
- character schema/progression/death/offline-training implementation before accepted Stage B/full GAME-CHAR closure;
- item/currency transaction implementation;
- broad gameplay/content implementation from the product baseline alone;
- broker/stream/warehouse/lake/dashboard selection or deployment;
- balance/security detector implementation;
- investigation/AI write authority;
- Platform migrations/writes;
- production analytics collection;
- gameplay runtime/deployment/traffic activation;
- Premium/VIP or other game-consumed entitlement activation.

The native client therefore remains legitimately pre-native-protocol until a separately authorized implementation task proves the transport/session/runtime path.

## 8. Next ordered architecture and proof work

The immediate programme is refined now that the first Reference target is owner-accepted:

1. `GAME-CHAR-01` Stage B — perform paper-only evidence reconciliation against the accepted 2026-07-28 target for naming/creation/progression/vocation/death/offline-training/quota semantics; `UNKNOWN` remains fail-closed rather than guessed.
2. `GAME-CHANNEL-01` — channel social/economic/PvP/UX policy may proceed in parallel before multichannel becomes a product feature.
3. `DUR-02 — Persistence v1` — bounded discovery may consume accepted DUR-01 + ANL-01 + GAME-CHAR Stage-A invariants and the Reference target as compatibility context; final character-bearing schema waits for accepted Stage B/full GAME-CHAR closure.
4. Build the versioned Reference evidence/parity manifest as the owning parity/evidence tooling contract is defined; target selection does not imply evidence completeness.
5. `GAME-ITEM-01` — accept item model/equipment/container/transform semantics against the same first Reference target where parity applies.
6. `DUR-03 — Item Transaction and Anti-Duplication Invariants` — consumes accepted DUR-02 + GAME-ITEM-01 + ANL-01 evidence semantics.
7. `DUR-04` minimum headless content path — schema -> validator -> deterministic compiler -> bundle -> loader; full Studio remains downstream.
8. `SIM-DETERMINISM-01` — freeze authoritative arithmetic/replay requirements before broad combat/AI implementation.
9. `NET-TRANSPORT-02` (or an equivalent bounded successor) — register QUIC transport profile, reconcile FND-04 fresh/recovery transport bindings and prove FND-02 ordered-lane/snapshot semantics before functional player QUIC admission.
10. Implement the umbrella `VSL-01` as ordered real-boundary slices: admission, movement, combat, persistence, recovery, then multichannel.
11. Establish minimal admin/security/SRE readiness before external alpha.

`PROD-ENTITLEMENTS-01` is no longer waiting on Platform issue #944: its producer-side security prerequisite is satisfied by Oteryn-Platform PR #968 / merge `afaa6d1d8340e44b1152b62d6d27e5fd1649804a`. It remains independently deferred and blocks only game-consumed entitlement implementation/activation until the Oteryn-v2 consumer/enforcement contract and cross-repository rollout proof are accepted.

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
-> currently registered architecture profile
-> intended initial/default safe baseline after runtime implementation
-> runtime adapter/listener NOT_STARTED

QUIC v1 + TLS 1.3
-> accepted future player-opt-in target
-> admission/recovery BLOCKED until registry + FND-04 profile reconciliation + ordering/resource evidence
-> FND-02 measured-benefit gate retained

all gameplay transport client modes
-> runtime unavailable now

GAME-VISION-01 minimum
-> ACCEPTED product semantics
-> implementation NOT_STARTED
-> no runtime authority

first Reference target
-> OWNER-ACCEPTED
-> Global Tibia production-observable behavior after 2026-07-28 server save/maintenance
-> immutable target; later Global changes require explicit later Reference revision
-> evidence gaps remain PROVEN/OBSERVED/DERIVED/UNKNOWN/CONFLICT/DECLARED_DIFFERENCE rather than guessed

GAME-CHAR-01 Stage A
-> OWNER-ACCEPTED partial baseline
-> bounded aggregate/lifecycle/revision/migration invariants are binding

GAME-CHAR-01 overall
-> PROPOSED / PLANNED / NOT_STARTED
-> Stage B UNBLOCKED for paper-only evidence reconciliation, still NOT ACCEPTED

DUR-02
-> bounded discovery may consume Stage-A invariants and use selected target as compatibility context
-> final character-bearing schema waits for accepted Stage B/full GAME-CHAR closure

GAME-CHANNEL-01 + SIM-DETERMINISM-01
-> still shape multichannel/simulation before their broad implementation scopes

DUR-03
-> waits for accepted DUR-02 + GAME-ITEM-01

PROD-ENTITLEMENTS-01
-> Platform producer prerequisite SATISFIED
-> Oteryn-v2 consumer contract NOT_ACCEPTED
-> Premium/VIP activation NOT_AUTHORIZED

runtime / production activation
-> still separately unauthorized
```
