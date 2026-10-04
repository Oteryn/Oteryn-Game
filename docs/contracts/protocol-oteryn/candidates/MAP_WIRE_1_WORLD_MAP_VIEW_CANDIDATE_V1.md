# MAP-WIRE-1: world map view wire (contract candidate)

**Status: candidate for owner acceptance. It does not bind until the owner accepts it.**
Until then it allocates no capability, state domain, snapshot type or delta type. It enables no
transport route and amends no accepted schema. The numbers in §3 are proposals. The control
plane leases them when MAP-WIRE-2 registers them.

- Parent: FND-02 (`docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`, the snapshot and delta
  limits of `RESOURCE_LIMITS_REGISTRY.json`).
- Origin:
  - ADR-0021 §5 MAP-WIRE-1 (`docs/architecture/ADR-0021-world-map-runtime-loading.md`);
  - the placement key of `OTERYN_WORLD_BUNDLE_FORMAT_V1.md` §7;
  - ARCH-WORLD-CONTENT-SERVE-1 §1.6 and §1.8 (#1792).
- Packets: `docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_WIRE_PACKETS_2026-10-04.md`.

## 1. What a client must see, and what carries it today

A World that boots from a bundle (MAP-CUTOVER-1) has client-visible item state that no
accepted domain carries:

- the base stack of each tile in view;
- base entries the channel overlay hides (picked up, moved, used or transformed);
- items the overlay adds;
- Ground items: player drops, loot, corpses and materialized map items.

The `WORLD_OBJECT_OVERLAY` domain carries only keyed object states, with `WOBJ-RL-03` (486
entries per snapshot). The `WORLD_SPATIAL_VISIBILITY` v2 entities carry at most 256 entries
(`MOVE-RL-11`) and no base stack. Neither can carry a map view.

## 2. Model: the server streams the stack of each tile in view

- **No client map file.** The client reads no bundle and no client projection of it. The
  server sends the visible stack of every tile in the viewport, as a Tibia server does. The
  server-only bundle (ADR-0021 §4.2) stays the only map artifact, and no map data is
  redistributed with the client.
- **One stack per tile.** The server composes it from the base stack, minus hidden entries, plus
  added overlay entries and Ground items, in stack order. The client never merges base and
  overlay itself.
- **Visible entries per tile.** A tile carries at most `MAPW-RL-01` = 10 item entries: the top
  10 of the composed stack, ground first, as a Tibia server sends them. When the stack is longer,
  the tile carries `more = true`. An item below the cut is not on the wire and cannot be targeted
  from the map, as in Tibia.
- **Viewport.** The window is 18x14 tiles, from `(x-8, y-6)` to `(x+9, y+7)` of the actor
  position.
  - Floors in view. On or above the surface (native floor -7 to 0), every floor from -7 to 0 is
    in view, at most 8. Below it, two floors above and two below the actor's floor are in view,
    bounded to -15, at most 5.
  - Each floor's window is shifted by its floor difference to the actor, as in the Tibia
    perspective.
  - This is the viewport of `MAP01-VIEWPORT-US`: 18x14 over every floor the client sees.
- **Snapshot.**
  - A snapshot carries every tile of the viewport in view: at most 18 x 14 x 8 = 2,016 tiles
    (`MAPW-RL-02`). A tile with no item is omitted, and the client draws it empty.
  - Snapshots are sent at join, on resync, on a floor change, on a teleport or any move longer
    than one step, and when the bundle digest or reset epoch changes.
- **Delta.**
  - A one-step move sends the newly visible tiles, at most 31 per floor and 248 in all
    (`MAPW-RL-03`), with the new window origin. The client drops the tiles that left the window.
  - A change to a tile in view resends that whole tile.
  - A delta over `MAPW-RL-03` tiles is sent as a snapshot instead, never truncated.

## 3. Wire shape (proposed)

The proposed numbers are:

- capability 18 `WORLD_MAP_VIEW_V1`, which requires 6 (`WORLD_SPATIAL_ENTITIES`) and 4
  (`ITEM_VIEW_MOVE_V1`), because item handles and `ItemTargetV1` are capability-4 types;
- state domain 17 `WORLD_MAP_VIEW`, owned by the current ChannelRuntime;
- snapshot type 1 `WORLD_MAP_VIEW_SNAPSHOT_V1`;
- delta type 1 `WORLD_MAP_VIEW_DELTA_V1`;
- the schema `docs/contracts/protocol-oteryn/v1/world_map_v1.proto`.

```proto
message MapViewHeaderV1 {
  bytes content_generation = 1;   // 32 bytes, the World's existing content generation
  bytes bundle_digest = 2;        // 32 bytes, the active bundle digest (format §5)
  uint64 reset_epoch = 3;         // the channel's current World reset epoch
  ActorPositionV1 origin = 4;     // the actor position the window is centred on
}

message MapItemV1 {
  uint32 item_definition_ref = 1; // non-zero; the WorldSpatialEntityV1 item reference space
  uint32 count = 2;               // 1..=100 for stackables, else 1
  uint32 sub_type = 3;            // fluid or charge subtype, 0 if none
  oneof origin {
    uint32 base_ordinal = 4;      // 0..63: an unhidden base entry; its key is derived (§4)
    uint64 item_handle = 5;       // non-zero: an overlay-added or Ground item within the handle budget
    bool display_only = 6;        // true: an overlay-added or Ground item beyond the handle budget (§3)
  }
  uint32 appearance_id = 7;       // 0..=65535: the 15.30 appearance object id, 0 if none (§3)
}

message MapTileV1 {
  ActorPositionV1 position = 1;
  repeated MapItemV1 items = 2;   // 1..=10 (MAPW-RL-01), stack order, ground first
  bool more = 3;                  // the composed stack has more than 10 entries
  uint32 ground_speed = 4;        // 0..=1000: the ground item's speed (WO-0), 0 or absent without a ground
}

message WorldMapViewSnapshotV1 {
  MapViewHeaderV1 header = 1;
  repeated MapTileV1 tiles = 2;   // 0..=2,016 (MAPW-RL-02), strictly ascending (floor, y, x)
}

message WorldMapViewDeltaV1 {
  MapViewHeaderV1 header = 1;     // digest and epoch must equal the snapshot's
  repeated MapTileV1 tiles = 2;   // changed or newly visible tiles
  repeated ActorPositionV1 cleared = 3; // tiles in view that became empty
  // tiles and cleared together: 1..=248 (MAPW-RL-03), disjoint, each strictly ascending
}
```

- **Fail closed.** These cases fail closed:
  - unknown, repeated or zero-valued required fields;
  - a tile outside the window;
  - unsorted or duplicate tiles;
  - a `base_ordinal` of 64 or more;
  - a zero handle, a `display_only` that is not `true`, or not exactly one origin;
  - a count outside its range;
  - a `ground_speed` above 1000;
  - an `appearance_id` above 65,535.
- **Bounds.**
  - `MapItemV1` encodes in at most 32 bytes, and `MapTileV1` in at most 360 bytes.
  - A snapshot payload is at most 2,016 x 360 + 128 = 725,888 bytes. It is streamed in two
    chunks under `FND02-SNAPSHOT-CHUNK-BYTES` (524,288) and stays far under
    `FND02-SNAPSHOT-ASSEMBLED-BYTES` (16 MiB).
  - A delta payload is at most 248 x 360 + 248 x 16 + 128 = 93,376 bytes, under
    `FND02-STATE-DELTA-PAYLOAD-BYTES` (262,144).
  - MAP-WIRE-2 registers `MAPW-RL-01` to `-04`, `ITEMV0-RL-03-MAP-VIEW` and the payload maxima
    computed from its codec.
- **Handle budget.** Base entries carry no handle; they are named by their ordinal (§4). Only
  overlay-added and Ground items carry an `item_handle`.
  - `MAPW-RL-04` = 1,024 handle-bearing entries in one map view. The server gives handles in a
    fixed order: the actor's floor first, then by floor distance; within a floor, by Chebyshev
    distance to the actor, then `(y, x)`, then stack order. An entry past the budget is sent as
    `display_only`. The client draws it but cannot target it until a later window brings it
    within the budget, and the tile is resent with its handle.
  - The session's live-handle bound adds the map view, as `ITEMV0-RL-03-CONTAINER-TREE` did for
    capability 14. With capability 18, MAP-WIRE-2 registers `ITEMV0-RL-03-MAP-VIEW` = 301 + 1,024
    = 1,325, and with capabilities 14 and 18 together, 637 + 1,024 = 1,661. The handle table is
    bounded by the selected value before inserting. A handle that leaves every view is dropped.
  - A handle that changes with a window move resends its tile. If that pushes a delta over
    `MAPW-RL-03`, a snapshot is sent instead.
  - Reach for `USE` and move is at most a few tiles, so every reachable item is within the budget
    unless more than 1,024 handle-bearing entries lie nearer to the actor.
- **Appearance.** `appearance_id` names the 15.30 client appearance object the client draws
  (owner #1793 Q5b). The server takes it from the entry's bundle palette key: `<id>` of
  `oteryn:item.tibia.i<id>` or `oteryn:terrain.tibia.i<id>`. Any other key sends 0, and the
  client draws a placeholder. `item_definition_ref` stays the item identity; `appearance_id` is
  only for drawing and is never trusted for a command. The field adds at most 4 bytes, so the
  per-item and per-tile bounds hold.
- **Generation match.** The client binds the view to `(content_generation, bundle_digest,
  reset_epoch)`. A delta whose header differs from the snapshot's is a `STATE_REVISION_MISMATCH`.
  The client sends the existing `ResyncRequest` and draws nothing from that delta. The server
  never sends a delta across a digest or epoch change; it sends a snapshot.
- **Ground speed.** Each tile carries its ground speed, so the client times a step with the
  same value as the server (ADR-0021 MAP-LOAD-1 amendment, SPEED-1). A bundle World uses the
  tile's ground speed on both sides; the 150 default stays only for the fixture World.
- **Ground items move here.** With capability 18 selected, Ground items and corpses are carried
  only in map tiles. They are not domain-1 `GROUND_ITEM` or `CORPSE` entities, so no item is drawn
  twice. Actors stay in domain 1.
- **One object domain per World.** A World booted from a bundle serves domain 17 and does not
  serve `WORLD_OBJECT_OVERLAY`. Doors, levers and other object states are item transforms in the
  overlay, so they reach the client as tile changes. The fixture World (`native_entry_room`) keeps
  domain 2 and does not serve domain 17. A client connecting to a bundle World without
  capability 18 is refused at admission with `CAPABILITY_MISMATCH`, because it could not see the
  map.

## 4. Wire target of a map entry

- **Decision.** The client names a map entry by its bundle placement key (format §7), not by a
  canonical `PlacementKey`:
  - an unhidden base entry: `placement_key = x << 32 | y << 16 | (-floor) << 8 | base_ordinal`,
    derived from the tile position and the ordinal it was sent with;
  - an added or Ground item: its `item_handle`, as today (`ItemTargetV1`).
- **Carriage.** A `USE` on a base entry sends `WorldObjectTargetV1.placement` as the 8-byte
  big-endian placement key, prefixed by the 32-byte bundle digest it was drawn from: 40 bytes in
  all, under the 512-byte bound. This adds no command type and no field. A bundle World accepts
  only this 40-byte form. The fixture World accepts only the canonical bytes it accepts today.
- **Resolution on the server.**
  - A digest that is not the active bundle digest is refused as stale (`STATE_REVISION_MISMATCH`),
    so a key never names an entry of another bundle.
  - The server reads the key's entry from the base and checks that the overlay does not hide it.
  - When the entry is bound to a RewardClaim placement (ARCH-WORLD-CONTENT-SERVE-1 §1.4), the
    server looks up the canonical `PlacementKey` through that binding. The canonical key is the
    `source_placement` of the MINT and audit, as today.
  - The lookup is the in-memory binding rebuilt at each activation (§1.6 there). It writes
    nothing.
- **Why not the canonical key.** Only RewardClaim placements have a canonical key. Every other
  base entry (doors, levers, pickupable items) has only its bundle key, so a canonical-only target
  cannot address the map. Canonical keys of up to 512 bytes per entry would also not fit the
  per-tile bound.
- **Durability is unchanged.** No durable row stores a bundle placement key except through
  `MapItemMaterialization` (ADR-0021 §4.4), which already carries it together with the digest.

## 5. Not decided here

- Animation, outfit and effect sprites. Static appearance sprites are decided (owner #1793
  Q5b; packets §2.2 MAP-SPRITE-1).
- Light, weather, minimap and tile flags beyond the item stack.
- Creatures on tiles: they stay domain-1 actors.
- Houses: the World-scoped house interior runtime serves owned house tiles (ADR-0021 §4.4), and
  its wire is a later child.
- Changing the per-tile cut of 10 entries or the viewport size: each is a schema revision.
