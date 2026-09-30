# ADR-0021: World map runtime loading

- Status: Candidate. Acceptance needs exact-head validation, independent review (persistence
  scope for §4.4 and the DUR-03 amendment) and protected integration.
- Date: 2026-09-30
- Decision owners: Oteryn project
- Role: Sol Supervising Architect
- Answers: the owner's architecture decision request "runtime loading of the Oteryn world map"
  (2026-09-30), about the base map of #1160 and #1170
- Owner decisions: D188-D196 (answers 1a to 8a and house tiles a), given directly in the architect
  session on 2026-09-30 and numbered by the control plane on #162 (§2).
- Builds on: ADR-0001 (multichannel), ADR-0005 (World Project, compiler and World Bundle),
  ADR-0010 (product profiles), `OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1`,
  `OTERYN_CRYSTALSERVER_LEGACY_SPATIAL_IMPORT_PROFILE_V1`, DUR-03 §9 and §39.3, DUR-04 §9, §11
  and §24
- Amends:
  - DUR-03 §39.1 and §39.3, applied in DUR-03 in this PR: map-item materialization and world-reset
    retirement, superseding for those shapes the death-only MINT source sentence and the D3
    "only path" sentence;
  - `RESOURCE_LIMITS_REGISTRY.json`: four budget rows, in this PR.
- Amended (§4.5, §4.6) by the architect ruling on #162 5910173902 (palette keys, zero-destination
  teleports), applied with ITEM-MOVE-WIRE-0.
- The wire child MAP-WIRE-1 needs owner acceptance of its own contract candidate.
- Runtime, migration, content and production authority: NONE. Each child in §5 needs its own
  #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

- **MAP-SPIKE-0** (evidence, impl worker). Commit the format and overlay benchmarks with their
  inputs and results under `docs/agents/evidence/`, and cover the DUR-04 §24 list:
  - 32x32 against 64x64 sectors;
  - byte identity;
  - corruption detection;
  - patch granularity.

  This is the spike evidence for the server bundle (§4.2).
- **MAP-BUNDLE-1** (compiler, impl worker; format doc reviewed with it). It writes `build_class`
  into the manifest (§4.2). A deterministic compiler
  from the World Project (B3 placements plus the World, FloorChange, Transition, House and area
  families) to a server World Bundle (§4.2). It:
  - maps the project frame to native positions (§4.3);
  - resolves palette keys against the Item and Terrain registries, failing closed (§4.5);
  - handles provisional keys and draft areas as in §4.5 and §4.6;
  - writes the manifest, per-sector checksums and the bundle digest;
  - registers its parser, decompression and per-tile (64 items) limits in
    `RESOURCE_LIMITS_REGISTRY.json`;
  - writes a bundle format document, accepted as a contract with the child (ADR-0005 §3).

  Tests: byte-identical output for identical inputs; a source-to-bundle equivalence check on
  every tile; unknown-key, over-limit and corrupt-sector rejection.
- **MAP-LOAD-1** (runtime, hard worker). The bundle reader and a compact in-memory base model,
  loaded eagerly and shared read-only by every channel of the World on a node (§4.1). Tests:
  load and equivalence, negative and fuzz tests of the reader, and the §4.8 budgets measured.
- **MAP-OVERLAY-1** (runtime and durability, hard worker, persistence review). It builds:
  - the per-channel overlay (§4.4);
  - the Ground rebuild after an unplanned restart, with origins re-hidden;
  - map-item pickup as a MINT;
  - the World reset record, the reset fence and the world-reset retirement (DUR-03 §39.3
    amendment), with the HOUSE-CUSTODY-0 preflight and recheck;
  - a boundary test of the overlay budget: a freeze-time hide at the limit is refused, and a
    rebuild re-hide at the limit is admitted and alarmed.
- **MAP-WIRE-1** (protocol, contract candidate, owner acceptance required). An FND-02 child for
  the item state a client must see:
  - base items hidden or added by the overlay;
  - Ground items;
  - viewport-scoped snapshots within the FND-02 snapshot limits;
  - the client `content_generation` matched to the active bundle.

  Today's `WORLD_OBJECT_OVERLAY` domain (limit `WOBJ-RL-03`) does not carry them.
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

