# OTV2-20260818-impl-simulation

```yaml
task_id: OTV2-20260818-impl-simulation
title: Implement deterministic simulation core
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: feat/otv2-20260818-impl-simulation
pr: 14
base_sha: 977e98b05738076744540a123d4e35c32cd94c2c
allocation_base_sha: 2fc59dd83a3d13e7de8954d4dbcce5415e346389
owner: worker-otv2-impl-simulation
created_at: 2026-08-18T17:36:00+02:00
updated_at: 2026-08-18T17:58:00+02:00
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
- `PROVEN`: worker branch starts from `main@977e98b05738076744540a123d4e35c32cd94c2c`; no sibling implementation branch is consumed.
- `PROVEN`: accepted SIM architecture requires deterministic numeric/RNG/time/order/hash machinery but leaves concrete implementation artifact/profile selection to implementation evidence.
- `PROVEN`: allocation permits implementation-owned profile revision `1` only for semantics implemented and tested here.
- `PROVEN`: no gameplay formula/Reference value, protocol/session/admission, persistence schema/durable transaction, security-randomness authority, production deployment or external-repository write is introduced.

## Implemented outcome

- production crate `oteryn-simulation-determinism` with immediate `apps/game-server` consumer;
- immutable typed profile revision `1` and explicit implementation profile/decision/hash identities;
- checked `ExactI64` and fixed-scale arithmetic with named rounding modes and fail-closed overflow/divide-zero/scale/range errors;
- domain-separated SHA-256 gameplay decision derivation from stable root + occurrence + bounded purpose + draw index, with retry/purpose/draw isolation fixtures;
- explicit `SemanticTimeMicros` with checked arithmetic and no system-clock read path;
- bounded length-prefixed canonical key/value state hashing with deterministic key sort and duplicate-key rejection;
- `GameplayDecisionRoot` intentionally has no `Debug` exposure and the crate grants no security-randomness authority;
- game-server exposes the active determinism profile while gameplay remains unavailable;
- workspace production boundary includes SIM only through the game-server consumer;
- Rust CI adds a lightweight exact-PR-head Windows SIM golden job while the protected Merge gate remains authoritative for full workspace/Linux/Windows/supply-chain validation.

## Development validation evidence

- guarded lock reconciliation restored exact base lock state and proved the complete lock delta is only one local `oteryn-simulation-determinism@0.1.0` package plus one game-server dependency edge; PR patch independently confirms no transitive package upgrade;
- `Rust workspace` run `32157307225`: Linux build/strict Clippy/workspace tests/synthetic harness/server smoke and policy/architecture/supply-chain all SUCCESS on unchanged product code; its obsolete duplicated Windows job was cancelled after CI optimization;
- `Rust workspace` run `32157886671`: `Rust / Windows SIM golden exact head` SUCCESS with explicit exact-head checkout/verification;
- development Merge gate generations confirm governance/policy/CodeQL/dependency/supply-chain checks while final freeze is still pending;
- temporary lock-reconcile and rustfmt write jobs are absent from the persistent diff.

## Acceptance criteria

- [x] production crate exists and has an immediate game-server consumer;
- [x] no process-global mutable RNG or wall-clock authority exists;
- [x] numeric overflow/divide-zero/out-of-range behavior is deterministic and tested;
- [x] retry/purpose/draw isolation has fixed golden tests;
- [x] canonical hash is insertion-order independent, bounded and duplicate-key fail-closed;
- [x] `Cargo.lock` delta is minimal: one local SIM package + one game-server edge, no unrelated transitive upgrade;
- [x] persistent CI is read-only and exact-head Windows SIM golden validation is isolated from duplicate full PR builds;
- [ ] final frozen-head Linux/Windows golden/workspace/metadata/fmt/Clippy/tests/cargo-deny/architecture checks pass;
- [ ] final frozen-head full-diff self-review is clean; independent review remains `NOT_REQUIRED` only if risk classification is unchanged;
- [ ] squash merge, post-merge verification, archive and ownership release complete.

## Context checkpoint

```yaml
last_progress: Product implementation is complete, lockfile is minimal, temporary CI mutators are removed, development Linux/full-workspace validation and exact-head Windows SIM golden are green; task metadata is entering final freeze.
status: validating
branch: feat/otv2-20260818-impl-simulation
head_sha: pending_final_freeze
pr: 14
blocker: null
owner_action_required: null
next_action: Freeze PR #14 head, perform full-diff self-review, mark Ready, require one final exact-head CI generation, then squash merge and archive SIM.
```
