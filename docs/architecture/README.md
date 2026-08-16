# Oteryn v2 Architecture Index

This directory contains the canonical architecture decisions, contracts, current-status overlays and planning registers for Oteryn-v2.

## Source hierarchy

Use this order when documents overlap:

1. explicit owner instruction and repository governance;
2. an explicit later ADR/contract that states it supersedes the older authority for the named scope;
3. the accepted ADR/contract that otherwise owns the domain;
4. `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` for current progression/status wording;
5. actively maintained review refinements, reconciliation overlays and decision registers;
6. historical analysis, evidence and archived task records.

Supersession precedence applies **only** to the scope explicitly named by the later decision. The older owning contract remains authoritative everywhere else. A newer date alone never supersedes an accepted semantic contract.

Architecture acceptance is not runtime implementation. Use [Architecture Status Model](ARCHITECTURE_STATUS_MODEL.md) to distinguish decision, delivery and implementation state.

## Current architecture entry points

- [Foundation programme current status](FOUNDATION_PROGRAMME_CURRENT_STATUS.md)
- [2026-08-16 post-wave A–F reconciliation](OTERYN_V2_POST_WAVE_A_F_RECONCILIATION_20260816.md) — supersedes only stale execution-status/coverage/next-action wording in older horizon/gap snapshots
- [Foundation decision backlog](FOUNDATION_DECISION_BACKLOG.md)
- [Global architecture decision register](GLOBAL_ARCHITECTURE_DECISION_REGISTER.md)
- [2026-08-10 multi-perspective architecture refinements](ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-10.md)
- [Gameplay and product architecture horizon](GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md)
- [Architecture analysis gap register](ARCHITECTURE_ANALYSIS_GAP_REGISTER.md)
- [Multichannel system scope matrix](MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md)

## Core ADRs

- [ADR-0001 — Native Rust stack and multichannel-first platform](ADR-0001-native-rust-multichannel-platform.md)
- [ADR-0002 — Repository ownership and client migration](ADR-0002-repository-ownership-and-client-migration.md)
- [ADR-0003 — Platform Identity/Game Gateway/admission boundary](ADR-0003-platform-identity-game-gateway-and-admission-boundary.md)
- [ADR-0004 — PostgreSQL and data ownership](ADR-0004-postgresql-and-data-ownership.md)
- [ADR-0005 — Native world format and Oteryn Studio boundary](ADR-0005-native-world-format-and-oteryn-studio.md)
- [ADR-0006 — Game Intelligence, analytics and audit](ADR-0006-game-intelligence-analytics-and-audit.md)
- [ADR-0007 — Native end-to-end test platform](ADR-0007-native-end-to-end-test-platform.md)
- [ADR-0008 — protocol-canary reference-only disposition](ADR-0008-protocol-canary-reference-only-migration-disposition.md)
- [ADR-0009 — GameNode capacity, deployment and recovery](ADR-0009-game-node-execution-capacity-deployment-and-recovery-baseline.md)
- [ADR-0010 — Reference and evolved world product profiles](ADR-0010-reference-and-evolved-world-product-profiles.md)
- [ADR-0011 — Native client pre-protocol migration state](ADR-0011-native-client-pre-protocol-migration-state.md)
- [ADR-0012 — Character authority and Platform lifecycle boundary](ADR-0012-character-authority-and-platform-lifecycle-boundary.md)
- [ADR-0013 — Platform database technology independence](ADR-0013-platform-database-technology-independence.md)
- [ADR-0014 — TCP-default, QUIC-opt-in dual gameplay transport strategy](ADR-0014-dual-gameplay-transport-tcp-default-quic-opt-in.md)
- [ADR-0015 — GameNode implementation shape is not yet frozen](ADR-0015-gamenode-implementation-shape-not-yet-frozen.md)
- [ADR-0016 — Gameplay transport client-mode runtime readiness](ADR-0016-gameplay-transport-client-mode-runtime-readiness.md)

## Accepted foundation contracts

- [FND-ID-01 — Foundation identifier contract](FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md)
- [FND-02 — protocol-oteryn v1](FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md)
- [FND-03 — Runtime execution](FND-03_RUNTIME_EXECUTION_CONTRACT.md)
- [FND-04 — Identity, session, admission and CharacterLease](FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md)
- [DUR-01 — Durable identifier representation](DUR-01_DURABLE_IDENTIFIER_REPRESENTATION_CONTRACT.md)
- [ANL-01 — Game event and audit foundation](ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md)

## Accepted gameplay, product, durability and determinism contracts

