# OTV2-20261015-rune-wire-1

```yaml
task_id: OTV2-20261015-rune-wire-1
title: RUNE-WIRE-1 Capability 22 RUNE_USE_V1, the USE position arm, dispositions 18 and 19
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/rune-wire-1
issue: 1622
pr: 2126
decision: RUNE-USE-0 §4 (accepted #2107), §6.1 as amended by #2106; MAGIC-ITEMS-1 Wave 2
builds_on: ITEM-USE-WIRE-1 (#1761); RUNE-CONTENT-1 (#2116); SPELL-D7 SpellTargetPositionV1
migration_lease: none
owned_paths:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json        # capability 22 row only
  - docs/contracts/protocol-oteryn/v1/world_object_v1.proto  # UseWithV1 field 5, dispositions 18 and 19
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json           # own RUNEUSE0-RL-04 row only
  - crates/protocol-oteryn/src/lib.rs                      # registered set, requirement 22 -> 15
  - crates/protocol-oteryn/src/actor_spell.rs              # position codec made crate-visible
  - crates/protocol-oteryn/src/world_object.rs
  - crates/protocol-oteryn/src/world_interaction.rs        # push result refuses 18 and 19
  - crates/protocol-oteryn/src/world_interaction_tests.rs
  - apps/game-server/src/gameplay_transport/capabilities.rs     # GATED row (22, [], [])
  - apps/game-server/src/gameplay_transport/capabilities_tests.rs  # rune_wire tests
  - apps/game-server/src/gameplay_transport/connection.rs       # USE dispatch and result gate
  - crates/session/src/lib.rs                              # result negotiation check
  - tools/synthetic-client-harness/src/live/model.rs       # apply_use match arms only
  - docs/agents/tasks/archive/OTV2-20261015-rune-wire-1.md
```

## Outcome

- Capability 22 `RUNE_USE_V1`: `requires: [15]`, `offered: false`, offer gate RUNE-CAST-1, no
  command type or domain. The number is leased by the #1622 control plane (20 is
  CONNECTION_TRACE_V1 and 21 is leased to LOGOUT-WIRE-1). It is in `REGISTERED_CAPABILITY_IDS_V1` and in the CAP-NEG-1 requirement
  closure (22 -> 15 -> 4). `PRODUCTION_OFFERED_CAPABILITIES` is unchanged.
- `UseWithV1` field 5 `position`: a `SpellTargetPositionV1` in the SPELL-D7 frame (sint32 x and
  y, an int16 floor), at most 16 bytes, decoded by the `actor_spell` codec. It decodes only when
  capabilities 4, 15 and 22 are selected, needs field 2 or 5, and is exclusive with the entity
  and map arms; anything else is Malformed. The encoder refuses the same.
- `UseDispositionV1` adds TARGET_ILLEGAL 18 and PROTECTION_ZONE 19, sent only under capability
  22: the server gate turns them into REJECTED without it and the session client fails closed on
  them. They are never a push result. A pacified or powerless rune refusal stays
  REQUIREMENT_NOT_MET on the wire (§6.1). The result stays at most 4 bytes.
- RUNEUSE0-RL-04: the USE_INTENT bound stays 529 bytes (530 refused) under capability 22; the
  largest intent with the position arm is 33 bytes.

## Dispatch (handover to RUNE-CAST-1)

- Without capability 22: the position arm is REJECTED and a rune is NOTHING_TO_USE through the
  existing paths.
- With capability 22, until RUNE-CAST-1 composes rune casting: the position arm is REJECTED.

## Excluded

- RUNE-CAST-1: the rune casting runtime, dispositions 18 and 19 in practice, the offer of
  capability 22 and the client.

## Validation

- `cargo fmt --all -- --check`: pass
- `cargo clippy --locked -p oteryn-protocol-oteryn -p oteryn-game-server -p oteryn-session -p oteryn-synthetic-client-harness --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-protocol-oteryn`: pass
- `cargo test --locked -p oteryn-game-server --lib`: pass
- `cargo test --locked -p oteryn-session`: pass
- `cargo test --locked -p oteryn-synthetic-client-harness`: pass
- `cargo check --locked --workspace --all-targets`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
