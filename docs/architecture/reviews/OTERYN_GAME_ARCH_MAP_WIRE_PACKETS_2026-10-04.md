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
| `apps/game-server/src/gameplay_transport/item_move.rs` | created by ITEM-MOVE-1; MAP-WIRE-2: the not-supported arm for the base-entry handle kind only; MAP-PICKUP-1: replaces that arm with the pickup | if ITEM-MOVE-1 has not merged when MAP-WIRE-2 is allocated, command 9 has no handler yet and cannot move a base entry; the control plane gives the arm and its test to whichever of the two merges second |

### 0.3 Order

The order is: owner accepts MAP-WIRE-1. MAP-WIRE-2 and MAP-SPRITE-1 then run in parallel, on
disjoint paths. MAP-CLIENT-1 runs after both merge.

- MAP-WIRE-2 also needs MAP-OVERLAY-1a (the per-channel overlay with hidden and added entries).
- MAP-PICKUP-1 (§2.4) runs after MAP-WIRE-2, MAP-OVERLAY-1b (the map item MINT) and ITEM-MOVE-1
  (the command-9 handler) merge. It runs in parallel with MAP-CLIENT-1, on disjoint paths.
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
- The cut keeps the bottom entry (the ground) and the 9 topmost entries (contract §2).
- The window is 18x14 tiles. Floors in view: -7 to 0 on or above the surface; otherwise two
  floors above and two below, down to -15. Every floor in view is sent.
- The server also sends `first_visible_floor`, the highest floor the client draws. It applies
  the OTClient roof rule to the full composed stack and the bundle's Terrain kinds, so the
  10-entry cut never decides it. It is presentation only, never line-of-sight authority (contract
  §2 Visible floors; graphics audit §9).
- These match the Tibia server and the `MAP01-VIEWPORT-US` viewport. A snapshot is at most
  725,888 bytes, and a delta at most 93,376 bytes (contract §3).

### 1.3 The view is bound to the bundle

- Every payload carries `content_generation`, `bundle_digest` and `reset_epoch`.
- A delta with another header is a `STATE_REVISION_MISMATCH`, and the client resyncs.
- The server sends a snapshot, never a delta, across a digest or epoch change. This is the
  client `content_generation` matched to the active bundle that ADR-0021 requires.
- The header's `origin` is not part of that binding. A delta's origin is the new window origin:
  the view's origin or one step from it on the same floor. A move whose newly visible tiles are
  all empty sends an origin-only delta (contract §3 Origin).

### 1.4 The wire target is the bundle placement key, resolved through the binding

- **Base entries.** A client targets a base entry with 40 bytes in the existing
  `WorldObjectTargetV1.placement` field: the 32-byte bundle digest, then the 8-byte big-endian
  placement key.
- **Server resolution.** The server refuses a stale digest. It resolves the entry in the base and
  checks that the overlay does not hide it. For a bound RewardClaim placement, it maps the key to
  the canonical `PlacementKey` through the in-memory binding of ARCH-WORLD-CONTENT-SERVE-1 §1.4.
- **Durable rows.** They keep canonical keys (§1.6 there). Nothing new is written.
- **Added and Ground items.** They keep `item_handle` targets.
- **Movable base entries.** A base entry eligible for pickup under ADR-0021 §4.4 carries an
  `item_handle`, bound to its digest, placement key and reset epoch. Command 9 names it by that
  handle, and the move is the §4.4 MINT then TRANSFER. No command field is added (contract §3
  Move source). MAP-WIRE-2 sends the handle; MAP-PICKUP-1 (§2.4) implements the move, and until
  then command 9 from it is `ITEM_MOVE_OUTCOME_NOT_SUPPORTED` with no write.
