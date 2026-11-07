# OTV2-20261003-item-equip-wire-1

```yaml
task_id: OTV2-20261003-item-equip-wire-1
title: "ITEM-EQUIP-WIRE-1: capability 12 ITEM_EQUIP_DROP_V1 (equip and drop wire)"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/item-equip-wire-1-20261003
pr: 1711
base_sha: e644020
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: oteryn-impl-worker (CP #1622)
created_at: 2026-10-03
updated_at: 2026-10-04
execution_policy: continuous_progress
migration_lease: none
owned_paths:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/contracts/protocol-oteryn/v1/item_view_v1.proto
  - crates/protocol-oteryn/src/lib.rs
  - crates/protocol-oteryn/src/item_view.rs
  - crates/protocol-oteryn/src/item_view_tests.rs
  - apps/game-server/src/gameplay_transport/capabilities.rs
  - docs/agents/tasks/archive/OTV2-20261003-item-equip-wire-1.md
public_contracts:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/contracts/protocol-oteryn/v1/item_view_v1.proto
depends_on: [ITEM-VIEW-1a]
blocks: [ITEM-MOVE-2a, ITEM-MOVE-2b]
cross_repository_coordination_id: null
external_repositories: []
```

Packet: `docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_ITEM_EQUIP_PACKETS_2026-10-03.md` §2.5
(decision ITEM-MOVE-WIRE-1 §3). The CP added `capabilities.rs` to the owned paths for the CAP-NEG-1
`GATED` row, as it did for capability 4.

## Outcome

- Capability 12 `ITEM_EQUIP_DROP_V1`: `offered: false`, `requires [4]`. It owns no command type or
  domain because it extends capability 4's command type 9 and domain 9. `REGISTERED_CAPABILITY_IDS_V1`
  and the CAP-NEG-1 `GATED` table (`(12, [], [])`) include it.
- Command type 9: the `equipment = 3` (`EquipmentSlotV1`) and `ground = 4`
  (`WorldTilePositionV1`, int16 floor) destinations. There is no count field. The bound is 29 bytes
  under capability 12 and stays 13 without it.
- Domain 9: `repeated EquippedItemV1 equipment = 3`. Slots are strictly ascending and unique. Handles
  are unique across the backpack, the entries and the slots, and the view holds at most 30 items
  (ITEMV0-RL-01). The bound is 966 bytes under capability 12 and stays 930 without it.
- Results `SLOT_MISMATCH` 10, `REQUIREMENT_NOT_MET` 11 and `BLOCKED` 12, still at most 4 bytes.
- Without the capability, the `_with_equip_drop(.., false)` codecs and the unchanged entry points
  decode exactly as in ITEM-VIEW-1a. Everything the capability adds fails closed (test).
- `EquipmentSlotV1` values are never durable keys. `EQUIPMENT_SLOT_SEMANTIC_KEYS` is a separate table
  that maps each slot to its GAME-ITEM-01 §6.1 key (doc comment and test).
- There is no server behaviour, offer, persistence or migration.

## Validation

- `cargo test --locked -p oteryn-protocol-oteryn`: pass (134)
- `cargo check --locked --workspace --all-targets`: pass
- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-protocol-oteryn -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --lib capabilities`: pass
- `cargo test --locked -p oteryn-session -p oteryn-dev-client`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK

## Merge result

Recorded on #1622 at merge (squash merge of #1711).
