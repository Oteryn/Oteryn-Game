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
- Simulation archive PR: `pending`
- State: `SIM_COMPLETED_CLOSEOUT_PENDING`

## Authority rule

This record is the live coordinator allocation required by `OTV2_IMPLEMENTATION_COORDINATOR.md`. Root governance is higher authority than historical prompt coordinates: all writes target `Oteryn/Oteryn-Game`; `blakinio/Oteryn-v2` remains read-only history.

Only lanes explicitly listed as `allocated` have worker write authority. During this closeout there is **no active implementation worker allocation**; all later executor aliases remain read-only until the coordinator publishes a new exact-base bounded allocation on `main`.

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
archive_pr: pending
owned_paths: []
branch: null
```

SIM delivered a real production `oteryn-simulation-determinism` crate consumed by `apps/game-server`, including checked deterministic numeric semantics, retry/purpose-isolated decision derivation, semantic time, bounded canonical state hashing and exact-head Windows golden fixtures. Gameplay remains fail-closed and no protocol/session/persistence/Reference/security-randomness authority was introduced.

## Next Wave 1 state

`FOUNDATION`, `DOMAIN`, `CONTENT` and `QA` are dependency-ready only in principle and are currently **not allocated**. After this SIM archive/ownership-release closeout merges, the coordinator may publish exactly bounded non-overlapping allocations using the then-current exact `main` SHA.

`FOUNDATION` carries a mandatory genuinely independent exact-head review gate when it implements protocol/session/admission/fencing semantics. No coordination record may weaken or bypass that requirement.

## Deferred allocations

`DURABILITY`, `ABILITY`, `INTERACTION`, `AI`, `CLIENT`, `MOVE`, `COMBAT`, `CHANNEL`, `ANALYTICS` and `CONTENT-FORMAT-SPIKE` remain **not allocated** until their DAG prerequisites are concretely merged and the coordinator publishes a bounded allocation.