| D | # | Owner choice (2026-09-30), summary |
|---|---|---|
| D188 | 1a | The full base is loaded into memory at start, in a compact server-owned model shared by the channels of a World. |
| D189 | 2a | B3 stays the source format. A compiler produces a server World Bundle, and the server reads only the bundle. |
| D190 | 3a | The project frame `global-target-2026-09-27` stays the source frame of all content families. The compiler maps it to native positions. |
| D191 | 4a | Tibia parity: map changes are volatile and reset at the planned world reset. Player items on the ground survive a crash and are retired at the planned reset. Picking up a map item is a MINT. The owner accepted this without an in-game test. |
| D192 | 5a | Unknown keys fail compilation. The five provisional keys are skipped with a diagnostic outside production and must be resolved before release. |
| D193 | 6a | The new families are the only source for the full world. Draft areas load behind a testing and preproduction flag and block release. `native_entry_room` stays a fixture. There is no dual loading. |
| D194 | 7a | A new base activates only at a planned world reset. |
| D195 | 8a | The budgets of §4.8 are accepted as initial gates, and the first compact-model measurement confirms them. |
| D196 | House tiles (a) | Until a house item custody family exists, Ground items on house tiles are retired at the reset like any other Ground item. |

**Verbatim record.** The options were presented in Polish, and the owner answered in Polish.

- **Answer to questions 1-7** (2026-09-30T07:22Z): "1a, 2a, 3a jesli uwazasz ze to jest ok, 4 dobra
  zrobmy to tak jak ma byc bez testow bo to nie ejst az tak istotne a zajmie czas, 5 czyli wiesz co
  jest lepsze, 6a, 7a". Answer 8a was given earlier in the same session ("8a").
- **Option 4a as presented:**
  - "Nakładka jest ulotna: przesunięte obiekty mapy, otwarte drzwi i dźwignie wracają do stanu
    bazy przy planowanym resecie, tak jak server save."
  - "Przedmioty graczy na ziemi są już trwałe przez DUR-03 (Ground custody). Po awarii wracają do
    nakładki, a przy planowanym resecie są wycofywane, tak jak w Tibii."
  - "Podniesienie przedmiotu z mapy to MINT (DUR-03 §9.2); po resecie przedmiot na mapie się
    odnawia, jak w Tibii."

  In English: the overlay is volatile and reset at the planned reset; player items on the ground
  survive a crash and are retired at the planned reset; **picking up a map item is a MINT**, and
  the map item respawns after the reset. The owner accepted it without the in-game test.
- **House tiles, option a as presented:** "Tymczasowo znikają przy resecie, jak wszystko inne na
  ziemi. Gdy powstanie kontrakt domów, dostaną własną ochronę i od tego momentu będą zostawać."
  (For now they disappear at the reset, like everything else on the ground. When the house
  contract exists, they get their own protection and stay from then on.)
  The owner's answer (07:42Z): "a ale trzeba zrobic ten knotaktu domu kto to ma robic" (a, but the
  house contract has to be made; who does it), followed by "zrob odrazu tj i niech worker wykona"
  (do it right away and let a worker implement it), which started HOUSE-CUSTODY-0.

**Architect note (interpretation, not an owner decision).** HOUSE-CUSTODY-0 is the house contract
named by D196. It reads D196 as: the tiles of a house without an owner stay ordinary Ground, and
their items are retired at the reset; the items of an owned house use `HouseInterior` and are
never retired (§4.4). No house can be owned yet, so today every house tile follows the first rule.

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
**DERIVED**, until MAP-SPIKE-0 commits it:

- The owner's benchmark, on 4 vCPU and 15.7 GiB:
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
  - a manifest with the fields ADR-0005 §3 and DUR-04 §9 require: `project_format_version`,
    `world_schema_version`, `content_revision`, the Content Lock digest, `compiler_version`, the
    minimum reader and runtime versions, the required capabilities, a provenance summary, the
    projection class (server), ruleset compatibility, the World bounds and floors, and
    `build_class` (`production` or `non-production`);
  - decompression limits;
  - one compiler-emitted `placement_key` per top-level base entry. It is bound to the bundle
    digest and used both as the MINT origin (§4.4) and as the wire placement identity. It is not
    a canonical identity (import profile §8).
- The bundle byte layout is specified in the MAP-BUNDLE-1 format document and reviewed with it.
  The owner's benchmark is the DUR-04 §24 spike evidence for the choice of 32x32 sectors and zstd
  level 3 (MAP-SPIKE-0).
- The server loads a bundle only if its digest, checksums, schema versions and content revision
  match the World's pinned revisions. Otherwise the World does not start.