- **Stateful base entries.** Every other base entry carries its overlay `object_revision`. A
  `USE` sends it as `expected_revision`, and a stale value is `STALE`. Each transition resends the
  tile with the new revision (contract §3 Object revision).
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
  - apps/game-server/src/gameplay_transport/item_move.rs   # the not-supported arm for a base-entry handle only (§0.2)
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
  - in `map/view.rs`, the composed stack (base minus hidden, plus added and Ground items), its cut
    (the bottom entry and the 9 topmost) and the window and floor set;
  - the domain-17 join snapshot, the per-step and per-tile deltas, and snapshot-instead-of-delta
    over `MAPW-RL-03`;
  - the move of Ground items and corpses out of domain 1 under capability 18;
  - the map-view handle budget (`MAPW-RL-04`, nearest-first, `display_only` beyond it) and the
    session handle table bounded by `ITEMV0-RL-03-MAP-VIEW` (contract §3);
  - handles for movable base entries (ADR-0021 §4.4 eligibility), bound to `(bundle_digest,
    placement_key, reset_epoch)`, in their own handle kind, which the command-9 path refuses as
    not supported until MAP-PICKUP-1 (contract §3 Move source);
  - in `map/view.rs`, the visible-floor resolver (contract §2 Visible floors) as a pure function
    of the actor position and the composed stacks, and `first_visible_floor` in every header,
    with its recomputation after each actor move and each change of a tile it reads;
  - each `base_ordinal` entry's `object_revision`, and the `expected_revision` check of a 40-byte
    `USE` (contract §3 Object revision);
  - the delta origin rule and origin-only move deltas (contract §3 Origin);
  - each entry's family-tagged definition reference: `item_definition_ref` for Items,
    `terrain_definition_ref` for Terrain palette entries (contract §3);
  - each entry's `appearance_id`, taken from its palette key, or for an overlay-added or Ground
    item from its item content definition key (contract §3);
  - the admission refusal without capability 18 on a bundle World;
  - the 40-byte target resolution, with the binding lookup to the canonical `PlacementKey`.
- **Not built here:** the command-9 pickup of a base entry (MAP-PICKUP-1, §2.4).
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
  - the cut of a 12-entry stack sends entry 0 and entries 3 to 11, in stack order, with `more`;
  - visible floors (contract §2 Visible floors), on the resolver alone, with the same result
    whatever the stack order of the non-bottom entries:
    - an actor in the open on the surface (floor -7, nothing above): `first_visible_floor` 0, so
      -7 to 0 are drawn;
    - an actor indoors under a roof (a `roof` or `ground` entry directly above): -7, so only the
      actor's floor is drawn;
    - an actor under a roof two floors up, with nothing on -6: -6;
    - an actor beside a roof edge: open above its own tile, roof above or covering an open
      orthogonal neighbour: the neighbour limits it; behind a `wall` neighbour or a
      `blocks_projectile` item, the same roof does not;
    - a roof on -6 only at the diagonal neighbour `(x + 1, y - 1)` does not limit it;
    - an actor in a doorway (an open door tile with a roof on one side): the side that can be
      looked through limits it;
    - a roof that covers the actor's tile in perspective, at `(x + k, y + k)` on floor `-7 + k`,
      limits it, and the same roof at `(x - k, y - k)` does not;
    - an actor underground at -10: -8 with nothing above, -9 under a ceiling on -8, and -10
      under a ceiling on -9;
    - a 12-entry tile whose limiting ground is entry 0 limits it, and a `blocks_projectile` item
      dropped by the cut still stops the look-through;
    - walking out of a house sends a delta whose header changes `first_visible_floor` from -7 to
      0, with no tile resent for that change, and walking back sends -7;
    - a stair to another floor sends a snapshot with the new floor's value;
    - an overlay change that removes a roof entry above the actor sends the new value;
    - a `first_visible_floor` of 1, one below the origin floor, or -7 with an origin at -10
      fails closed; a delta that changes only `first_visible_floor` is valid;
  - floor sets on the surface (8 floors) and underground (5 floors, bounded at -15), and the
    perspective shift;
  - a one-step diagonal move sends at most 31 tiles per floor and the client-side window
    matches a fresh snapshot;
  - a teleport, a floor change and a reset-epoch change each send a snapshot;
  - a delta with another digest or epoch is refused by the decoder's header check;
  - origin (contract §3 Origin):
    - a one-step move delta whose origin is one step from the view's origin is applied, and the
      client window then matches a fresh snapshot at that origin;
    - a delta whose origin is two steps away, or on another floor, fails closed and the client
      resyncs;
    - a step into an area whose newly visible tiles are all empty sends a delta with the new
      origin and no tile or cleared entry, and the client window moves;
    - a delta with no tile, no cleared entry and an unchanged origin fails closed;
  - move source (contract §3 Move source):
    - a pickupable, unbound base entry is sent with an `item_handle` and no `base_ordinal`;
    - command 9 from that handle is `ITEM_MOVE_OUTCOME_NOT_SUPPORTED` and writes nothing (no
      MINT, no hide, no tile resent);
    - a `USE` with that handle on an entry the overlay has hidden is `STALE`;
    - a door, a bound chest and a furniture entry are sent with `base_ordinal` and no handle;
    - a movable base entry beyond the budget is `display_only`, and counts against
      `MAPW-RL-04`;
  - object revision (contract §3 Object revision):
    - a door with no overlay state is sent with `object_revision` 0, and a `USE` with 0 opens
      it;
    - the open resends its tile with revision 1, and a `USE` with 1 closes it;
    - a second `USE` with the old revision 0 is `STALE` and changes nothing;
    - an `object_revision` on a handle or `display_only` entry fails closed;
    - a 10-entry tile of `base_ordinal` entries with the largest revision values is at most
      360 bytes;
  - a 40-byte target with the active digest resolves; a stale digest is refused, and a hidden
    entry gives `NOTHING_TO_USE`;
  - a bound chest resolves to its canonical `PlacementKey`, and the MINT `source_placement` is the
    canonical key;
  - the fixture World still serves domain 2, accepts canonical bytes, and does not serve domain 17;
  - the composition plus encode of an 18x14 viewport over 8 floors is measured against
    `MAP01-VIEWPORT-US`. The p99 is recorded, and a breach blocks the offer gate.
