# OTV2-20260925-reference-static-kernel-movement-proof

```yaml
task_id: OTV2-20260925-reference-static-kernel-movement-proof
title: Unactivated production Movement cardinal-step proof
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-movement-engine-local-step-proof
issue: 162
pr: null
base_sha: 1f1c29d2614c098a6665cee34d06b06c2ad92339
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: allocated Sol 6 writer under #162
created_at: 2026-09-25T00:00:00Z
updated_at: 2026-09-25T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/lib.rs
  - apps/game-server/src/movement.rs
  - apps/game-server/src/foundation/mod.rs
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - docs/agents/tasks/active/OTV2-20260925-reference-static-kernel-movement-proof.md
  - docs/agents/evidence/OTV2-20260925-reference-static-kernel-movement-proof.json
public_contracts: []
depends_on: [504, 870, 881, 883]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Crate-private, unactivated `REFERENCE_LOCAL_STEP_STATIC_KERNEL/v1` makes exactly one cardinal destination query through the real Content engineering index and applies one owner-mediated position compare-commit to the real Foundation ordinary actor slot. Rejection preserves position/revision. A distinct borrowed Movement capability rejects CreatureOccupied on read and commit without exposing actor admission or Ability damage.

## Architecture and source of truth

- PROVEN: #162 comment 5829139409 leases exactly these six paths at protected base `1f1c29d2`; #881 and #883 supply integrated seams.
- PROVEN: #504 correction separates ENGINE_READINESS from REFERENCE_DATA_ADMISSION; #483 gates concrete July-28 Reference field admission/activation.
- PROVEN locally: one attempted candidate per Movement decision; second attempt fails before second real Content key lookup and actor write. This is distinct from four cells per engineering Content index and RL-02 inputs per owner work cycle.
- UNKNOWN: Reference cell collision values and accepted RL-03 registry maximum.

## High-risk authority/recovery qualification

The position mutation consumes independently current owner continuity and a private actor slot. Boundary: `CurrentOwnerMovementPosition::commit_cardinal`. Identity/binding: world/channel/scope generation, actor local id/generation, exact position context and snapshot. Current liveness/authority: borrowed continuity and exclusive owner carrier checked again at write. Temporal/provenance: exact revision compare and no replay. Applicable negative operators: absent/recycled actor, stale owner generation, stale snapshot/revision, context mismatch, creature slot, overflow, blocked/unresolved cell and second candidate. No persisted recovery/PG reload or expiration time exists at this pure local boundary (`NOT_APPLICABLE`). Tests mutate one invariant per rejection and compare position/revision; no helper derives current owner from expected snapshot.

## Acceptance criteria

- [x] Real Content + Foundation actor interfaces; N/E/S/W checked adjacency and unchanged floor.
- [x] Exactly one destination attempt and actual Content lookup; second same-decision attempt returns `CapacityExceeded` before second lookup, write or success publication.
- [x] Missing, blocked, unqualified, conflict, scope, overflow, stale actor/owner/context/revision, creature and replay negatives leave position/revision unchanged.
- [ ] Exact-head governance, full `game-gate`, author and independent review.

## Excluded scope

Reference admission/activation, production Movement composition or owner loop, Creature movement, registry and accepted RL-03 bound, RL-02 ownership, pathfinding/diagonal/stairs/teleport/visibility/occupancy, client/wire #642.

## Validation

Focused real-interface `cargo test --offline --locked -p oteryn-game-server --lib real_movement`: 6 PASS. Rustfmt, JSON, diff check and local governance: PASS. Strict Clippy `cargo clippy --offline --locked -p oteryn-game-server --lib --tests -- -D warnings`: PASS, including Foundation path-included integration crates. Exact-head hosted full gate and independent review: pending. Final exact SHA belongs in PR/#162 immutable evidence after source freeze; no self-referential task-file commit.

## Self-review

Author whole-diff review before freeze: exactly six leased paths; Foundation adds only borrowed Movement position read/compare-commit and test-only factory, without exposing grant/admission/slots or Ability capability. Movement attempts increment before real Content lookup and the second attempt latches `CapacityExceeded`; commit refuses after any rejection. Tests assert one real lookup and position/revision equality after second attempt, blocked/unqualified/conflict/missing/mismatch/overflow, stale/replayed snapshot, and creature rejection at both read and commit. No material finding. Recheck remote exact tree at freeze.

## Independent review

One Luna 6 exact-head review routed by #162 coordinator after author self-review; worker does not trigger review or merge.
