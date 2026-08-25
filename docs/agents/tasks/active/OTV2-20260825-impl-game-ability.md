# OTV2-20260825-impl-game-ability

```yaml
task_id: OTV2-20260825-impl-game-ability
title: Implement bounded ability occurrence and effect-plan engine
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: impl/game-ability-engine
issue: 166
pr: null
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: impl ability
created_at: 2026-08-25T23:24:03+02:00
updated_at: 2026-08-25T23:24:03+02:00
execution_budget_minutes: 120
large_budget_reason: revision-bound idempotent effect pipeline with deterministic boundary proof
owned_paths:
  - apps/game-server/src/ability/mod.rs
  - apps/game-server/src/ability/occurrence.rs
  - apps/game-server/src/ability/intent.rs
  - apps/game-server/src/ability/plan.rs
  - apps/game-server/src/ability/commit.rs
  - apps/game-server/src/ability/effects.rs
  - apps/game-server/src/ability/tests.rs
  - apps/game-server/tests/ability_engine.rs
  - docs/agents/tasks/active/OTV2-20260825-impl-game-ability.md
public_contracts:
  - GAME-ABILITY-01 whole-gate acceptance
depends_on:
  - issue:162 allocation merge
  - issue:166
  - main@c57ddb5253cdfec126a768232d53f8a9bb292e3f or recorded successor
blocks:
  - future Combat readiness only after serial integration prerequisites
external_repositories: []
```

## Outcome

Revision-bound, deterministic, idempotent occurrence/intent/plan/commit pipeline with immediate stateless typed damage/heal fixtures.

## Architecture and source of truth

- `PROVEN`: limits are 2 target candidates, 2 resolved targets, 2 effect-plan entries, 4096 effect-plan bytes and 8 calculation stages.
- `PROVEN`: fixture behavior is structural evidence, not Reference parity; unregistered mechanics remain excluded.
- `DERIVED`: `apps/game-server/src/lib.rs` composition is a coordinator-only serialized lease, so this task uses its allocated standalone test harness before post-delivery composition.

## Acceptance criteria

- [ ] Occurrence/revision and retry no-double-commit proof passes.
- [ ] Partial/sequential groups and deterministic calculation/effect order pass.
- [ ] Each allocated limit has max/max+1 proof before effects.
- [ ] Proposal-only/direct-mutation negatives, exact-head CI/review/merge/archive lifecycle pass.

## Excluded scope

No formula/geometry/retarget/multihit/channel/timer/cooldown/condition/reaction invention; no durability/value, Movement, protocol/UI/persistence/production Content, Cargo/workspace/registry mutation.

## Validation

### Focused

- command/run: pending allocation merge
- result: pending

### Component/integration

- command/run: pending allocation merge
- result: pending

### E2E

- scenario: `NOT_APPLICABLE` until real listener/client boundary exists
- result: `NOT_APPLICABLE`

## Context checkpoint

```yaml
last_progress: task packet created; no worker authority before allocation merge
status: waiting
branch: impl/game-ability-engine
head_sha: null
pr: null
final_head_sha: null
owner_action_required: null
blocker: allocation authority is unmerged
next_action: coordinator merges exact allocation then records worker base SHA
```