- **Not in scope:** offering capability 18 on a live node (MAP-CUTOVER-1), the client
  (MAP-CLIENT-1), the base-entry pickup (MAP-PICKUP-1), the ground-speed switch, houses, light and
  minimap.

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
  - A drawn entry has at most `MAX_ENTRY_CELLS` cells. It is the measured maximum over the
    pinned 15.30 appearances, frame group 0 and phase 0, all layers, and at least 16 (a 4x4-cell
    object, such as a 2x2-tile object of 64x64 sprites). The value is recorded in the task record.
    An entry that would resolve to more draws the placeholder cell and is counted.
  - Quads are drawn in bounded batches. `MAX_BATCH_QUADS` rises from 16,384 to 81,920, and a frame
    that needs more quads issues further batches of at most that size, in draw order. A frame is
    never truncated. The worst case is 2,016 tiles x 10 entries x `MAX_ENTRY_CELLS`; with 16
    cells, that is 322,560 quads in 4 batches.
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
    - appearance 104, a 4x4-cell object, resolves to 16 cells with their offsets;
  - an unknown id, id 0 and id 65,536 return an error, not a panic;
  - the sheet cache holds 64 sheets and evicts the least recently used one on the 65th;
  - the atlas page evicts least recently drawn cells, and a frame with 4,097 distinct cells
    draws one placeholder cell and counts it;
  - a sheet decode stays under a measured bound, recorded in the task record;
  - batches:
    - a test over every pinned appearance asserts that none resolves to more than
      `MAX_ENTRY_CELLS` cells;
    - an entry built to resolve to `MAX_ENTRY_CELLS` + 1 cells draws the placeholder and is
      counted;
    - a frame of 2,016 tiles x 10 entries of 16 cells is drawn in 4 batches of at most
      81,920 quads, and the drawn quad count equals 322,560;
    - a frame of exactly 81,920 quads uses one batch, and 81,921 uses two.
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
  - the client view state: it applies deltas, drops tiles that leave the window together with
    their handles, resyncs on a header mismatch, and keeps the header's `first_visible_floor`;
    a delta that changes only that value redraws without touching a tile;
  - a tile with `more` holds only its received entries. The client draws and targets those and
    never infers the dropped ones;
  - drawing (`map_draw.rs`), as render phases (graphics audit §5, §6 and §8):
    - floors are drawn from the lowest in view up to `first_visible_floor`, each shifted by its
      floor difference to the actor. A floor above `first_visible_floor` is not drawn, nor any
      actor on it;
    - within a floor, tiles are drawn back to front, in ascending `(y, x)`;
    - each tile is drawn in phases, the per-tile render-phase merge: first its items classed by
      the MAP-SPRITE-1 draw-order flags as ground, ground border, on-bottom, then its common
      items, all in stack order; then the domain-1 actors whose position is that tile; then its
      on-top items. So an actor stands over the items under it and under a doorframe or arch on
      its tile;
    - actors join the map by position only: the client groups the domain-1 actors by position
      each frame. An actor on a tile domain 17 omits (no item) is drawn in that tile's creature
      phase;
    - a walking actor is drawn, at its interpolated offset, in the creature phase of whichever
      of its source and destination tiles is drawn later, so no ground of either tile covers it;
    - each entry is drawn by its `appearance_id` through MAP-SPRITE-1, with displacement and
      elevation. Its cells extend up and left from its tile, and later tiles draw over them, so no
      neighbour is redrawn;
    - an `appearance_id` of 0, or one that fails to resolve, is drawn with the placeholder cell
      for its definition reference;
  - the asset directory is `content/assets/files/` by default, or `--assets <dir>`. When the
    pinned catalogue is missing or does not match, the client logs it once and draws
    placeholders;
  - targeting: a click resolves to the topmost drawn floor that has a tile at that screen cell,
    never a floor above `first_visible_floor`, and to that tile's topmost received entry. The
    entry's `origin` alone selects the target (contract §4):
    - `base_ordinal`: the 40-byte `WorldObjectTargetV1` (the view's `bundle_digest`, then the
      big-endian key from the tile position and the ordinal), with `expected_revision` equal to
      the entry's `object_revision`;
    - `item_handle`: `ItemTargetV1` with the handle for a `USE`, and the `source_handle` of
      command 9 for a drag;
    - `display_only`: no command; the client shows that the entry is out of reach;
  - the joint ground-speed switch (§1.6).
