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
- State: `WAVE1_ALLOCATION_PENDING_EXACT_BASE`

## Authority rule

This record is the live coordinator allocation required by `OTV2_IMPLEMENTATION_COORDINATOR.md`. Root governance is higher authority than historical prompt coordinates. All implementation writes governed by this record target the canonical `Oteryn/Oteryn-Game` repository only.

Only lanes explicitly listed as `allocated` have worker write authority. A lane listed as `allocated_pending_exact_base` is reserved but remains **read-only** until a later coordinator exact-base bind has merged to `main` and this record names its exact `worker_base_sha`.

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

SIM delivered a real production `oteryn-simulation-determinism` crate consumed by `apps/game-server`, including checked deterministic numeric semantics, retry/purpose-isolated decision derivation, semantic time, bounded canonical state hashing and exact-head Windows golden fixtures. Gameplay remains fail-closed and no protocol/session/persistence/Reference/security-randomness authority was introduced.

## Wave 1 allocation gate

The coordinator resolved live `main` immediately before this allocation as `7694c8a5e1ebc1dbffa937adf6b5cb775f7745f2` (post-SIM launch-pack PR #44). The four dependency-ready lanes below are path-reserved only by allocation PR #45. They are deliberately `allocated_pending_exact_base`, so **no Wave 1 worker currently has write authority**.

The exact-base bind MUST be a later coordinator PR created only after allocation PR #45 merges. That bind must use the exact allocation merge SHA as every Wave 1 worker's `worker_base_sha`. Worker task files are created only after that exact-base bind is merged.

### Reserved lane — Foundation

```yaml
lane_id: OTV2-IMPL-FOUNDATION
task_id: OTV2-20260822-impl-foundation-runtime
worker_alias: Oteryn: impl foundation runtime
status: allocated_pending_exact_base
risk: XHigh
allocation_pr: 45
branch: agent/otv2-impl-foundation-runtime-01
allocation_source_main_sha: 7694c8a5e1ebc1dbffa937adf6b5cb775f7745f2
worker_base_sha: pending_exact_allocation_merge
owned_paths:
  - apps/game-server/src/foundation/**
  - docs/agents/tasks/active/OTV2-20260822-impl-foundation-runtime.md
public_contracts_read_only:
  - docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
  - docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md
  - docs/contracts/FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md
independent_review_required: true
```

Foundation owns the new server-side protocol/runtime/admission module family only after exact-base bind. It does **not** receive authority to invent post-15s recovery behavior, resource ceilings, gameplay command/state IDs, persistence semantics or a new crate topology. Protocol/session/admission/fencing delivery requires genuinely independent exact-head review.

### Reserved lane — Domain

```yaml
lane_id: OTV2-IMPL-DOMAIN
task_id: OTV2-20260822-impl-domain-core
worker_alias: Oteryn: impl domain core
status: allocated_pending_exact_base
risk: High
allocation_pr: 45
branch: agent/otv2-impl-domain-core-01
allocation_source_main_sha: 7694c8a5e1ebc1dbffa937adf6b5cb775f7745f2
worker_base_sha: pending_exact_allocation_merge
owned_paths:
  - apps/game-server/src/domain/**
  - docs/agents/tasks/active/OTV2-20260822-impl-domain-core.md
```

Domain owns protocol/persistence-neutral Character/Item/Inventory/Equipment/Ability-definition semantics required by the accepted first slice. It does not own wire IDs, persistence mechanics, UI or Reference-unknown product values.

### Reserved lane — Content

```yaml
lane_id: OTV2-IMPL-CONTENT
task_id: OTV2-20260822-impl-vsl-content
worker_alias: Oteryn: impl vsl content
status: allocated_pending_exact_base
risk: High
allocation_pr: 45
branch: agent/otv2-impl-vsl-content-01
allocation_source_main_sha: 7694c8a5e1ebc1dbffa937adf6b5cb775f7745f2
worker_base_sha: pending_exact_allocation_merge
owned_paths:
  - apps/game-server/src/content/**
  - docs/agents/tasks/active/OTV2-20260822-impl-vsl-content.md
```

Content owns the minimum typed VSL content/compiler/loader seam and bounded synthetic/evidence fixtures required by accepted Stage-C contracts. It must not select the permanent physical bundle/world encoding or introduce gameplay C++/Blueprint authority.

### Reserved lane — QA

```yaml
lane_id: OTV2-IMPL-QA
task_id: OTV2-20260822-impl-qa-e2e
worker_alias: Oteryn: impl qa e2e
status: allocated_pending_exact_base
risk: High
allocation_pr: 45
branch: agent/otv2-impl-qa-e2e-01
allocation_source_main_sha: 7694c8a5e1ebc1dbffa937adf6b5cb775f7745f2
worker_base_sha: pending_exact_allocation_merge
owned_paths:
  - apps/game-server/tests/**
  - docs/agents/tasks/active/OTV2-20260822-impl-qa-e2e.md
```

QA owns only real-boundary integration/E2E proof under the server integration-test path. It may evolve incrementally as real seams merge; mocks or test-only success adapters are never terminal proof and may not enter production artifacts.

## Serialized shared-mutation lease

The four code lanes above are non-overlapping. The following composition/workspace paths are **not** concurrently owned by any worker and remain coordinator-serialized:

```yaml
shared_paths:
  - apps/game-server/src/lib.rs
  - apps/game-server/Cargo.toml
  - Cargo.toml
  - Cargo.lock
  - workspace-boundaries.toml
lease_state: reserved_inactive_until_exact_base_bind
initial_lease_order:
  - OTV2-IMPL-FOUNDATION
  - OTV2-IMPL-DOMAIN
  - OTV2-IMPL-CONTENT
  - OTV2-IMPL-QA
```

A worker that needs one of these paths must stop that mutation and request the coordinator lease; the coordinator may grant only one shared-path writer at a time. `docs/contracts/**`, stable-ID registries, `.github/workflows/**`, architecture policy/tooling and any new workspace/crate topology are **not allocated** by this wave. Any proven need to mutate them requires a separate explicit coordinator allocation update before the mutation.

Wave 1 code may develop in parallel after exact-base bind. Shared composition/integration mutations and merge sequencing remain serialized. Foundation receives the first shared lease because it establishes the real protocol/runtime/admission endpoint and its high-risk review evidence before downstream composition relies on that seam.

## Deferred allocations

`DURABILITY`, `ABILITY`, `INTERACTION`, `AI`, `CLIENT`, `MOVE`, `COMBAT`, `CHANNEL`, `ANALYTICS` and `CONTENT-FORMAT-SPIKE` remain **not allocated** until their DAG prerequisites are concretely merged and the coordinator publishes a bounded allocation.