- A World deployed as production refuses a bundle whose `build_class` is not `production`, and a
  missing or unknown `build_class` counts as `non-production`. This is the fail-closed release
  gate for provisional keys and drafts (§4.5, §4.6).

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
  - items added to the overlay keep their full attributes;
  - a per-tile expiry index removes volatile added items when they decay, at 1 s granularity;
  - a base map stack is never merged with another item in the overlay, so each `placement_key`
    keeps exactly one entry.
- **Tile limit.** A tile holds at most 64 base items, the reach of the hidden-item bitmask. The
  base map maximum is 26, and the compiler rejects more.
- **Volatile map state.** Moved or used map objects, doors, levers and removed map items are
  overlay state. They are never written to PostgreSQL, and the planned world reset discards
  them.
- **Player items on the ground.** These stay DUR-03 Ground custody.
  - After an unplanned restart, the channel rebuilds them into its overlay from their durable
    locations, so a crash loses no item.
  - The rebuild fails closed if an item's `map_revision` differs from the active bundle digest.
  - At the planned world reset they are retired, as in a Tibia server save (§4.7).
- **House tiles** (HOUSE-CUSTODY-0, amending this section).
  - The tiles of a house without an owner are ordinary map tiles. Ground items there are
    retired at the reset (D196, read as in the §2 architect note).
  - The tiles of an owned house are served by its World-scoped interior runtime, not by the
    channel overlay. Items there use the `HouseInterior` family and are never retired by a
    reset.
  - Map-authored items on house tiles are never pickupable, so a reset cannot respawn them into
    an owned house. Before any house can become owned, the ownership child decides what happens
    to live Ground items on its tiles in each channel (HOUSE-CUSTODY-0 §3.5).
