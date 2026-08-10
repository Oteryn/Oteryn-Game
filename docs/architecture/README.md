# Oteryn v2 Architecture Index

This directory contains the canonical architecture decisions, contracts, current-status overlays and planning registers for Oteryn-v2.

## Source hierarchy

Use this order when documents overlap:

1. explicit owner instruction and repository governance;
2. an accepted ADR/contract that explicitly owns the domain;
3. an explicit later superseding ADR/contract for the named scope;
4. `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` for current progression/status wording;
5. actively maintained review refinements and decision registers;
6. historical analysis, evidence and archived task records.

A newer date alone does not supersede an accepted semantic contract. Supersession must be explicit about what changes and what remains binding.

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
- [ADR-0013 — Platform database technology independence](ADR-0013-platform-database-technology-independence.md)
- [ADR-0014 — TCP-default, QUIC-opt-in dual gameplay transport strategy](ADR-0014-dual-gameplay-transport-tcp-default-quic-opt-in.md)

## Accepted foundation contracts

- [FND-ID-01 — Foundation identifier contract](FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md)
- [FND-02 — protocol-oteryn v1](FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md)
- [FND-03 — Runtime execution](FND-03_RUNTIME_EXECUTION_CONTRACT.md)
- [FND-04 — Identity, session, admission and CharacterLease](FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md)
- [DUR-01 — Durable identifier representation](DUR-01_DURABLE_IDENTIFIER_REPRESENTATION_CONTRACT.md)
- [ANL-01 — Game event and audit foundation](ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md)

## Machine-readable contracts

- [Transport policy](../contracts/PROTOCOL_OTERYN_TRANSPORT_POLICY.json) — one `protocol-oteryn`; current registered transport is TCP+TLS 1.3 profile `1`; QUIC is a future player-opt-in target and is blocked until protocol/FND-04 transport-profile reconciliation; no 0-RTT/DATAGRAM baseline.
- [Game event foundation registry](../contracts/GAME_EVENT_FOUNDATION_REGISTRY.json)
- [Resource limits registry](../contracts/RESOURCE_LIMITS_REGISTRY.json)
- [Cross-repository contract lock](../contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json)

## Current programme dependency refinement

The 2026-08-10 review adds the following ordering constraints without claiming runtime implementation:

```text
GAME-VISION-01 minimum
+ GAME-CHANNEL-01

GAME-CHAR-01
-> final character-bearing DUR-02

GAME-ITEM-01
+ accepted DUR-02
-> DUR-03

SIM-DETERMINISM-01
+ minimal DUR-04 headless toolchain
-> small real-boundary VSL sequence
```

`DUR-02` discovery may continue from already accepted DUR-01/ANL-01 inputs, but its final durable character semantics wait for the character lifecycle/product gate.

## Transport rule

`ADR-0014` accepts the dual-transport strategy but does not register QUIC as an authoritative transport profile. FND-02 remains authoritative for the current transport registry, application protocol, framing/semantic requirements, sequencing, revisions, bounded inputs and related invariants.

Functional QUIC admission/recovery requires a later accepted delivery that registers a stable QUIC transport profile and reconciles both FND-04 fresh-admission and reauthenticated-recovery grant contracts. No QUIC adapter, library choice, endpoint rollout or production traffic is authorized by ADR-0014 alone.
