# OTV2-20260818-implementation-coordinator

```yaml
task_id: OTV2-20260818-implementation-coordinator
title: Coordinate first native implementation wave
mode: COORDINATE
status: wave1_active
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-coordinator-wave1-active
pr: null
base_sha: 79e2f3baf17bd3b2231ab71c5dc5019e9aa0441e
head_sha: null
owner: chat-github-20260818-implementation-coordinator
created_at: 2026-08-18T16:10:00+02:00
updated_at: 2026-08-22T19:08:00+02:00
execution_budget_minutes: 60
owned_paths:
  - docs/agents/tasks/active/OTV2-20260818-implementation-coordinator.md
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
public_contracts:
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md
  - docs/agents/prompts/OTV2_POST_SIM_WAVE1_PARALLEL_LAUNCH.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
depends_on:
  - OTV2-20260805-foundation-preimplementation-contracts
  - Oteryn-Game#44
  - Oteryn-Game#45
  - Oteryn-Game#46
blocks: []
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
```

## Outcome

Coordinate the explicitly invoked `Oteryn: implementation coordinator` programme on canonical `Oteryn/Oteryn-Game`, keeping dependency-ready implementation lanes moving under exact-base, path-ownership, serialized-shared-mutation and review discipline.

## Current proven state

- `PROVEN`: Bootstrap and SIM lifecycles are terminally completed.
- `PROVEN`: Wave 1 allocation PR #45 merged as `33cec30b8075c73290d7d76e9f59df4701771650`.
- `PROVEN`: exact-base bind PR #46 merged as `fd39c6aa026e82062a8b29af24811d467c115f19`.
- `PROVEN`: FOUNDATION, DOMAIN, CONTENT and QA worker branches/task records exist and originate from post-bind main.
- `PROVEN`: current live main was re-resolved at `79e2f3baf17bd3b2231ab71c5dc5019e9aa0441e`; intervening PR #47 is unrelated read-only audit-prompt documentation.
- `PROVEN`: full `cargo test --workspace` PASS on live main using the authorized Molehill-PC checkout.
- `PROVEN`: FOUNDATION observed head `5dd9c528338adc7463ef0e8fa4453b2941d3255f` passes 6 focused tests and game-server Clippy `-D warnings`.
- `FINDING`: FOUNDATION is not closeout-ready; GameSession/CharacterLease/admission and FND-02 state snapshot/delta/resync acceptance remain missing.
- `PROVEN`: DOMAIN observed head `28aa20468bf3cb3f2406078d4249525087d16e10` passes 5 standalone tests and production `rustc -D warnings` compile.
- `PROVEN`: CONTENT observed head `7b07a8cd9d82e1063700f2e78f8a772d8a6dfcb5` and QA observed head `63350b3a165cabc378af1b5497e6a506d78f1453` currently contain only their start/task checkpoints; implementation evidence remains absent.
- `PROVEN`: registered FOUNDATION limits used by current code (`1,048,576` frame bytes and `64` outstanding commands) match `RESOURCE_LIMITS_REGISTRY.json`; they are not guessed constants.
- `PROVEN`: shared composition/workspace lease remains with FOUNDATION. Stable contract/registry/CI/policy/new-crate mutations remain outside allocation.
- `PROVEN`: FOUNDATION still requires genuinely independent exact-head review before merge of protocol/session/admission/fencing delivery.

## Active Wave 1 execution

```yaml
foundation:
  branch: agent/otv2-impl-foundation-runtime-01
  observed_head: 5dd9c528338adc7463ef0e8fa4453b2941d3255f
  state: implementing_incomplete
  shared_lease: active

domain:
  branch: agent/otv2-impl-domain-core-01
  observed_head: 28aa20468bf3cb3f2406078d4249525087d16e10
  state: implementing_green_standalone
  shared_lease: waiting

content:
  branch: agent/otv2-impl-vsl-content-01
  observed_head: 7b07a8cd9d82e1063700f2e78f8a772d8a6dfcb5
  state: recovery_required_no_code_progress
  shared_lease: waiting

qa:
  branch: agent/otv2-impl-qa-e2e-01
  observed_head: 63350b3a165cabc378af1b5497e6a506d78f1453
  state: recovery_required_no_code_progress
  shared_lease: waiting
```

## Merge discipline

FOUNDATION may mutate the serialized shared composition/workspace paths; DOMAIN/CONTENT/QA may write only their primary allocated paths until the coordinator advances the lease. No lane is allowed to convert architecture acceptance into Reference parity, production deployment, Platform authority or a permanent content format.

A worker PR is not merge-ready merely because focused tests pass. Acceptance checklist completeness, current-main reconciliation, exact-head CI, required review classification and unresolved material findings all remain gates.

## Context checkpoint

```yaml
last_progress: Wave 1 is live; main baseline is green; FOUNDATION and DOMAIN kernels were directly validated; FOUNDATION has a material incompleteness finding; CONTENT and QA have no code progress and are candidates for autonomous recovery.
status: wave1_active
branch: agent/otv2-coordinator-wave1-active
head_sha: pending_final_freeze
pr: null
blocker: null
owner_action_required: null
next_action: publish this reconciliation, recover CONTENT and QA in their allocated paths, and keep FOUNDATION moving through missing FND-02/FND-04 seams without weakening its independent-review gate.
```
