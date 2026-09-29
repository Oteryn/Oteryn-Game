---
task_id: OTV2-20260927-first-control-wire-m1
title: First-control wire M1 - registries and codecs
mode: IMPLEMENT
status: completed-on-merge
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/first-control-wire-m1-20260927
base_sha: e5cf5b48621e0ac1869bd31749eda330e0429a23
issue: 162
jira: KAN-13
allocation_comment: 5853786121
owned_paths:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/protocol-oteryn/v1/world_spatial_v1.proto
  - apps/game-server/src/foundation/protocol.rs
  - apps/game-server/src/gameplay_transport/world_spatial.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - docs/agents/tasks/archive/OTV2-20260927-first-control-wire-m1.md
---

# First-control wire M1: registries and codecs

Authority:
- #642 `FIRST-CONTROL-WIRE-V1`, owner acceptance 5853424280;
- #139 `FIRST-CONTROLLED-STEP-LIMITS-V1`, owner acceptance 5853424970.

Allocation: #162 5853786121.

## Outcome

- **Protocol registry.** Adds command type 1 `WORLD_ACTOR_STEP_INTENT` and state domain 1
  `WORLD_SPATIAL_VISIBILITY`, with delta type 1 and snapshot type 1. All four IDs were free on
  base `e5cf5b48`. Each entry names its payload schema in
  `docs/contracts/protocol-oteryn/v1/world_spatial_v1.proto` and its accepted byte bound.
- **Resource registry.** Adds `MOVE-RL-02` (1 step input per actor per Channel owner work cycle)
  and `MOVE-RL-11` (1 entity per snapshot or delta), in the `MOVE-RL-03` row format.
- **Typed payload codecs.** `gameplay_transport/world_spatial.rs` holds the step intent, step
  result, delta and snapshot codecs. Decoding is strict: zero or unknown enum values, unknown or
  repeated fields, over-bound payloads and an out-of-range floor all fail closed.
- **FND-02 server-side codecs.** `foundation/protocol.rs` adds encoders for `CommandResult`
  and `StateDelta` (both server-sequenced) and for a single-chunk
  `SnapshotBegin`/`SnapshotChunk`/`SnapshotCommit` transfer (unsequenced). It also adds a
  `ClientCommand` decoder that is generation-checked. Every encoded frame passes the existing
  foundation ingress validation.
- **Registry binding test.** A test ties the registry JSON to the code constants and limits.

The M1 codecs are not composed yet (`#[allow(dead_code)]`, the same pattern as the existing
unused resume and liveness encoders).

Excluded (M2):
- the Server Seam composition: initial snapshot, command handling, and the movement kernel over the
  active generation;
- the end-to-end step, return and blocked proof.