- [GAME-ITEM-01 — Item model and equipment analysis](GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_ANALYSIS.md)
- [GAME-ITEM-01 — Item model and equipment contract](GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md)
- [DUR-03 — Item transaction and anti-duplication analysis](DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_ANALYSIS.md)
- [DUR-03 — Item transaction and anti-duplication contract](DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md)
- [GAME-CHANNEL-01 — Channel product policy analysis](GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_ANALYSIS.md)
- [GAME-CHANNEL-01 — Channel product policy contract](GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md)
- [DUR-04 — Content, world detail and scripting analysis](DUR-04_CONTENT_WORLD_AND_SCRIPTING_ANALYSIS.md)
- [DUR-04 — Content, world detail and scripting contract](DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md)
- [SIM-DETERMINISM-01 — Authoritative simulation determinism analysis](SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_ANALYSIS.md)
- [SIM-DETERMINISM-01 — Authoritative simulation determinism contract](SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md)
- [Reference evidence/parity manifest v1 owner acceptance](REFERENCE_EVIDENCE_PARITY_MANIFEST_V1_OWNER_ACCEPTANCE.md)
- [GAME-ABILITY-01 — First Reference evidence and pending fixture package](GAME-ABILITY-01_FIRST_REFERENCE_EVIDENCE_FIXTURE_PACKAGE.md)

`GAME-ITEM-01`, `DUR-03`, `GAME-CHANNEL-01`, `DUR-04` and `SIM-DETERMINISM-01` are accepted/lifecycle-closed architecture with implementation `NOT_STARTED`. GAME-ITEM + DUR-03 freeze typed item legality, one durable item location, ItemInstanceId lifecycle transitions, conservation/source-sink lineage, idempotency/retry/ambiguous-commit handling, runtime↔durable pickup/drop fencing, custody and bounded audit. GAME-CHANNEL freezes player Channel selection/recommendation/queue/co-location, durable anti-hopping/prior-Channel semantics, explicit source/reward multiplicity classes, qualitative public Channel create/drain/drain-abort/retirement predicates, same-Channel recovery and one-World community/economy boundaries while leaving all numeric capacity/windows/hysteresis to PERF/OPS. DUR-04 freezes stable semantic package/content identity, deterministic locked compilation, immutable bundle staging/activation/migration, bounded fail-closed loading and the target Component Model + project-owned WIT capability boundary with proposal-only authoritative mutations and versioned deterministic execution-profile semantics. SIM-DETERMINISM freezes explicit numeric/rounding/failure semantics, purpose-isolated deterministic gameplay RNG, exact semantic revision binding, logical time/order, normalized external nondeterminism, replay provenance, future-determining state hashing/divergence evidence and supported-target determinism. None of these acceptances authorizes Rust/runtime/client/compiler/loader/Studio/SIM/combat/AI/script implementation, exact formulas or RNG/numeric/hash dependencies, PostgreSQL DDL/migrations, Platform writes, broad content import, production mutation or entitlement activation.

The accepted Reference evidence/parity registry remains a paper-only evidence authority. Schema v1 is pinned and manifest revision 3 is `ACCEPTED`; `ABILITY_COMBAT` has four registered Light Healing/Ice Strike cases delivered by PR #255 merge `d04f0939f0078cb677ca3ad66f5949e9f3dadc8d`. Agent A PR #271 later completed the bounded continuity/provenance pass with **0/4 promoted**. All four remain fail-closed: target `UNKNOWN`, source/case provenance `PENDING`, legal review `PENDING`, implementation `NOT_STARTED`, parity `PARITY_PENDING_EVIDENCE`. The human package contains pending fixture blueprints only; it is not executable content and does not establish mechanic parity.

## Delivered first-wave proposals/candidates — not yet owner-accepted

These files are merged and their delivery lifecycles are closed, but their `DecisionStatus` remains proposal/candidate as shown. Do not list them under accepted contracts until a later explicit owner-decision package says so.

- [GAME-ABILITY-01 whole-gate analysis](GAME-ABILITY-01_WHOLE_GATE_GAP_ANALYSIS.md) + [candidate](GAME-ABILITY-01_WHOLE_GATE_CONTRACT_CANDIDATE.md) — `CANDIDATE / LIFECYCLE_CLOSED / NOT_STARTED`, PR #268 merge `0cfd8d8ee3ecf4fbb1cb76cbc9680b53a152e3c1`.
- [GAME-AI-01 analysis](GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_ANALYSIS.md) + [successor candidate](GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_CONTRACT_CANDIDATE.md) — `PROPOSED / LIFECYCLE_CLOSED / NOT_STARTED`, successor PR #276 merge `f1bd64a62b9392223589e6b0609149570f5a76b5`; predecessor #272 superseded.
- [GAME-INTERACTION-01 successor analysis](GAME-INTERACTION-01_SUCCESSOR_CHILD_IDENTITY_RETRY_ANALYSIS.md) + [successor candidate](GAME-INTERACTION-01_SUCCESSOR_CHILD_IDENTITY_RETRY_CONTRACT_CANDIDATE.md) — `PROPOSED / LIFECYCLE_CLOSED / NOT_STARTED`, successor PR #277 merge `c8d8ae20471acf004db7bbf6015a2d1b710aa8af`; predecessor #269 superseded.
- [ALPHA-CLIENT-01 analysis](ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_ANALYSIS.md) + [candidate](ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_CONTRACT_CANDIDATE.md) — `CANDIDATE / LIFECYCLE_CLOSED / NOT_STARTED`, PR #273 merge `b7f239a32081fc43f5d3306517eadde850b5be6b`.
- [ANL-02 analysis](ANL-02_GAMEPLAY_BALANCE_WORLD_ANALYTICS_ANALYSIS.md) + [candidate](ANL-02_GAMEPLAY_BALANCE_WORLD_ANALYTICS_CONTRACT_CANDIDATE.md) — `CANDIDATE / LIFECYCLE_CLOSED / NOT_STARTED`, PR #270 merge `32ff2ae75530cb9334463833462eb02c44dc435b`.
- [ANL-03 analysis](ANL-03_ECONOMY_INTEGRITY_SECURITY_ANALYTICS_ANALYSIS.md) + [candidate](ANL-03_ECONOMY_INTEGRITY_SECURITY_ANALYTICS_CONTRACT_CANDIDATE.md) — `CANDIDATE / LIFECYCLE_CLOSED / NOT_STARTED`, same PR #270.

