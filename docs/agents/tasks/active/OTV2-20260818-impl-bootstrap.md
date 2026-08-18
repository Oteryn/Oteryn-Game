# OTV2-20260818-impl-bootstrap

```yaml
task_id: OTV2-20260818-impl-bootstrap
title: Bootstrap real native game-server workspace shape
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: feat/otv2-20260818-impl-bootstrap
pr: 10
base_sha: d9c5ef68e1c88b88b4782219051395eacb0f8e67
allocation_base_sha: 86200e6d044287bcb2fbb122d224e825b9084a7a
owner: worker-otv2-impl-bootstrap
created_at: 2026-08-18T16:31:00+02:00
updated_at: 2026-08-18T16:59:00+02:00
execution_budget_minutes: 60
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
  - docs/architecture/ADR-0015-gamenode-implementation-shape-not-yet-frozen.md
depends_on:
  - PR #8 / 86200e6d044287bcb2fbb122d224e825b9084a7a
  - PR #9 / d9c5ef68e1c88b88b4782219051395eacb0f8e67
blocks:
  - first post-Bootstrap implementation wave
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
```

## Authority and base reconciliation

- `PROVEN`: live coordinator allocation on `main` names this lane/task/branch and owned paths.
- `PROVEN`: allocation PR #8 merged as `86200e6d044287bcb2fbb122d224e825b9084a7a`.
- `PROVEN`: coordinator exact-base reconciliation PR #9 merged as base `d9c5ef68e1c88b88b4782219051395eacb0f8e67` without touching Bootstrap implementation-owned paths.
- `PROVEN`: entry baseline had 19 workspace members, only `apps/client`, a `game-server` forbidden fragment and architecture-check hard-coding the historical member count/client production root.
- `PROVEN`: Canary was absent and remains forbidden.

## Implemented outcome

The branch atomically replaces the historical client-only/pre-native workspace assumption with the smallest real native server application shape:

1. `apps/game-server` is a real production binary/library composition root consuming `oteryn-foundation` and remaining explicitly gameplay/network/session/persistence unavailable;
2. focused lifecycle/fail-closed smoke tests cover deterministic shutdown and unavailable gameplay;
3. Cargo membership/lockfile and workspace-boundary role/edge policy include the new immediate-consumer server;
4. architecture-check member cardinality and production-root closure are structural rather than hard-coded to 19/client-only;
5. Rust and Merge gate production-closure checks validate both production roots and Linux smoke-tests the server bootstrap;
6. scoped `apps/game-server/AGENTS.md` protects later high-risk semantics.

No protocol/session/admission/persistence/gameplay semantics or public IDs are implemented in this lane.

## Validation evidence

- `PROVEN`: deterministic one-off reconciliation restored the exact base `Cargo.lock`, ran `cargo +1.94.0 check --workspace`, and proved that the complete lockfile delta is exactly one local workspace package: `oteryn-game-server@0.1.0` with only `oteryn-foundation` and `tokio`.
- `PROVEN`: the same reconciliation proved the intended static Merge gate transformation before the runner push was rejected solely because its GitHub App token lacked workflow-file permission; the connector then applied the static workflow update directly.
- `PROVEN`: the temporary self-mutating Rust workflow implementation and accidental transitive lockfile upgrades found by pre-freeze self-review were removed.
- `PROVEN`: the one-off reconciliation workflow has been removed from the branch before final freeze.
- `PROVEN`: an earlier exact head passed Agent governance, Architecture semantic audit, Merge authority audit, Rust policy, Linux workspace, CodeQL, dependency review and supply-chain checks; those results are development evidence only and will not substitute for final exact-head CI.

## Acceptance criteria

- [x] `apps/game-server` is a real production member with immediate foundation consumer and deterministic fail-closed bootstrap behavior.
- [x] workspace machine policy includes the server and keeps Canary/protocol/session/persistence fragments forbidden.
- [x] architecture-check validates arbitrary structurally coherent member counts and all declared production roots.
- [x] negative tests prove a production root cannot reach synthetic/test/tool packages.
- [x] Rust/merge CI definitions validate both production roots and smoke-test the bootstrap server.
- [ ] final exact-head metadata/fmt/build/clippy/test/cargo-deny/architecture checks pass.
- [ ] final exact-head full-diff self-review is clean; independent review remains `NOT_REQUIRED` only if the frozen diff contains no protocol/session/security semantics.
- [ ] squash merge, post-merge verification, archive and ownership release complete.

## Context checkpoint

```yaml
last_progress: Bootstrap implementation is complete; deterministic lock reconciliation and static CI repair are applied, one-off workflow is removed, and PR #10 is entering final freeze.
status: validating
branch: feat/otv2-20260818-impl-bootstrap
head_sha: pending_final_freeze
pr: 10
blocker: null
owner_action_required: null
next_action: Freeze the resulting PR #10 head, perform final exact-head diff review and repository CI, then squash merge and archive the Bootstrap task.
```
