# OTV2-20260818-implementation-coordinator

```yaml
task_id: OTV2-20260818-implementation-coordinator
title: Coordinate first native implementation wave
mode: COORDINATE
status: sim_allocated
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/otv2-20260818-sim-exact-base
pr: 13
base_sha: ed84415f4a55d8c16f703b7c1a130c0e43a1c1a1
owner: chat-github-20260818-implementation-coordinator
created_at: 2026-08-18T16:10:00+02:00
updated_at: 2026-08-18T17:33:00+02:00
execution_budget_minutes: 60
owned_paths:
  - docs/agents/tasks/active/OTV2-20260818-implementation-coordinator.md
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
public_contracts:
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
depends_on:
  - OTV2-20260805-foundation-preimplementation-contracts
blocks:
  - other root-workspace Wave 1 allocations while SIM is active
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
```

## Outcome

Coordinate the explicitly invoked `Oteryn: implementation coordinator` programme on canonical `Oteryn/Oteryn-Game`, beginning with serial Bootstrap and then releasing dependency-ready bounded worker lanes under exact-base and path-ownership discipline.

## Current proven state

- `PROVEN`: Bootstrap lifecycle is closed and ownership released.
- `PROVEN`: SIM allocation PR #12 passed exact-head governance and merged as `2fc59dd83a3d13e7de8954d4dbcce5415e346389`.
- `PROVEN`: PR #13 binds `OTV2-IMPL-SIM` to exact worker base `2fc59dd83a3d13e7de8954d4dbcce5415e346389` before any worker write.
- `PROVEN`: SIM owns serialized root Cargo/lock/workspace-policy/CI paths plus `crates/simulation-determinism/**` and the bounded `apps/game-server/**` consumer integration.
- `PROVEN`: no existing stable SIM implementation/profile registry existed before this lane; implementation-owned profile revision `1` is permitted only for implemented/tested semantics and grants no gameplay/Reference/security-randomness authority.
- `PROVEN`: Foundation/Domain/Content/QA remain read-only until SIM releases shared root paths.
- `PROVEN`: Foundation later requires genuinely independent exact-head review for protocol/session/admission/fencing semantics; that gate remains intact.
- `PROVEN`: no production/protected/live-data/Platform/external-repository authority is granted.

## Merge discipline

After PR #13 merges, `OTV2-IMPL-SIM` starts from that exact resulting `main`; no worker write may precede the reconciliation merge. No other implementation lane may mutate SIM-owned root paths until SIM terminal closeout releases ownership.

## Context checkpoint

```yaml
last_progress: SIM allocation PR #12 merged as 2fc59dd83a3d13e7de8954d4dbcce5415e346389; PR #13 binds the worker to that exact allocation merge before implementation.
status: sim_allocated
branch: docs/otv2-20260818-sim-exact-base
head_sha: pending_final_freeze
pr: 13
blocker: null
owner_action_required: null
next_action: Merge PR #13 through exact-head governance CI, then create feat/otv2-20260818-impl-simulation from the resulting main and implement OTV2-IMPL-SIM.
```
