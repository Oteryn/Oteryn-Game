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
and command type 15 `PUSH_INTENT` (`PushIntentV1`, at most 49 bytes; result at most 4 bytes), adds
`UseIntentV1` field 6 `use_with_map_item` (needs capability 11, excludes field 4) and the USE dispositions
11 to 17 (LOCKED, KEY_MISMATCH, SEALED, PZ_BLOCKED, NOT_MOVABLE, NO_ROOM, NOT_POSSIBLE), per
WORLD-INTERACTION-0 §10.1. The server gates command 15, rejects field 6 and refuses dispositions 11 to 17
without capability 11. Wire only: no runtime, no migration, nothing offered.

## Deviations (reported to the control plane)

- The map-item target is `UseIntentV1` field 6, not `EntityRefV1` field 3: the latter would change the
  VIS-2 `leave` message.
- `PUSH_INTENT` is bounded at 49 bytes (measured), not the 32 of WORLDINT0-RL-21; USE stays within 529.
- `SEALED` carries no message id: the result bound is 4 bytes.
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
