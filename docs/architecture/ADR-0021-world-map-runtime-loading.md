# ADR-0021: World map runtime loading

- Status: Candidate. Acceptance needs exact-head validation, independent review (persistence
  scope for §4.4 and the DUR-03 amendment) and protected integration.
- Date: 2026-09-30
- Decision owners: Oteryn project
- Role: Sol Supervising Architect
- Answers: the owner's architecture decision request "runtime loading of the Oteryn world map"
  (2026-09-30), about the base map of #1160 and #1170
- Owner decisions: answers 1a, 2a, 3a, 4a, 5a, 6a, 7a and 8a, given directly in the architect
  session on 2026-09-30 (§2). The control plane assigns their D-numbers.
- Builds on: ADR-0001 (multichannel), ADR-0005 (World Project, compiler and World Bundle),
  ADR-0010 (product profiles), `OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1`,
  `OTERYN_CRYSTALSERVER_LEGACY_SPATIAL_IMPORT_PROFILE_V1`, DUR-03 §9 and §39.3, DUR-04 §9, §11
  and §24
- Amends: DUR-03 §39.3 (map-item materialization and world-reset retirement, applied in DUR-03
  in this PR); `RESOURCE_LIMITS_REGISTRY.json` (four budget rows, in this PR)
- Runtime, migration, content and production authority: NONE. Each child in §5 needs its own
  #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

- **MAP-SPIKE-0** (evidence, impl worker). Commit the format and overlay benchmarks with their
  inputs and results under `docs/agents/evidence/`. This closes the DUR-04 §24 spike gate for the
  server bundle (§4.2).
- **MAP-BUNDLE-1** (compiler, impl worker; format doc reviewed with it). A deterministic compiler
  from the World Project (B3 placements plus the World, FloorChange, Transition, House and area
  families) to a server World Bundle (§4.2). It:
  - maps the project frame to native positions (§4.3);
  - resolves palette keys against the Item and Terrain registries, failing closed (§4.5);
  - handles provisional keys and draft areas as in §4.5 and §4.6;
  - writes the manifest, per-sector checksums and the bundle digest;
  - registers its parser and decompression limits in `RESOURCE_LIMITS_REGISTRY.json`.

  Tests: byte-identical output for identical inputs; a source-to-bundle equivalence check on
  every tile; unknown-key, over-limit and corrupt-sector rejection.
- **MAP-LOAD-1** (runtime, hard worker). The bundle reader and a compact in-memory base model,
  loaded eagerly and shared read-only by every channel of the World on a node (§4.1). Tests:
  load and equivalence, negative and fuzz tests of the reader, and the §4.8 budgets measured.
- **MAP-OVERLAY-1** (runtime and durability, hard worker, persistence review). It builds:
  - the per-channel overlay (§4.4);
  - the rebuild of DUR-03 Ground items after an unplanned restart;
  - map-item pickup as a MINT;
  - the world-reset retirement (DUR-03 §39.3 amendment).
- **MAP-CUTOVER-1** (runtime, impl worker). Worlds boot from the bundle.
  `native_entry_room.json` stays a test fixture world. Draft areas are flag-gated (§4.6).
- Binding sections: §4 of this ADR; DUR-03 §39.3 "Map items and world reset"; the coordinate and
  CrystalServer import profiles.

## 1. Question

The full base map exists as source data: 1,208 B3 region files with about 19.35 M tiles and
24.96 M items, plus the world metadata families. The game server does not read any of it. It
still runs on the legacy WorldProject package and `native_entry_room.json`. Before a loader is
built, eight things must be fixed:

1. how the base is loaded;
2. what file the server reads;
3. how source positions become native positions;
4. how channels share the base, and what state survives a restart;
5. how keys are resolved;
6. how floor changes, teleports, houses and draft areas enter the runtime;
7. how a new base reaches a live world;
8. how the result is qualified.

## 2. Owner decisions

