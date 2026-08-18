# OTV2-20260818-impl-bootstrap

```yaml
task_id: OTV2-20260818-impl-bootstrap
title: Bootstrap real native game-server workspace shape
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: feat/otv2-20260818-impl-bootstrap
pr: null
base_sha: d9c5ef68e1c88b88b4782219051395eacb0f8e67
allocation_base_sha: 86200e6d044287bcb2fbb122d224e825b9084a7a
owner: worker-otv2-impl-bootstrap
created_at: 2026-08-18T16:31:00+02:00
updated_at: 2026-08-18T16:31:00+02:00
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
  - PR #8
  - PR #9
blocks:
  - first post-Bootstrap implementation wave
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
```

## Authority and base reconciliation

- `PROVEN`: live coordinator allocation on `main` names this lane/task/branch and owned paths.
- `PROVEN`: allocation PR #8 merged as `86200e6d044287bcb2fbb122d224e825b9084a7a`.
- `PROVEN`: coordinator-only exact-base reconciliation PR #9 merged as current main `d9c5ef68e1c88b88b4782219051395eacb0f8e67` without touching any Bootstrap implementation-owned path.
- `DERIVED`: worker branch starts from current main `d9c5ef68...`, which contains the live allocation and differs from allocation content base only by coordinator-owned records. No implementation dependency changed between the two SHAs.
- `PROVEN`: current baseline remains 19 workspace members and only `apps/client`; machine policy still forbids `game-server` and hard-codes 19 members / client-only production closure.
- `PROVEN`: Canary is absent and remains forbidden.

## Outcome

Atomically replace the historical client-only/pre-native workspace assumption with the smallest real native server application shape:

1. add a real `apps/game-server` binary/library composition root that consumes `oteryn-foundation` and remains explicitly gameplay/network/session/persistence unavailable;
2. add focused lifecycle/fail-closed smoke tests;
3. update Cargo membership/lockfile and workspace-boundary role/edge policy;
4. make architecture-check member cardinality and production-root closure structural rather than hard-coded to 19/client-only;
5. update Rust/merge CI production-closure checks and add game-server bootstrap smoke;
6. add nearest scoped `apps/game-server/AGENTS.md` protecting later high-risk semantics.

No protocol/session/admission/persistence/gameplay semantics or public IDs are implemented in this lane.

## Acceptance criteria

- [ ] `apps/game-server` is a real production member with immediate foundation consumer and deterministic fail-closed bootstrap behavior.
- [ ] workspace machine policy includes the server and keeps Canary/protocol/session/persistence fragments forbidden.
- [ ] architecture-check validates arbitrary structurally coherent member counts and all declared production roots.
- [ ] negative tests prove a production root cannot reach synthetic/test/tool packages.
- [ ] Rust/merge CI validates both production roots and smoke-tests the bootstrap server.
- [ ] exact-head metadata/fmt/build/clippy/test/cargo-deny/architecture checks pass.
- [ ] full-diff self-review is clean; independent review is `NOT_REQUIRED` because this lane intentionally avoids protocol/session/security semantics.
- [ ] squash merge, post-merge verification, archive and ownership release complete.

## Context checkpoint

```yaml
last_progress: Live allocation and current main reconciled; Bootstrap task record created before implementation writes.
status: implementing
branch: feat/otv2-20260818-impl-bootstrap
head_sha: pending
pr: null
blocker: null
owner_action_required: null
next_action: Implement the allocated game-server/workspace/machine-policy/CI bootstrap atomically and open its PR.
```
