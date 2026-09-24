# OTV2-20260924-channel-actor-position-owner-seam

```yaml
task_id: OTV2-20260924-channel-actor-position-owner-seam
title: Private Channel carrier position owner seam
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/channel-actor-position-owner-seam-20260924
issue: 162
pr: null
base_sha: f812f6dc5586f0120a6e832fe58b7edd5c6ea474
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Sol 6 allocation C
created_at: 2026-09-24T14:03:42Z
updated_at: 2026-09-24T14:07:31Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/foundation/mod.rs
  - apps/game-server/src/foundation/channel_actor_position_tests.rs
  - docs/agents/tasks/active/OTV2-20260924-channel-actor-position-owner-seam.md
  - docs/agents/evidence/OTV2-20260924-channel-actor-position-owner-seam.md
public_contracts: []
depends_on: ["#162 allocation C", "#508 Phase A"]
blocks: ["REFERENCE_LOCAL_STEP_STATIC_KERNEL/v1 composition"]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and boundary

The existing private and uncomposed Foundation Channel carrier keeps an optional, revisioned local position **inside its occupied actor slot**. An admitted actor without a qualified position fails closed. A private one-time preproduction initializer binds explicit World/Channel/owner generation and separate frame/map/content generation fixture markers at revision one. A direct owner read and compare-commit check the exact actor generation and independently current continuity. The commit compares its entire snapshot, rejects changed context, increments revision with checked arithmetic and changes one slot only after all failures are excluded.

The fixture marker numbers are not identifiers or proof from activated Content; no Reference artifact, owner issuer or gameplay caller is composed. No Foundation-to-Content dependency, second actor store, new public service, `lib.rs` modification, or Movement behavior is introduced.

## Acceptance criteria

- [x] Existing actor admission and exact-actor resolver continue using the same slot; removal discards position, reuse cannot inherit it.
- [x] Absent position fails closed; one-time initialization binds all six context fields and starts at a nonzero revision.
- [x] Owner read and compare-commit enforce exact actor, independently current owner and full expected snapshot, with no mutation on failure.
- [x] Focused tests independently change World, Channel, owner generation, frame, map revision, content generation, actor identity/generation, position and revision, and cover exhaustion.
- [x] Remote Linux Rust 1.94: four focused tests, strict game-server Clippy and formatter pass.
- [ ] Exact-head protected gate and independent review after publication.

## Resource and architectural qualifications

The carrier still allocates a fixed capacity once, with `checked_mul(size_of::<Slot>())` and `try_reserve_exact`, and reads/commits a single indexed slot. The additional optional versioned position increases physical slot size; historical #541 byte measurements cannot be assumed current until remeasured by its resource owner. This seam proves neither #139 per-owner-work-cycle input bound nor an active Content static fact. A production initializer and accepted current Content context binding remain separate gates.

## Validation and evidence

The test matrix and high-risk mutation review are in `docs/agents/evidence/OTV2-20260924-channel-actor-position-owner-seam.md`. Focused tests were authored before executable remote validation, but an executed RED failure was **not** captured and is not claimed. The revised candidate passed four focused tests under isolated remote Linux Rust 1.94, strict `cargo +1.94.0 clippy --locked -p oteryn-game-server --lib --tests -- -D warnings`, and `cargo +1.94.0 fmt --all -- --check`; each process exited 0. Exact-head protected gate and independent review remain pending publication.
