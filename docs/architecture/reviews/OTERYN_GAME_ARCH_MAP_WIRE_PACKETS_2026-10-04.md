# ARCH-MAP-WIRE-1: the world map view wire and its packets

- Decision id: ARCH-MAP-WIRE-1.
- Status:
  - The §1 rulings and the §2 packets are accepted on merge.
  - The MAP-WIRE-1 contract
    (`docs/contracts/protocol-oteryn/candidates/MAP_WIRE_1_WORLD_MAP_VIEW_CANDIDATE_V1.md`) is a
    candidate. It binds only after owner acceptance, and both packets wait for that acceptance.
- Origin:
  - ADR-0021 §5 (MAP-WIRE-1, MAP-CLIENT-1, the SPEED-1 amendment to MAP-LOAD-1);
  - ARCH-WORLD-CONTENT-SERVE-1 §1.6 and §1.8 (#1792);
  - owner `1a` for this batch.

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
| `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`, `RESOURCE_LIMITS_REGISTRY.json` | MAP-WIRE-2 only | numbers leased by the control plane |

### 0.3 Order

The order is: owner accepts MAP-WIRE-1, then MAP-WIRE-2, then MAP-CLIENT-1.

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
  - apps/game-server/src/map/view.rs                     # new: tile stack composition and window
  - apps/game-server/src/map/mod.rs                      # the module line only
  - apps/game-server/src/gameplay_transport/world_map.rs # new: domain-17 snapshot and delta
  - apps/game-server/src/gameplay_transport/world_map_tests.rs
  - apps/game-server/src/gameplay_transport/mod.rs       # module line and the 40-byte target resolution
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

### 2.2 MAP-CLIENT-1 (the client draws the imported map)

```yaml
task_id: OTV2-20261004-map-client-1
decision: ARCH-MAP-WIRE-1 §1.1-§1.6; MAP-WIRE-1 contract (accepted)
depends_on: [OTV2-20261004-map-wire-2]
worker: oteryn-impl-worker
review: Codex, on the frozen head
branch: agent/map-client-1-20261004
base: main, after MAP-WIRE-2 merges
owned_paths:
  - crates/session/src/lib.rs                # select capability 18, decode domain 17, re-export the view types
  - apps/client/src/map_view.rs              # new: client view state (snapshot, deltas, window, resync)
  - apps/client/src/scene.rs                 # draw the map view instead of the fixture cells when present
  - apps/client/src/input.rs                 # step timing from the tile ground speed
  - apps/client/src/lib.rs                   # the module line only
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
  - the scene draws each tile's stack bottom-up, with floors above the actor's drawn in
    perspective;
  - a click on a base entry builds the 40-byte target;
  - the joint ground-speed switch (§1.6).
- **Drawing assets.** Each `item_definition_ref` maps deterministically to a cell of the existing
  placeholder atlas (`crates/placeholder-assets`), with the ground layer drawn as terrain. Real
  appearance sprites are not in this packet (owner question 5).
- **Acceptance:**
  - a session test over a recorded domain-17 snapshot and delta stream reproduces the server's
    window tile by tile;
  - a header mismatch triggers `ResyncRequest` and draws nothing from that delta;
  - a scene test draws a known tile stack in the right order and floors in perspective;
  - a `USE` click on a base entry sends the 40-byte target that MAP-WIRE-2 resolves, and an added
    or Ground item sends its handle;
  - on a bundle World, a step onto a tile with a non-150 ground speed takes the same duration on
    the client and the server, and the fixture World still uses 150 on both;
  - the end-to-end harness (`oteryn-synthetic-client-harness`) walks a step on a test bundle with
    the client view matching the server.
- **Not in scope:** real sprites and appearance assets, light, minimap, house interiors, and
  offering capability 18 on a live node.

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
   - Real sprites and appearance assets.
   - Light, weather and minimap.
   - The house interior wire.
   - Creatures on tiles (they stay in domain 1).
   - Offering capability 18 on a live node, which is MAP-CUTOVER-1.

**Risks and trade-offs.**
- A streamed view costs bandwidth on every step, which the client file would avoid. In return
  there is no client map distribution and no version skew.
- Placeholder drawing makes the map navigable but not faithful until the sprite question is
  decided.
- The capability-18 refusal on bundle Worlds strands older clients by design.

**Proof conditions.**
- The MAP-WIRE-2 size and viewport measurements are within the stated bounds.
- MAP-CLIENT-1's end-to-end window match holds.
