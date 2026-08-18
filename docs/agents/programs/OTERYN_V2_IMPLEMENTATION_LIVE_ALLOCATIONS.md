# Oteryn v2 Implementation Live Allocations

- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Coordinator task: `OTV2-20260818-implementation-coordinator`
- Canonical repository: `Oteryn/Oteryn-Game`
- Initial allocation PR: `#8`
- Bootstrap delivery PR: `#10`
- Bootstrap merge: `0809004252db228e8f3fac3cdb6638c3c2a7fbda`
- State: `BOOTSTRAP_COMPLETED_WAVE1_PENDING_ALLOCATION`

## Authority rule

This record is the live coordinator allocation required by `OTV2_IMPLEMENTATION_COORDINATOR.md`. Root governance is higher authority than historical prompt coordinates: all writes target `Oteryn/Oteryn-Game`; `blakinio/Oteryn-v2` remains read-only history.

Only lanes explicitly listed as `allocated` have worker write authority. At this closeout checkpoint there is **no active implementation allocation**. Later executor aliases remain read-only until the coordinator publishes a new exact-base allocation on `main`.

## Completed allocation — Bootstrap

```yaml
lane_id: OTV2-IMPL-BOOTSTRAP
task_id: OTV2-20260818-impl-bootstrap
worker_alias: Oteryn: impl bootstrap
status: completed
execution_mode: serial
allocation_merge_sha: 86200e6d044287bcb2fbb122d224e825b9084a7a
worker_base_sha: d9c5ef68e1c88b88b4782219051395eacb0f8e67
final_head_sha: 43243c4998224517a4c828bc05e735264b3e3394
delivery_pr: 10
delivery_merge_sha: 0809004252db228e8f3fac3cdb6638c3c2a7fbda
owned_paths: []
branch: null
```

Bootstrap delivered the real foundation-only `apps/game-server` production root, structural workspace/production-root machine policy and static CI validation while keeping Canary/protocol/session/persistence/gameplay semantics fail-closed and deferred.

## Next dependency-ready wave

After this Bootstrap archive/ownership-release closeout is merged, the coordinator may publish bounded allocations for the first dependency-ready wave:

- `OTV2-IMPL-FOUNDATION`;
- `OTV2-IMPL-SIM`;
- `OTV2-IMPL-DOMAIN`;
- `OTV2-IMPL-CONTENT`;
- `OTV2-IMPL-QA`.

Those allocations must name exact post-closeout base SHA, non-overlapping owned paths, stable registry/workspace serialization rules and prerequisite merge order before any worker writes. Foundation is high-risk and requires genuinely independent exact-head review when it implements protocol/session/admission/fencing semantics.

## Deferred allocations

`DURABILITY`, `ABILITY`, `INTERACTION`, `AI`, `CLIENT`, `MOVE`, `COMBAT`, `CHANNEL`, `ANALYTICS` and `CONTENT-FORMAT-SPIKE` remain **not allocated** until their DAG prerequisites are concretely merged and the coordinator publishes a bounded allocation.