| # | Owner choice (2026-09-30) |
|---|---|
| 1a | The full base is loaded into memory at start, in a compact server-owned model shared by the channels of a World. |
| 2a | B3 stays the source format. A compiler produces a server World Bundle, and the server reads only the bundle. |
| 3a | The project frame `global-target-2026-09-27` stays the source frame of all content families. The compiler maps it to native positions. |
| 4a | Tibia parity: map changes are volatile and reset at the planned world reset. Player items on the ground survive a crash and are retired at the planned reset. The owner accepted this without an in-game test. |
| 5a | Unknown keys fail compilation. The five provisional keys are skipped with a diagnostic outside production and must be resolved before release. |
| 6a | The new families are the only source for the full world. Draft areas load behind a testing and preproduction flag and block release. `native_entry_room` stays a fixture. There is no dual loading. |
| 7a | A new base activates only at a planned world reset. |
| 8a | The budgets of §4.8 are accepted as initial gates, and the first compact-model measurement confirms them. |

## 3. Facts

**PROVEN**

- ADR-0005 §1 and §3 (accepted) keep the editable World Project separate from the compiled World
  Bundle. The bundle is read-only, indexed, checksummed and bounded, and the runtime must not treat
  an implementation-specific serializer as a permanent format without an accepted schema
  contract.
- ADR-0005 §6: dynamic item placement is runtime and PostgreSQL state. A running server never
  rewrites the authored map.
- ADR-0001 §7 (accepted): ground items, corpses, local loot and "public-map tile overlays" are
  channel-local. World-level state is character, economy and house ownership.
- DUR-04 (PROPOSED): the bundle is immutable and content-addressed (§9); loading and activation
  are separate, and activation happens at a quiescent boundary (§11); hot reload never mutates a
  live revision (§20); a spike gate applies before a bundle encoding is frozen (§24).
- `OTERYN_CRYSTALSERVER_LEGACY_SPATIAL_IMPORT_PROFILE_V1`: `native.x = legacy.x`,
  `native.y = legacy.y`, `native.floor = -legacy.z`, with checked arithmetic.
- The coordinate profile §6: a larger `FloorId` is higher.
- The project frame stores legacy `z` in a field named `floor`, for example encounter anchors
  (`"floor": 14`) and the B3 header (`z u8`, at most 15).