- **Picking up a map-authored item** (DUR-03 §39.3 amendment).
  - **Eligible items.** A top-level entry that is pickupable, is not on a house tile, and has no
    action, unique, door, depot or teleport binding (including a dropped zero-destination
    teleport), no contents, and no `text`, `description`,
    `charges` or other attribute that the ItemInstance state cannot represent. Everything else
    stays in place.
  - **Pickup.** A MINT into Ground at the item's current tile, of the whole origin entry, followed
    by the ordinary Ground-to-inventory TRANSFER.
  - **Cause.** `MapItemMaterialization {world, channel, base bundle digest, placement_key, reset
    epoch}`. A channel can take each map entry once per reset epoch. As a consequence of
    multichannel, the same entry can be taken once in each channel. Rewards meant to be claimed
    once use `RewardClaim` (#162 5905746509).
  - **Hiding the origin.** The origin is hidden when the MINT is frozen and unhidden only on
    proven non-commit. After a crash, the Ground rebuild re-hides every origin that has a receipt
    for this channel, digest and epoch. A retried MINT that returns an existing item is still
    checked for reach before its TRANSFER.
- **Overlay budget** (`MAP01-CHANNEL-OVERLAY-BYTES`).
  - It covers volatile entries and hidden origins. A new volatile entry that would exceed it is
    refused atomically.
  - A freeze-time hide of an origin may be refused, and then the pickup is refused. A rebuild
    re-hide of an origin that has a receipt is never refused: it is only counted, and an overflow
    raises an alarm.
  - Durable Ground items (loot, corpses, drops, materialized map items) are bounded by their own
    DUR-03 and combat limits. They are never refused or dropped for this budget. They are
    counted, and an overflow raises an operational alarm.
- **No overlay snapshots and no overlay journal.**
- **Character positions.** Durable Character positions, such as the DEATH-0 respawn position
  and a later logout position, are outside this section. Admission validates them against the
  active bundle, and the Character contract owns the fallback.

### 4.5 Keys (5a)

- A palette key resolves only through the Item registry (A12 keys) or a catalogue family
  (Terrain or WorldObject). Any other key fails compilation. *Amendment (ruling 5910173902):* an
  id with an Item record uses its Item key, and the compiler follows the Item's A12 §4.6 pointer
  (`routed_to`) to its Terrain or WorldObject record; only an id without an Item record uses its
  Terrain or WorldObject catalogue key directly.
- **Provisional keys.** The five provisional donor keys are skipped with a diagnostic in a
  non-production bundle build. A production bundle build fails until they are resolved. "Testing
  or preproduction" here means the deployment environment gate used by D171 and D172. It is not
  the ADR-0010 product profile.
- **Legacy ids.** `action`, `unique`, `door` and `depot` ids are kept as source bindings in the
  bundle. The runtime gives them no behaviour unless an admitted definition binds them explicitly,
  as `RewardClaim` placements (D39) or doors do.
- **Teleports.** A `teleport` attribute must agree with its Transition.Teleport record.
  Otherwise compilation fails. *Amendment (ruling 5910173902):* a `teleport` attribute whose
  destination is (0,0,0) is not a teleport. The compiler drops it with a diagnostic; the tile keeps
  its legacy `action`/`unique` binding, and the dropped attribute still excludes the entry from
  map-item materialization (§4.4). The dropped list is a parity report before a production
  release, not a release blocker.

### 4.6 Families, drafts and cut-over (6a)

- The runtime reads floor changes, teleports and house tiles only from the FloorChange,
  Transition.Teleport and House families, compiled into the bundle.
- The seven minimap draft areas are compiled with a `draft` marker, in non-production builds
  only.
- A production World accepts only a bundle with `build_class = production` (§4.2), so drafts and
  skipped provisional keys never reach it. The drafts are a release blocker, like D171 and D172.
- The B3 tile house id must match the House family, as a teleport must match its Transition
  record (except a zero-destination teleport, §4.5). Otherwise compilation fails.
- `native_entry_room.json` stays a test fixture World.
- There is no parallel legacy loader: the legacy path has no full map to compare. Equivalence is
  proved source to bundle to runtime (§4.8).

### 4.7 Updates (7a)

- A new bundle is staged and verified, then activated only at a planned world reset. The reset
  is the DUR-04 §11 quiescent boundary.
- A durable World reset record `{world, reset epoch N, target digest, RETIRING | ACTIVATED}`
  drives the reset:
  1. The record is written as RETIRING, and admission to every channel of the World closes.
     Players are logged out.
  2. For each channel scope of the World, the existing assignment writer gives the resetting node
     a fresh ownership generation, with admission still closed. The old generation ends, so
     in-flight Ground, MINT and TRANSFER commits fail their fence. The retirements run under that
     ordinary fence; no new fence kind or World-level scope is added.
  3. Every live DUR-03 Ground item of the World is retired, including container entries (entries
     first, D3 order). Each retirement is a one-item `DECAY_RETIRE` keyed
     `WorldReset {world, epoch, item}`. It shares the per-item retirement uniqueness with
     `CorpseDecay`.
  4. When none remains, activation writes the new digest, epoch N+1 and ACTIVATED atomically,
     bound to the content activation record. Channels then start with empty overlays.
- **Crash recovery.** A crash while the record is RETIRING makes boot refuse admission and resume
  from step 2, which is idempotent, then step 3. The old bundle never boots over a half-retired Ground. A crash restart outside a reset
  reuses the active bundle and never switches revisions.
- **Scope.** No overlay or Ground position survives a reset, so neither needs a position
  migration. In DUR-04 §12 terms, Ground at activation is `REMOVED_WITH_EXPLICIT_POLICY`.
  Character positions are handled at admission (§4.4).
- **Duration.** The reset takes one transaction per Ground item, and the World stays closed
  meanwhile. MAP-OVERLAY-1 measures it and registers `MAP01-RESET-RETIRE-MS` with a measured
  bound before any production World resets.
- **First cutover.** The first boot of a World from a bundle is itself a reset. It retires every
  pre-cutover Ground item, which carries the legacy map revision, and activates the bundle.
  MAP-CUTOVER-1 therefore depends on MAP-OVERLAY-1.

### 4.8 Qualification (8a)

- **Budgets.** Rows in `RESOURCE_LIMITS_REGISTRY.json`, confirmed or revised by the first
  MAP-LOAD-1 measurement. The reference node is the owner's benchmark node (4 vCPU, 15.7 GiB),
  pinned by MAP-SPIKE-0:
  - `MAP01-BASE-LOAD-MS`: full base load at most 5,000 ms on the reference 4 vCPU node;
  - `MAP01-BASE-RSS-BYTES`: base model at most 1 GiB per World;
  - `MAP01-CHANNEL-OVERLAY-BYTES`: at most 64 MiB per channel;
  - `MAP01-VIEWPORT-US`: an 18x14 viewport at most 100 µs p99, over every floor the client
    sees.
- **Tests.**
  - deterministic compilation;
  - tile-by-tile source-to-bundle-to-runtime equivalence;
  - negative, corrupt and fuzz tests of the bundle reader (ADR-0005 robustness);
  - Ground rebuild after a crash;
  - map-item MINT idempotence per cause;
  - reset retirement and activation order;
  - crash recovery during a reset;
  - origin re-hiding after a crash.
- **Reviews.** An independent review of this ADR, a persistence review of MAP-OVERLAY-1, and a
  security review of the bundle reader.

## 5. Delivery

| Child | Scope | Depends on |
|---|---|---|
| MAP-SPIKE-0 | Benchmark and evidence | this ADR |
| MAP-BUNDLE-1 | Compiler, format doc, key resolution, drafts, limits | this ADR; #1160 and #1170 merged; A12 item keys |
| MAP-LOAD-1 | Reader, compact model, budgets | MAP-BUNDLE-1 |
| MAP-OVERLAY-1 | Overlay, Ground rebuild, map-item MINT, reset retirement with the HOUSE-CUSTODY-0 preflight and recheck | MAP-LOAD-1; DUR-03 §39.3 amendment accepted |
| MAP-CUTOVER-1 | Boot from the bundle through a first reset, fixture world, draft gate | MAP-LOAD-1; MAP-OVERLAY-1 |
| HOUSE-CUSTODY-0 | Decided in this PR (`reviews/OTERYN_GAME_HOUSE_CUSTODY0_HOUSE_ITEM_CUSTODY_DECISION_2026-09-30.md`); implementation child HOUSE-CUSTODY-1 (storage only). MAP-OVERLAY-1 builds its reset preflight and recheck. | none |
| MAP-WIRE-1 | Item overlay and Ground state domain, viewport snapshots. Needed before a playable release, because clients cannot see pickups or hidden origins without it; may run in parallel. | owner acceptance |

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
- **Deliberately not decided:** the reset schedule, depots, the client
  bundle, instances, and the bundle byte layout (MAP-BUNDLE-1 format document).

## 8. Before-freeze checklist

1. **Contract amendments.**
   - DUR-03 §39.1 and §39.3 are amended in this PR, including the superseded death-only MINT
     sentence and the D3 "only path" sentence.
   - The registry rows are in this PR.
   - The coordinate and import profiles are applied, not amended.
   - The wire contract is a separate child (MAP-WIRE-1).
2. **Serialization.**
   - Overlays are single-writer in the channel tick.
   - Ground and MINT follow the DUR-03 fences and the `character_root` lock.
   - A reset assigns each channel scope a fresh generation to the resetting node before it
     retires anything, using the existing assignment writer and ordinary fences.
   - `WorldReset` and `CorpseDecay` share one per-item retirement uniqueness.
3. **Restart.**
   - Ground is rebuilt and origins are re-hidden.
   - A reset resumes from its durable record, starting again at step 2.
   - The active bundle is pinned by digest and bound to the activation record.
4. **Typed references.** `MapItemMaterialization` names world, channel, digest, placement key and
   epoch. `WorldReset` names world, epoch and item.
5. **Wire.** A change is needed and is declared as MAP-WIRE-1, which needs owner acceptance.
   Nothing on the wire changes before it.
6. **Split work.**
   - Map pickup is a MINT, then a TRANSFER, each complete on its own.
   - A reset is ordered by its record, retires items one at a time, and activates only when none
     remains.
7. **Self-review.** `oteryn-hard-worker`, read-only, on the first draft (`196e40ae`). Its eight
   material findings are fixed in §4.4, §4.7, the DUR-03 amendment and MAP-WIRE-1:
   - the per-item reset cause;
   - corpse entries;
   - the reset fence;
   - the reset record;
   - origin re-hiding;
   - the MINT provenance;
   - the wire;
   - house tiles.

   A second read-only check (`51eabf93`) confirmed the fixes. It found four more problems, all
   fixed in §4.7 and the amendment:
   - the reset fence now uses per-channel generations;
   - crash recovery resumes at step 2;
   - the `0015` receipt shares one uniqueness for both causes;
   - the first cutover is a reset.

   Its hardening points 9-18 are also covered:
   - character positions;
   - placement identity;
   - the benchmark marked DERIVED;
   - the manifest fields;
   - the deployment gate;
   - the overlay budget;
   - the registry notes;
   - the DUR-04 §12 class;
   - the house id check;
   - the reset duration row.
