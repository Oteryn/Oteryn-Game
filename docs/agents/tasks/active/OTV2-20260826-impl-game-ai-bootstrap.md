# OTV2-20260826-impl-game-ai-bootstrap

```yaml
task_id: OTV2-20260826-impl-game-ai-bootstrap
title: Implement bounded pure-local AI bootstrap
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: impl/game-ai-bootstrap
issue: 174
pr: null
base_sha: null
head_sha: null
final_head_sha: null
owner: Oteryn: impl ai
created_at: 2026-08-26T00:13:00+02:00
updated_at: 2026-08-26T00:13:00+02:00
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
integration_admission_base_sha: d86d2c5ad001ae563b371558d202a30b0ac3a062
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
depends_on: [issue:162 allocation merge, issue:174, main@d86d2c5ad001ae563b371558d202a30b0ac3a062]
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
last_progress: task packet created from owner-accepted bootstrap decision; worker waits for allocation merge
status: waiting
branch: impl/game-ai-bootstrap
head_sha: null
pr: null
final_head_sha: null
owner_action_required: null
blocker: allocation authority is unmerged
next_action: coordinator merges exact allocation and records worker base SHA
```