- The B3 codec (#1170, `tools/content-schema/world-authoring/world_region_codec.py`):
  - 256x256 regions per floor, 32x32 sectors, one zstd frame per sector, LEB128 varints;
  - a palette of Item, Terrain and provisional donor keys;
  - item attributes: count, action, unique, door, text, charges, description, teleport, depot and
    container depth.
- DUR-03 Ground custody is implemented (migrations `0010` to `0015`). A Ground item binds the
  accepted map and content revisions (§39.3).
- The owner's benchmark (not yet committed), on 4 vCPU and 15.7 GiB:
  - B3: 21.9 MB, full load 1.71 s, about 1.87 GB RSS in the naive model, 30 µs per random sector.
  - Overlay: 45-80 B per dropped item, 13 µs per 18x14 viewport (40 µs at hotspots), a 4.8 MB
    snapshot for 1 M items.

**UNKNOWN**

- The RSS of the compact model. MAP-LOAD-1 measures it (§4.8).
- The daily reset time and schedule. That is an operational setting, not decided here.
- Whether any player-dropped ground item survives a Tibia server save. The owner accepted parity
  without a test (4a).

## 4. Decision

### 4.1 Loading (1a)

- A World's base is decoded completely into a compact, server-owned, read-only model at node
  start, before any channel of that World is admitted.
- One instance per World per node is shared by all channels of that World (`Arc`).
- Lazy loading and mmap zero-copy are not used. mmap may be proposed later under its own format
  contract if §4.8 measurements exceed the RSS budget.

### 4.2 Representation (2a)

- **Source.** B3 placements and the world families are the World Project source. The server never
  reads them.
- **Server World Bundle.** The compiler produces one. It reuses the B3 sector container (32x32
  sectors, one zstd frame each) and adds:
  - native positions (§4.3);
  - palette entries resolved to the content revision's compact ids, with the stable keys kept in
    the manifest;
  - per-sector checksums and a bundle digest;
  - a manifest with `project_format_version`, `world_schema_version`, `content_revision`,
    `compiler_version`, World bounds and floors;
  - decompression limits.
- The bundle byte layout is specified in the MAP-BUNDLE-1 format document and reviewed with it.
  The owner's benchmark is the DUR-04 §24 spike evidence for the choice of 32x32 sectors and zstd
  level 3 (MAP-SPIKE-0).
- The server loads a bundle only if its digest, checksums, schema versions and content revision
  match the World's pinned revisions. Otherwise the World does not start.

### 4.3 Coordinates (3a)

- All content families keep the project frame (`floor` = legacy `z`): placements, FloorChange,
  Transition, House, areas, encounters and NPC destinations.
- The compiler is the only adapter. It maps with the CrystalServer import profile, `x` and `y`
  unchanged and `floor = -z`, using checked arithmetic.
- The runtime, `protocol-oteryn` and Atlas see only native `WorldTilePosition`.
- The World record declares the bounds (half-open) and the floor set `[-15 .. 0]`.
- A position outside the declared bounds or floors fails compilation.

### 4.4 Channels and durability (4a)

- **Base and overlay.** The base is shared (§4.1). Each channel has its own overlay (ADR-0001
  §7):
  - per tile: base items hidden, and items added;
  - items added to the overlay keep their full attributes.
- **Tile limit.** A tile holds at most 64 base items, the reach of the hidden-item bitmask. The
  base map maximum is 26, and the compiler rejects more.
- **Volatile map state.** Moved or used map objects, doors, levers and removed map items are
  overlay state. They are never written to PostgreSQL, and the planned world reset discards
  them.
- **Player items on the ground.** These stay DUR-03 Ground custody.
  - After an unplanned restart, the channel rebuilds them into its overlay from their durable
    locations, so a crash loses no item.
  - At the planned world reset they are retired, as in a Tibia server save (DUR-03 §39.3
    amendment).
  - Items in house custody are not affected; houses have their own contract.
- **Picking up a map-authored item.** This is a MINT (DUR-03 §9.2) into Ground at the item's
  current tile, followed by the ordinary Ground-to-inventory TRANSFER.
  - The MINT cause is `MapItemMaterialization {world, channel, base bundle digest, origin
    position, origin base stack index, reset epoch}`. It names the authored origin even after the
    overlay moved the item.
  - The MINT is idempotent per cause, so each channel can pick up a map item once per reset
    epoch.
  - The overlay hides the base item from the MINT on. The next reset restores it.
  - A known consequence of multichannel is that the same map item can be taken once in each
    channel. Items worth protecting are not map-placed pickups.
  - Rewards meant to be claimed once use `RewardClaim` instead (#162 5905746509).
- **No overlay snapshots and no overlay journal.**

### 4.5 Keys (5a)

- A palette key resolves only through the Item registry (A12 keys) or the Terrain family. Any
  other key fails compilation.
- **Provisional keys.** The five provisional donor keys are skipped with a diagnostic when the
  bundle's build profile is testing or preproduction. A production-profile build fails until they
  are resolved.
- **Legacy ids.** `action`, `unique`, `door` and `depot` ids are kept as source bindings in the
  bundle. The runtime gives them no behaviour unless an admitted definition binds them explicitly,
  as `RewardClaim` placements (D39) or doors do.
- **Teleports.** A `teleport` attribute must agree with its Transition.Teleport record.
  Otherwise compilation fails.

### 4.6 Families, drafts and cut-over (6a)

- The runtime reads floor changes, teleports and house tiles only from the FloorChange,
  Transition.Teleport and House families, compiled into the bundle.
- The seven minimap draft areas are compiled with a `draft` marker. A draft-bearing bundle loads
  only when the World's profile is testing or preproduction. A production World refuses it, so
  the drafts are a release blocker, like 33c and 34b.
- `native_entry_room.json` stays a test fixture World.
- There is no parallel legacy loader: the legacy path has no full map to compare. Equivalence is
  proved source to bundle to runtime (§4.8).

### 4.7 Updates (7a)

- A new bundle is staged and verified, then activated only at a planned world reset, which is
  the DUR-04 §11 quiescent boundary.
- The order at a reset:
  1. channels stop admitting players, and players are logged out;
  2. every DUR-03 Ground item of the World outside house custody is retired (§4.4);
  3. the new bundle is activated;
  4. channels start with empty overlays.
- No overlay or Ground position survives the reset, so no position migration exists. A crash
  restart reuses the active bundle and never switches revisions.

### 4.8 Qualification (8a)

- **Budgets.** Rows in `RESOURCE_LIMITS_REGISTRY.json`, confirmed or revised by the first
  MAP-LOAD-1 measurement:
  - `MAP01-BASE-LOAD-MS`: full base load at most 5,000 ms on the reference 4 vCPU node;
  - `MAP01-BASE-RSS-BYTES`: base model at most 1 GiB per World;
  - `MAP01-CHANNEL-OVERLAY-BYTES`: at most 64 MiB per channel;
  - `MAP01-VIEWPORT-US`: an 18x14 viewport at most 100 µs p99.
- **Tests.**
  - deterministic compilation;
  - tile-by-tile source-to-bundle-to-runtime equivalence;
  - negative, corrupt and fuzz tests of the bundle reader (ADR-0005 robustness);
  - Ground rebuild after a crash;
  - map-item MINT idempotence per cause;
  - reset retirement and activation order.
- **Reviews.** An independent review of this ADR, a persistence review of MAP-OVERLAY-1, and a
  security review of the bundle reader.

## 5. Delivery

| Child | Scope | Depends on |
|---|---|---|
| MAP-SPIKE-0 | Benchmark and evidence | this ADR |
| MAP-BUNDLE-1 | Compiler, format doc, key resolution, drafts, limits | this ADR; #1160 and #1170 merged; A12 item keys |
| MAP-LOAD-1 | Reader, compact model, budgets | MAP-BUNDLE-1 |
| MAP-OVERLAY-1 | Overlay, Ground rebuild, map-item MINT, reset retirement | MAP-LOAD-1; DUR-03 §39.3 amendment accepted |
| MAP-CUTOVER-1 | Boot from the bundle, fixture world, draft gate | MAP-LOAD-1 |

## 6. Rejected options

- **Lazy loading (1b).** Sector loads inside the tick, and harder AI and pathfinding across
  sectors, to save memory the budget does not need.
- **mmap zero-copy now (1c, 2c).** It binds a Rust layout without a format contract (ADR-0005
  §3). It remains a later option on measured evidence.
- **The server reads B3 (2b).** It breaks the source and bundle separation, and validation would
  run at every start.
- **Native floors in the source (3b).** It either breaks coordinate profile §6 or rewrites every
  content family.
- **Overlay snapshots or journal (4b, 4c).** Not Tibia behaviour. Snapshots do not protect against
  a crash, and a journal is a persistent-world cost with no product need.
- **Dual legacy and new loading (6c).** The legacy path has no map to compare.
- **Live region swap (7b).** It is complex and unnecessary with a daily reset.

## 7. Decision test

- **Must decide now:** YES. No World can run the real map, and chests, encounters, teleports and
  houses have no world to act on.
- **Blocked:** MAP-BUNDLE-1 to MAP-CUTOVER-1, CHEST-CONTENT placements, and encounter anchor
  binding (E2).
- **Harder later:** the frame adapter and the bundle identity. Both are kept in the compiler, so
  they can change without touching content or the runtime.
- **Superseding evidence:**
  - a compact-model RSS above budget, which would reopen mmap;
  - Reference evidence that ground items survive a server save, which would change §4.4
    retirement into restoration, a policy change only;
  - a product need for a persistent world.
- **Deliberately not decided:** the reset schedule, house item persistence, depots, the client
  bundle, instances, and the bundle byte layout (MAP-BUNDLE-1 format document).

## 8. Before-freeze checklist

1. Contract amendments: DUR-03 §39.3 and the registry rows are in this PR. The coordinate and
   import profiles are applied, not amended.
2. Serialization: overlays are channel-owned and single-writer in the channel tick. Ground and
   MINT follow DUR-03 fences and the `character_root` lock. Reset retirement runs at the quiescent
   boundary with no channel live.
3. Restart: Ground items are durable and rebuilt. Overlay loss is the intended reset semantics.
   The active bundle is pinned by digest.
4. Typed references: the MINT cause is fully typed, and Ground binds the base revision.
5. Wire: no wire change; the runtime sends native positions as today.
6. Split work: reset steps are ordered (§4.7). Each retirement is a one-item DUR-03 transaction,
   and the activation waits for all of them.
