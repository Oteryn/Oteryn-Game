# OTV2-20260818-implementation-coordinator

```yaml
task_id: OTV2-20260818-implementation-coordinator
title: Coordinate first native implementation wave
mode: COORDINATE
status: bootstrap_allocated
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/otv2-20260818-implementation-coordinator-bootstrap-base
pr: pending
base_sha: ed84415f4a55d8c16f703b7c1a130c0e43a1c1a1
owner: chat-github-20260818-implementation-coordinator
created_at: 2026-08-18T16:10:00+02:00
updated_at: 2026-08-18T16:27:00+02:00
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
  - OTV2-IMPL-BOOTSTRAP
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
```

## Outcome

Coordinate the explicitly invoked `Oteryn: implementation coordinator` programme from current canonical `Oteryn/Oteryn-Game` main. The first released worker allocation is serial `OTV2-IMPL-BOOTSTRAP`; later lanes remain allocation-gated until Bootstrap is merged and archived.

## Live classification

- `PROVEN`: PR #8 merged the coordinator allocation as `86200e6d044287bcb2fbb122d224e825b9084a7a` after exact-head governance checks.
- `PROVEN`: canonical repository is `Oteryn/Oteryn-Game`; legacy `blakinio/Oteryn-v2` is read-only historical source under current root governance.
- `PROVEN`: Bootstrap is the serial first lane; direct later workers remain allocation-gated.
- `PROVEN`: current workspace is the accepted 19-member pre-native baseline, only `apps/client` exists, native server/protocol/session/persistence fragments are forbidden by machine policy, and architecture-check still hard-codes the historical 19-member shape.
- `PROVEN`: no active implementation path ownership overlaps Bootstrap.
- `PROVEN`: no production/protected/live-data/Platform/external-repository authority is granted.

## Allocation state

`docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md` now binds `OTV2-IMPL-BOOTSTRAP` to exact merged allocation base `86200e6d044287bcb2fbb122d224e825b9084a7a` and dependency PR #8. Worker writes remain blocked until this base-binding update itself is merged to `main`.

## Merge discipline

Stable workspace policy is serialized under Bootstrap. No sibling-branch output is consumable. Later Foundation/SIM/Domain/Content/QA allocations are not released yet.

## Context checkpoint

```yaml
last_progress: Coordinator allocation PR #8 merged; exact Bootstrap base is being rebound to the merged allocation SHA before worker writes.
status: bootstrap_allocated
branch: docs/otv2-20260818-implementation-coordinator-bootstrap-base
head_sha: pending
pr: pending
blocker: null
owner_action_required: null
next_action: Merge the exact-base allocation update, then create OTV2-20260818-impl-bootstrap from main at that merged SHA.
```
