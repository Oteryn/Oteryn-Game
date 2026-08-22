# Oteryn v2 Implementation Live Allocations

- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Coordinator task: `OTV2-20260818-implementation-coordinator`
- Canonical repository: `Oteryn/Oteryn-Game`
- Bootstrap delivery PR: `#10`
- Bootstrap closeout PR: `#11`
- Simulation allocation PR: `#12`
- Simulation exact-base PR: `#13`
- Simulation delivery PR: `#14`
- Simulation delivery merge: `66619daf5837f31f7c54676e9f8351ed4ae220b0`
- Simulation archive PR: `#15`
- Wave 1 allocation source main: `7694c8a5e1ebc1dbffa937adf6b5cb775f7745f2`
- Wave 1 allocation PR: `#45`
- Wave 1 allocation merge: `33cec30b8075c73290d7d76e9f59df4701771650`
- Wave 1 exact-base PR: `#46`
- Wave 1 exact-base merge: `fd39c6aa026e82062a8b29af24811d467c115f19`
- State: `WAVE1_ACTIVE`

## Authority rule

This record is the live coordinator allocation required by `OTV2_IMPLEMENTATION_COORDINATOR.md`. Root governance is higher authority than historical prompt coordinates. All implementation writes governed by this record target the canonical `Oteryn/Oteryn-Game` repository only.

Only lanes explicitly listed as `allocated` on merged `main` have worker write authority. PR #46 is merged; all four Wave 1 lanes below are live. Unmerged sibling branches are never implicit dependencies.

## Completed allocation — Bootstrap

```yaml
lane_id: OTV2-IMPL-BOOTSTRAP
task_id: OTV2-20260818-impl-bootstrap
status: completed
final_head_sha: 43243c4998224517a4c828bc05e735264b3e3394
delivery_pr: 10
delivery_merge_sha: 0809004252db228e8f3fac3cdb6638c3c2a7fbda
archive_pr: 11
owned_paths: []
branch: null
```

## Completed allocation — Simulation

```yaml
lane_id: OTV2-IMPL-SIM
task_id: OTV2-20260818-impl-simulation
worker_alias: Oteryn: impl simulation
status: completed
execution_mode: serial_workspace_mutation
allocation_pr: 12
allocation_merge_sha: 2fc59dd83a3d13e7de8954d4dbcce5415e346389
exact_base_pr: 13
worker_base_sha: 977e98b05738076744540a123d4e35c32cd94c2c
final_head_sha: 7a0d71bbabdd00c54951aa8e0084d62f3dce748b
delivery_pr: 14
delivery_merge_sha: 66619daf5837f31f7c54676e9f8351ed4ae220b0
archive_pr: 15
owned_paths: []
branch: null
```

## Wave 1 active state

PR #45 reserved the four pairwise non-overlapping primary paths. PR #46 bound all workers to allocation merge `33cec30b8075c73290d7d76e9f59df4701771650` and merged as post-bind main `fd39c6aa026e82062a8b29af24811d467c115f19`. All worker branches/task records were created from that post-bind main.

### Foundation

```yaml
lane_id: OTV2-IMPL-FOUNDATION
task_id: OTV2-20260822-impl-foundation-runtime
worker_alias: Oteryn: impl foundation runtime
status: implementing
risk: XHigh
allocation_pr: 45
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
exact_base_pr: 46
post_bind_main_sha: fd39c6aa026e82062a8b29af24811d467c115f19
branch: agent/otv2-impl-foundation-runtime-01
worker_base_sha: 33cec30b8075c73290d7d76e9f59df4701771650
observed_head_sha: 5dd9c528338adc7463ef0e8fa4453b2941d3255f
owned_paths:
  - apps/game-server/src/foundation/**
  - docs/agents/tasks/active/OTV2-20260822-impl-foundation-runtime.md
shared_lease: active
independent_review_required: true
```

Coordinator verification on 2026-08-22: focused Foundation tests `6/6` PASS and `cargo clippy -p oteryn-game-server --all-targets -- -D warnings` PASS at observed head. This is partial progress only: GameSession/CharacterLease/admission and state snapshot/delta/resync acceptance remain incomplete and must not be represented as delivered.

