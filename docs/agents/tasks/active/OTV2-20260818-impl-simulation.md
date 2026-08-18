# OTV2-20260818-impl-simulation

```yaml
task_id: OTV2-20260818-impl-simulation
title: Implement deterministic simulation core
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: feat/otv2-20260818-impl-simulation
pr: 14
base_sha: 977e98b05738076744540a123d4e35c32cd94c2c
allocation_base_sha: 2fc59dd83a3d13e7de8954d4dbcce5415e346389
owner: worker-otv2-impl-simulation
created_at: 2026-08-18T17:36:00+02:00
updated_at: 2026-08-18T17:38:00+02:00
execution_budget_minutes: 60
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
depends_on:
  - PR #12 / 2fc59dd83a3d13e7de8954d4dbcce5415e346389
  - PR #13 / 977e98b05738076744540a123d4e35c32cd94c2c
blocks:
  - later Wave 1 root-workspace allocations until terminal closeout
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
```

## Authority and scope

- `PROVEN`: PR #12 allocated `OTV2-IMPL-SIM`; PR #13 merged the exact-base coordination state before this worker write.
- `PROVEN`: worker branch starts from current `main@977e98b05738076744540a123d4e35c32cd94c2c`; the semantic allocation merge is `2fc59dd83a3d13e7de8954d4dbcce5415e346389`, and the difference is coordinator-only allocation bookkeeping.
- `PROVEN`: accepted SIM architecture requires deterministic numeric/RNG/time/order/hash machinery but leaves concrete implementation artifact/profile selection to implementation evidence.
- `PROVEN`: allocation permits implementation-owned profile revision `1` only for semantics implemented and tested here.
- `PROVEN`: no gameplay formula/Reference value, protocol/session/admission, persistence schema/durable transaction, security-randomness authority, production deployment or external-repository write is authorized.

## Target outcome

Implement one production `oteryn-simulation-determinism` crate and consume it immediately from `apps/game-server` while gameplay remains fail-closed.

Minimum scope:

1. typed `SimulationDeterminismProfileRevision` and immutable active profile identity;
2. checked exact integer and fixed-scale ratio arithmetic with explicit named rounding and fail-closed errors;
3. deterministic domain-separated gameplay decision derivation using stable occurrence + purpose + draw identity, with retry stability and purpose isolation;
4. normalized semantic time values with no system-clock read;
5. bounded canonical key/value state hashing independent of insertion order and rejecting duplicate keys;
6. Linux/Windows-stable golden fixtures for decision/hash outputs;
7. game-server exposes the active determinism profile without enabling gameplay;
8. workspace membership, machine role/edges and production closure remain valid.

## Acceptance criteria

- [ ] production crate exists and has an immediate game-server consumer;
- [ ] no process-global mutable RNG or wall-clock authority exists;
- [ ] numeric overflow/divide-zero/out-of-range behavior is deterministic and tested;
- [ ] retry/purpose/draw isolation has fixed golden tests;
- [ ] canonical hash is insertion-order independent, bounded and duplicate-key fail-closed;
- [ ] exact golden outputs pass on Linux and Windows CI;
- [ ] `Cargo.lock` change is minimal for one local workspace package and no unrelated transitive upgrade;
- [ ] workspace metadata/fmt/build/clippy/tests/cargo-deny/architecture checks pass on exact final head;
- [ ] full-diff self-review is clean; independent review is `NOT_REQUIRED` unless final scope introduces a root-policy high-risk boundary;
- [ ] squash merge, post-merge verification, archive and ownership release complete.

## Context checkpoint

```yaml
last_progress: Exact allocation/base coordination is merged and draft PR #14 is open from exact current main; task metadata is durable before product writes.
status: implementing
branch: feat/otv2-20260818-impl-simulation
head_sha: pending
pr: 14
blocker: null
owner_action_required: null
next_action: Implement the bounded simulation-determinism crate, game-server consumer, workspace policy and minimal lockfile delta.
```
