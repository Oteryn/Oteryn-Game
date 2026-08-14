# OTV2-20260815-game-interaction-architecture

```yaml
task_id: OTV2-20260815-game-interaction-architecture
title: Design GAME-INTERACTION-01 world interaction architecture
mode: ARCHITECTURE
status: active
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/arch-d-game-interaction
pr: null
base_sha: 088b46638ac014cd7928d6b0b75cee44902fe22c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: DOMAIN_ARCHITECTURE_DESIGN_AGENT_D
created_at: 2026-08-15T00:18:00+02:00
updated_at: 2026-08-15T00:18:00+02:00
execution_budget_minutes: 60
owned_paths:
  - docs/agents/tasks/active/OTV2-20260815-game-interaction-architecture.md
  - docs/architecture/GAME-INTERACTION-01_WORLD_INTERACTION_ANALYSIS.md
  - docs/architecture/GAME-INTERACTION-01_WORLD_INTERACTION_CONTRACT_CANDIDATE.md
public_contracts:
  - docs/architecture/GAME-INTERACTION-01_WORLD_INTERACTION_CONTRACT_CANDIDATE.md
depends_on:
  - FND-03 accepted authoritative runtime execution contract
  - GAME-CHANNEL-01 accepted channel product policy
  - GAME-ITEM-01 accepted item model/equipment contract
  - DUR-03 accepted item transaction and anti-duplication contract
  - DUR-04 accepted content/world/scripting contract
  - SIM-DETERMINISM-01 accepted authoritative simulation contract
  - GAME-ABILITY-01 owner-accepted partial baselines; overall gate remains open
blocks:
  - GAME-INTERACTION-01 implementation authority until coordinator canonicalization and later implementation task
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Produce a bounded, paper-only GAME-INTERACTION-01 analysis and contract candidate for coordinator audit. Cover interaction intent/command routing, server authority, deterministic object state transitions, doors/switches/levers/teleports/fields/traps/hazards, readable/writable objects, item-use routing, movement/timer-triggered interaction, multichannel scope, reset/persistence/recovery/content-revision migration, script capability boundaries and anti-abuse/resource-limit requirements.

The task grants **no runtime, protocol, database, Platform, production or merge authority**.

## Architecture/source truth

Primary authority consumed by this worker:

- `AGENTS.md` and `AGENTS.override.md`;
- `docs/agents/AGENTS.md`;
- `docs/agents/prompts/OTV2_DOMAIN_ARCHITECTURE_DESIGN_AGENT.md`;
- `docs/agents/MULTI_AGENT_ARCHITECTURE_ORCHESTRATION.md`;
- `docs/agents/programs/OTERYN_V2_ARCHITECTURE_PARALLEL_WORK_ALLOCATION.md`;
- `docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md`;
- `docs/architecture/GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md`;
- `docs/architecture/GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md`;
- `docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md`;
- `docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md`;
- `docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md`;
- GAME-ABILITY-01 owner-accepted partial baselines;
- foundation error/resource/failure registries.

## Acceptance checklist

- [ ] analysis covers required GAME-INTERACTION-01 taxonomy and alternatives
- [ ] candidate declares authoritative owner and scope matrix
- [ ] item legality/location/value conservation remain delegated to GAME-ITEM/DUR-03
- [ ] ability/combat semantics remain delegated to GAME-ABILITY
- [ ] scripts remain bounded proposal-only components under DUR-04
- [ ] world/channel/instance state obeys FND-03 and GAME-CHANNEL ownership
- [ ] deterministic ordering/replay requirements obey SIM-DETERMINISM-01
- [ ] reset, persistence, recovery and content-revision migration are explicit
- [ ] anti-abuse/resource classes are explicit without invented numeric limits
- [ ] deterministic acceptance/failure scenarios are included
- [ ] `DECISIONS_NOT_TAKEN` and `CROSS_DOMAIN_FINDINGS` are explicit
- [ ] focused governance/policy validation passes
- [ ] exact-head ordinary PR CI is inspected
- [ ] full diff self-review is complete
- [ ] draft PR remains draft for Architecture Coordinator audit

## Excluded

- Rust/runtime/script implementation;
- protocol/client UI layout;
- PostgreSQL DDL/migrations;
- Platform or external-repository changes;
- GAME-ITEM item legality/location semantics;
- DUR-03 value-conservation mechanics;
- GAME-ABILITY targeting/combat/effect formula ownership;
- AI behavior policy;
- global architecture/status overlays;
- Reference-specific interaction formulas/ordering without sufficient evidence;
- production activation, deployment or merge.

## Findings

Cross-domain findings will be recorded in the candidate/PR using the orchestration schema and routed to the coordinator; this worker will not edit sibling-domain authority.

## Validation

### Focused

Pending final candidate content.

### Component

Documentation/governance validators only; no runtime component exists in this scope.

### E2E

`NOT_APPLICABLE` — paper-only architecture change with no executable/runtime boundary.

### Exact-head CI

Pending final remote head.

## Self-review

Pending full final diff review.

## Independent review

Architecture Coordinator audit is the only authorized next review stage for this worker lane. No owner-funded external reviewer is authorized for this task.

## PR closeout

Worker closeout is limited to `INTEGRATION_READY — DRAFT PR — COORDINATOR ACTION REQUIRED`. The worker must not merge, mark ready-for-review, archive this task or perform lifecycle closeout.

## Context checkpoint

```yaml
last_progress: Trusted worker lane verified at main@088b46638ac014cd7928d6b0b75cee44902fe22c; live main drift is bookkeeping-only and no GAME-INTERACTION owned-path collision was found.
status: active
branch: docs/arch-d-game-interaction
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: false
blocker: null
next_action: Author GAME-INTERACTION-01 analysis and contract candidate on the owned branch, then run focused validation and prepare the draft PR for Architecture Coordinator audit.
```

MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY
