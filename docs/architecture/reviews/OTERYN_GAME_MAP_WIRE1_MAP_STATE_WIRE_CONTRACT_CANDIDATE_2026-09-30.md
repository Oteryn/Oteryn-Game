# MAP-WIRE-1 Map state on the wire

- Contract: `MAP-WIRE1-MAP-STATE-V1`
- Status: **CONTRACT CANDIDATE** (ADR-0021 §5). The owner answered M1 with b (2026-09-30, §10), the
  design written here: the server streams the map as in Tibia. Acceptance needs exact-head
  validation, independent protocol and security review and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the control plane's map lane scheduling (#162 5914289140: MAP-WIRE-1 after CONDITIONS-0;
  DEPOT-WIRE-1 and the house views depend on it) and ADR-0021 §5's wire child
- Builds on: ADR-0021 (candidate: shared base bundle per World, per-channel overlay, hidden
  origins, `placement_key`, native floors `[-15 .. 0]` with `floor = -z`, activation only at a
  planned reset); MOVE-RL-11 (D84-D87: interest area, floors, ground items and corpses as entities
  of `WORLD_SPATIAL_VISIBILITY`, 256 entities, send order, D222 amendment); USE-WIRE-V1 (domain 2
  `WORLD_OBJECT_OVERLAY` keyed by `placement_key`; `content_generation` = the client artifact
  digest); ITEM-MOVE-WIRE-0 §4.1 (base-item handles owed to this contract) and -1; FND-02
  §9-§19; DUR-04 §8 (allowlisted projections); DUR-03 §39.1 (map-item MINT); owner rule 5905825574
  (Global parity)
- Runtime, protocol registration and production authority: NONE. The children need their own
  #162 allocations; numbers are reserved there.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| MAP-WIRE-2 | hard (protocol), protocol and security review | capability `MAP_STATE_V1`, domain `MAP_TILES` (§4-§7), the `map_item_handle` (§6), the bounds and their measurement (§8) | MAP-LOAD-1; MAP-OVERLAY-1; VIS-2 |
| MAP-CLIENT-1 | client lane | the native client's decoder and renderer for `MAP_TILES`, drawing the combined tile order (§5) | MAP-WIRE-2 |

**Amendment (ARCH-ITEM-PACKETS-AMEND-1; `reviews/OTERYN_GAME_ARCH_BATCH_ITEM_EQUIP_PACKETS_2026-10-03.md`
§1.11), pending on acceptance of MAP-WIRE-1.** MAP-WIRE-2 carries each described tile's ground
speed in `MAP_TILES`, omitted when 150, with max and absent codec tests. MAP-CLIENT-1 paces client
steps from it and switches SPEED-1's server seam to MAP-LOAD-1's map source in the same PR, as a
merge condition, with a production-path test on a tile whose ground speed is not 150. The field
joins the §3 allowlist and the §8 bounds as amended there (#1702 P2 4175398450).

## 1. Question

How does a client learn what lies on the map it sees: the base map, the base items the channel
overlay hides or moves, and the items players leave on the ground?

## 2. Facts

**PROVEN**

- ADR-0021: one base bundle per World; each channel's overlay hides base items per tile (64-bit
  mask, at most 64 base items per tile, 26 in the base map); a moved base object keeps its one
  `placement_key` entry (§4.4); picking up a map item is a MINT that hides the origin (DUR-03
  §39.1); owned house tiles are served by the house interior runtime; the server refuses a
  non-`production` bundle in production; the bundle changes only at a planned reset.
- MOVE-RL-11 D85: creatures, players, corpses and ground items are entities of
  `WORLD_SPATIAL_VISIBILITY` (VIS-2), at most 256 per snapshot or delta; D87 with D222 is the send
  and cutoff order (actors before items, nearest first). `interest.rs` uses legacy floors.
- USE-WIRE-V1: domain 2 carries world-object states keyed by `placement_key`, each checked against
  `content_generation`, which is `content_pin().client_artifact_digest()`
  (`gameplay_transport/mod.rs:545`): item appearances, not the map.
- ITEM-MOVE-WIRE-0 §4.1 leaves base-item handles to this contract; ITEM-MOVE-WIRE-1 §9 defers
  map-object and Ground-to-Ground moves.
