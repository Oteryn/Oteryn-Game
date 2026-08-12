# Oteryn v2 Architecture Index

This directory contains the canonical architecture decisions, contracts, current-status overlays and planning registers for Oteryn-v2.

## Source hierarchy

Use this order when documents overlap:

1. explicit owner instruction and repository governance;
2. an explicit later ADR/contract that states it supersedes the older authority for the named scope;
3. the accepted ADR/contract that otherwise owns the domain;
4. `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` for current progression/status wording;
5. actively maintained review refinements and decision registers;
6. historical analysis, evidence and archived task records.

Supersession precedence applies **only** to the scope explicitly named by the later decision. The older owning contract remains authoritative everywhere else. A newer date alone never supersedes an accepted semantic contract.

Architecture acceptance is not runtime implementation. Use [Architecture Status Model](ARCHITECTURE_STATUS_MODEL.md) to distinguish decision, delivery and implementation state.

## Current architecture entry points

- [Foundation programme current status](FOUNDATION_PROGRAMME_CURRENT_STATUS.md)
- [Foundation decision backlog](FOUNDATION_DECISION_BACKLOG.md)
- [Global architecture decision register](GLOBAL_ARCHITECTURE_DECISION_REGISTER.md)
- [2026-08-10 multi-perspective architecture refinements](ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-10.md)
- [Gameplay and product architecture horizon](GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md)
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

## Accepted gameplay and durability contracts

- [GAME-ITEM-01 — Item model and equipment analysis](GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_ANALYSIS.md)
- [GAME-ITEM-01 — Item model and equipment contract](GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md)
- [DUR-03 — Item transaction and anti-duplication analysis](DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_ANALYSIS.md)
- [DUR-03 — Item transaction and anti-duplication contract](DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md)

`GAME-ITEM-01` and `DUR-03` are accepted/lifecycle-closed architecture with implementation `NOT_STARTED`. Together with DUR-01/DUR-02/ANL-01 and the accepted runtime/session authority contracts they freeze the semantic envelope for durable item/currency/value mutation: typed item legality, one durable item location, ItemInstanceId lifecycle transitions, conservation/source-sink lineage, idempotency/retry/ambiguous-commit handling, runtime↔durable pickup/drop fencing, typed custody and bounded durable evidence. They do **not** authorize Rust/runtime implementation, PostgreSQL DDL/migrations, production mutation or entitlement activation.

## Machine-readable contracts

- [Transport policy](../contracts/PROTOCOL_OTERYN_TRANSPORT_POLICY.json) — one `protocol-oteryn`; TCP+TLS 1.3 profile `1` is the currently registered initial/default architecture profile, while **all gameplay transport runtime modes remain unavailable until implementation is separately authorized and proven**; QUIC is a future player-opt-in target blocked on protocol/FND-04 transport-profile reconciliation and evidence; no 0-RTT/DATAGRAM baseline.
- [Game event foundation registry](../contracts/GAME_EVENT_FOUNDATION_REGISTRY.json)
- [Resource limits registry](../contracts/RESOURCE_LIMITS_REGISTRY.json)
- [Cross-repository contract lock](../contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json)

## Current programme dependency refinement

The current ordering constraints do not claim runtime implementation:

```text
GAME-VISION-01 minimum
+ GAME-CHANNEL-01

GAME-CHAR-01
-> accepted DUR-02 Character persistence envelope

accepted GAME-ITEM-01
+ accepted DUR-01/DUR-02/ANL-01
-> accepted DUR-03 transaction/conservation architecture
-> runtime item mutation still separately unauthorized

SIM-DETERMINISM-01
+ minimal DUR-04 headless toolchain
-> small real-boundary VSL sequence
```

After DUR-03 lifecycle closeout, `GAME-CHANNEL-01` is the earliest still-unresolved paper-only gate in the owner-accepted recommended ordering. Reference evidence/parity tooling, `DUR-04` and `SIM-DETERMINISM-01` remain independently ownable paper-only work. Any executable server/persistence/gameplay increment still requires separate explicit implementation authority.

## Transport rule

`ADR-0014` accepts the dual-transport strategy but does not register QUIC as an authoritative transport profile and does not claim any gameplay transport runtime implementation. For the explicitly refined deferred-QUIC strategy scope, ADR-0014 takes precedence; FND-02 remains authoritative everywhere else, including the current transport registry, application protocol, framing/semantic requirements, sequencing, revisions, bounded inputs and measured-benefit prerequisite.

`ADR-0016` clarifies that transport mode names such as `TCP_ONLY` are future mode vocabulary until their transport/runtime path is separately implemented and proven; the machine-readable transport policy remains authoritative for current runtime availability.

`ADR-0015` clarifies that the modular-monolith GameNode wording in the 2026-08-10 programme refinement is a preferred starting hypothesis for **internal module/crate decomposition**, not a frozen decomposition. It explicitly preserves ADR-0009's accepted boundary that one `GameNode` is the logical identity of one game-server process; any separately deployed adjacent service remains a distinct process/authority boundary unless a future dedicated ADR explicitly supersedes ADR-0009.

Functional gameplay networking requires later implementation authority and proof. Functional QUIC admission/recovery additionally requires a later accepted delivery that registers a stable QUIC transport profile and reconciles both FND-04 fresh-admission and reauthenticated-recovery grant contracts. No QUIC adapter, library choice, endpoint rollout or production traffic is authorized by ADR-0014 alone.