- **Acceptance:**
  - a session test over a recorded domain-17 snapshot and delta stream reproduces the server's
    window tile by tile;
  - a header mismatch triggers `ResyncRequest` and draws nothing from that delta;
  - a draw test turns a known tile stack into quads in stack order:
    - with real cells for known appearance ids;
    - with the placeholder cell for id 0 and for an unknown id;
    - with floors in perspective;
  - visible floors: with `first_visible_floor` -7 an actor on -7 is drawn with no quad from
    floors -6 to 0; with 0 every floor from -7 to 0 is drawn; a delta that changes only the value
    redraws the floors and changes no tile;
  - overlap (render phases), each asserting the quad order:
    - an actor on a tile with an on-top doorframe or arch is drawn after the tile's ground and
      common items and before the doorframe;
    - an actor on a tile with a common item (a table or a dropped item) is drawn after it;
    - a 64x64 object anchored at a tile covers the ground of its three up-left neighbours,
      which are drawn before it, and an actor on a later tile is drawn over it;
    - an actor behind a large object (on an earlier tile) is drawn before it;
    - an actor walking east and an actor walking north are each drawn after the ground of both
      the source and destination tiles;
    - an actor on an omitted (empty) tile is still drawn;
  - a `more` tile draws only its received entries, and a click on it targets the topmost
    received entry;
  - with the asset directory missing, the client still draws the map with placeholders;
  - targeting, one case per origin:
    - a `USE` click on a door (`base_ordinal`) sends the 40-byte target with its
      `object_revision` as `expected_revision`, which MAP-WIRE-2 resolves;
    - a `USE` click on an added or Ground item sends `ItemTargetV1` with its handle;
    - a `USE` click on a movable base entry (`item_handle`) sends `ItemTargetV1` with its handle,
      never the 40-byte target, and a drag of it sends command 9 with that `source_handle`;
    - a click on a `display_only` entry sends no command;
    - a click over a hidden upper floor targets the actor's floor;
    - after a tile leaves the window, its handles are gone and no command names them;
  - on a bundle World, a step onto a tile with a non-150 ground speed takes the same duration on
    the client and the server, and the fixture World still uses 150 on both;
  - the end-to-end harness (`oteryn-synthetic-client-harness`) walks a step on a test bundle with
    the client view matching the server.
- **Not in scope:**
  - animation, light and minimap;
  - house interiors;
  - offering capability 18 on a live node.

