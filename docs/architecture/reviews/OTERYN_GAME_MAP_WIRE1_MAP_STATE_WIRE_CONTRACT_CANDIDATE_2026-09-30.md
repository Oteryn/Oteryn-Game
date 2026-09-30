# MAP-WIRE-1 Map state on the wire

- Contract: `MAP-WIRE1-MAP-STATE-V1`
- Status: **CONTRACT CANDIDATE, OWNER ACCEPTANCE REQUIRED** (ADR-0021 §5). Acceptance also needs
  exact-head validation, independent protocol and security review and protected integration. The
  design is the Tibia one: the server streams the map (owner question M1, §10, recommended b).
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
| MAP-WIRE-2 | hard (protocol), protocol and security review | capability `MAP_STATE_V1`, domain `MAP_TILES` (§4-§7), the `MapItemRef` handle (§6), the bounds and their measurement (§8) | MAP-LOAD-1; MAP-OVERLAY-1; VIS-2 |
| MAP-CLIENT-1 | client lane | the native client's decoder and renderer for `MAP_TILES`, drawing the combined tile order (§5) | MAP-WIRE-2 |

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
- **Allowlist** (DUR-04 §8). Per tile: its position (implicit, §7). Per base item:
  - the client item type (the appearance id already in the client artifact);
  - count or subtype, only where the appearance needs it (stackables, fluids);
  - its stack position (§5);
  - `placement_key`, only for items the client can act on (usable, movable, containers, doors,
    or with a domain-2 state), as a `MapItemRef` (§6).

  Nothing else leaves the server: action, unique, door and depot ids, reward bindings, teleport
  destinations, text and descriptions (sent only by Look), house membership, protection-zone and
  no-logout flags, `build_class`.
- Only a `production` bundle is ever described; draft areas cannot reach a client.
- `placement_key` must be opaque: MAP-BUNDLE-1 proves it is not derivable from legacy action or
  unique ids.
- **Owned house tiles** are described by the house interior runtime (HOUSE-RUNTIME-0); until then
  no house can be owned, so every tile is an overlay tile.
- **Map revision.** Each snapshot names the bundle digest (`map_revision`) once; it differs from
  `content_generation` and changes only at a planned reset, when every session ends.

## 4. Base items and domain 1

- Player items on the ground (DUR-03 custody) are domain 1 entities only (D85); `MAP_TILES` never
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
- The client draws in this order and shows at most the first 10 things of a tile, as Tibia;
  the server bounds each source (§8), so nothing depends on the cut.

## 6. Handles

- `MapItemRef {position, placement_key, tile_revision}` names a base item on the wire. It is a new
  source variant of command 9 (ITEM-MOVE-WIRE-0 §4.1) and of USE-WIRE-V1's use target, refused
  with a typed stale result when `tile_revision` differs. Which moves it enables stays with
  ITEM-MOVE-WIRE-1 §9; pickup follows DUR-03 §39.1.

## 7. Snapshots and deltas (MAP-WIRE-2)

- **Floors.** The area uses native `FloorId` (ADR-0021 §4.3). At or above the surface (floor ≥
  −7) the client gets floors −7 to 0; below it, the observer's floor ± 2 within `[-15 .. 0]`. Each
  floor's tiles are offset by `target floor − observer floor` diagonally, as Tibia draws it.
  MAP-WIRE-2 maps `interest.rs`'s legacy floors to these.
- **Snapshot** (`AREA_SNAPSHOT`): one repeated list per floor, each with its origin and its tiles
  in row-major order; `map_revision` once. Triggers: admission, reconnect, channel transfer, floor
  change, teleport, respawn, and any movement whose delta would pass `MAPW1-RL-04`.
- **Deltas:**
  - `AREA_ENTER` for one step on the same floor, cardinal or diagonal: the new origin and the
    entering row, column or both on every visible floor; the client drops what left the area.
  - `TILE_SET`: one tile's full description after an overlay change.
- **Revision:** one domain revision per GameSession (FND-02 §15), stored in the resume state
  (`resume.rs`) like the other domains and kept above any seen across a reconnect.

## 8. Bounds (MAP-WIRE-2 registers and measures)

Worst case per encoded item 22 bytes (type 4, count 4, stack position 3, `MapItemRef` 11) and per
tile 10 items; the largest configurable area is 36 × 28 (MOVE-RL-11).

| Row | Value |
|---|---|
| `MAPW1-RL-01` items described per tile | 10 (Tibia's per-tile cap) |
| `MAPW1-RL-02` tiles per floor list | 36 × 28 = 1,008, under FND-02's 4,096 |
| `MAPW1-RL-03` snapshot | 8 floors × 1,008 × 226 bytes ≈ 1.82 MB worst, in 4 chunks, under 16 MiB with the other domains |
| `MAPW1-RL-04` `AREA_ENTER` | a diagonal step at 36 × 28: 63 × 8 tiles × 226 bytes ≈ 114 KB, under 256 KiB |

- Before activation, MAP-WIRE-2 measures on B3 and records on #162: typical and p99 bytes of a
  login snapshot and of a cardinal and a diagonal step, and server time per description.

## 9. Capability

- **`MAP_STATE_V1`**, requiring `WORLD_SPATIAL_ENTITIES` (VIS-2). A session without it keeps
  today's placeholder; it is not refused. Numbers, the domain id and delta types are reserved on
  #162 at allocation.

## 10. Owner question

**M1. Where does the base map come from?**
- **b (recommended):** streamed by the server as in Tibia (§3-§8): nothing to download when the
  map changes, only visited areas reach the client, ADR-0021 measured the server read at 13-40 µs
  per view.
- **a:** from a map file in the client, checked at login. It departs from Global parity; it ships
  the whole map, hidden and quest areas included, to every client; every reset that changes the map
  forces a client update through an updater and signing that are not yet decided (ALPHA-CLIENT-01
  §17); it needs a new admission check and a digest in `ClientBootstrap`.

Under the Global-parity rule, b applies unless the owner chooses a.

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

1. **Contract amendments:** MAP-WIRE-2 adds the `MapItemRef` source to ITEM-MOVE-WIRE-0 §4.1 and
   USE-WIRE-V1, and registers the domain, rows and capability; no FND-02 or FND-04 change.
2. **Serialization:** one domain revision per committed overlay change; a map-item pickup publishes
   with its domain 1 entity in one sync unit.
3. **Restart:** descriptions rebuild from the bundle and the overlay, which ADR-0021 rebuilds.
4. **Typed references:** `placement_key`, `MapItemRef`, `map_revision`, native `FloorId`.
5. **Wire:** §3-§9.
