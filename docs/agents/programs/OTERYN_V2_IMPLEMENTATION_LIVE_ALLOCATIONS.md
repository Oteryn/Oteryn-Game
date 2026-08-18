# Oteryn v2 Implementation Live Allocations

- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Coordinator task: `OTV2-20260818-implementation-coordinator`
- Canonical repository: `Oteryn/Oteryn-Game`
- Bootstrap delivery PR: `#10`
- Bootstrap closeout PR: `#11`
- Bootstrap closeout main: `231d063ff877b41f01a8032018284fc2f910161e`
- Simulation allocation PR: `#12`
- State: `SIM_ALLOCATION_PENDING_EXACT_BASE`

## Authority rule

This record is the live coordinator allocation required by `OTV2_IMPLEMENTATION_COORDINATOR.md`. Root governance is higher authority than historical prompt coordinates: all writes target `Oteryn/Oteryn-Game`; `blakinio/Oteryn-v2` remains read-only history.

Only lanes explicitly listed below have worker write authority, and a lane marked `allocated_pending_exact_base` remains read-only until the coordinator binds it to the exact merge SHA containing this allocation on `main`.

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

## Wave 1 allocation — Simulation

```yaml
lane_id: OTV2-IMPL-SIM
task_id: OTV2-20260818-impl-simulation
worker_alias: Oteryn: impl simulation
status: allocated_pending_exact_base
execution_mode: serial_workspace_mutation
allocation_pr: 12
allocation_generation_base_sha: 231d063ff877b41f01a8032018284fc2f910161e
worker_base_sha: pending_allocation_merge
branch: feat/otv2-20260818-impl-simulation
execution_budget_minutes: 60
merge_order: 2
owned_paths:
  - docs/agents/tasks/active/OTV2-20260818-impl-simulation.md
  - Cargo.toml
  - Cargo.lock
  - workspace-boundaries.toml
  - crates/simulation-determinism/**
  - apps/game-server/**
  - .github/workflows/rust.yml
  - .github/workflows/merge-gate.yml
public_contracts:
  - docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md
  - docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_ANALYSIS.md
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
dependency_prs:
  - 10
  - 11
excluded_scope:
  - gameplay formulas, rates, XP curves, loot values or Reference behavior
  - transport, protocol framing, admission, Game Session or CharacterLease
  - persistence schemas, durable-value transactions or production database work
  - process-global mutable gameplay RNG
  - production deployment or live resources
```

### Simulation implementation shape

The lane must create a real production `crates/simulation-determinism` library with an immediate consumer in `apps/game-server` in the same delivery. It may use existing workspace `sha2` for deterministic domain-separated decisions/state hashes, but must not add a new third-party dependency or claim cryptographic/security-randomness authority.

The initial profile artifact may define implementation-owned revision `1` only for the semantics implemented and tested in that crate. It must not allocate gameplay command/state/event IDs, Reference formula values or durable database representations.

Required minimum observable outcomes:

- typed `SimulationDeterminismProfileRevision` and explicit profile identity;
- checked exact integer/fixed-scale arithmetic with named deterministic failure/rounding behavior;
- stable purpose-isolated retry-stable deterministic gameplay decision derivation;
- normalized semantic time values with no wall-clock read;
- canonical ordering/serialization/state-hash evidence independent of unordered insertion order;
- game-server composition root consumes and exposes the active determinism profile while gameplay remains unavailable;
- Linux and Windows exact golden fixtures prove cross-target deterministic outputs;
- workspace role/edge policy remains structurally valid and production closure remains clean.

## Other Wave 1 lanes

`FOUNDATION`, `DOMAIN`, `CONTENT` and `QA` are dependency-ready in principle but remain **not allocated** while SIM owns serialized root workspace/lock/policy/CI paths. `FOUNDATION` additionally carries a mandatory genuinely independent exact-head review gate for protocol/session/admission/fencing semantics.

## Deferred allocations

`DURABILITY`, `ABILITY`, `INTERACTION`, `AI`, `CLIENT`, `MOVE`, `COMBAT`, `CHANNEL`, `ANALYTICS` and `CONTENT-FORMAT-SPIKE` remain **not allocated** until their DAG prerequisites are concretely merged and the coordinator publishes a bounded allocation.
