# OTV2-20261004-item-use-wire-1

```yaml
task_id: OTV2-20261004-item-use-wire-1
title: ITEM-USE-WIRE-1 Capability 15 ITEM_USE_V1, USE_INTENT fields 4 and 5, dispositions 7 to 10
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/item-use-wire-1-20261004
issue: 1622
pr: PENDING
decision: ITEM-USE-0 §3; ARCH-BATCH-ROOT-PACKETS-V1 §0.1 and §2.2 (D485), wire lane
builds_on: ATTACK-WIRE-1 (#1758) pattern; VIS-2 EntityRefV1; ITEM-MOVE-WIRE-0 §4.3 (field 2)
migration_lease: none
owned_paths:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json        # capability 15 only
  - docs/contracts/protocol-oteryn/v1/world_object_v1.proto  # fields 4 and 5, field 3 reserved, dispositions 7 to 10
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json           # own ITEMUSE0-RL-03 row only
  - crates/protocol-oteryn/src/lib.rs                      # registered set, requirement 15 -> 4
  - crates/protocol-oteryn/src/world_object.rs
  - apps/game-server/src/gameplay_transport/capabilities.rs     # GATED row (15, [], [])
  - apps/game-server/src/gameplay_transport/capabilities_tests.rs  # its #[path] test module: item_use_wire tests
  - apps/game-server/src/gameplay_transport/connection.rs       # USE dispatch
  - crates/protocol-oteryn/src/{attack,attack_tests}.rs and docs/contracts/protocol-oteryn/v1/attack_v1.proto  # deferred P2 4177423686, granted by the #1622 control plane
  - tools/synthetic-client-harness/src/live/model.rs     # apply_use match arms only, granted by the #1622 control plane (D529)
  - docs/agents/tasks/archive/OTV2-20261004-item-use-wire-1.md
```

## Outcome

- Capability 15 `ITEM_USE_V1`: `requires: [4]`, `offered: false`, offer gate ITEM-USE-1, no
  command type or domain. It is in `REGISTERED_CAPABILITY_IDS_V1` and in the CAP-NEG-1
  requirement closure, so a selected set naming 15 without 4 is refused on encode and decode of
  both acceptance messages. `PRODUCTION_OFFERED_CAPABILITIES` is unchanged.
- `UseIntentV1`: field 5 `ItemByDefinitionV1 {definition_index}` joins the target oneof; field 4
  `use_with` is the existing `EntityRefV1` outside the oneof; field 3 stays reserved. Fields 4 and
  5 decode only when capabilities 4 and 15 are both selected. Field 4 needs field 2 or 5 (alone or
  with field 1 it is Malformed), at most once, with a 16-byte identity and a non-zero generation.
  Field 5 is a 1-based u32: zero, absent, above u32, repeated or with an unknown field is
  Malformed.
- `UseDispositionV1` adds REQUIREMENT_NOT_MET 7, EXHAUSTED 8, FULL 9 and NO_TARGET 10, sent only
  under capability 15. The result stays at most 4 bytes.
- ITEMUSE0-RL-03: the USE_INTENT bound stays 529 bytes (530 refused); the largest intent with
  fields 4 and 5 is 44 bytes.

## Dispatch (handover to ITEM-USE-1)

- Without capability 15: fields 4 and 5 are REJECTED, a non-corpse field 2 stays NOTHING_TO_USE
  and a corpse field 2 still opens.
- With capability 15, until ITEM-USE-1 composes item use: a `use_with` is REJECTED (no item is
  usable on a creature yet), a use by definition is NOTHING_TO_USE, and field 2 without
  `use_with` takes the existing container and corpse path.
- NO_TARGET for an unseen or stale `use_with` creature needs the session's entity visibility,
  which the dispatcher does not hold. It is deferred to ITEM-USE-1 with the rest of the
  dispositions 7 to 10.

## Deferred P2 4177423686 (ATTACK-WIRE-1)

`attack.rs` decodes an explicitly encoded proto3 bool 0 as false (`secure`, `in_fight`) and
refuses only values above 1; `attack_tests.rs` and the `attack_v1.proto` header follow.

## Excluded

- ITEM-USE-1: the item-use runtime, consumption and charges, dispositions 7 to 10, NO_TARGET,
  the offer of capability 15 and the client.

## Validation

- `cargo fmt --all -- --check`: pass
- `cargo clippy --locked -p oteryn-protocol-oteryn -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-protocol-oteryn`: pass
- `cargo test --locked -p oteryn-game-server item_use_wire`: pass
- `cargo test --locked -p oteryn-game-server --lib`: pass
- `cargo test --locked -p oteryn-session`: pass
- `cargo test --locked -p oteryn-synthetic-client-harness`: pass
- `cargo check --locked --workspace --all-targets`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
