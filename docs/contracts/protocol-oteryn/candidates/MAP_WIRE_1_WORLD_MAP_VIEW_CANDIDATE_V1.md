# MAP-WIRE-1: world map view wire (contract candidate)

**Status: accepted by the owner. MAP-WIRE-2 (`OTV2-20261004-map-wire-2`) registers capability 18, domain 17 and the limits of §3.**
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
- **Visible entries per tile.** A tile carries at most `MAPW-RL-01` = 10 item entries, in stack
  order (bottom first).
  - The cut keeps the bottom entry of the composed stack (the ground, when the tile has one) and
    the 9 topmost entries. When the stack is longer, the entries between them are dropped and the
    tile carries `more = true`, as a Tibia server keeps the ground and the top things.
  - A dropped entry is not on the wire. The client draws only the received entries and cannot
    target a dropped one from the map, as in Tibia.
  - The cut never decides visible floors. The server decides them from the full composed stack
    (§2 Visible floors).
- **Viewport.** The window is 18x14 tiles, from `(x-8, y-6)` to `(x+9, y+7)` of the actor
  position.
  - Floors in view. On or above the surface (native floor -7 to 0), every floor from -7 to 0 is
    in view, at most 8. Below it, two floors above and two below the actor's floor are in view,
    bounded to -15, at most 5.
  - Each floor's window is shifted by its floor difference to the actor, as in the Tibia
    perspective.
  - This is the viewport of `MAP01-VIEWPORT-US`: 18x14 over every floor the client sees.
  - Every floor in view is sent, whatever is drawn. Which floors are drawn is the separate
    `first_visible_floor` (§2 Visible floors), so a roof that hides or shows floors never changes
    the tile set.
- **Visible floors (roofs).** The server sends, in every header, `first_visible_floor`: the
  highest floor the client draws. This is the presentation boundary of the graphics audit §9
  (`docs/architecture/OTERYN_GRAPHICS_RENDERING_DEEP_AUDIT_AND_WORLD_VFX_GATE_2026-09-09.md`).
  It is never line-of-sight or any other server authority: no command, reach or visibility check
  reads it.
  - **Facts.** The rule reads only bundle and content facts the server holds, over the full
    composed stack, never over the 10-entry cut:
    - a tile *limits the view* when the bottom entry of its composed stack is a Terrain entry of
      kind `ground`, `wall` or `roof` (format §12);
    - a tile *can be looked through* when no entry of its composed stack is of kind `wall` and no
      entry's item definition has `blocks_projectile` true.
  - **Rule.** For an actor at native floor `f` (a higher native floor is further up):
    1. Start with `first = 0` on or above the surface (`f >= -7`), else `first = min(f + 2, -8)`.
    2. Check the actor's tile, then each orthogonal neighbour (north, east, south, west) that can
       be looked through. Diagonal neighbours are not checked.
    3. For a checked position `(px, py)`, walk up `k = 1, 2, ...` while `f + k <= first` and
       `first > f`. At each step, check two tiles at floor `f + k`: first the tile physically above,
       `(px, py)`, then the tile that covers it in perspective, `(px + k, py + k)`. If either
       limits the view, set `first = f + k - 1` and stop this position.
    4. `first_visible_floor = first`. It always lies between `f` and the start value.
  - This is the OTClient `calcFirstVisibleFloor` rule over the same facts. The bundle has no
    `dontHide` appearance flag, so it is not applied; a later revision may add it.
  - **Where it changes.** The server recomputes the value after every actor move and after every
    composed-stack change of a tile the rule reads (at most 5 positions x 7 floors x 2 tiles = 70
    tiles). A change is carried by the next header: a snapshot, or a delta. A delta whose only
    change is the value is valid with no tile and no cleared entry (§3 Origin).
  - **Fixtures.** MAP-WIRE-2 tests the resolver on its own, independent of stack order, with the
    audit §9 fixtures: open surface, underground room, covered tile under a roof, roof edge,
    doorway, wall-adjacent actor, stairs and floor change, and a walk out of a house.
