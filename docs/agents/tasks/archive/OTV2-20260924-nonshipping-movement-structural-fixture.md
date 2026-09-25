# OTV2-20260924-nonshipping-movement-structural-fixture

```yaml
task_id: OTV2-20260924-nonshipping-movement-structural-fixture
issue: 162
title: Non-shipping structural Movement fixture
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/nonshipping-movement-structural-fixture-20260924
pr: 870
base_sha: c516182255d3ea1724e91671c8d2187eb622e3df
head_sha: 3c366e47b2c389ac94becb17dc1b90d0814d7ec5
final_head_sha: 3c366e47b2c389ac94becb17dc1b90d0814d7ec5
final_head_frozen_at: null
owner: delegated Sol 6 writer under coordinator #162
created_at: 2026-09-24
updated_at: 2026-09-25
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/foundation/movement_static_kernel_structural_tests.rs
  - docs/agents/tasks/archive/OTV2-20260924-nonshipping-movement-structural-fixture.md
  - docs/agents/evidence/OTV2-20260924-nonshipping-movement-structural-fixture.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and provenance

- PROVEN: [#162 allocation](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5823029665) reserved four exclusive implementation paths. Its original task path was `docs/agents/tasks/active/OTV2-20260924-nonshipping-movement-structural-fixture.md`; this archived path and evidence update belong to a separate [#162 archive-only allocation](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5826803896).
- PROVEN: [PR #870](https://github.com/Oteryn/Oteryn-Game/pull/870) merged exact implementation head `3c366e47b2c389ac94becb17dc1b90d0814d7ec5` through governed Merge Queue. Successful [merge_group run 36071383357](https://github.com/Oteryn/Oteryn-Game/actions/runs/36071383357) included the aggregate `game-gate`. Protected `main@1680eb5dc6145aa3e271ac8665f33a50ed837b76` was read back as the PR merge commit.
- PROVEN: [#162 closeout](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5823905431) released the implementation allocation; [#139 evidence](https://github.com/Oteryn/Oteryn-Game/issues/139#issuecomment-5823901619) records the component's physical tests while keeping production gates open.

## Qualification

- [x] Checked adjacent N/E/S/W destinations on the same floor; exactly one complete-key candidate lookup per qualified attempt, without scan or fallback.
- [x] Synthetic WALKABLE commits via the existing private actor position owner; BLOCKED, absent, unqualified, conflict and static scope mismatch reject with unchanged position and revision.
- [x] Coordinate overflow, stale actor/owner, stale snapshot and context reject before lookup; intervening compare-commit and replay reject without changing the committed position or revision.
- [x] Six focused fixture tests passed. Exact-head agent governance [36070156300](https://github.com/Oteryn/Oteryn-Game/actions/runs/36070156300), architecture audit [36070131181](https://github.com/Oteryn/Oteryn-Game/actions/runs/36070131181), and full merge gate [36070156440](https://github.com/Oteryn/Oteryn-Game/actions/runs/36070156440) succeeded; strict Clippy, fmt, Rust workspace tests and PostgreSQL E2E were included in the gate.
- [x] Independent Luna 6 exact-head [review](https://github.com/Oteryn/Oteryn-Game/pull/870#issuecomment-5823576557) passed; governed queue request is [META #196 comment 5823748228](https://github.com/Oteryn/Oteryn/issues/196#issuecomment-5823748228), executor run `36071334534`, queue UUID `e470234d-546d-4ade-9d7e-340cf20b6bb8`.

## Boundary

The `#[cfg(test)]` child uses the **exact existing test-only Content static-cell index source**, loaded with minimal **synthetic input-type stand-ins** to compile Foundation-only test crates. This verifies the one-key lookup algorithm and private owner compare-commit structure; it does not verify actual Content type binding, activation or runtime wiring. It neither establishes Reference parity nor a production `MOVE-RL-03` maximum. `MOVE-RL-02=NOT_EXERCISED_BY_COMPONENT` applies only to this isolated fixture. #139 remains open with production `MOVE-RL-03=REQUIRED_NOW/UNDECIDED_MAX` and the immutable 2026-07-28 Reference evidence gate separate.

No diagonal movement, stairs, pathfinding, occupancy, visibility, teleport, timing or production Movement was in scope. No `lib.rs`, Cargo, resource registry, protocol, production authority or existing Item profile was changed by the fixture. Detailed cases and exact evidence are retained in `docs/agents/evidence/OTV2-20260924-nonshipping-movement-structural-fixture.md`.