### Domain

```yaml
lane_id: OTV2-IMPL-DOMAIN
task_id: OTV2-20260822-impl-domain-core
worker_alias: Oteryn: impl domain core
status: implementing
risk: High
allocation_pr: 45
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
exact_base_pr: 46
post_bind_main_sha: fd39c6aa026e82062a8b29af24811d467c115f19
branch: agent/otv2-impl-domain-core-01
worker_base_sha: 33cec30b8075c73290d7d76e9f59df4701771650
observed_head_sha: 28aa20468bf3cb3f2406078d4249525087d16e10
owned_paths:
  - apps/game-server/src/domain/**
  - docs/agents/tasks/active/OTV2-20260822-impl-domain-core.md
shared_lease: waiting_for_foundation
```

Coordinator verification on 2026-08-22: standalone Domain tests `5/5` PASS and production `rustc --edition 2024 --crate-type lib -D warnings` compile PASS. Workspace composition remains intentionally deferred while FOUNDATION owns the shared lease.

### Content

```yaml
lane_id: OTV2-IMPL-CONTENT
task_id: OTV2-20260822-impl-vsl-content
worker_alias: Oteryn: impl vsl content
status: implementing
risk: High
allocation_pr: 45
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
exact_base_pr: 46
post_bind_main_sha: fd39c6aa026e82062a8b29af24811d467c115f19
branch: agent/otv2-impl-vsl-content-01
worker_base_sha: 33cec30b8075c73290d7d76e9f59df4701771650
observed_head_sha: 7b07a8cd9d82e1063700f2e78f8a772d8a6dfcb5
owned_paths:
  - apps/game-server/src/content/**
  - docs/agents/tasks/active/OTV2-20260822-impl-vsl-content.md
shared_lease: waiting_for_foundation
```

Content branch currently contains only its start/task checkpoint; implementation remains unproven.

### QA

```yaml
lane_id: OTV2-IMPL-QA
task_id: OTV2-20260822-impl-qa-e2e
worker_alias: Oteryn: impl qa e2e
status: implementing
risk: High
allocation_pr: 45
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
exact_base_pr: 46
post_bind_main_sha: fd39c6aa026e82062a8b29af24811d467c115f19
branch: agent/otv2-impl-qa-e2e-01
worker_base_sha: 33cec30b8075c73290d7d76e9f59df4701771650
observed_head_sha: 63350b3a165cabc378af1b5497e6a506d78f1453
owned_paths:
  - apps/game-server/tests/**
  - docs/agents/tasks/active/OTV2-20260822-impl-qa-e2e.md
shared_lease: waiting_for_foundation
```

QA branch currently contains only its start/task checkpoint; real evidence-shell implementation remains unproven.

## Serialized shared-mutation lease

```yaml
shared_paths:
  - apps/game-server/src/lib.rs
  - apps/game-server/Cargo.toml
  - Cargo.toml
  - Cargo.lock
  - workspace-boundaries.toml
lease_state: active
current_lease_owner: OTV2-IMPL-FOUNDATION
lease_order:
  - OTV2-IMPL-FOUNDATION
  - OTV2-IMPL-DOMAIN
  - OTV2-IMPL-CONTENT
  - OTV2-IMPL-QA
```

`docs/contracts/**`, stable-ID registries, `.github/workflows/**`, architecture policy/tooling and any new workspace/crate topology remain not allocated. Any proven need to mutate them requires a separate explicit coordinator allocation update before mutation.

Current live `main` may advance for unrelated coordination/documentation work; workers remain bound to the established allocation/base lifecycle and must reconcile current `main` before integration/merge without silently consuming sibling branches.

## Deferred allocations

`DURABILITY`, `ABILITY`, `INTERACTION`, `AI`, `CLIENT`, `MOVE`, `COMBAT`, `CHANNEL`, `ANALYTICS` and `CONTENT-FORMAT-SPIKE` remain not allocated until their DAG prerequisites are concretely merged and the coordinator publishes a bounded allocation.
