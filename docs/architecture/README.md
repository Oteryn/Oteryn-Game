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

- [Foundation programme current status](FOUNDATION_PROGRAMME_CURRENT_STATUS.md) — current three-axis status and implementation-handoff state.
- [Global architecture decision register](GLOBAL_ARCHITECTURE_DECISION_REGISTER.md) — stable gate IDs, accepted state and remaining horizon.
- [Stage-C VSL owner acceptance](OTERYN_V2_STAGE_C_VSL_OWNER_ACCEPTANCE_20260816.md) — owner acceptance of `VSL-MOVE-01`, `VSL-COMBAT-01` and `VSL-CONTENT-01`.
- [Remaining first-wave owner acceptance baseline](OTERYN_V2_REMAINING_FIRST_WAVE_OWNER_ACCEPTANCE_BASELINE_20260816.md) — owner acceptance of GAME-INTERACTION, ALPHA-CLIENT, GAME-AI and ANL-02/03.
- [GAME-ABILITY whole-gate owner acceptance baseline](GAME-ABILITY-01_WHOLE_GATE_OWNER_ACCEPTANCE_BASELINE.md).
- [Implementation executor DAG](../agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md) — released dependency/order contract after PR #314 merge.
- [Implementation prompt evaluation](../agents/evidence/OTV2-20260816-final-executor-prompt-evaluation.md) — 17/17 execution prompts PASS across all required prompt gates.
- [Reusable prompt index](../agents/prompts/README.md) — aliases and execution rules; normal implementation entry point is `Oteryn: implementation coordinator`.
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
- `OTERYN_V2_REMAINING_FIRST_WAVE_OWNER_ACCEPTANCE_BASELINE_20260816.md`;
- `OTERYN_V2_STAGE_C_VSL_OWNER_ACCEPTANCE_20260816.md`, accepting the bounded scope in:
  - `VSL-MOVE-01_MINIMAL_MOVEMENT_VISIBILITY_CONTRACT_CANDIDATE.md`;
  - `VSL-COMBAT-01_MINIMAL_COMBAT_DEATH_LOOT_CONTRACT_CANDIDATE.md`;
  - `VSL-CONTENT-01_MINIMAL_NATIVE_CONTENT_SLICE_CONTRACT_CANDIDATE.md`.

The Stage-C candidate filenames remain historical names because they are the exact reviewed artifacts accepted by the later owner-acceptance baseline; do not infer `CANDIDATE` DecisionStatus from the filename.

## Historical first-wave / Stage-C preparation artifacts

The following remain immutable design/review history but are no longer current DecisionStatus authority after explicit owner acceptance:

- GAME-ABILITY whole-gate analysis/candidate;
- GAME-INTERACTION successor-child analysis/candidate;
- ALPHA-CLIENT analysis/candidate;
- GAME-AI analysis/successor candidate;
- ANL-02/ANL-03 analyses/candidates;
- `OTERYN_V2_REMAINING_FIRST_WAVE_OWNER_DECISION_PACKAGE_20260816.md`;
- `OTERYN_V2_STAGE_C_VSL_OWNER_DECISION_PACKAGE_20260816.md`.

Do not rewrite those historical artifacts merely to change their proposal/candidate labels.

## Reference evidence/parity

- `REFERENCE_EVIDENCE_PARITY_MANIFEST_V1_OWNER_ACCEPTANCE.md` is accepted paper evidence authority.
- Four `ABILITY_COMBAT` cases are registered.
- Agent A #271 promoted **0/4**.
- Target evidence remains `UNKNOWN`, provenance/legal `PENDING`, implementation `NOT_STARTED`, parity `PARITY_PENDING_EVIDENCE`.

Architecture acceptance and implementation-prompt release do not change those facts.

## Stage-C architecture

`VSL-MOVE-01`, `VSL-COMBAT-01` and `VSL-CONTENT-01` are `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED` after PR #311 and lifecycle closeout #318. Exact Reference values remain evidence-gated. Permanent World Project/World Bundle physical encoding remains undecided and still requires the DUR-04 format spike plus later owner decision. `QA-E2E-01` executable evidence remains required for terminal vertical-slice proof.

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

When PR #314 merges, the final evaluated implementation prompt package is released, but implementation is **not** started by the merge.

```text
EXECUTOR_PROMPTS: RELEASED
DEFAULT_ENTRYPOINT: Oteryn: implementation coordinator
DIRECT_WORKERS: ALLOCATION_GATED
IMPLEMENTATION_STARTED: NO
```

Normal next action is an explicit owner invocation of:

```text
Oteryn: implementation coordinator
```

The coordinator must then create bounded allocations and execute the serial Bootstrap gate before releasing compatible non-overlapping workers. Direct worker aliases without an active allocation are read-only.

Prompt release does not grant production/protected-environment/live-data, Platform/external-repository, entitlement, Reference-parity or owner-funded-AI authority. High-risk implementation lanes retain genuinely independent exact-head review requirements.

`IMPLEMENTATION_AUTHORITY: NONE_UNTIL_OWNER_INVOCATION`
