# WORLDINT-WIRE-1

```yaml
task_id: WORLDINT-WIRE-1
title: "WORLDINT-WIRE-1: World interaction wire (capability 11, PUSH_INTENT 15, USE map_item arm)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/loop-world-1-worldint-wire-1-20261013
issue: null
governing_issue: 1622
pr: 2094
base_sha: 39f77b53
created_at: 2026-10-13
updated_at: 2026-10-14
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
  - apps/game-server/src/gameplay_transport/container_view_tests.rs
  - apps/game-server/src/gameplay_transport/item_move_tests.rs
  - apps/game-server/src/gameplay_transport/item_view_tests.rs
  - apps/game-server/src/gameplay_transport/map_door.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/qualification.rs
  - apps/client/src/play.rs
  - crates/protocol-oteryn/src/container_tree_tests.rs
  - crates/session/src/lib.rs
  - tools/dev-client/src/lib.rs
  - tools/synthetic-client-harness/src/live/model.rs
  - tools/synthetic-client-harness/src/live/tests.rs
  - docs/contracts/protocol-oteryn/v1/item_view_v1.proto
  - docs/contracts/protocol-oteryn/v1/world_interaction_v1.proto
  - docs/contracts/protocol-oteryn/v1/world_object_v1.proto
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/architecture/reviews/OTERYN_GAME_WORLD_INTERACTION0_DOORS_LEVERS_FIELDS_AND_WORLD_CLOCK_DECISION_2026-10-01.md
  - docs/agents/tasks/archive/WORLDINT-WIRE-1.md
public_contracts: [PROTOCOL_OTERYN_V1_REGISTRY.json]
external_repositories: []
```

## Outcome

Registers capability 11 `WORLD_INTERACTION_V1` (requires 15 and 18, `offered: false`, gate WORLDINT-USE-1)
and command type 15 `PUSH_INTENT` (`PushIntentV1`, flat, at most 41 bytes; result at most 4 bytes), adds
the `map_item` arm of `UseIntentV1` field 4 `use_with` (`UseWithV1` field 3, a handle; needs capabilities 15 and 11, exclusive with the entity arm), the `GROUND` destination and the `BLOCKED` result of command type 9 for a map-item or Ground source under capability 11 (`EQUIPMENT` stays with capability 12) and the USE dispositions
11 to 17 (LOCKED, KEY_MISMATCH, SEALED, PZ_BLOCKED, NOT_MOVABLE, NO_ROOM, NOT_POSSIBLE), per
WORLD-INTERACTION-0 §10.1. The server gates command 15, rejects the map arm and refuses dispositions 11 to 17
without capability 11. Wire only: no runtime, no migration, nothing offered.

## Deviations (reported to the control plane)

- Field 4 of `UseIntentV1` becomes the `UseWithV1` message (`EntityRefV1` fields 1 and 2 plus the
  `map_item` arm, field 3), wire-compatible with the previous entity-only field 4. This replaces the
  earlier top-level fields 6 and 7 (never shipped; the contract fixes the arm under field 4).
- `PUSH_INTENT` is flat: identity, the full `uint64` generation, destination x and y over the full
  `sint32` domain, both always present (zero is an explicit varint; a payload without either fails
  closed). OWNER DECISION (2026-10-14, option A): keep the `uint64` generation and the full `sint32`
  coordinates, so the bound is 41 bytes (18 + 11 + 6 + 6) and `WORLDINT0-RL-21` is amended docs-only from
  32 to 41 in the WORLD-INTERACTION-0 decision (§10.1 and §12). This also closes the Codex P1 on the
  former 3-byte coordinate range check, which is removed. The destination floor is not sent: a
  creature push never changes floor (Canary `Game::playerMoveCreature`: throw range 1 against z
  distance times 4).
- `SEALED` carries the gate message id (`UseResultV1` / `PushResultV1` field 2, 1 to 127, so the result
  stays within 4 bytes); other dispositions refuse it and `SEALED` without it fails closed. The server
  carries it from `UseOutcome.gate_message_id` through the gate-aware codec (an invalid pair becomes
  `Rejected`), covered end to end.
- The session crate fails closed on dispositions 11 to 17 or a gate id without a negotiated capability 11.
- Command 9 under capability 11: sources are item handles, so a map-item or Ground source needs no new
  field; the capability admits the `GROUND` destination (field 4) and the `BLOCKED` result, while
  `EQUIPMENT` and results 10 and 11 stay with capability 12. The server answers `NOT_SUPPORTED` until
  WORLDINT-USE-1 composes the runtime.
- Command number 15 does not collide with 14 (HIGHSCORES); no BED_SLEEP_INTENT number is used.

## Validation

- `cargo fmt --all --check`: pass
- `cargo test -p oteryn-protocol-oteryn`: pass (225)
- `cargo test -p oteryn-game-server --lib gameplay_transport`: pass (546)
- `cargo test -p oteryn-session`: pass (60); synthetic harness 36; dev-client 34
- `python tools/agents/validate_governance.py`: pass ("Governance validation passed for Oteryn/Oteryn-Game. Validated 22 required policy documents and 9 project lanes.")
- `python -m unittest discover -s tools/agents/tests`: pass ("Ran 60 tests ... OK")
- `cargo clippy` on both crates: no findings in the changed files
- RL-21 amendment commit (41-byte bound, full `sint32` coordinates): `cargo fmt --all --check` pass;
  `cargo clippy --locked -p oteryn-protocol-oteryn --all-targets -- -D warnings` pass;
  `cargo test --locked -p oteryn-protocol-oteryn` pass (225, including the registry checks);
  governance validation and `tools/agents/tests` (60) pass; `git diff --check` pass
- `oteryn-architecture-check workspace .`: pass

## Self-review

Registry ids and GATED stay in ascending order (tests enforce it); decoder and encoder refuse the same
values; capability 11 is not in any offered set.
