# Oteryn v2 Implementation Live Allocations

- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Coordinator task: `OTV2-20260818-implementation-coordinator`
- Canonical repository: `Oteryn/Oteryn-Game`
- Allocation generation base: `ed84415f4a55d8c16f703b7c1a130c0e43a1c1a1`
- State: `BOOTSTRAP_ALLOCATED_SERIAL`

## Authority rule

This record is the live coordinator allocation required by `OTV2_IMPLEMENTATION_COORDINATOR.md`. Root governance is higher authority than historical prompt coordinates: all writes target `Oteryn/Oteryn-Game`; `blakinio/Oteryn-v2` remains read-only history.

Only lanes explicitly listed as `allocated` below have write authority. All other executor aliases remain read-only. Unmerged sibling branches are not dependencies.

## Active allocation

```yaml
lane_id: OTV2-IMPL-BOOTSTRAP
task_id: OTV2-20260818-impl-bootstrap
worker_alias: Oteryn: impl bootstrap
status: allocated
execution_mode: serial
base_sha: ed84415f4a55d8c16f703b7c1a130c0e43a1c1a1
branch: feat/otv2-20260818-impl-bootstrap
execution_budget_minutes: 60
merge_order: 1
owned_paths:
  - docs/agents/tasks/active/OTV2-20260818-impl-bootstrap.md
  - Cargo.toml
  - Cargo.lock
  - workspace-boundaries.toml
  - apps/game-server/**
  - tools/architecture-check/**
  - .github/workflows/merge-gate.yml
  - .github/workflows/rust.yml
public_contracts:
  - docs/architecture/FND-01_WORKSPACE_AND_RUST_MIGRATION_CONTRACT.md
  - docs/architecture/FND-01_OWNER_ACCEPTANCE_AND_CRATE_FITNESS_REVIEW.md
  - docs/architecture/ADR-0001-native-rust-multichannel-platform.md
  - docs/architecture/ADR-0002-repository-and-code-ownership.md
  - docs/architecture/ADR-0008-canary-reference-only.md
  - docs/architecture/ADR-0011-pre-native-protocol-fail-closed.md
  - docs/architecture/ADR-0015-gamenode-implementation-shape-not-yet-frozen.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
dependency_prs: []
requires_merged_allocation: true
excluded_scope:
  - gameplay movement/combat/content semantics
  - gameplay protocol/session/admission semantics or public IDs
  - persistence schema or migrations
  - permanent content format
  - production deployment or live resources
```

## Bootstrap shape decision

Bootstrap is allocated the smallest real, non-speculative server-side shape consistent with accepted architecture:

- one real `apps/game-server` application/composition root;
- immediate consumption of the already-implemented `oteryn-foundation` seam;
- lifecycle/bootstrap behavior and focused tests only, with no network listener, gameplay protocol, session/admission or persistence authority;
- atomic workspace membership, production-boundary validation and CI updates;
- architecture-check member-count and production-closure rules become structural rather than frozen to the historical client-only 19-member baseline.

No new protocol/session/persistence crate is allocated. Those remain for later Foundation/Durability lanes, preventing speculative empty crates and avoiding high-risk semantic changes in Bootstrap.

## Deferred allocations

`FOUNDATION`, `SIM`, `DOMAIN`, `CONTENT`, `QA`, `DURABILITY`, `ABILITY`, `INTERACTION`, `AI`, `CLIENT`, `MOVE`, `COMBAT`, `CHANNEL`, `ANALYTICS` and `CONTENT-FORMAT-SPIKE` are **not allocated**. The coordinator may release only dependency-ready non-overlapping lanes after Bootstrap is merged and archived.
