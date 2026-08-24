# Oteryn v2 Architecture Index

This directory contains canonical architecture decisions, owner baselines, current-status overlays, planning registers and historical analysis for Oteryn-v2.

## Source hierarchy

When documents overlap, use this order:

1. explicit owner instruction and repository governance;
2. an explicit later owner-acceptance baseline / ADR / contract that names the superseded scope;
3. the accepted ADR/contract that owns the domain;
4. `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` for current execution/status wording;
5. live implementation allocation + exact worker task/branch/PR/CI state for implementation execution truth;
6. `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md` and other actively maintained coordination surfaces;
7. historical proposal/candidate/backlog/checkpoint analysis, evidence and archived task records.

A newer date alone never supersedes semantic authority. Supersession applies only to the scope explicitly named.

Architecture acceptance is not runtime implementation or Reference parity. See `ARCHITECTURE_STATUS_MODEL.md`.

## Current entry points

- [Foundation programme current status](FOUNDATION_PROGRAMME_CURRENT_STATUS.md) — canonical current three-axis status, implementation-start reconciliation and next safe execution state.
- [Implementation live allocations](../agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md) — coordinator-owned exact worker allocations/leases; combine with live worker task/branch/PR/CI state.
- [Global architecture decision register](GLOBAL_ARCHITECTURE_DECISION_REGISTER.md) — stable gate IDs, accepted state and remaining horizon.
- [Stage-C VSL owner acceptance](OTERYN_V2_STAGE_C_VSL_OWNER_ACCEPTANCE_20260816.md) — owner acceptance of `VSL-MOVE-01`, `VSL-COMBAT-01` and `VSL-CONTENT-01`.
- [Remaining first-wave owner acceptance baseline](OTERYN_V2_REMAINING_FIRST_WAVE_OWNER_ACCEPTANCE_BASELINE_20260816.md) — owner acceptance of GAME-INTERACTION, ALPHA-CLIENT, GAME-AI and ANL-02/03.
- [GAME-ABILITY whole-gate owner acceptance baseline](GAME-ABILITY-01_WHOLE_GATE_OWNER_ACCEPTANCE_BASELINE.md).
- [Implementation executor DAG](../agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md) — released dependency/order contract after PR #314 merge.
- [Implementation prompt evaluation](../agents/evidence/OTV2-20260816-final-executor-prompt-evaluation.md) — 17/17 execution prompts PASS across all required prompt gates.
- [Reusable prompt index](../agents/prompts/README.md) — aliases and execution rules; normal implementation entry point is `Oteryn: implementation coordinator`.
- [Foundation decision backlog](FOUNDATION_DECISION_BACKLOG.md) — stable/historical gate definitions; current execution wording is superseded by current status and live implementation state where they differ.
- [Gameplay/product architecture horizon](GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md) — detailed later product horizon; stale implementation-start wording is not current execution authority.
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
- `FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md` plus its A/B/C component contracts/profiles;
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
  - `VSL-CONTENT-01_MINIMAL_NATIVE_CONTENT_SLICE_CONTRACT_CANDIDATE.md`;
- `PROD-ENTITLEMENTS-01_GAME_CONSUMER_ENFORCEMENT_CONTRACT_CANDIDATE.md` as the historical-named artifact accepted later through target PR #20.

Candidate filenames retained for accepted artifacts are historical names and do not by themselves determine current DecisionStatus.

## Historical first-wave / Stage-C preparation artifacts

The following remain immutable design/review history but are no longer current DecisionStatus or execution-status authority after explicit owner acceptance/later reconciliation:

- GAME-ABILITY whole-gate analysis/candidate;
- GAME-INTERACTION successor-child analysis/candidate;
- ALPHA-CLIENT analysis/candidate;
- GAME-AI analysis/successor candidate;
- ANL-02/ANL-03 analyses/candidates;
- `OTERYN_V2_REMAINING_FIRST_WAVE_OWNER_DECISION_PACKAGE_20260816.md`;
- `OTERYN_V2_STAGE_C_VSL_OWNER_DECISION_PACKAGE_20260816.md`.

Do not rewrite historical artifacts merely to retrofit later labels or implementation progress. Use the current-status overlay to state what changed later.

## Reference evidence/parity

- `REFERENCE_EVIDENCE_PARITY_MANIFEST_V1_OWNER_ACCEPTANCE.md` is accepted paper evidence authority.
- Four `ABILITY_COMBAT` cases are registered.
- Agent A #271 promoted **0/4**.
- Target evidence remains `UNKNOWN`, provenance/legal `PENDING`, implementation `NOT_STARTED`, parity `PARITY_PENDING_EVIDENCE`.

