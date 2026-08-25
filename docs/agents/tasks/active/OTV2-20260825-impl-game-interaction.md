# OTV2-20260825-impl-game-interaction

```yaml
task_id: OTV2-20260825-impl-game-interaction
title: Implement bounded deterministic interaction lifecycle
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: impl/game-interaction-lifecycle
issue: 165
pr: null
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: impl interaction
created_at: 2026-08-25T23:24:03+02:00
updated_at: 2026-08-25T23:24:03+02:00
execution_budget_minutes: 120
large_budget_reason: bounded authoritative lifecycle implementation with retry/reconciliation and deterministic limit proof
owned_paths:
  - apps/game-server/src/interaction/mod.rs
  - apps/game-server/src/interaction/identity.rs
  - apps/game-server/src/interaction/plan.rs
  - apps/game-server/src/interaction/lifecycle.rs
  - apps/game-server/src/interaction/dispatch.rs
  - apps/game-server/src/interaction/tests.rs
  - apps/game-server/tests/interaction_workflow.rs
  - docs/agents/tasks/active/OTV2-20260825-impl-game-interaction.md
public_contracts:
  - GAME-INTERACTION-01 successor-child identity/retry acceptance
depends_on:
  - issue:162 allocation merge
  - issue:165
  - main@c57ddb5253cdfec126a768232d53f8a9bb292e3f or recorded successor
blocks:
  - future Movement readiness only after merged interaction and independent prerequisites
external_repositories: []
```

## Outcome

Bounded deterministic interaction lifecycle with truthful retry/reconciliation and no foreign-owner mutation.

## Architecture and source of truth

- `PROVEN`: only registry limits cascade depth 2, child fan-out 8, root work 8, trigger candidates 16 and retained child lifecycles 8 are authorized.
- `PROVEN`: Movement, Ability, DUR-03 and Foundation retain their respective state authority.
- `DERIVED`: `apps/game-server/src/lib.rs` composition is a coordinator-only serialized lease, so this task must use its allocated standalone test harness until post-delivery composition.

## Acceptance criteria

- [ ] Stable identity/retry/reconciliation and deterministic order/RNG have focused proof.
- [ ] All five limits have max/max+1 rejection proof before effects.
- [ ] Duplicate, stale/cancel and foreign-owner direct-write negatives pass.
- [ ] Exact-head CI/review/merge/archive lifecycle is complete.

## Excluded scope

No Movement/handoff, Ability effects, value durability, protocol/events, transport/client/persistence, Cargo/workspace/registry or unregistered limits.

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
branch: impl/game-interaction-lifecycle
head_sha: null
pr: null
final_head_sha: null
owner_action_required: null
blocker: allocation authority is unmerged
next_action: coordinator merges exact allocation then records worker base SHA
```
