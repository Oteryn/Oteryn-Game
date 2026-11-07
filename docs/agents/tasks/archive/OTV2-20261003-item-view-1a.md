# OTV2-20261003-item-view-1a

```yaml
task_id: OTV2-20261003-item-view-1a
title: ITEM-VIEW-1a ITEM_VIEW_MOVE_V1 wire (capability 4, domains 9 and 11, command type 9, USE item target, D85 item handle), codecs and bounds
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
base_branch: main
branch: claude/item-view-1a-20261003
pr: "the one named in the #1622 FREEZE_SHA entry"
base_sha: a1d9f6e
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: work coordinator task worker (impl, protocol review), #1622
created_at: 2026-10-03T00:00:00Z
updated_at: 2026-10-03T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/protocol-oteryn/v1/item_view_v1.proto
  - docs/contracts/protocol-oteryn/v1/world_object_v1.proto
  - docs/contracts/protocol-oteryn/v1/world_spatial_v1.proto
  - crates/protocol-oteryn/src/{lib,item_view,item_view_tests,world_object,world_spatial_entities}.rs
  - docs/agents/tasks/archive/OTV2-20261003-item-view-1a.md
public_contracts:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/protocol-oteryn/v1/item_view_v1.proto
  - docs/contracts/protocol-oteryn/v1/world_object_v1.proto
  - docs/contracts/protocol-oteryn/v1/world_spatial_v1.proto
depends_on: ["ITEM-MOVE-WIRE-0 D212", "ARCH-BATCH-ITEM-EQUIP-PACKETS §1.1 and §2.1 (#1698)"]
blocks: [ITEM-VIEW-1b, ITEM-EQUIP-WIRE-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

ITEM-VIEW-1a registers the whole ITEM-MOVE-WIRE-0 wire, with the numbers D212 assigned. It has no
server or client behaviour.

- **Capability 4 `ITEM_VIEW_MOVE_V1`**, `offered: false` until ITEM-MOVE-1, `requires: [6]`. It
  joins `REGISTERED_CAPABILITY_IDS_V1`, so a selected 4 decodes. The server selects none.
- **Domain 9 `CHARACTER_INVENTORY`** and **domain 11 `OPEN_CONTAINER`**, snapshot type 1 and delta
  type 1 each. A delta carries the whole view after the commit. Entries are
  `ItemEntryV1 {handle, item_definition_ref, count, sub_type}`. Domain 9 is the main backpack slot
  and its direct entries; domain 11 is the open corpse's handle and its entries. Entries without
  their holder, and a handle repeated within one view, fail closed.
- **Command type 9 `ITEM_MOVE_INTENT`:** `{source_handle, destination oneof {main_backpack}}`. The
  result is the WIRE-0 §5 outcome enum (`MOVED` to `REJECTED`), at most 4 bytes.
- **USE field 2** is now `ItemTargetV1 {handle}`. Fields 3 and 4 stay reserved.
  `decode_use_intent_target` accepts field 2 only when capability 4 is selected. The existing
  `decode_use_intent`, which the server calls, still refuses it.
- **D85 entry field 10 `item_handle`:** encoded and required on objects only by the
  `_with_item_handles` codecs. The plain codecs leave the handle out, so the bytes are unchanged
  byte for byte, and they refuse field 10. The client view decoders pick the codec from the
  selected capabilities.
- **Rows, measured:**
  - `ITEMV0-RL-01`: 30 items per domain-9 view (930 bytes).
  - `ITEMV0-RL-02`: 16 entries per domain-11 view, equal to
    `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX` (507 bytes).
  - `ITEMV0-RL-03`: 301 live handles per session, which is 255 + 30 + 16.
  - Max and max+1 tests cover both directions. All bytes are within FND-02.

## Notes for protocol review

1. The packet named `world_spatial.rs` as owned. The D85 entry codec is in
   `world_spatial_entities.rs`, so this PR edits that file instead (QUESTION to the #1622 control
   plane, option a).
2. A non-capability-4 session's encoder leaves the handle out instead of refusing it. ITEM-VIEW-1b
   can then build one entity list and encode it per session.
3. `ITEMV0-RL-03` is enforced by the ITEM-VIEW-1b handle table. Here a test only binds the
   derivation.
4. WIRE-0 also asks for a back-pointer in the MOVE-RL-11 decision. That file is outside these owned
   paths. The proto comments point to WIRE-0 instead.