- **Snapshot.**
  - A snapshot carries every tile of the viewport in view: at most 18 x 14 x 8 = 2,016 tiles
    (`MAPW-RL-02`). A tile with no item is omitted, and the client draws it empty.
  - Snapshots are sent at join, on resync, on a floor change, on a teleport or any move longer
    than one step, and when the bundle digest or reset epoch changes.
  - The client drops every item handle of a tile it drops, so a handle never outlives the tile
    that carried it. The server drops a handle that leaves every view (§3 Handle budget).
- **Delta.**
  - A one-step move sends the newly visible tiles, at most 31 per floor and 248 in all
    (`MAPW-RL-03`), with the new window origin. The client drops the tiles that left the window.
    When every newly visible tile is empty, the delta carries only the new origin, with no tile
    and no cleared entry.
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
  sint32 first_visible_floor = 5; // the highest native floor drawn (§2 Visible floors); absent means 0
}

message MapItemV1 {
  oneof definition {              // exactly one, non-zero (§3 Definition reference)
    uint32 item_definition_ref = 1;    // an Item: the WorldSpatialEntityV1 item reference space
    uint32 terrain_definition_ref = 8; // a Terrain record: its bundle palette compact id
  }
  uint32 count = 2;               // 1..=100 for stackables, else 1
  uint32 sub_type = 3;            // fluid or charge subtype, 0 if none
  oneof origin {
    uint32 base_ordinal = 4;      // 0..63: an unhidden base entry with no handle; its key is derived (§4)
    uint64 item_handle = 5;       // non-zero: a handle-bearing entry within the handle budget (§3)
    bool display_only = 6;        // true: a handle-bearing entry beyond the handle budget (§3)
  }
  uint32 appearance_id = 7;       // 0..=65535: the 15.30 appearance object id, 0 if none (§3)
  uint64 object_revision = 9;     // base_ordinal entries only: the entry's overlay revision, 0 if none (§3)
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
  MapViewHeaderV1 header = 1;     // generation, digest and epoch equal the snapshot's; origin is the new window origin (§3)
  repeated MapTileV1 tiles = 2;   // changed or newly visible tiles
  repeated ActorPositionV1 cleared = 3; // tiles in view that became empty
  // tiles and cleared together: 0..=248 (MAPW-RL-03), disjoint, each strictly ascending;
  // 0 only when the origin moved (an origin-only move delta)
}
```

- **Fail closed.** These cases fail closed:
  - unknown, repeated or zero-valued required fields;
  - a tile outside the window;
  - unsorted or duplicate tiles;
  - a `base_ordinal` of 64 or more;
  - a zero handle, a `display_only` that is not `true`, or not exactly one origin;
  - an `object_revision` on an entry whose origin is not `base_ordinal`;
  - not exactly one definition reference, or a zero one;
  - a count outside its range;
  - a `ground_speed` above 1000;
  - an `appearance_id` above 65,535;
  - a delta whose origin is not the view's origin or one step from it on the same floor (§3
    Generation match);
  - a `first_visible_floor` above 0 or below the origin floor, or, with an origin floor below -7,
    above `min(origin.floor + 2, -8)`;
  - a delta with no tile and no cleared entry whose origin and `first_visible_floor` both equal the
    view's.
- **Bounds.**
  - `MapItemV1` encodes in at most 32 bytes, and `MapTileV1` in at most 360 bytes. The largest
    item is a `base_ordinal` entry with an `object_revision`: definition 6, count 2, sub-type 6,
    ordinal 2, appearance 4 and revision 11 bytes, 31 in all. A handle entry is at most 29.
  - The header encodes in at most 95 bytes with world coordinates in `0..=65,535`: two digests of
    34, the epoch 11, the origin 12, `first_visible_floor` 2 and the field tag and length 2. It is
    inside the 128-byte overhead of each payload bound, so the bounds below are unchanged.
  - A snapshot payload is at most 2,016 x 360 + 128 = 725,888 bytes. It is streamed in two
    chunks under `FND02-SNAPSHOT-CHUNK-BYTES` (524,288) and stays far under
    `FND02-SNAPSHOT-ASSEMBLED-BYTES` (16 MiB).
  - A delta payload is at most 248 x 360 + 248 x 16 + 128 = 93,376 bytes, under
    `FND02-STATE-DELTA-PAYLOAD-BYTES` (262,144).
  - MAP-WIRE-2 registers `MAPW-RL-01` to `-04`, `ITEMV0-RL-03-MAP-VIEW` and the payload maxima
    computed from its codec.
- **Handle budget.** The handle-bearing entries are overlay-added items, Ground items and
  movable base entries (§3 Move source). Every other base entry carries no handle; it is named by
  its ordinal (§4).
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
- **Move source.** `ItemMoveIntentV1` (command 9) names its source only by `source_handle`, so a
  base entry that can be moved carries a handle.
  - A movable base entry is one eligible for pickup under ADR-0021 §4.4: pickupable, not on a
    house tile, with no action, unique, door, depot or teleport binding, no contents and no
    unrepresentable attribute. The server decides eligibility from the bundle and the content
    generation; the client never does.
  - It is sent with an `item_handle` within the handle budget, else `display_only`. It never
    carries `base_ordinal` or `object_revision`.
  - The server binds the handle to `(bundle_digest, placement_key, reset_epoch)`, never to an
    ItemInstance. A `USE` with it resolves the base entry and checks that the overlay does not
    hide it, else `STALE`, then runs the base-entry `USE` path of §4.
  - A move of it is the §4.4 pickup: a MINT into Ground at its tile with the
    `MapItemMaterialization` cause, then the ordinary TRANSFER to the intent's destination. Its
    reach and destination checks are the existing ones. After the MINT, the origin is hidden and
    its tile is resent, so the handle no longer resolves to the base entry.
  - The pickup is implemented by MAP-PICKUP-1 (packets §2.4), after the MINT of MAP-OVERLAY-1b and
    the command-9 handler of ITEM-MOVE-1. Until it merges, command 9 from a base-entry handle is
    refused with `ITEM_MOVE_OUTCOME_NOT_SUPPORTED` and writes nothing.
  - Every other base entry cannot be moved from the map in this contract (§5); it has no handle,
    so no command 9 can name it.
- **Object revision.** A base entry with state (a door, lever or other transform) is fenced by its
  overlay revision.
  - `object_revision` is the overlay revision of the entry's bundle placement key in the current
    reset epoch, 0 while the overlay holds no state for it. Only `base_ordinal` entries carry it.
  - Every transition raises the revision and resends the tile, so the client always holds the
    current value.
  - A `USE` on the entry sends it as `WorldObjectTargetV1.expected_revision`. The server compares
    it with the current revision after the digest and hide checks. A different value is `STALE`
    and changes nothing, and the client's next `USE` from the resent tile carries the new value.
  - A reset-epoch change sends a snapshot, so a revision of another epoch never reaches a
    command.
  - The field adds at most 11 bytes to an entry with no handle, so the per-item bound holds.
- **Definition reference.** Item ids and Terrain catalogue ids are separate spaces, so an entry
  names its family.
  - A base entry whose palette entry has family `item`, and every overlay-added or Ground item,
    sends `item_definition_ref`, in the WorldSpatialEntityV1 item reference space.
  - A base entry whose palette entry has family `terrain` sends `terrain_definition_ref`. This
    is the palette entry's compact `id` in the Terrain catalogue of the bundle's content
    revision, which the header's `bundle_digest` pins.
  - Neither reference is trusted for a command. A command names a base entry by its key (§4),
    and an item by its handle.
  - The oneof sends one 1-byte tag and one varint, so the per-item bound holds.
- **Appearance.** `appearance_id` names the 15.30 client appearance object the client draws
  (owner #1793 Q5b). The server derives it from a definition key:
  - `<id>` of an `oteryn:item.tibia.i<id>` or `oteryn:terrain.tibia.i<id>` key;
  - `source_item_id` of a provisional donor key (ARCH-WORLD-CONTENT-SERVE-1 §1.4).
  - The key of a base entry is its bundle palette key. An overlay-added or Ground item is not
    in the palette, so its key is the definition key of its item content definition (the key
    the active content generation maps its `item_definition_ref` to).
  - Any other key sends 0, and the client draws a placeholder.
  - The definition reference stays the identity. `appearance_id` is only for drawing and is
    never trusted for a command.
  - The field adds at most 4 bytes, so the per-item and per-tile bounds hold.
- **Generation match.** The client binds the view to `(content_generation, bundle_digest,
  reset_epoch)`. A delta whose binding differs from the snapshot's is a
  `STATE_REVISION_MISMATCH`. The client sends the existing `ResyncRequest` and draws nothing from
  that delta. The server never sends a delta across a digest or epoch change; it sends a snapshot.
- **Origin.** The header's `origin` is not part of the binding. It is the window origin after the
  payload.
  - A snapshot sets the view's origin.
  - A delta's origin is either the view's origin (a tile change) or one step from it on the same
    floor, a Chebyshev distance of 1 (a one-step move). Any other origin fails closed, and the
    client resyncs.
  - The client applies a delta in this order: it moves the window to the new origin and drops the
    tiles that left it, then applies the tiles and cleared entries, which must all lie in the new
    window.
  - A delta with no tile and no cleared entry is valid only when its origin moved or its
    `first_visible_floor` changed.
  - `first_visible_floor` is not part of the binding either. Each header sets the view's value.
    A change of it alone never moves the window and never resends a tile.
- **Ground speed.** Each tile carries its ground speed, so the client times a step with the
  same value as the server (ADR-0021 MAP-LOAD-1 amendment, SPEED-1). A bundle World uses the
  tile's ground speed on both sides; the 150 default stays only for the fixture World.
- **Ground items move here.** With capability 18 selected, Ground items and corpses are carried
  only in map tiles. They are not domain-1 `GROUND_ITEM` or `CORPSE` entities, so no item is drawn
  twice. Actors stay in domain 1. The client draws each actor in the creature phase of the tile at
  its domain-1 position (packets §2.3 Drawing), whether or not domain 17 sends that tile.
- **One object domain per World.** A World booted from a bundle serves domain 17 and does not
  serve `WORLD_OBJECT_OVERLAY`. Doors, levers and other object states are item transforms in the
  overlay, so they reach the client as tile changes. The fixture World (`native_entry_room`) keeps
  domain 2 and does not serve domain 17. A client connecting to a bundle World without
  capability 18 is refused at admission with `CAPABILITY_MISMATCH`, because it could not see the
  map.

## 4. Wire target of a map entry

- **Decision.** The client names a map entry by its bundle placement key (format §7), not by a
  canonical `PlacementKey`:
  - an unhidden base entry with no handle: `placement_key = x << 32 | y << 16 | (-floor) << 8 |
    base_ordinal`, derived from the tile position and the ordinal it was sent with;
  - a movable base entry, an added or a Ground item: its `item_handle`, as today
    (`ItemTargetV1`), and as the `source_handle` of command 9 (§3 Move source);
  - a `display_only` entry: nothing. It is not targetable until it is resent with a handle.
- **The client branches on origin.** The entry's `origin` alone selects the target: a
  `base_ordinal` entry sends the 40-byte `WorldObjectTargetV1` below, with `expected_revision`
  equal to its `object_revision`; an `item_handle` entry sends `ItemTargetV1` (or the
  `source_handle` of a move); a `display_only` entry sends no command. The definition reference
  and `appearance_id` never select it.
- **Carriage.** A `USE` on a base entry sends `WorldObjectTargetV1.placement` as the 8-byte
  big-endian placement key, prefixed by the 32-byte bundle digest it was drawn from: 40 bytes in
  all, under the 512-byte bound. This adds no command type and no field. A bundle World accepts
  only this 40-byte form. The fixture World accepts only the canonical bytes it accepts today.
- **Resolution on the server.**
  - A digest that is not the active bundle digest is refused as stale (`STATE_REVISION_MISMATCH`),
    so a key never names an entry of another bundle.
  - The server reads the key's entry from the base and checks that the overlay does not hide it.
  - It checks `expected_revision` against the entry's overlay revision (§3 Object revision).
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
- Moving a base entry that is not eligible for pickup under ADR-0021 §4.4 (furniture, bound
  items). It needs a durable move model for map-authored entries.
- The `dontHide` appearance flag in the visible-floor rule (§2 Visible floors), and any floor
  fading or partial roof transparency.
- Creatures on tiles: they stay domain-1 actors.
- Houses: the World-scoped house interior runtime serves owned house tiles (ADR-0021 §4.4), and
  its wire is a later child.
- Changing the per-tile cut of 10 entries or the viewport size: each is a schema revision.