Architecture acceptance and implementation progress elsewhere do not change those facts.

## Current implementation state

Implementation has started under the live coordinator programme.

Verified reconciliation snapshot:

```text
main: 79e2f3baf17bd3b2231ab71c5dc5019e9aa0441e
Wave 1 exact-base bind: fd39c6aa026e82062a8b29af24811d467c115f19
BOOTSTRAP: completed and lifecycle-closed
SIM: completed and lifecycle-closed
SIM bounded core: implemented on main via PR #14 / 66619daf5837f31f7c54676e9f8351ed4ae220b0
Wave 1 allocation: merged via PR #45 / 33cec30b8075c73290d7d76e9f59df4701771650
FOUNDATION: implementing on worker branch
DOMAIN: implementing on worker branch
CONTENT: implementing on worker branch
QA: implementing on worker branch
```

PR #47 / `79e2f3baf17bd3b2231ab71c5dc5019e9aa0441e` added an independent read-only programme audit prompt and is disjoint from the implementation allocation/status semantics above.

Active but unmerged worker code is not canonical merged implementation. Resolve exact progress from the live allocation and worker task/branch/PR/CI state.

`SIM-DETERMINISM-01` current merged implementation status is `IMPLEMENTED` for the bounded executor-defined core only. Whole-contract `PROVEN` remains withheld until named downstream replay/consumer/VSL evidence exists.

## Stage-C architecture

`VSL-MOVE-01`, `VSL-COMBAT-01` and `VSL-CONTENT-01` are `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED` on merged main. Exact Reference values remain evidence-gated. Permanent World Project/World Bundle physical encoding remains undecided and still requires the DUR-04 format spike plus later owner decision. `QA-E2E-01` executable evidence remains required for terminal vertical-slice proof.

## FND-04 reconnect/disconnect timing precedence

Accepted FND-04 is current authority over older disconnect checkpoint timing prose.

- historical reconnect/liveness/grace values `2s / 5s / 15s`: **non-canonical / deferred**;
- exact defensive PvE re-entry protection after eligible valid re-entry: **4 seconds accepted**;
- probe cadence/hysteresis/control-loss detection, stale transport cleanup, same-session grace, stable-control re-arm, lease timing and prepared/rate/resource limits require later measured evidence before implementation activation.

Historical lag/disconnect task/checkpoint files remain provenance only where they conflict with FND-04.

## Machine-readable contracts

- `../contracts/PROTOCOL_OTERYN_TRANSPORT_POLICY.json`;
- `../contracts/GAME_EVENT_FOUNDATION_REGISTRY.json`;
- `../contracts/RESOURCE_LIMITS_REGISTRY.json`;
- `../contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json`;
- `../contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json` and schema.

Machine-readable runtime availability wins over architecture target vocabulary. TCP profile registration does not mean a working gameplay adapter. QUIC remains future profile/reconciliation/evidence work.

## Entitlements

`PROD-ENTITLEMENTS-01` Game consumer/enforcement architecture is `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED` after independent exact-head review and PR #20 merge `d40a225e5fedca0396f34b4f2b6c1e343161e6ff`, with lifecycle closeout PR #27 / `84f485089b97cfaba1b5c6628ed8e0ba6655dc51`.

Platform remains commercial entitlement authority; Game owns gameplay enforcement/mutation/result truth. Acceptance does **not** authorize entitlement runtime implementation, Premium/VIP activation, payments, product benefits, physical storage/transport choices or production rollout.

## Current execution rule

```text
EXECUTOR_PROGRAMME: RELEASED_AND_ACTIVE
DEFAULT_ENTRYPOINT: Oteryn: implementation coordinator
DIRECT_WORKERS: ALLOCATION_GATED
IMPLEMENTATION_WORKERS_STARTED: YES
IMPLEMENTATION_AUTHORITY_OUTSIDE_LIVE_COORDINATOR_ALLOCATION: NONE
```

The coordinator continues to serialize shared workspace/registry/stable-ID changes and to release only dependency-ready bounded lanes. High-risk implementation lanes retain genuinely independent exact-head review requirements.

Prompt/architecture acceptance does not grant production/protected-environment/live-data, Platform/external-repository, entitlement activation, Reference-parity or owner-funded-AI authority.

`PRODUCTION_AUTHORITY: NONE`
