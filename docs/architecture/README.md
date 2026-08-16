# Oteryn v2 Architecture Index

This directory contains canonical architecture decisions, owner baselines, current-status overlays, planning registers and historical analysis for Oteryn-v2.

## Source hierarchy

When documents overlap, use this order:

1. explicit owner instruction and repository governance;
2. an explicit later owner-acceptance baseline / ADR / contract that names the superseded scope;
3. the accepted ADR/contract that owns the domain;
4. `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` for current execution/status wording;
5. `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md` and other actively maintained coordination surfaces;
6. historical proposal/candidate analysis, evidence and archived task records.

A newer date alone never supersedes semantic authority. Supersession applies only to the scope explicitly named.

Architecture acceptance is not runtime implementation or Reference parity. See `ARCHITECTURE_STATUS_MODEL.md`.

## Current entry points

- [Foundation programme current status](FOUNDATION_PROGRAMME_CURRENT_STATUS.md) — current three-axis status and next safe gate.
- [Global architecture decision register](GLOBAL_ARCHITECTURE_DECISION_REGISTER.md) — stable gate IDs, accepted state and remaining horizon.
- [Remaining first-wave owner acceptance baseline](OTERYN_V2_REMAINING_FIRST_WAVE_OWNER_ACCEPTANCE_BASELINE_20260816.md) — owner acceptance of GAME-INTERACTION, ALPHA-CLIENT, GAME-AI and ANL-02/03.
- [GAME-ABILITY whole-gate owner acceptance baseline](GAME-ABILITY-01_WHOLE_GATE_OWNER_ACCEPTANCE_BASELINE.md).
- [Foundation decision backlog](FOUNDATION_DECISION_BACKLOG.md) — stable historical gate definitions; current execution wording may be superseded by current status.
- [Gameplay/product architecture horizon](GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md) — detailed later product horizon.
- [Architecture decision discipline](../agents/ARCHITECTURE_DECISION_DISCIPLINE.md).

## Core accepted ADRs

- ADR-0001 — native Rust stack and multichannel-first platform.
- ADR-0002 — repository ownership and client migration.
- ADR-0003 — Platform Identity / Game Gateway / final game admission boundary.
- ADR-0004 — PostgreSQL and data ownership.
- ADR-0005 — native world format and Oteryn Studio boundary.
- ADR-0006 — Game Intelligence, analytics and audit.
- ADR-0007 — native three-tier end-to-end test platform.
- ADR-0008 — `protocol-canary` reference-only disposition.
- ADR-0009 — GameNode process/capacity/deployment/recovery baseline.
- ADR-0010 — Reference/Evolved world product profiles.
- ADR-0011 — native client pre-protocol fail-closed state.
- ADR-0012 — Character authority and Platform lifecycle boundary.
- ADR-0013 — Platform database technology independence.
- ADR-0014 — TCP-default / future QUIC-opt-in one-protocol strategy.
- ADR-0015 — GameNode internal implementation shape not frozen.
- ADR-0016 — gameplay transport mode vocabulary does not imply runtime readiness.

## Accepted foundation / durability / gameplay contracts

Current accepted architecture includes:

- `FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`;
- `FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md`;
- `FND-03_RUNTIME_EXECUTION_CONTRACT.md`;
- `FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md`;
- `DUR-01_DURABLE_IDENTIFIER_REPRESENTATION_CONTRACT.md`;
- `DUR-02_PERSISTENCE_V1_OWNER_BASELINE.md` plus Character persistence sub-baseline;
- `DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md`;
- `DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md`;
- `ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md`;
- `GAME-VISION-01_MINIMUM_OWNER_BASELINE.md` and immutable first Reference baseline;
- `GAME-CHAR-01_STAGE_A_OWNER_BASELINE.md` + Stage B baseline;
- `GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md`;
- `GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md`;
- `SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md`;
- `GAME-ABILITY-01_WHOLE_GATE_OWNER_ACCEPTANCE_BASELINE.md`;
- `OTERYN_V2_REMAINING_FIRST_WAVE_OWNER_ACCEPTANCE_BASELINE_20260816.md`.

## Historical first-wave proposal/candidate artifacts

The following remain important immutable design/review history but are no longer the current DecisionStatus authority after explicit owner acceptance:

- `GAME-ABILITY-01_WHOLE_GATE_GAP_ANALYSIS.md` + `GAME-ABILITY-01_WHOLE_GATE_CONTRACT_CANDIDATE.md`;
- `GAME-INTERACTION-01_SUCCESSOR_CHILD_IDENTITY_RETRY_ANALYSIS.md` + candidate;
- `ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_ANALYSIS.md` + candidate;
- `GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_ANALYSIS.md` + successor candidate;
- `ANL-02_GAMEPLAY_BALANCE_WORLD_ANALYTICS_ANALYSIS.md` + candidate;
- `ANL-03_ECONOMY_INTEGRITY_SECURITY_ANALYTICS_ANALYSIS.md` + candidate;
- `OTERYN_V2_REMAINING_FIRST_WAVE_OWNER_DECISION_PACKAGE_20260816.md` — resolved historical decision-preparation record.

Do not rewrite those files merely to change their historical proposal/candidate status.

## Reference evidence/parity

- `REFERENCE_EVIDENCE_PARITY_MANIFEST_V1_OWNER_ACCEPTANCE.md` is accepted paper evidence authority.
- Four `ABILITY_COMBAT` Light Healing/Ice Strike cases are registered.
- Agent A #271 promoted **0/4**.
- Target evidence remains `UNKNOWN`, provenance/legal `PENDING`, implementation `NOT_STARTED`, parity `PARITY_PENDING_EVIDENCE`.

Architecture acceptance does not change those facts.

## Current Stage-C blockers before gameplay executors

Three named architecture contracts remain required before their corresponding implementation prompts can be released:

- `VSL-MOVE-01` — movement/collision/floor/teleport/visibility/reconciliation vertical-slice contract;
- `VSL-COMBAT-01` — minimal combat/death/corpse/loot/XP/pickup vertical-slice contract;
- `VSL-CONTENT-01` — minimum World Project/World Bundle/compiler/loader physical vertical-slice contract.

They are registered gates, not yet accepted contracts. `QA-E2E-01` architecture is accepted, but executable evidence remains required for terminal slice proof.

## Machine-readable contracts

- `../contracts/PROTOCOL_OTERYN_TRANSPORT_POLICY.json`;
- `../contracts/GAME_EVENT_FOUNDATION_REGISTRY.json`;
- `../contracts/RESOURCE_LIMITS_REGISTRY.json`;
- `../contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json`;
- `../contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json` and schema.

Machine-readable runtime availability wins over architecture target vocabulary. TCP profile registration does not mean a working gameplay adapter. QUIC remains future profile/reconciliation/evidence work.

## Entitlements

`PROD-ENTITLEMENTS-01_PLATFORM_GAME_ENFORCEMENT_DEPENDENCY.md` pins the satisfied Platform producer prerequisite, but the Oteryn-v2 consumer/enforcement contract remains unaccepted. No Premium/VIP/game-consumed entitlement executor or activation is authorized.

## Current execution rule

```text
accepted first-wave architecture
!= implemented gameplay runtime

before movement/combat/content executors:
  accept VSL-MOVE-01
  accept VSL-COMBAT-01
  accept VSL-CONTENT-01

then:
  audit executor prompts against current accepted architecture
  grant only lane-specific implementation authority
  preserve all resource/evidence/foreign-owner blockers
```

`EXECUTOR_PROMPTS: HOLD`
`IMPLEMENTATION_AUTHORITY: NONE`