### 2.4 MAP-PICKUP-1 (command 9 from a movable base entry)

The minimum-sufficient route keeps the pickup out of MAP-WIRE-2: its MINT is MAP-OVERLAY-1b's,
and its handler arm is ITEM-MOVE-1's command-9 handler. MAP-PICKUP-1 joins the two once both are
on `main`.

```yaml
task_id: OTV2-20261004-map-pickup-1
decision: ARCH-MAP-WIRE-1 §1.4; MAP-WIRE-1 contract §3 Move source (accepted)
depends_on: [OTV2-20261004-map-wire-2, MAP-OVERLAY-1b, ITEM-MOVE-1]
worker: oteryn-hard-worker
review: persistence (Codex), on the frozen head
branch: agent/map-pickup-1-20261004
base: main, after MAP-WIRE-2, MAP-OVERLAY-1b and ITEM-MOVE-1 merge
owned_paths:
  - apps/game-server/src/gameplay_transport/item_move.rs        # the base-entry source arm only
  - apps/game-server/src/gameplay_transport/item_move_tests.rs
  - apps/game-server/src/gameplay_transport/world_map_tests.rs  # the pickup cases only
  - docs/agents/tasks/archive/OTV2-20261004-map-pickup-1.md
validation:
  - cargo fmt --check
  - cargo clippy --locked --workspace --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server
  - python tools/agents/validate_governance.py
  - git diff --check
```

- **Builds:** the command-9 arm for a base-entry handle. It resolves the handle to its
  `(bundle_digest, placement_key, reset_epoch)`, checks the active digest and epoch and that the
  overlay does not hide the entry (else `STALE`), runs the existing reach and destination checks,
  then calls the MAP-OVERLAY-1b MINT into Ground at the tile with the `MapItemMaterialization`
  cause and the ordinary TRANSFER to the destination. It replaces the MAP-WIRE-2 refusal.
- **Acceptance:**
  - command 9 from a movable base entry's handle to the backpack MINTs with the
    `MapItemMaterialization` cause and TRANSFERs, hides the origin and resends its tile;
  - a second command 9 with the same handle is `STALE` and writes nothing;
  - in another channel, the same entry can still be taken once;
  - a destination check that fails (`NO_ROOM`, `TOO_FAR`) writes nothing and leaves the entry
    visible;
  - a crash between the MINT and the TRANSFER recovers through MAP-OVERLAY-1b's receipt, with the
    item in Ground at its tile and the origin hidden, never both;
  - a handle from another digest or reset epoch is `STALE`.
- **Not in scope:** moving entries that are not eligible under ADR-0021 §4.4, and the client
  (MAP-CLIENT-1 already sends command 9 with the handle).

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
- **A handle for every base entry.** A full view holds up to 20,160 entries, far past the
  1,024 handle budget and the session handle table. Only movable base entries need a handle,
  because command 9 names its source only by handle.
- **A client-side visible-floor resolver.** The client holds only the 10-entry cut, so a
  `blocks_projectile` item or a roof entry below the cut would be missed, and the client has no
  Terrain kinds. The server holds the full stack and the bundle facts, and the value costs 2
  bytes per header.
- **Per-floor or per-tile visibility bits.** One floor bound per view is all the OTClient rule
  yields, so per-tile bits would add bytes to every tile for no extra fact.
- **Sending only the visible floors.** A roof change would then add or drop whole floors of
  tiles, which is a snapshot or an oversized delta on every door step. Sending every floor in
  view keeps the change to the header.
- **The pickup inside MAP-WIRE-2.** It would couple the protocol packet to the MAP-OVERLAY-1b
  MINT and the ITEM-MOVE-1 handler, and make it wait for both. The handle and its refusal are
  enough for MAP-WIRE-2.
- **A new placement-key source field in command 9.** It would amend `ItemMoveIntentV1` and its
  bounds. A handle bound to the bundle key reuses the accepted command as it is.
- **No revision on base entries.** A `USE` would then have no `expected_revision` to send, so
  either every `USE` after a transition would be stale or no stale check would hold.
- **A single fixed sprite batch.** 2,016 tiles x 10 entries x 16 cells is 322,560 quads. One
  buffer of that size wastes memory on every frame, and a truncated batch would drop entries.
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
