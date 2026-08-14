# Oteryn v2 Foundation Programme — Current Status

- Status: Canonical current execution-status overlay
- Date: 2026-08-14
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: accepted foundation progression and next ordered architecture gates
- FND-04 lifecycle closeout merge: `adb0882a5ddbe42944fe955f5effb78fd5495422`
- DUR-01 lifecycle closeout merge: `ef42fa47ab054ab8aa304c017307c1945f931b59`
- ANL-01 delivery merge: `af2fa495c1126080ffc1d0717b7d0ef54f6b29ca`
- Dual-transport architecture closeout merge: `05544969baf58c3a40354f366438d759bfd159e5`
- Platform entitlement producer-remediation merge: `blakinio/Oteryn-Platform@afaa6d1d8340e44b1152b62d6d27e5fd1649804a`
- DUR-04 delivery merge: `568236c33cd23da017bca1dbd1ed98afc8da71f4`
- SIM-DETERMINISM delivery merge: `1e16b32069868f14aa1761a512b6cd8b1024e277`
- Reference evidence manifest v1 acceptance delivery: PR #252 / merge `52ef65f67e8a0e9c6f31d4754f8a6b7322d8d6d8`
- First representative ABILITY_COMBAT evidence delivery: PR #255 / merge `d04f0939f0078cb677ca3ad66f5949e9f3dadc8d`
- Current phase: `foundation/game/content/determinism architecture accepted/lifecycle-closed where delivered + Reference evidence/parity manifest v1 accepted/pinned with manifest revision 3 and first representative ABILITY_COMBAT cases delivered fail-closed; runtime largely NOT_STARTED; all executable runtime/DDL/production work still requires separate explicit owner authority; the selected next paper-only programme action is target-continuity + provenance-clearance evidence for the four registered ABILITY_COMBAT cases`

## 1. Authority of this overlay

This document answers what is accepted now and what may happen next. Detailed review/CI/repair evidence lives in accepted contracts, archived task records and merged PRs.

Older backlog/register prose that describes completed FND/DUR/ANL/gameplay/SIM gates as live is historical execution narrative. Accepted contracts plus this overlay govern current progression. Stable decision IDs and future dependency requirements remain valid unless explicitly superseded.

`docs/architecture/ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-10.md` adds owner-accepted programme ordering and product/operations refinements. `docs/architecture/ADR-0014-dual-gameplay-transport-tcp-default-quic-opt-in.md` accepts the long-term dual-transport strategy but explicitly preserves TCP transport profile `1` as the only currently registered gameplay transport profile. QUIC player admission remains blocked until `PROTOCOL_OTERYN_V1_REGISTRY.json` and both FND-04 fresh/recovery grant profiles are reconciled by a later accepted delivery. The application protocol and all security/sequencing/fencing semantics remain one `protocol-oteryn` contract.

`ARCHITECTURE_STATUS_MODEL.md` is normative for current status presentation. Every row below separates `DecisionStatus`, `DeliveryStatus` and `ImplementationStatus`; one axis never implies another. `PLANNED` means the gate is registered for future work but no active task/PR currently owns its delivery; `OPEN` is reserved for a concrete active delivery or unresolved lifecycle record.

## 2. Foundation and Stage-B progression