- FND-02 §19: a delta is at most 256 KiB, a snapshot chunk 512 KiB, at most 256 chunks and 16 MiB
  assembled, and at most 4,096 entries in a repeated field. `SnapshotBody` is shared by all domains.
- ADR-0021 measured 13 µs per 18 × 14 overlay read (40 µs at hotspots) on B3.
- The native client renders a placeholder; no map reaches it today.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b51`, OTClient)

- The Tibia protocol has no client map: the server describes the visible area over its floors at
  login, a row or column per floor on a cardinal step, a row and a column on a diagonal step
  (`sendMoveCreature`, `protocolgame.cpp:8840-8853`), and a full description after a floor change,
  teleport or respawn; at most 10 things per tile (`GetTileDescription`).
- Tibia draw order on a tile: ground, top items, creatures, then the other items, newest first.

## 3. What the server sends (M1 b)

- The server describes each tile of the observer's interest area from the active bundle and the
  channel overlay: the base entries the overlay does not hide, where the overlay currently places
  them. The client holds no map file and sees only the area it is in, as in Tibia.
- **Allowlist** (DUR-04 §8). Per tile: its position (implicit, §7) and, under the amendment of the
  Implementation brief, its ground speed: a varint in 1..=1,000 (the content maximum is 850),
  omitted when 150 or when the ground item has none; a value outside the range fails the encode.
  Per base item:
  - the client item type (the appearance id already in the client artifact);
  - count or subtype, only where the appearance needs it (stackables, fluids);
  - its stack position (§5);
  - a per-session `map_item_handle` (§6), only for items the client can act on (usable, movable,
    containers, doors, or with a domain-2 state). The handle is the wire placement identity; the
    server maps it to the `placement_key`, which is never encoded on the wire (checklist item 1).

  Nothing else leaves the server: action, unique, door and depot ids, reward bindings, teleport
  destinations, text and descriptions (sent only by Look), house membership, protection-zone and
  no-logout flags, `build_class`.
- Only a `production` bundle is ever described; draft areas cannot reach a client.
- `placement_key` is not encoded on the wire; the handle stands for it. MAP-BUNDLE-1 still proves
  it is not derivable from legacy action or unique ids, because domain 2 keys on it.
- **Owned house tiles** (HOUSE-CUSTODY-0 §7 item 5 gives their view to this contract): their base
  entries are described by `MAP_TILES` like any tile, with the house interior runtime as the source
  of what is hidden; `HouseInterior` items are durable custody and travel as domain 1 entities,
  exactly like Ground items (§4). HOUSE-RUNTIME-0 supplies the runtime, not a new wire.
- **Map revision.** Each snapshot names the bundle digest (`map_revision`) once; it differs from
  `content_generation` and changes only at a planned reset, when every session ends.

## 4. Base items and domain 1

- Player items on the ground and in house interiors (DUR-03 custody) are domain 1 entities only
  (D85); `MAP_TILES` never
  carries them. The overlay's `added` items are not on the wire: no accepted command produces a
  volatile added item today; a later producer amends this contract.
- **Pickup of a map item.** While its MINT is in doubt the origin stays described. After the MINT
  commits, the hide in `MAP_TILES` and the new domain 1 entity are published in the same sync unit,
  under one `server_sequence`. An aborted MINT changes nothing. An origin is never described while
  its minted item is also an entity.

## 5. One order on a tile

- Each thing on a tile has a stack class and a position:
  1. ground;
  2. top items (by their appearance's top order);
  3. creatures and players (domain 1, in D87 order);
  4. other items: base items (`MAP_TILES`) and ground items (domain 1) in one sequence, newest on
     top. A base item's position is its compiler order; a domain 1 ground item carries a position
     in the same space, assigned on drop above every existing one.
- `MAP_TILES` describes, per tile, at most `MAPW1-RL-01` (10) base items: the first 10 in this
  order (ground, top items, then other items newest first); the rest of a tile's up to 26 base
  entries are not sent, as Tibia sends at most 10 things. Domain 1 bounds ground items per
  snapshot (D85's 256). The client draws the combined order and shows the first 10 things; the cut
  is display only and changes no server state.

## 6. Handles

- **`map_item_handle`** follows ITEM-MOVE-WIRE-0 §4.1: a per-session u64, issued in snapshots and
  deltas for actionable base items, reissued in every snapshot (so after every reconnect, transfer
  or floor change), and answered `STALE` when old. The server maps it to (`overlay_incarnation`,
  position, `placement_key`, `tile_revision`).
- **`tile_revision`**: u32 per tile per channel overlay, raised by every overlay change of that
  tile. **`overlay_incarnation`**: a u64 the channel draws at every start or restart, different per
  channel. A handle binds both, so a restart (volatile overlay lost) or a transfer to another
  channel makes every old handle `STALE`; a counter that restarts never matches an old handle.
- It is a new source variant of command 9 (ITEM-MOVE-WIRE-0 §4.1) and of USE-WIRE-V1's use target:
  `map_item {handle}`, at most 11 bytes (a u64 varint of up to 10 bytes and its 1-byte tag). Which
  moves it enables stays with ITEM-MOVE-WIRE-1 §9.
- **Handle budget.** Map handles have their own per-session table, separate from
  `ITEMV0-RL-03`: at most `MAPW1-RL-07` (80,640 = 8 floors × 1,008 tiles × 10 items), the handles of
  the described area. Each snapshot replaces the whole table (older map handles answer `STALE`);
  `AREA_ENTER` and `TILE_SET` add the handles of the tiles they describe and drop those of tiles
  that left the area or changed, so the table never exceeds the area. Only actionable items get a
  handle, so the typical table is far smaller; MAP-WIRE-2 measures it with the bytes (§8).
- **Replay of a pickup.** No ItemInstanceId exists before the MINT. The command's replay binding is
  (CommandRef, `overlay_incarnation`, `placement_key`); the MINT plans the new item's identity in
  that transaction (DUR-03 §11.3, §39.1) and records it with the CommandRef, so a replay returns the
  first outcome with the same ItemInstanceId, and a replay after a restart finds the recorded
  outcome, never a second MINT.
- **Look** on an item without a handle names `{position, stack position}` of the last described
  state; a tile changed since then answers `STALE`.

## 7. Snapshots and deltas (MAP-WIRE-2)

- **Floors.** The area uses native `FloorId` (ADR-0021 §4.3). At or above the surface (floor ≥
  −7) the client gets floors −7 to 0; below it, the observer's floor ± 2 within `[-15 .. 0]`. Each
  floor's tiles are offset by `target floor − observer floor` diagonally, as Tibia draws it.
  MAP-WIRE-2 maps `interest.rs`'s legacy floors to these.
- **Snapshot** (`AREA_SNAPSHOT`): one repeated list per floor, each with its origin and its tiles
  in row-major order; `map_revision` once. Triggers: admission, reconnect, channel transfer, floor
  change, teleport, respawn, and any movement whose delta would pass `MAPW1-RL-04`.
- **Snapshot rate.** At most one snapshot in flight per session; triggers while one is in flight
  coalesce into one pending snapshot of the latest area. At most `MAPW1-RL-05` (4) snapshots per
  10 s per session; a further trigger waits for the window, and step deltas meanwhile are dropped
  because the pending snapshot supersedes them.
- **Deltas:**
  - `AREA_ENTER` for one step on the same floor, cardinal or diagonal: the new origin and the
    entering row, column or both on every visible floor; the client drops what left the area.
  - `TILE_SET`: one tile's full description after an overlay change; at most `MAPW1-RL-06`
    (1,024) per sync unit, beyond which the server sends a snapshot instead.
- **Revision:** one domain revision per GameSession (FND-02 §15), stored in the resume state
  (`resume.rs`) like the other domains and kept above any seen across a reconnect.

## 8. Bounds (MAP-WIRE-2 registers and measures)

Worst case per encoded item 22 bytes (type 4, count 4, stack position 3, handle 11 with its tag;
positions are implicit in the per-floor lists); per tile at most 10 items, 10 × 22 = 220 bytes plus
6 bytes of tile framing (226); the largest configurable area is 36 × 28 (MOVE-RL-11).
*Amended (ground speed, pending on acceptance of MAP-WIRE-1):* the tile framing adds at most
3 bytes for ground speed (a 1-byte tag and a 2-byte varint), so 9 bytes and 229 per tile; the rows
below use 229.

| Row | Value |
|---|---|
| `MAPW1-RL-01` items described per tile | 10 (Tibia's per-tile cap) |
| `MAPW1-RL-02` tiles per floor list | 36 × 28 = 1,008, under FND-02's 4,096 |
| `MAPW1-RL-03` snapshot | 8 floors × 1,008 × 229 bytes ≈ 1.85 MB worst, in 4 chunks, under 16 MiB with the other domains |
| `MAPW1-RL-04` `AREA_ENTER` | a diagonal step at 36 × 28: 63 × 8 tiles × 229 bytes ≈ 115 KB, under 256 KiB |
| `MAPW1-RL-05` snapshots per session | 4 per 10 s, one in flight, coalesced |
| `MAPW1-RL-06` `TILE_SET` per sync unit | 1,024 |
| `MAPW1-RL-07` live map handles per session | 80,640 (the described area), separate from `ITEMV0-RL-03` |

- Before activation, MAP-WIRE-2 measures on B3 and records on #162: typical and p99 bytes of a
  login snapshot and of a cardinal and a diagonal step, and server time per description.

## 9. Capability

- **`MAP_STATE_V1`**, requiring `WORLD_SPATIAL_ENTITIES` (VIS-2). A session without it keeps
  today's placeholder; it is not refused. Numbers, the domain id and delta types are reserved on
  #162 at allocation.

## 10. Owner question (answered: b)

**M1. Where does the base map come from?** The owner chose b on 2026-09-30.
- **b (recommended):** streamed by the server as in Tibia (§3-§8): nothing to download when the
  map changes, only visited areas reach the client, ADR-0021 measured the server read at 13-40 µs
  per view.
- **a:** from a map file in the client, checked at login. It departs from Global parity; it ships
  the whole map, hidden and quest areas included, to every client; every reset that changes the map
  forces a client update through an updater and signing that are not yet decided (ALPHA-CLIENT-01
  §17); it needs a new admission check and a digest in `ClientBootstrap`.

Option a is not taken.

## 11. Rejected options

- **Ground items in this domain.** D85 already puts them in domain 1 with one ceiling and order.
- **Sending the whole overlay of a channel.** Unbounded; the interest area bounds it.
- **A client sector cache keyed by ADR-0021's sector checksums.** It stores visited map on the
  client and needs a cache format and eviction; the stream is cheap enough (§2) and the cache can
  be added later behind a capability if measurement asks for it.
- **Tile flags on the wire.** Tibia sends none with the map.

## 12. Decision test

- **Must decide now:** YES. Clients cannot see the map, pickups or moved map objects, and
  DEPOT-WIRE-1 and the house views depend on it (#162 5914289140).
- **Minimum sufficient:** one domain describing tiles per interest area, one handle, bounded
  snapshots and deltas.
- **Superseding evidence:** measured bytes over budget (§8) or an owner choice of a.
- **Deliberately not decided:** line of sight; house interiors (HOUSE-RUNTIME-0); volatile added
  items; map-object moves (ITEM-MOVE-WIRE-1 §9).

## 13. Before-freeze checklist

1. **Contract amendments:** MAP-WIRE-2 adds the `map_item {handle}` source to ITEM-MOVE-WIRE-0 §4.1 and
   USE-WIRE-V1, and registers the domain, rows and capability; no FND-02 or FND-04 change. Pending
   on acceptance of MAP-WIRE-1, two texts read "`placement_key` as the wire placement identity":
   ADR-0021 (the bundle's `placement_key`, lines 202-203) and ITEM-MOVE-WIRE-0 §4.1 (the
   map-authored base items paragraph). Both are amended to: the wire placement identity is the
   per-session `map_item_handle`, which the server maps to the `placement_key`; the key itself is
   never encoded on the wire.
2. **Serialization:** one domain revision per committed overlay change; a map-item pickup publishes
   with its domain 1 entity in one sync unit.
3. **Restart:** descriptions rebuild from the bundle and the overlay, which ADR-0021 rebuilds.
4. **Typed references:** `placement_key`, `map_item_handle`, `overlay_incarnation`, `tile_revision`, `map_revision`, native `FloorId`.
5. **Wire:** §3-§9.
