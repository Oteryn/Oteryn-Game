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
- Wave 1 exact-base PR: `pending`
- State: `WAVE1_EXACT_BASE_BIND_PENDING`

## Authority rule

This record is the live coordinator allocation required by `OTV2_IMPLEMENTATION_COORDINATOR.md`. Root governance is higher authority than historical prompt coordinates. All implementation writes governed by this record target the canonical `Oteryn/Oteryn-Game` repository only.

Only lanes explicitly listed as `allocated` **on merged `main`** have worker write authority. The exact-base changes below are not authoritative while this coordinator bind branch/PR is unmerged. Unmerged sibling branches are never implicit dependencies.

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

## Wave 1 exact-base bind

Allocation PR #45 passed exact-head governance/merge gates and squash-merged as `33cec30b8075c73290d7d76e9f59df4701771650`. This coordinator bind records that exact allocation merge as every Wave 1 lane's `worker_base_sha`, matching the established SIM #12 → #13 lifecycle.

No worker may write merely because this bind branch exists. Worker authority becomes live only when this exact-base bind itself lawfully merges to `main`. Worker branches/task files must then be created from the resulting post-bind `main` before implementation writes.

### Wave 1 lane — Foundation

```yaml
lane_id: OTV2-IMPL-FOUNDATION
task_id: OTV2-20260822-impl-foundation-runtime
worker_alias: Oteryn: impl foundation runtime
status: allocated
risk: XHigh
allocation_pr: 45
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
exact_base_pr: pending
branch: agent/otv2-impl-foundation-runtime-01
worker_base_sha: 33cec30b8075c73290d7d76e9f59df4701771650
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

Foundation owns the new server-side protocol/runtime/admission module family after this bind merges. It does **not** receive authority to invent post-15s recovery behavior, resource ceilings, gameplay command/state IDs, persistence semantics or a new crate topology. Protocol/session/admission/fencing delivery requires genuinely independent exact-head review.

### Wave 1 lane — Domain

```yaml
lane_id: OTV2-IMPL-DOMAIN
task_id: OTV2-20260822-impl-domain-core
worker_alias: Oteryn: impl domain core
status: allocated
risk: High
allocation_pr: 45
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
exact_base_pr: pending
branch: agent/otv2-impl-domain-core-01
worker_base_sha: 33cec30b8075c73290d7d76e9f59df4701771650
owned_paths:
  - apps/game-server/src/domain/**
  - docs/agents/tasks/active/OTV2-20260822-impl-domain-core.md
```

Domain owns protocol/persistence-neutral Character/Item/Inventory/Equipment/Ability-definition semantics required by the accepted first slice. It does not own wire IDs, persistence mechanics, UI or Reference-unknown product values.

### Wave 1 lane — Content

```yaml
lane_id: OTV2-IMPL-CONTENT
task_id: OTV2-20260822-impl-vsl-content
worker_alias: Oteryn: impl vsl content
status: allocated
risk: High
allocation_pr: 45
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
exact_base_pr: pending
branch: agent/otv2-impl-vsl-content-01
worker_base_sha: 33cec30b8075c73290d7d76e9f59df4701771650
owned_paths:
  - apps/game-server/src/content/**
  - docs/agents/tasks/active/OTV2-20260822-impl-vsl-content.md
```

Content owns the minimum typed VSL content/compiler/loader seam and bounded synthetic/evidence fixtures required by accepted Stage-C contracts. It must not select the permanent physical bundle/world encoding or introduce gameplay C++/Blueprint authority.

### Wave 1 lane — QA

```yaml
lane_id: OTV2-IMPL-QA
task_id: OTV2-20260822-impl-qa-e2e
worker_alias: Oteryn: impl qa e2e
status: allocated
risk: High
allocation_pr: 45
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
exact_base_pr: pending
branch: agent/otv2-impl-qa-e2e-01
worker_base_sha: 33cec30b8075c73290d7d76e9f59df4701771650
owned_paths:
  - apps/game-server/tests/**
  - docs/agents/tasks/active/OTV2-20260822-impl-qa-e2e.md
```

QA owns only real-boundary integration/E2E proof under the server integration-test path. It may evolve incrementally as real seams merge; mocks or test-only success adapters are never terminal proof and may not enter production artifacts.

## Serialized shared-mutation lease

The four primary code/test lane paths are non-overlapping. The following composition/workspace paths remain one-writer-at-a-time under the coordinator lease:

```yaml
shared_paths:
  - apps/game-server/src/lib.rs
  - apps/game-server/Cargo.toml
  - Cargo.toml
  - Cargo.lock
  - workspace-boundaries.toml
lease_state: pending_bind_merge
initial_lease_owner_after_bind: OTV2-IMPL-FOUNDATION
lease_order:
  - OTV2-IMPL-FOUNDATION
  - OTV2-IMPL-DOMAIN
  - OTV2-IMPL-CONTENT
  - OTV2-IMPL-QA
```

After this bind merges, FOUNDATION holds the first shared-path lease. The other three lanes remain free to develop only inside their non-overlapping primary paths until the coordinator advances the shared lease. `docs/contracts/**`, stable-ID registries, `.github/workflows/**`, architecture policy/tooling and any new workspace/crate topology remain **not allocated**; any proven need to mutate them requires a separate explicit coordinator allocation update before mutation.

## Deferred allocations

`DURABILITY`, `ABILITY`, `INTERACTION`, `AI`, `CLIENT`, `MOVE`, `COMBAT`, `CHANNEL`, `ANALYTICS` and `CONTENT-FORMAT-SPIKE` remain **not allocated** until their DAG prerequisites are concretely merged and the coordinator publishes a bounded allocation.
