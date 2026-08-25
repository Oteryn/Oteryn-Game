# OTV2-20260826-impl-game-ai-bootstrap

```yaml
task_id: OTV2-20260826-impl-game-ai-bootstrap
title: Implement bounded pure-local AI bootstrap
mode: IMPLEMENT
status: ready_for_integration
repository: Oteryn/Oteryn-Game
base_branch: main
branch: impl/game-ai-bootstrap
issue: 174
pr: null
base_sha: f3ba25234791d981d32a5f1d901803454ed4a6cb
head_sha: 4611b8e781e0c61a683f6a01f957b5819cebf58d
final_head_sha: null
owner: Oteryn: impl ai
created_at: 2026-08-26T00:13:00+02:00
updated_at: 2026-08-26T00:54:00+02:00
execution_budget_minutes: 120
large_budget_reason: deterministic bounded AI proof with eight independent ceilings
owned_paths:
  - apps/game-server/src/ai/mod.rs
  - apps/game-server/src/ai/snapshot.rs
  - apps/game-server/src/ai/perception.rs
  - apps/game-server/src/ai/resolution.rs
  - apps/game-server/src/ai/path_proposal.rs
  - apps/game-server/src/ai/tests.rs
  - apps/game-server/tests/ai_bootstrap.rs
  - docs/agents/tasks/active/OTV2-20260826-impl-game-ai-bootstrap.md
public_contracts:
  - OTERYN_GAME_AI_BOOTSTRAP_SLICE_OWNER_DECISION_2026-08-25
integration_admission_base_sha: f3ba25234791d981d32a5f1d901803454ed4a6cb
final_merge_sha_guard: recorded_before_worker_dispatch
resource_limits:
  AI01-ACTIVE-ACTORS: 256
  AI01-AUTHORED-UNITS: 4
  AI01-EVALUATION-WORK: 8
  AI01-PERCEPTION-CANDIDATES: 64
  AI01-PATH-REQUESTS-PER-ACTOR: 2 (slice config <=1)
  AI01-PATH-SEARCH-WORK: 1024
  AI01-ROUTE-STEPS: 128
  AI01-ROUTE-BYTES: 4096
depends_on: [issue:162 allocation merge, issue:174, main@f3ba25234791d981d32a5f1d901803454ed4a6cb]
blocks: []
external_repositories: []
```

## Outcome

Bounded deterministic pure-local AI snapshot/perception/resolution/path-proposal evidence without gameplay-authoritative integration.

## Acceptance criteria

- [ ] Deterministic tie/order and stale provenance negatives pass.
- [ ] Every eight ceilings has max/max+1 and overflow fail-closed proof.
- [ ] No direct foreign mutation API is available; excluded dimensions are unreachable.
- [ ] Exact-head CI/review/merge/archive lifecycle completes.

## Excluded scope

No Ability, Interaction, Movement, persistence/value/reward, spawn/timer/retry/script/controller, protocol/content/schema/production/Reference authority.

## Context checkpoint

```yaml
last_progress: exact CI formatter job 97998843405 selected the final active-actor test signature layout; that one non-semantic hunk was applied in 4611b8e781e0c61a683f6a01f957b5819cebf58d
status: ready_for_integration
branch: impl/game-ai-bootstrap
head_sha: 4611b8e781e0c61a683f6a01f957b5819cebf58d
pr: null
final_head_sha: null
owner_action_required: null
blocker: local cargo/rustfmt are unavailable; exact-head Rust CI and independent review remain required
next_action: coordinator publishes this local worker result, verifies exact head, and starts the allocated review/CI lifecycle
```