## Machine-readable contracts

- [Transport policy](../contracts/PROTOCOL_OTERYN_TRANSPORT_POLICY.json) — one `protocol-oteryn`; TCP+TLS 1.3 profile `1` is the currently registered initial/default architecture profile, while **all gameplay transport runtime modes remain unavailable until implementation is separately authorized and proven**; QUIC is a future player-opt-in target blocked on protocol/FND-04 transport-profile reconciliation and evidence; no 0-RTT/DATAGRAM baseline.
- [Game event foundation registry](../contracts/GAME_EVENT_FOUNDATION_REGISTRY.json)
- [Resource limits registry](../contracts/RESOURCE_LIMITS_REGISTRY.json)
- [Cross-repository contract lock](../contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json)
- [Reference evidence/parity manifest v1](../contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json)
- [Reference evidence/parity manifest v1 schema](../contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.schema.json)

## Current programme dependency refinement

The current ordering constraints do not claim runtime implementation:

```text
accepted GAME-VISION-01
+ accepted GAME-CHANNEL-01

accepted GAME-CHAR-01
-> accepted DUR-02 Character persistence envelope

accepted GAME-ITEM-01
+ accepted DUR-01/DUR-02/ANL-01
-> accepted DUR-03 transaction/conservation architecture
-> runtime item mutation still separately unauthorized

accepted DUR-04 content/world/scripting architecture
+ accepted SIM-DETERMINISM-01
-> accepted/pinned Reference evidence/parity manifest v1
-> first representative ABILITY_COMBAT cases delivered fail-closed
-> Agent A continuity/provenance pass completed: 0/4 promoted
-> merged GAME-ABILITY whole-gate candidate
-> next: bounded GAME-ABILITY-01 owner-decision package
-> later real-boundary VSL sequence only after accepted owning architecture + explicit implementation authority
```

The selected next bounded paper-only programme action is the **`GAME-ABILITY-01` owner-decision package**. It consumes the merged whole-gate analysis/candidate, all accepted GAME-ABILITY partial baselines and the canonical Agent-A 0/4 fail-closed result. It must apply the architecture decision-timing test, preserve cross-domain ownership and `UNKNOWN/PENDING` Reference truth, and obtain an explicit owner disposition without implementing runtime or claiming parity. After that decision is delivered/lifecycle-closed, re-evaluate the merged GAME-AI, GAME-INTERACTION, ALPHA-CLIENT and ANL-02/ANL-03 packages before selecting their owner-decision order. Any executable server/persistence/channel/item/content/SIM/client/analytics increment still requires separate explicit implementation authority.

## Transport rule

`ADR-0014` accepts the dual-transport strategy but does not register QUIC as an authoritative transport profile and does not claim any gameplay transport runtime implementation. For the explicitly refined deferred-QUIC strategy scope, ADR-0014 takes precedence; FND-02 remains authoritative everywhere else, including the current transport registry, application protocol, framing/semantic requirements, sequencing, revisions, bounded inputs and measured-benefit prerequisite.

`ADR-0016` clarifies that transport mode names such as `TCP_ONLY` are future mode vocabulary until their transport/runtime path is separately implemented and proven; the machine-readable transport policy remains authoritative for current runtime availability.

`ADR-0015` clarifies that the modular-monolith GameNode wording in the 2026-08-10 programme refinement is a preferred starting hypothesis for **internal module/crate decomposition**, not a frozen decomposition. It explicitly preserves ADR-0009's accepted boundary that one `GameNode` is the logical identity of one game-server process; any separately deployed adjacent service remains a distinct process/authority boundary unless a future dedicated ADR explicitly supersedes ADR-0009.

Functional gameplay networking requires later implementation authority and proof. Functional QUIC admission/recovery additionally requires a later accepted delivery that registers a stable QUIC transport profile and reconciles both FND-04 fresh-admission and reauthenticated-recovery grant contracts. No QUIC adapter, library choice, endpoint rollout or production traffic is authorized by ADR-0014 alone.