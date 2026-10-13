# WORLDINT-WIRE-1

```yaml
task_id: WORLDINT-WIRE-1
title: "WORLDINT-WIRE-1: World interaction wire (capability 11, PUSH_INTENT 15, USE field 6)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/loop-world-1-worldint-wire-1-20261013
issue: null
governing_issue: 1622
pr: null
base_sha: 39f77b53
created_at: 2026-10-13
updated_at: 2026-10-13
execution_policy: continuous_progress
owned_paths:
  - crates/protocol-oteryn/src/world_interaction.rs
  - crates/protocol-oteryn/src/world_interaction_tests.rs
  - crates/protocol-oteryn/src/world_object.rs
  - crates/protocol-oteryn/src/item_view.rs
  - crates/protocol-oteryn/src/lib.rs
  - apps/game-server/src/gameplay_transport/capabilities.rs
  - apps/game-server/src/gameplay_transport/capabilities_tests.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - docs/contracts/protocol-oteryn/v1/world_interaction_v1.proto
  - docs/contracts/protocol-oteryn/v1/world_object_v1.proto
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/agents/tasks/archive/WORLDINT-WIRE-1.md
public_contracts: [PROTOCOL_OTERYN_V1_REGISTRY.json]
external_repositories: []
```

## Outcome

Registers capability 11 `WORLD_INTERACTION_V1` (requires 15 and 18, `offered: false`, gate WORLDINT-USE-1)
and command type 15 `PUSH_INTENT` (`PushIntentV1`, flat, at most 37 bytes; result at most 4 bytes), adds
`UseIntentV1` field 6 `use_with_map_item` (a handle) and field 7 `use_with_map_object` (placement and revision, for handleless map objects such as doors; both need capability 11, at most one, exclude field 4) and the USE dispositions
11 to 17 (LOCKED, KEY_MISMATCH, SEALED, PZ_BLOCKED, NOT_MOVABLE, NO_ROOM, NOT_POSSIBLE), per
WORLD-INTERACTION-0 §10.1. The server gates command 15, rejects field 6 and refuses dispositions 11 to 17
without capability 11. Wire only: no runtime, no migration, nothing offered.

## Deviations (reported to the control plane)

- The map-item target is `UseIntentV1` field 6, not `EntityRefV1` field 3: the latter would change the
  VIS-2 `leave` message.
- `PUSH_INTENT` is flat: identity, the full `uint64` generation, destination x and y (-2^20 to 2^20 - 1),
  both always present (zero is an explicit varint; a payload without either fails closed). The bound is
  37 bytes, 5 over `WORLDINT0-RL-21`'s 32: a flat absolute destination with a `uint64` generation cannot
  fit 32. OWNER DECISION PENDING: `uint32` generation = exactly 32 B (narrows the session-generation
  width), or a one-byte relative direction = 31 B with `uint64`. The destination floor is not sent: a
  creature push never changes floor (Canary `Game::playerMoveCreature`: throw range 1 against z
  distance times 4).
- `SEALED` carries the gate message id (`UseResultV1` / `PushResultV1` field 2, 1 to 127, so the result
  stays within 4 bytes); other dispositions refuse it and `SEALED` without it fails closed. The server
  carries it from `UseOutcome.gate_message_id` through the gate-aware codec (an invalid pair becomes
  `Rejected`), covered end to end.
- Field 7 `use_with_map_object` (Codex P1): the map publishes non-movable base entries (doors) as base
  ordinals without a handle, so a tool or key could not name them by field 6. Field 7 names them as
  field 1 already does, by placement and revision; placement non-empty and at most 499 bytes so the
  intent stays within 529 (`WORLDINT0-RL-22`). Open for the CP: a client that holds a base ordinal
  needs its placement from the overlay; if the ordinal must be addressable instead, that is a map
  wire change outside this slice.
- The session crate fails closed on dispositions 11 to 17 or a gate id without a negotiated capability 11.
- Capability 11 changes no other command: the command 9 map-item and Ground sources are a separate
  registry change, so the capability promises nothing unusable.
- Command number 15 does not collide with 14 (HIGHSCORES); no BED_SLEEP_INTENT number is used.

## Validation

- `cargo fmt --all --check`: pass
- `cargo test -p oteryn-protocol-oteryn`: pass (224)
- `cargo test -p oteryn-game-server --lib gameplay_transport`: pass (544)
- `cargo clippy` on both crates: no findings in the changed files
- `oteryn-architecture-check workspace .`: pass

## Self-review

Registry ids and GATED stay in ascending order (tests enforce it); decoder and encoder refuse the same
values; capability 11 is not in any offered set.
