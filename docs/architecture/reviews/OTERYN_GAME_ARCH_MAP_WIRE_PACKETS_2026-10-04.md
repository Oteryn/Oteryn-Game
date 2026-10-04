# ARCH-MAP-WIRE-1: the world map view wire and its packets

- Decision id: ARCH-MAP-WIRE-1.
- Status:
  - The §1 rulings and the §2 packets are accepted on merge.
  - The MAP-WIRE-1 contract
    (`docs/contracts/protocol-oteryn/candidates/MAP_WIRE_1_WORLD_MAP_VIEW_CANDIDATE_V1.md`) is a
    candidate. The owner accepted it (#1793 Q1a), effective once Codex review of the fixed head
    is clean. Every packet waits for that.
  - Owner decisions on #1793: Q1a accepted; Q2a the digest plus placement key target; Q3a 10
    entries plus `more`; Q4a a server-streamed viewport; Q5b real 15.30 sprites, with the
    client assets distributed to the client.
- Origin:
  - ADR-0021 §5 (MAP-WIRE-1, MAP-CLIENT-1, the SPEED-1 amendment to MAP-LOAD-1);
  - ARCH-WORLD-CONTENT-SERVE-1 §1.6 and §1.8 (#1792);
  - owner `1a` for this batch, and the #1793 decisions Q1a-Q5b.

## 0. Gaps and order

### 0.1 What is missing on `main`

- **No domain carries the map.**
  - `WORLD_OBJECT_OVERLAY` (domain 2) carries keyed object states only, at most 486 per snapshot
    (`WOBJ-RL-03`).
  - `WORLD_SPATIAL_VISIBILITY` v2 carries at most 256 entities (`MOVE-RL-11`) and no base
    stack.
- **No client draws a real map.** `apps/client/src/scene.rs` draws a fixed placeholder scene
  (`PlaceholderScene`, 15x11 cells, four fixture actors and one ground item). `crates/session`
  decodes domains 1 and 2 but no map tile.
- **No wire target names a bundle entry.** `WorldObjectTargetV1.placement` carries canonical
  `PlacementKey` bytes (ITEM-USE-WIRE-1). A bundle entry has only its digest-bound placement key
  (format §7).
- **The client's step timing uses a constant.** It uses `DEFAULT_GROUND_SPEED` = 150
  (`apps/client/src/input.rs:70`). The server keeps 150 in production until MAP-CLIENT-1
  (ADR-0021 MAP-LOAD-1 amendment).

### 0.2 Shared files

| File | Packets | Rule |
| --- | --- | --- |
| `crates/session/src/lib.rs` | MAP-CLIENT-1 only | MAP-WIRE-2 does not touch it |
| `apps/game-server/src/gameplay_transport/connection.rs` | MAP-WIRE-2: the domain-17 join snapshot and delta hook | MAP-CLIENT-1 does not touch it |
| `apps/game-server/src/movement/speed.rs` | MAP-CLIENT-1: the ground-speed source switch for a bundle World | MAP-WIRE-2 only reads the ground speed through `WorldBase` |
| `apps/game-server/src/gameplay_transport/item_view.rs`, `crates/protocol-oteryn/src/item_view.rs` | MAP-WIRE-2: the map-view handle bound | MAP-CLIENT-1 does not touch them |
| `Cargo.toml`, `Cargo.lock`, `crates/renderer/**` | MAP-SPRITE-1 only | MAP-WIRE-2 and MAP-CLIENT-1 do not touch them |
| `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`, `RESOURCE_LIMITS_REGISTRY.json` | MAP-WIRE-2 only | numbers leased by the control plane |

### 0.3 Order

The order is: owner accepts MAP-WIRE-1. MAP-WIRE-2 and MAP-SPRITE-1 then run in parallel, on
disjoint paths. MAP-CLIENT-1 runs after both merge.

- MAP-WIRE-2 also needs MAP-OVERLAY-1a (the per-channel overlay with hidden and added entries).
- Offering capability 18 on a live node needs MAP-CUTOVER-1, which boots a World from a bundle.
  Until then the capability stays `offered: false` with an offer gate, as for capabilities 1 and
  17.

## 1. Rulings

### 1.1 The server streams the stack of each tile in view

The client gets no map file. The server composes the visible stack of each tile, and sends a
viewport snapshot and per-step deltas within the FND-02 limits (contract §2, §3). This keeps the
server-only bundle the only map artifact (ADR-0021 §4.2) and redistributes no map data with the
client.

### 1.2 Ten entries per tile, an 18x14 window, Tibia floors

- Each tile carries at most the top 10 entries of its composed stack, with `more` when it is
  cut.
- The window is 18x14 tiles. Floors in view: -7 to 0 on or above the surface; otherwise two
  floors above and two below, down to -15.
- These match the Tibia server and the `MAP01-VIEWPORT-US` viewport. A snapshot is at most
  725,888 bytes, and a delta at most 93,376 bytes (contract §3).

### 1.3 The view is bound to the bundle

- Every payload carries `content_generation`, `bundle_digest` and `reset_epoch`.
- A delta with another header is a `STATE_REVISION_MISMATCH`, and the client resyncs.
- The server sends a snapshot, never a delta, across a digest or epoch change. This is the
  client `content_generation` matched to the active bundle that ADR-0021 requires.

### 1.4 The wire target is the bundle placement key, resolved through the binding

- **Base entries.** A client targets a base entry with 40 bytes in the existing
  `WorldObjectTargetV1.placement` field: the 32-byte bundle digest, then the 8-byte big-endian
  placement key.
- **Server resolution.** The server refuses a stale digest. It resolves the entry in the base and
  checks that the overlay does not hide it. For a bound RewardClaim placement, it maps the key to
  the canonical `PlacementKey` through the in-memory binding of ARCH-WORLD-CONTENT-SERVE-1 §1.4.
- **Durable rows.** They keep canonical keys (§1.6 there). Nothing new is written.
- **Added and Ground items.** They keep `item_handle` targets.
- **What this settles.** It is the #1792 §1.8 question: the target becomes the bundle key, and
  the canonical-key lookup is added here.

### 1.5 One object domain per World; Ground items move into tiles

- A bundle World serves domain 17 and not domain 2, and it refuses a client without capability
  18.
- With capability 18 selected, Ground items and corpses are carried only in map tiles, not as
  domain-1 entities. Actors stay in domain 1.
- The fixture World is unchanged.

### 1.6 Ground speed switches with the client

- Each tile carries its ground speed.
- MAP-CLIENT-1 switches the server's ground-speed source to the bundle for a bundle World, and the
  client step timing to the tile's value, in one PR. This is the joint switch the ADR-0021
  MAP-LOAD-1 amendment requires.
- The fixture World keeps 150 on both sides.

## 2. Packets

### 2.1 MAP-WIRE-2 (`MAP_STATE_V1`)

```yaml
task_id: OTV2-20261004-map-wire-2
decision: ARCH-MAP-WIRE-1 §1.1-§1.5; MAP-WIRE-1 contract (accepted)
depends_on: [MAP-WIRE-1 owner acceptance, MAP-OVERLAY-1a]
worker: oteryn-hard-worker
review: protocol (Codex), on the frozen head
branch: agent/map-wire-2-20261004
base: main
owned_paths:
  - docs/contracts/protocol-oteryn/v1/world_map_v1.proto                # new; the contract §3 schema
  - docs/contracts/protocol-oteryn/candidates/MAP_WIRE_1_WORLD_MAP_VIEW_CANDIDATE_V1.md  # status line only
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json      # capability 18, domain 17, as leased
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json         # MAPW-RL-01..04, ITEMV0-RL-03-MAP-VIEW and the payload maxima
  - crates/protocol-oteryn/src/world_map.rs              # new codec
  - crates/protocol-oteryn/src/world_map_tests.rs        # new
  - crates/protocol-oteryn/src/lib.rs                    # the module line only
  - crates/protocol-oteryn/src/item_view.rs              # the ITEMV0-RL-03-MAP-VIEW constant only
  - crates/protocol-oteryn/src/item_view_tests.rs
  - apps/game-server/src/map/view.rs                     # new: tile stack composition and window
  - apps/game-server/src/map/mod.rs                      # the module line only
  - apps/game-server/src/gameplay_transport/world_map.rs # new: domain-17 snapshot and delta
  - apps/game-server/src/gameplay_transport/world_map_tests.rs
  - apps/game-server/src/gameplay_transport/mod.rs       # module line and the 40-byte target resolution
  - apps/game-server/src/gameplay_transport/item_view.rs # the handle table bound selected by capability 18
  - apps/game-server/src/gameplay_transport/item_view_tests.rs
  - apps/game-server/src/gameplay_transport/connection.rs  # the domain-17 join and delta hook only
  - apps/game-server/src/gameplay_transport/capabilities.rs
  - apps/game-server/src/gameplay_transport/capabilities_tests.rs
  - docs/agents/tasks/archive/OTV2-20261004-map-wire-2.md
validation:
  - cargo fmt --check
  - cargo clippy --locked --workspace --all-targets -- -D warnings
  - cargo test --locked -p oteryn-protocol-oteryn
  - cargo test --locked -p oteryn-game-server
  - python tools/agents/validate_governance.py
  - git diff --check
```

- **Builds:**
  - the schema and the strict codec;
  - the registry and limit rows, with capability 18 requiring 6 and 4, `offered: false` and an
    offer gate naming MAP-CUTOVER-1;
  - in `map/view.rs`, the composed stack (base minus hidden, plus added and Ground items, cut at
    10) and the window and floor set;
  - the domain-17 join snapshot, the per-step and per-tile deltas, and snapshot-instead-of-delta
    over `MAPW-RL-03`;
  - the move of Ground items and corpses out of domain 1 under capability 18;
  - the map-view handle budget (`MAPW-RL-04`, nearest-first, `display_only` beyond it) and the
    session handle table bounded by `ITEMV0-RL-03-MAP-VIEW` (contract §3);
  - each entry's family-tagged definition reference: `item_definition_ref` for Items,
    `terrain_definition_ref` for Terrain palette entries (contract §3);
  - each entry's `appearance_id`, taken from its palette key, or for an overlay-added or Ground
    item from its item content definition key (contract §3);
  - the admission refusal without capability 18 on a bundle World;
  - the 40-byte target resolution, with the binding lookup to the canonical `PlacementKey`.
- **Acceptance:**
  - codec round trips, and fail-closed decoding of every contract §3 malformed case;
  - `ground_speed` 1000 and an absent `ground_speed` round-trip, and 1001 fails closed;
  - with 1,025 handle-bearing entries in view, the 1,024 nearest carry handles and the farthest
    is `display_only`; a step that brings it within the budget resends its tile with a handle;
  - the handle table holds 1,325 handles with capability 18 (1,661 with 14 and 18) and refuses
    one more;
  - capability 18 without capability 4 is refused at negotiation;
  - an `oteryn:item.tibia.i<id>` or `oteryn:terrain.tibia.i<id>` entry is sent with
    `appearance_id` = `<id>`, and a provisional donor entry with its `source_item_id` (28827 for
    the donor chest). Any other key is sent with 0. An `appearance_id` above 65,535 fails
    closed;
  - a Ground item whose definition is not in the bundle palette is sent with the
    `appearance_id` of its item content definition key;
  - a Terrain base entry is sent with `terrain_definition_ref` and an Item entry with
    `item_definition_ref`; both, neither or a zero reference fails closed;
  - encoded-size tests at the bounds: a 10-entry tile with the largest values is at most 360 bytes;
    a full 2,016-tile snapshot is at most 725,888 bytes, in at most two chunks; a 248-tile delta
    is at most 93,376 bytes;
  - on a test bundle with an overlay: a hidden base entry is absent, an added item shows with its
    handle, a Ground item is in its tile and not in domain 1, and an 11-entry stack has `more`;
  - floor sets on the surface (8 floors) and underground (5 floors, bounded at -15), and the
    perspective shift;
  - a one-step diagonal move sends at most 31 tiles per floor and the client-side window
    matches a fresh snapshot;
  - a teleport, a floor change and a reset-epoch change each send a snapshot;
  - a delta with another digest or epoch is refused by the decoder's header check;
  - a 40-byte target with the active digest resolves; a stale digest is refused, and a hidden
    entry gives `NOTHING_TO_USE`;
  - a bound chest resolves to its canonical `PlacementKey`, and the MINT `source_placement` is the
    canonical key;
  - the fixture World still serves domain 2, accepts canonical bytes, and does not serve domain 17;
  - the composition plus encode of an 18x14 viewport over 8 floors is measured against
    `MAP01-VIEWPORT-US`. The p99 is recorded, and a breach blocks the offer gate.
- **Not in scope:** offering capability 18 on a live node (MAP-CUTOVER-1), the client
  (MAP-CLIENT-1), the ground-speed switch, houses, light and minimap.

### 2.2 MAP-SPRITE-1 (the 15.30 appearance and sprite pipeline)

Owner Q5b: the client draws real appearance sprites from the in-repository 15.30 client assets
(`content/assets/files/`, D154 and the 2026-09-29 redistribution supersession), not placeholder
atlas cells. The pipeline is its own packet, so MAP-CLIENT-1 stays an implementation slice. It
reads no map and no wire, so it runs in parallel with MAP-WIRE-2.

```yaml
task_id: OTV2-20261004-map-sprite-1
decision: ARCH-MAP-WIRE-1 §2.2; owner #1793 Q5b
depends_on: [owner acceptance of MAP-WIRE-1]
worker: oteryn-impl-worker
review: Codex, on the frozen head
branch: agent/map-sprite-1-20261004
base: main
owned_paths:
  - crates/client-assets/**                  # new crate oteryn-client-assets
  - crates/renderer/src/batch.rs             # the sprite atlas page and the quad bound
  - crates/renderer/src/scene_gpu.rs         # sub-rectangle cell upload
  - crates/renderer/src/lib.rs               # re-exports only
  - Cargo.toml                               # the workspace member and `lzma-rs`
  - Cargo.lock
  - docs/agents/tasks/archive/OTV2-20261004-map-sprite-1.md
validation:
  - cargo fmt --check
  - cargo clippy --locked --workspace --all-targets -- -D warnings
  - cargo test --locked -p oteryn-client-assets
  - cargo test --locked -p oteryn-renderer
  - git diff --check
```

- **Source and pin.**
  - The asset directory is `content/assets/files/`.
  - Every file is checked against the `sha256` of its entry, found by file name, in
    `imports/official/client-assets/15.30/manifest.json`. This covers `catalog-content.json`,
    the `appearances` file it names, and each sprite sheet when it is first loaded.
  - The hash token in a file name is never the check.
  - A file with no manifest entry, a mismatch or a missing file fails closed for that file. The loader returns an error, and
    the caller draws the placeholder cell. Nothing panics.
- **Upstream first.**
  - Appearances are decoded with the workspace `prost` (`=0.14.4`) derive, on hand-written
    messages. They carry only the fields this packet reads, with the field numbers of the
    upstream `appearances.proto`: Canary's `src/protobuf/appearances.proto` at the
    15.30-capable pin of
    `docs/agents/programs/OTERYN_GAME_VERSION_1530_AND_OTS_BRANCHES_DECISION_20260928.md`.
    Unknown fields are skipped, and no build script or generated code is added.
  - Sheets are decoded with `lzma-rs` `=0.3.0`, the version the World VFX experiment already
    qualified. The decoder strips the 32-byte CIP header and reads the BMP as 384x384 RGBA, with
    magenta as transparent.
  - The decoder is ported from `experiments/world-vfx-real-content/src/content.rs`. The
    experiment stays unchanged as evidence.
- **Builds:**
  - `AppearanceIndex`: object id to frame group 0, with pattern sizes, layers, sprite ids,
    displacement, elevation and the draw-order flags (ground, ground border, on-bottom,
    on-top). Animation phases are kept, but only phase 0 is drawn in this packet.
  - `SpriteSheets`: maps a sprite id to its sheet and layout (catalog `spritetype` 0-3: 32x32,
    32x64, 64x32, 64x64). It decodes sheets on demand.
  - `resolve(appearance_id, count, sub_type, x, y, floor)`: returns the 32x32 cells to draw,
    each with its pixel offset. The pattern is chosen as the Tibia client does:
    - position patterns `x % pattern_width`, `y % pattern_height`, `z % pattern_depth`.
      Here `z = -floor` normalises the native floor `-15..=0` to the client's `0..=15`, the
      inverse of the import profile `native.floor = -legacy.z`. For example, floor -7 with
      pattern depth 2 selects depth pattern `7 % 2 = 1`. A raw negative floor is never used as
      a pattern index;
    - the stackable count pattern from the count thresholds 1, 2, 3, 4, 5, 10, 25, 50;
    - fluid and splash sub-types.
  - In the renderer, a sprite atlas page of 32-pixel cells. Cells are written by sub-rectangle
    upload, and a cell is evicted when it was least recently drawn in a frame.
- **Limits.** They are constants of the crate, with a test at each bound. They are
  client-local and register no wire or server limit.
  - `appearances` file: at most 8 MiB (the 15.30 file is 5,017,996 bytes).
  - `catalog-content.json`: at most 2 MiB (1,042,224 bytes).
  - A compressed sheet: at most 2 MiB. A decompressed sheet: at most 384 x 384 x 4 + 65,536
    bytes.
  - `appearance_id`: 1..=65,535 (the 15.30 maximum is 55,117).
  - Decoded sheets in memory: at most 64, least recently used first out, about 37 MiB.
  - The GPU sprite atlas page: 2,048 x 2,048, which holds 4,096 cells of 32 pixels (16 MiB
    RGBA).
    - When a frame needs more cells than the page holds, the excess entries are drawn with the
      placeholder cell and counted in a diagnostic counter. The frame does not fail.
  - `MAX_BATCH_QUADS` rises from 16,384 to 81,920, which covers 2,016 tiles x 10 entries x 4
    cells, with a test on the instance buffer size.
  - No derived cache is written to disk; decoding happens in memory, on demand.
- **Acceptance (over the real `content/assets/files/`):**
  - the pinned catalogue and appearances load, with 43,516 objects and a maximum id of 55,117;
  - a corrupted byte in a sheet, the catalogue or the appearances file fails closed with an
    error, and the test asserts the error;
  - a sheet renamed to the hash token of its altered bytes still fails against its manifest
    `sha256`, and a sheet with no manifest entry fails closed;
  - floors -7 and 0 select depth patterns from `z` = 7 and 0, never from a negative index;
  - known ids decode to their recorded frame size and first-pixel colour:
    - a ground with a 4x4 position pattern resolves to different cells at `(0,0)` and `(1,0)`;
    - a stackable resolves to different cells at counts 1, 5 and 100;
    - a 64x64 object resolves to 4 cells with their offsets;
  - an unknown id, id 0 and id 65,536 return an error, not a panic;
  - the sheet cache holds 64 sheets and evicts the least recently used one on the 65th;
  - the atlas page evicts least recently drawn cells, and a frame with 4,097 distinct cells
    draws one placeholder cell and counts it;
  - a sheet decode stays under a measured bound, recorded in the task record.
- **Not in scope:**
  - animation, outfits, creatures, effects, missiles and light;
  - inventory and container sprites;
  - a disk cache;
  - packaging the assets in an installer.

### 2.3 MAP-CLIENT-1 (the client draws the imported map)

```yaml
task_id: OTV2-20261004-map-client-1
decision: ARCH-MAP-WIRE-1 §1.1-§1.6; MAP-WIRE-1 contract (accepted)
depends_on: [OTV2-20261004-map-wire-2, OTV2-20261004-map-sprite-1]
worker: oteryn-impl-worker
review: Codex, on the frozen head
branch: agent/map-client-1-20261004
base: main, after MAP-WIRE-2 and MAP-SPRITE-1 merge
owned_paths:
  - crates/session/src/lib.rs                # select capability 18, decode domain 17, re-export the view types
  - apps/client/src/map_view.rs              # new: client view state (snapshot, deltas, window, resync)
  - apps/client/src/map_draw.rs              # new: tile stacks to sprite quads through oteryn-client-assets
  - apps/client/src/scene.rs                 # draw the map view instead of the fixture cells when present
  - apps/client/src/input.rs                 # step timing from the tile ground speed
  - apps/client/src/lib.rs                   # module lines and the asset directory option
  - apps/client/Cargo.toml                   # the oteryn-client-assets dependency
  - apps/game-server/src/movement/speed.rs   # the bundle ground-speed source for a bundle World
  - docs/agents/tasks/archive/OTV2-20261004-map-client-1.md
validation:
  - cargo fmt --check
  - cargo clippy --locked --workspace --all-targets -- -D warnings
  - cargo test --locked -p oteryn-session
  - cargo test --locked -p oteryn-client
  - cargo test --locked -p oteryn-game-server
  - git diff --check
```

- **Builds:**
  - the session selects capability 18 and decodes domain-17 snapshots and deltas;
  - the client view state: it applies deltas, drops tiles that leave the window, and resyncs on a
    header mismatch;
  - drawing (`map_draw.rs`):
    - each tile's stack is drawn bottom-up by its `appearance_id` through MAP-SPRITE-1, with
      displacement, elevation and the draw-order flags;
    - floors above the actor's are drawn in perspective;
    - an `appearance_id` of 0, or one that fails to resolve, is drawn with the placeholder cell
      for its definition reference;
  - the asset directory is `content/assets/files/` by default, or `--assets <dir>`. When the
    pinned catalogue is missing or does not match, the client logs it once and draws
    placeholders;
  - a click on a base entry builds the 40-byte target;
  - the joint ground-speed switch (§1.6).
- **Acceptance:**
  - a session test over a recorded domain-17 snapshot and delta stream reproduces the server's
    window tile by tile;
  - a header mismatch triggers `ResyncRequest` and draws nothing from that delta;
  - a draw test turns a known tile stack into quads in stack order:
    - with real cells for known appearance ids;
    - with the placeholder cell for id 0 and for an unknown id;
    - with floors in perspective;
  - with the asset directory missing, the client still draws the map with placeholders;
  - a `USE` click on a base entry sends the 40-byte target that MAP-WIRE-2 resolves, and an added
    or Ground item sends its handle;
  - on a bundle World, a step onto a tile with a non-150 ground speed takes the same duration on
    the client and the server, and the fixture World still uses 150 on both;
  - the end-to-end harness (`oteryn-synthetic-client-harness`) walks a step on a test bundle with
    the client view matching the server.
- **Not in scope:**
  - animation, light and minimap;
  - house interiors;
  - offering capability 18 on a live node.

## 3. Rejected options

- **A client map file with server overlay diffs.** It needs a client projection of the bundle,
  its distribution and version pinning, and it redistributes map data. The streamed view fits
  the FND-02 limits as it is.
- **Extending `WORLD_OBJECT_OVERLAY`.** It is keyed by object state, with 486 entries per
  snapshot. It cannot carry base stacks or Ground items.
- **Canonical `PlacementKey` as the wire target.** Only RewardClaim placements have one, and it
  does not fit the per-tile bound.
- **All 64 base entries per tile.** A tile would grow to about 2 KiB, and a step delta would pass
  the 262,144-byte delta limit. Tibia shows 10.
- **Ground items in both domains.** The client would draw them twice.
- **Placeholder atlas cells only.** The owner chose real 15.30 sprites (Q5b).
- **A client-side table from `item_definition_ref` to appearance.** The reference is a compact
  id of one content revision (format §3), so the table would need versioned distribution. The
  server already holds the palette key, which names the appearance, so it sends `appearance_id`.
- **One definition reference for Items and Terrain.** The Item and Terrain catalogue compact
  ids overlap, so an untagged reference would be ambiguous. A bundle palette index alone cannot
  name overlay-added or Ground items, which are not in the palette.
- **Trusting the hash token in a sheet's file name.** Renaming a file would bypass the check.
  The manifest `sha256` is the pin.
- **A prepared on-disk sprite cache, as in the experiment.** On-demand decoding in memory is
  enough for the viewport, and a disk cache adds invalidation for no measured need.

## 4. Decision test

1. **Must it be decided now?** Yes.
   - #1792 §1.8 leaves the wire target to MAP-WIRE-1.
   - Without a map domain, the imported World is not playable by a client, which is the next
     step of the playable path.
   - The contract itself waits for owner acceptance. The packets are ready behind it.
2. **What is blocked?**
   - Any client view of the imported World, including the bound chests and quest steps of
     #1789 and #1792.
   - The ground-speed switch held by the ADR-0021 MAP-LOAD-1 amendment.
   - The offer of a bundle World to players.
3. **What becomes harder later?**
   - The 10-entry cut, the 18x14 window and the floor set are wire limits. Changing one is a
     schema revision.
   - The 40-byte target couples the client to the digest it drew. A new bundle invalidates every
     key, which is intended (format §7).
   - Moving Ground items out of domain 1 under capability 18 makes the two capabilities
     interdependent for bundle Worlds.
4. **What would justify superseding it?**
   - A measured `MAP01-VIEWPORT-US` breach.
   - A bandwidth measurement that makes streamed stacks too costly.
   - A requirement for offline or client-side map rendering, such as a full minimap.
   - A client that must see more than 10 entries per tile.
5. **What is deliberately not decided?**
   - Animation, outfits and effects sprites.
   - Light, weather and minimap.
   - The house interior wire.
   - Creatures on tiles (they stay in domain 1).
   - Offering capability 18 on a live node, which is MAP-CUTOVER-1.

**Risks and trade-offs.**
- A streamed view costs bandwidth on every step, which the client file would avoid. In return
  there is no client map distribution and no version skew.
- Real sprites make the client depend on the 15.30 asset files. A missing or mismatched file
  falls back to placeholders, so the map stays navigable.
- The capability-18 refusal on bundle Worlds strands older clients by design.

**Proof conditions.**
- The MAP-WIRE-2 size and viewport measurements are within the stated bounds.
- MAP-SPRITE-1 decodes the pinned 15.30 assets and fails closed on a corrupted file.
- MAP-CLIENT-1's end-to-end window match holds.
