# OTV2-20260813-game-ability-targeting-baseline — archived

```yaml
task_id: OTV2-20260813-game-ability-targeting-baseline
title: Record GAME-ABILITY-01 deterministic targeting and legality boundary
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
delivery_branch: docs/game-ability-targeting-baseline
delivery_pr: 228
base_sha: be80a3c6a8a5d3fd71c5a23786d3e34c7572aef3
final_head_sha: 9813e250da4ba08f36a794e464f0b1c6588ba864
delivery_merge_sha: 84da9a923ee504788444074eb71097e77a6463af
lifecycle_closeout_branch: docs/targeting-lifecycle-closeout
lifecycle_closeout_pr: 230
owner: released_after_closeout
created_at: 2026-08-13T17:58:00+02:00
completed_at: 2026-08-13T18:04:30+02:00
execution_budget_minutes: 60
implementation_status: NOT_STARTED
runtime_client_authority: NONE
postgresql_ddl_migration_authority: NONE
platform_write_authority: NONE
production_authority: NONE
owned_paths:
  - docs/agents/tasks/archive/OTV2-20260813-game-ability-targeting-baseline.md
  - docs/architecture/GAME-ABILITY-01_TARGETING_AND_LEGALITY_BOUNDARY_OWNER_BASELINE.md
public_contracts:
  - docs/architecture/GAME-ABILITY-01_TARGETING_AND_LEGALITY_BOUNDARY_OWNER_BASELINE.md
depends_on:
  - docs/architecture/GAME-ABILITY-01_TYPED_EFFECT_PIPELINE_OWNER_BASELINE.md
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
  - docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md
  - docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md
  - docs/architecture/ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md
blocks_released:
  - safe continuation of GAME-ABILITY-01 timing, costs, cooldowns, conditions and effect composition decisions
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
```

## Outcome

Delivered the owner-accepted second bounded `GAME-ABILITY-01` subdecision:

```text
Ability Invocation
-> Target Intent
-> bounded typed Target Query
-> authoritative Target Resolver
-> Resolved Target Set
-> Legality Evaluation
-> Validated Target Set / structured failure
-> typed Effect Plan
```

Ability content describes target policy but cannot supply authoritative final targets. Player, AI, NPC and system origins share one resolver. Target membership/order is bounded and deterministic. Effect planning cannot silently select different targets; chains/jumps/dynamic targeting require explicit bounded resolution steps.

Overall `GAME-ABILITY-01` remains open / `REQUIRED_FOR_ALPHA`.

## Acceptance result

- [x] Added the canonical targeting/legality owner baseline.
- [x] Preserved one resolver pipeline for player, AI, NPC and system origins.
- [x] Separated Target Intent, Target Query, authoritative resolution, legality and effect planning.
- [x] Required deterministic bounded target membership/order and explicit tie-breaking.
- [x] Prevented client/content/Wasm from supplying authoritative final target sets.
- [x] Required explicit bounded re-resolution for chains/jumps/dynamic retargeting.
- [x] Preserved FND-03 scope/ownership, DUR-04 capabilities, SIM determinism and ANL read-only evidence.
- [x] Kept exact gameplay algorithms/values and runtime/protocol/DDL/production authority out of scope.
- [x] Exact-head self-review and required CI passed before merge.

## Delivery validation

Exact delivery head: `9813e250da4ba08f36a794e464f0b1c6588ba864`.

- full two-file diff inspected against typed-effect, FND-03, DUR-04, SIM-DETERMINISM and ANL-01 boundaries;
- self-review: **PASS**, material findings `0`;
- Agent Governance `31718641829`: **PASS**;
- Dependency Review `31718641832`: **PASS**;
- CodeQL `31718641839`: **PASS**;
- unresolved review threads: `0`;
- independent review: **NOT_REQUIRED** for this bounded paper-only partial baseline;
- component/integration/runtime E2E: **NOT_APPLICABLE**;
- squash merge: `84da9a923ee504788444074eb71097e77a6463af`.

Issue creation was unavailable through the connector; the task record, PR #228 and workflow evidence are the durable sources.

## Deliberately unresolved

Exact target/query grammar, geometry catalogue, range/LoS/z-level algorithms, PvP/PZ/friendly-fire/immunity rules, legality/error precedence, partial-target semantics, Reference tie priorities, dynamic retarget snapshot timing, spatial index, protocol/client UX, physical authoring format, cast timing and combat formulas remain later decisions.

## Excluded scope preserved

No Rust runtime, target resolver implementation, combat formula, protocol schema, client UI, database schema/migration, Platform write, production behavior or external-repository mutation was introduced.

## Context checkpoint

```yaml
status: completed
delivery_pr: 228
final_head_sha: 9813e250da4ba08f36a794e464f0b1c6588ba864
delivery_merge_sha: 84da9a923ee504788444074eb71097e77a6463af
lifecycle_closeout_pr: 230
owner_action_required: false
blocker: null
next_action: Continue GAME-ABILITY-01 with cast/channel/interruption timing and cost-commit semantics; do not implement runtime.
```
