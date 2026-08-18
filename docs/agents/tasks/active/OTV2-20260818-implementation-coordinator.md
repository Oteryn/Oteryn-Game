# OTV2-20260818-implementation-coordinator

```yaml
task_id: OTV2-20260818-implementation-coordinator
title: Coordinate first native implementation wave
mode: COORDINATE
status: allocating_bootstrap
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/otv2-20260818-implementation-coordinator-allocation
pr: 8
base_sha: ed84415f4a55d8c16f703b7c1a130c0e43a1c1a1
owner: chat-github-20260818-implementation-coordinator
created_at: 2026-08-18T16:10:00+02:00
updated_at: 2026-08-18T16:24:00+02:00
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

- `PROVEN`: canonical repository is `Oteryn/Oteryn-Game`; legacy `blakinio/Oteryn-v2` is read-only historical source under current root governance.
- `PROVEN`: entry main is `ed84415f4a55d8c16f703b7c1a130c0e43a1c1a1` and protected.
- `PROVEN`: programme status is `ready_for_owner_initiated_implementation`; the owner explicitly invoked the coordinator.
- `PROVEN`: Bootstrap is the serial first lane; direct later workers remain allocation-gated.
- `PROVEN`: current workspace is the accepted 19-member pre-native baseline, only `apps/client` exists, native server/protocol/session/persistence fragments are forbidden by machine policy, and architecture-check still hard-codes the historical 19-member shape.
- `PROVEN`: active disconnect-analysis checkpoints are architecture-only and declare no implementation path ownership; no Bootstrap path overlap is present.
- `PROVEN`: the only unrelated open PR at programme entry is Dependabot PR #2.
- `PROVEN`: no production/protected/live-data/Platform/external-repository authority is granted.

## Allocation state

The canonical live allocation is `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md` in PR #8. It must merge to `main` before the Bootstrap worker performs its first implementation write.

## Merge discipline

Stable workspace policy is serialized under Bootstrap. No sibling-branch output is consumable. Later Foundation/SIM/Domain/Content/QA allocations are not released yet.

## Context checkpoint

```yaml
last_progress: PR #8 contains the serial Bootstrap allocation and coordinator task; persistent diff is coordination-only.
status: allocating_bootstrap
branch: docs/otv2-20260818-implementation-coordinator-allocation
head_sha: pending_final_freeze
pr: 8
blocker: null
owner_action_required: null
next_action: Freeze PR #8, complete exact-head governance CI and squash merge, then start OTV2-IMPL-BOOTSTRAP from the merged allocation.
```