| Gate | DecisionStatus | DeliveryStatus | ImplementationStatus | Canonical evidence / note |
|---|---|---|---|---|
| `FND-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `PROVEN` | applied 19-member workspace/dependency cutover proven by PR #50 exact final head `5092f868a42d545f47a98c0b9723210570cd9d45`, squash merge `78988f72a80cc904aa9176ae850c50d4efa0b0f0`; exact-head Agent Governance `31095853261`, Dependency Review `31095853437`, CodeQL `31095853606`, Rust workspace `31095853343` and adversarial migration audit `31095053578` all PASS |
| `VSL-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `PROVEN` | native-client cutover/migration proven by the same PR #50 exact final head `5092f868a42d545f47a98c0b9723210570cd9d45` and merge `78988f72a80cc904aa9176ae850c50d4efa0b0f0`; Linux/Windows build, tests, pre-native fail-closed, supply-chain and named exact-head checks recorded PASS in PR #50 |
| `FND-ID-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | semantic identity architecture is accepted; PR #147 merge `81db47966d76709a0e44dfbf1bc3979f38a24ffa` archived the remaining stale merged FND-ID support records and post-merge lifecycle normalization releases its cleanup owner; no runtime identity implementation is claimed |
| `FND-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | `protocol-oteryn` v1 architecture accepted and its task lifecycle is archived; TCP profile `1` is registered architecturally, but production gameplay protocol/transport runtime remains separately gated |
| `NET-TRANSPORT-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | final successor PR #149 exact head `641de04b1397cb910f6f26e7dd1594babb8ad1ac` passed exact-head self-review, Agent Governance, Dependency Review, CodeQL and required independent review; squash merge `05544969baf58c3a40354f366438d759bfd159e5`; no gameplay transport adapter/listener or player mode is runtime-available |
| `FND-03` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | authoritative runtime execution architecture accepted and its task lifecycle is archived; GameNode runtime implementation not claimed |
| `FND-04A/B/C` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | admission + reconnect/recovery + integration architecture accepted; runtime implementation separately gated |
| `FND-04` overall | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | programme #112 architecture lifecycle complete; no gameplay admission runtime claimed |
| `DUR-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | durable representation + ItemInstanceId accepted; physical PostgreSQL implementation is downstream |
| `ANL-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | event/audit foundation accepted; no runtime event collector/outbox/broker/warehouse implementation |
| `GAME-VISION-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | minimum product-vision baseline and the immutable first Reference target are accepted; target delivery PR #181 final head `2a07643653d6d56a94ada89caf79005fce09e58a` passed self-review `4910439614`, Agent Governance `31532701302`, Dependency Review `31532701303` and CodeQL `31532701319`, then squash-merged as `b57b382cb929b2c8a20d5c81197e933b0526764f`; no runtime authority |
| `GAME-CHANNEL-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | canonical `GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md`; delivery PR #209 exact final head `ca1112191ede7d316c874189f3053ad7f8247579` passed self-review `4918161329`, fresh independent Codex no-suggestion review request `5268790260` with PR 👍 `450588928`, Agent Governance `31611424137`, Dependency Review `31611424147`, CodeQL `31611424261`, then squash-merged unchanged as `54dddbcfcb64baaf66e6cfd9fcd2cc3e1b4f04f1`; architecture freezes channel product/lifecycle/anti-hopping/multiplicity semantics but grants no runtime/client/Platform/DDL/production authority |
| `GAME-CHAR-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | Stage A + Stage B semantic architecture are owner-accepted; delivery PR #193 exact head `bc4942cab0e58b3aa4ed9713cc3f23b11b83aaa6` passed self-review `4911343351`, Agent Governance `31542939497`, Dependency Review `31542939533` and CodeQL `31542939487`, then squash-merged as `08775e378db8c1fd6bb97bedf66bf08b3541f35f`; unresolved target rules remain per-behavior parity gates and runtime authority is NONE |
| `GAME-ITEM-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | canonical `GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md`; delivery PR #205 exact final head `53d0189a114c99b4e7d44ca8d0db7a6bf5f3ea1a` passed terminal self-review `4915880173`, independent Codex no-suggestion review and Agent Governance `31591336315`, Dependency Review `31591336312`, CodeQL `31591336340`, then squash-merged as `5c502d24557621efc798def87b68f137ba23fad8`; no runtime/DDL/entitlement authority |
| `SIM-DETERMINISM-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | canonical `SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md`; delivery PR #214 exact repaired final head `4c6684328123aebd657696808372a5855980d34e`; independent review `4924203877` found one replay-provenance P1 on superseded head `5dc628f32ca4573725bcb4a42c3a7702536d7f35`; owner-authorized repair cycle 4 restored server/build, protocol and exact World Bundle provenance; terminal self-review `4924321455` PASS, repeat self-review `4924423397` PASS, Agent Governance `31676250271`, Dependency Review `31676250273`, CodeQL `31676250272` PASS; owner explicitly overrode the otherwise-required fresh independent-review-after-repair gate for exact final head and instructed finalization; squash merge `1e16b32069868f14aa1761a512b6cd8b1024e277`; no runtime/combat/AI/script/formula/dependency/DDL/production authority |
| `DUR-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | owner-accepted whole Persistence-v1 baseline in `DUR-02_PERSISTENCE_V1_OWNER_BASELINE.md`; delivery PR #201 exact head `900be9f499981e638a6f8089fb46331b43ba321c` passed self-review `4914253621`, independent Codex review `5264011166`, Agent Governance `31576235871`, Dependency Review `31576235909` and CodeQL `31576235921`, then squash-merged as `ec4b840b0742967370a4235d87094b29a802fe28`; Character persistence partial baseline remains binding; no PostgreSQL DDL/migration or runtime authority |
| `DUR-03` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | canonical `DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md`; delivery PR #207 exact final head `a1d949362e219373a5d314c0e9ddf8de110362dd` passed self-review `4916797999`, independent Codex no-suggestion review, Agent Governance `31599369738`, Dependency Review `31599369737`, CodeQL `31599369780`, then squash-merged as `63380bcba469027e90677aaf4db571fa941be2f4`; architecture freezes item/value location, conservation, idempotency, runtime↔durable handoff and anti-duplication semantics but grants no runtime/DDL/production authority |
| `DUR-04` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | canonical `DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md`; delivery PR #212 exact final head `77e68ffb9e9e0e31ca751d42ff5f7c03466b2b23` passed owner-directed exact-head self-review `4921665072` under the explicit 2026-08-13 owner override of the independent-review mechanism, Agent Governance `31646998515`, Dependency Review `31646998564`, CodeQL `31646998517`, then squash-merged unchanged as `568236c33cd23da017bca1dbd1ed98afc8da71f4`; repair budget `3/3`; no runtime/client/compiler/loader/Studio/DDL/content-import/production authority |
| `PROD-ENTITLEMENTS-01` | `PROPOSED` | `PLANNED` | `NOT_STARTED` | Platform producer security prerequisite #944 is satisfied by PR #968 merge `afaa6d1d8340e44b1152b62d6d27e5fd1649804a`; Oteryn-v2 consumer/enforcement contract remains unaccepted and Premium/VIP/game-consumed entitlement activation remains unauthorized |

The `ImplementationStatus` column describes only the scope of the named gate. `NOT_STARTED` does not mean adjacent repository code is absent; it means this overlay does not claim the gate's production/runtime behavior as implemented. `PROVEN` is used only where exact revision and named evidence are present in the row or directly named delivery record.

## 3. Accepted baseline preserved

FND-02 retains one `protocol-oteryn` application protocol, TLS/protobuf gameplay semantics, GameSession-scoped nonzero uint64 CommandId, server sequencing/revisions, reconciliation, bounded inputs and fail-closed compatibility/security behavior. The current accepted registry contains TCP+TLS 1.3 transport profile `1` only as architecture/compatibility registration; **no gameplay transport adapter/listener is implemented or authorized by these architecture documents**. ADR-0014 refines future transport direction, not current admission compatibility: QUIC remains blocked until a stable QUIC transport profile and matching FND-04 fresh/recovery grant semantics are accepted. FND-02's measured-benefit prerequisite remains binding. Any future QUIC adapter must preserve identical application/security authority and visible ordering.

FND-03 retains one logical authoritative mutation owner per channel/instance, separate ownership generation, owner-scoped RuntimeExecutionOrdinal, bounded queues, fail-closed stale work and measured capacity requirements. ADR-0009 remains binding that one `GameNode` is one game-server process. ADR-0015 leaves internal module/crate decomposition unfrozen; a domain-modular monolith is only the preferred nonbinding starting hypothesis until measured deployment/security/data/failure evidence justifies a distinct adjacent service boundary or a later explicit superseding decision.

FND-04 remains accepted/lifecycle-closed with ownership-before-world admission, purpose-separated grant profiles, anti-rollback security evidence, PREPARE/COMMIT reconnect, healthy-binding non-preemption, ControlLossEpoch, exactly 4 seconds eligible defensive PvE re-entry protection and fail-closed recovery. Any future change to the four-second value requires explicit game-design evidence and superseding policy rather than hidden per-server drift.

DUR-01 remains accepted/lifecycle-closed: UUIDv7 native durability uses PostgreSQL `uuid`, persisted CommandId preserves full uint64 via `numeric(20,0)`, ItemInstanceId is a game-owned UUIDv7 identity, legacy imports use stable source namespace identity, and internal IDs are not automatically public.

`GAME-VISION-01` minimum product semantics are accepted in `GAME-VISION-01_MINIMUM_OWNER_BASELINE.md`. The accepted loop is player-goal driven across preparation, risk/activity, committed progress/value and recovery/planning, with persistent character/equipment/exploration/social/prestige horizons. Reference uses mechanical source/sink parity rather than historical market-state parity, conservation precedes tuning, and intentional differences remain explicit/versioned/measurable. Success is measured by Reference correctness, interaction quality, progress/value trust, core-loop health, economy health and product/operational health. Numeric targets remain milestone-owned.

`GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md` fixes the first Reference external behavior cut to **Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance change boundary**. The target is immutable; later Global changes are candidate evidence for a later explicit Reference revision rather than silent mutation. Target selection remains separate from evidence completeness: individual mechanics may be `PROVEN`, `OBSERVED`, `DERIVED`, `UNKNOWN`, `CONFLICT` or `DECLARED_DIFFERENCE`. Official public sources are primary but not assumed exhaustive; controlled black-box observation may provide target evidence; community sources are corroborative/discovery inputs; Canary/crystalserver/other OTS remain hypothesis/inventory inputs rather than proof of Global behavior. Patch-note/search absence is not evidence of no change. Security/integrity/legal/provenance constraints override defect compatibility.

`REFERENCE_EVIDENCE_PARITY_MANIFEST_V1_OWNER_ACCEPTANCE.md` accepts and pins the paper registry for that immutable first target: schema version 1 remains the unchanged schema blob `208506f461231eb3ed8966ae16dade0764eb39b8`, manifest revision 3 is `ACCEPTED`, `ABILITY_COMBAT` is `MECHANIC_CASES_REGISTERED`, four bounded Light Healing/Ice Strike cases are registered with target evidence `UNKNOWN`, source/case provenance `PENDING`, legal review `PENDING`, Oteryn implementation `NOT_STARTED` and parity `PARITY_PENDING_EVIDENCE`, and `canonical_digest` remains null until accepted canonicalization/digest tooling exists. Registry acceptance and case registration create no mechanic-level parity claim, no mutable `latest` consumer authority and no runtime/release/content/DDL/Platform/production authority. Delivery PR #255 merged as `d04f0939f0078cb677ca3ad66f5949e9f3dadc8d`.

`GAME-CHAR-01_STAGE_A_OWNER_BASELINE.md` remains binding for baseline-neutral Character ownership/lifecycle/revision/migration safety. `GAME-CHAR-01_STAGE_B_OWNER_BASELINE.md` accepts the Reference-sensitive semantic closure layered on Stage A: global logical naming namespace with versioned canonical-comparison policy; versioned lifecycle/quota policy; versioned creation/starter context; five Reference vocation families/promoted forms plus pre-vocation state and eight skill categories; formula-neutral progression ownership; promotion achievement versus entitlement-derived activation; profile-scoped death/protection; character-owned offline-training counter semantics; modern character-specific progression ownership scope; and explicit hard parity gates for every unresolved `UNKNOWN/CONFLICT` target rule. Architecture acceptance does **not** imply `PARITY_CONFIRMED`, physical schema, runtime implementation or all-profile schema completeness.

`DUR-02_PROFILE_NEUTRAL_CHARACTER_PERSISTENCE_OWNER_BASELINE.md` remains binding for the profile-neutral Character persistence sub-scope: normalized current-state persistence with one CharacterRevision, account portfolio guards, a domain-canonical global name registry, typed Character/profile child relations, distinct FND-04 presence/lease/session/ControlLoss authority, atomic fresh admission and reconnect/recovery authority transitions, durable idempotency receipts, explicit lock/isolation proofs, atomic retained-audit/publication evidence, normalized current-state checkpoint authority, staged migration and fail-closed no-authority-resurrection after restore. It forbids a generic JSON/KV/EAV miscellaneous-state escape hatch and keeps unresolved Reference/profile/operational values outside core schema invariants.

`DUR-02_PERSISTENCE_V1_OWNER_BASELINE.md` closes the **whole common Persistence-v1 architecture** above that Character sub-baseline. It accepts: one ordered game-owned migration history for the current `oteryn_game` boundary with explicit immutable migration artifacts and a dedicated least-privilege migrator; anomaly-proof transaction isolation/locking/retry rules; one ANL-compatible durable journal plus crash-safe publication checkpoint substrate; strict separation of acknowledged committed durability from FND-03 runtime checkpoints and disaster RPO; PITR-capable restore-tested fail-closed recovery with non-rollback authority fencing; and game-wide expand/migrate/validate/cut-over/contract schema evolution. The fourteen historical DUR-02 subjects are exhaustively assigned to `SATISFIED`, whole-DUR-02, exact gameplay/domain owners or implementation/PERF evidence. Whole-DUR-02 acceptance creates no DDL, migration-execution or runtime authority.

`GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md` is accepted/lifecycle-closed architecture. It fixes ItemType versus ItemInstance versus authored StaticItemPlacement semantics; typed bounded item capabilities; server-authoritative equipment occupancy and container legality; explicit definition compatibility/migration; Reference fail-closed evidence discipline; and delegates item/currency/value conservation, atomic location, retry/crash and create/destroy/split/merge/transform identity-transition semantics to the now accepted DUR-03 contract. GAME-ITEM acceptance creates no item runtime, SQL, entitlement or production authority.

`DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md` is accepted/lifecycle-closed architecture. It freezes one typed immediate semantic location for every live durable ItemInstance; current runtime ownership versus durable recoverability; non-blocking runtime reservation -> asynchronous game-DB durable linearization -> runtime reconciliation for ground/instance value crossings; transaction-scoped new ItemInstanceId semantics; split/merge/transform lifecycle rules; exact transfer/mint/burn/transform/conversion conservation and provenance; CommandRef/OperationId/TransactionId retry/ambiguous-commit behavior; current GameSession/CharacterLease/runtime ownership fencing; typed custody; bounded durable audit; and fail-closed restore reconciliation. It preserves downstream loot/trade/market/bank/depot/mail/reward/house/crafting/entitlement policy ownership and grants no runtime/client/DDL/migration/production authority.

`GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md` is accepted/lifecycle-closed architecture. It freezes canonical ChannelRef versus display identity; recommendation/explicit target and bounded pre-admission queue semantics; privacy-bounded party/friend co-location; durable Character+World anti-hopping/prior-Channel state; recovery-safe destination switch admission + guard advancement; fail-closed source/reward multiplicity classification; qualitative `DEMAND_PRESSURE` / `RECOVERY_PRESSURE` / `LOW_LOAD_CONSOLIDATION_CANDIDATE` / `CHANNEL_UNHEALTHY` lifecycle trigger vocabulary; legitimate/forbidden new-Channel creation predicates; low-load drain, drain-abort and terminal retirement rules; same-ChannelId recovery; one-World community/economy semantics; and strict PERF/OPS ownership of numeric thresholds and orchestration. GAME-CHANNEL acceptance does not implement Gateway queue/recommendation, switch persistence, dynamic scaling, client UI, PvP/reward logic or production behavior.

`DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md` is accepted/lifecycle-closed architecture. It freezes stable semantic package/content identity and exact dependency locks; deterministic compiler and immutable bundle/staging/activation/migration rules; explicit client-safe/server-authoritative projection; bounded fail-closed loading; legacy provenance/conversion boundaries; WebAssembly Component Model + project-owned versioned WIT as the target authoritative script capability boundary; snapshot-bound reads and proposal-only mutations; authority-scoped action plans; deterministic logical time/RNG/query/floating/fuel semantics bound by `script_execution_profile_revision`; typed persistent extension state; and a mandatory Resource Limits Registry plus reversible physical-format spike before final serializer/container/chunk/compression choices. DUR-04 acceptance does not add Wasmtime/WIT implementation files, compiler/loader/Studio runtime, DDL/migrations, broad content import, signing/CDN or production activation.

`SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md` is accepted/lifecycle-closed architecture. It freezes reproducibility from canonical future-determining state plus exact owner-local normalized input order, exact semantic revision/profile set and normalized external facts; keeps FND-03 RuntimeExecutionOrdinal owner-local with no global total order; versions cross-cutting numeric/RNG/tie-break/hash/supported-target semantics through `SimulationDeterminismProfileRevision`; binds retryable/delayed occurrences to exact behavior-affecting revisions; defines explicit numeric classes, formula descriptors, rounding and fail-closed invalid-state behavior; preserves DUR-03 exact conservation; requires purpose-isolated deterministic gameplay RNG with retry/failover stability, authoritative advancement and anti-prediction protection; separates wall clock, monotonic elapsed time and authoritative execution order; normalizes external nondeterminism; retains exact server/build, `protocol-oteryn`, World Bundle, formula/script/RNG/time/external provenance in replay; treats NodeId/process-incarnation as optional forensic attribution rather than replay placement authority; hashes all future-determining state including active revisions, RNG state and pending work; requires read-only hierarchical divergence evidence and identical normalized outcomes across supported authoritative targets. SIM acceptance does not select concrete numeric/RNG/hash libraries, exact gameplay RNG algorithm/formulas/scales, global tick, scheduler/thread counts, replay backend or production hash cadence, and grants no runtime/combat/AI/script/DDL/production authority.

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

Accepted FND/DUR-01/DUR-02/DUR-03/DUR-04/ANL-01/NET-TRANSPORT-01/GAME-VISION-01/GAME-CHANNEL/GAME-CHAR/GAME-ITEM/SIM-DETERMINISM-01 architecture plus the accepted Reference evidence/parity manifest v1 paper contract does **not** itself authorize:

- TCP or QUIC gameplay adapter/listener implementation;
- any currently functional gameplay transport client mode;
- a QUIC transport profile, QUIC admission/recovery, functional player QUIC option, QUIC library selection, 0-RTT or DATAGRAM activation;
- runtime event collector implementation;
- PostgreSQL table/outbox/checkpoint/migration implementation;
- PostgreSQL DDL or migration execution;
- transaction isolation/locking/retry/RPO/RTO runtime implementation;
- Character schema/progression/death/offline-training runtime implementation merely because semantic/persistence architecture is accepted;
- profile-specific Character persistence without its owning profile semantics;
- any unresolved `UNKNOWN/CONFLICT` Character rule as Reference behavior;
- any mechanic parity claim from registry acceptance, catalogue presence or implementation similarity alone;
- runtime/release/content consumption of the manifest through mutable `latest` lookup or without a separately accepted immutable artifact/digest contract where such a consumer requires cryptographic pinning;
- item/currency transaction implementation, runtime ground↔durable handoff implementation or anti-duplication runtime merely because DUR-03 is accepted;
- Channel directory/recommendation/queue/switch-guard/dynamic-scaling runtime or Platform implementation merely because GAME-CHANNEL is accepted;
- compiler/loader/Studio/scripting runtime, final physical world/bundle encoding, WIT host implementation, broad content import or live content activation merely because DUR-04 is accepted;
- authoritative SIM kernel/RNG/state-hash/replay implementation, numeric/RNG/hash dependency adoption, combat/AI/progression/script implementation or exact gameplay/Reference/balance formulas merely because SIM-DETERMINISM is accepted;
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

Whole `DUR-02 — Persistence v1`, `GAME-ITEM-01`, `DUR-03`, `GAME-CHANNEL-01`, `DUR-04` and `SIM-DETERMINISM-01` architecture are accepted/lifecycle-closed while implementation remains `NOT_STARTED`. Reference evidence/parity manifest v1 is accepted/pinned/lifecycle-closed as a paper registry and now contains the first representative `ABILITY_COMBAT` case set. A **separately authorized server/persistence/content foundation implementation programme** may consume accepted common scopes, but architecture acceptance does not itself grant that authority.

`REFERENCE_EVIDENCE_PARITY_MANIFEST_V1_OWNER_ACCEPTANCE.md` freezes schema version 1 and the immutable 2026-07-28 first Reference target plus fail-closed evidence/provenance policy. Manifest revision 3 is `ACCEPTED`; `ABILITY_COMBAT` is `MECHANIC_CASES_REGISTERED`; four Light Healing/Ice Strike cases are registered with target evidence `UNKNOWN`, source/case provenance `PENDING`, legal review `PENDING`, Oteryn implementation `NOT_STARTED`, exact revision null, no fixture/test link and parity `PARITY_PENDING_EVIDENCE`. `canonical_digest=null` remains truthful until accepted digest tooling exists. The registry must not be rebuilt or duplicated.

The remaining named pre-VSL paper-only programme action is a bounded **target-continuity + provenance-clearance evidence package for those four registered `ABILITY_COMBAT` cases**. It must locate provenance-cleared, time-appropriate evidence that directly bridges or captures the immutable 2026-07-28 target boundary and then either promote or reject the indexed official-state hypotheses. Patch-note/search absence is not continuity proof. Do not broaden mechanic inventory or freeze physical catalogue tooling before proving this representative historical-evidence path; do not invent a new stable gate ID.

The next ordered work is:

1. Obtain provenance-cleared, time-appropriate target-continuity evidence for the four registered Light Healing/Ice Strike cases and update classifications only when the evidence contract permits it; keep `UNKNOWN/PENDING` fail-closed otherwise and do not claim executable parity.
2. Obtain explicit implementation authority before any bounded server/persistence/Channel/DUR/content/SIM executable increment. A safe decomposition remains GameNode/bootstrap shell -> `protocol-oteryn` transport/runtime adapter -> admission/GameSession/CharacterLease -> PostgreSQL migration/persistence substrate -> Character/FND-04 persistence -> minimal ChannelRuntime; item/value implementation additionally consumes GAME-ITEM/DUR-03 and concrete ANL/resource-limit evidence; content implementation additionally consumes DUR-04 and required physical-format/resource-limit/WIT execution-profile evidence; deterministic simulation/combat/AI/progression implementation additionally consumes SIM-DETERMINISM and exact formula/Reference evidence; Channel product implementation additionally consumes GAME-CHANNEL and later PERF/OPS numeric/orchestration decisions.
3. `NET-TRANSPORT-02` (or equivalent bounded successor) remains later evidence work for QUIC profile/FND-04 reconciliation.
4. Expand `VSL-01` through real-boundary slices only after each owning architecture/implementation gate is ready: admission, movement, combat, persistence, recovery, multichannel.
5. Establish minimal admin/security/SRE readiness before external alpha.

`PROD-ENTITLEMENTS-01` remains independently deferred. Its Platform producer prerequisite is satisfied, but game-consumed Premium/VIP/commerce behavior still requires its own Oteryn-v2 consumer/enforcement contract and rollout proof.

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
-> ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED

GAME-CHANNEL-01
-> ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
-> channel selection/queue/co-location/switch/multiplicity/lifecycle policy accepted
-> runtime/client/Platform/scaling implementation authority NONE

GAME-CHAR-01
-> ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
-> exact unresolved target behavior remains hard parity-gated

DUR-02 overall
-> ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
-> DDL/migrations/runtime NOT_AUTHORIZED

GAME-ITEM-01
-> ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
-> runtime / DDL / entitlement authority NONE

DUR-03
-> ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
-> durable item/currency/value transaction/conservation/anti-duplication architecture accepted
-> runtime / DDL / production authority NONE

DUR-04
-> ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
-> deterministic content/package/bundle/migration/scripting architecture accepted
-> runtime/client/compiler/loader/Studio/WIT-host/DDL/content-import/production authority NONE

SIM-DETERMINISM-01
-> ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
-> deterministic arithmetic/RNG/order/replay/state-hash architecture accepted
-> exact formulas/libraries/runtime NOT_AUTHORIZED

Reference evidence/parity manifest v1
-> ACCEPTED / LIFECYCLE_CLOSED
-> schema version 1 pinned; manifest revision 3 ACCEPTED
-> ABILITY_COMBAT has four registered Light Healing/Ice Strike cases
-> target UNKNOWN; source/case/legal provenance PENDING; implementation NOT_STARTED; parity PENDING
-> canonical_digest null until accepted tooling exists
-> no mechanic parity implied

next paper-only programme action
-> target-continuity + provenance-clearance evidence for the four registered ABILITY_COMBAT cases
-> directly bridge/capture the immutable 2026-07-28 target boundary
-> no patch/search-silence continuity inference; no broaden-before-proof; no invented stable gate ID

server/persistence/channel/content/SIM implementation
-> explicit owner implementation authorization still REQUIRED

PROD-ENTITLEMENTS-01
-> Platform producer prerequisite SATISFIED
-> Oteryn-v2 consumer contract NOT_ACCEPTED
-> Premium/VIP activation NOT_AUTHORIZED

runtime / production activation
-> still separately unauthorized
```
