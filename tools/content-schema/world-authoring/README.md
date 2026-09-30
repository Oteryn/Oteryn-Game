# World metadata authoring (HuntingPlace and Island Area, teleport Transition)

This package populates the world tree from the pinned CrystalServer map. Steps 1 and 2
cover **metadata**: teleports, plus hunting places from the English TibiaWiki and
map-verified islands. Step 3 covers the **base map** (19.3 M tiles, 24.9 M items) in a binary
region format selected by measurement (see "Base map (step 3)" below). Other world families
have their own owners: Cities and Regions are the Area catalogue
(`content/world/areas/{cities,regions}/`, `area-authoring`, AREAS-1; the official-client region
converter and the city fact enrichment of earlier drafts are removed in its favour), Terrain and
WorldObjects are the WO-2 catalogue (`content/world/{terrain,objects}/`) and Houses are the
House catalogue in `content/houses/` (`OTERYN_HOUSE_CATALOGUE_OWNER_CONTRACT_V1`).

| Family | Path | Records | Shard schema |
|---|---|---:|---|
| `Area.HuntingPlace` | `content/world/areas/hunting-places/` | 445 | `OTERYN_AREA_AUTHORING_SHARD/v1` |
| `Area.Island` | `content/world/areas/islands/` | 59 | `OTERYN_AREA_AUTHORING_SHARD/v1` (`island.schema.json`) |
| `Transition.Teleport` | `content/world/transitions/` | 872 | `OTERYN_TRANSITION_AUTHORING_SHARD/v1` |
| `World` | `content/world/worlds/` | 1 | `OTERYN_WORLD_AUTHORING_SHARD/v1` (`world-record.schema.json`) |

`content/world/terrain/` (8,548 Terrain records, `oteryn:terrain.tibia.i<id>`) and
`content/world/objects/` (12,782 WorldObject records, `oteryn:world-object.tibia.i<id>`) are not
written here: they are the A12 section 4.6 catalogues of WO-2
(`../world-object-authoring/`, `build_catalogue.py`, WO-0 D93/D94) and this tooling only reads
them. This package used to carry its own `Terrain` family (`oteryn:terrain.a<id>`) and a
`WorldObject.FloorChange` family; both are removed in favour of the catalogues.

Each directory has an `OTERYN_FAMILY_INDEX/v1` `index.json`, plus canonical JSON shards of
500 records named `<stem>-<start>-<end>.json`. Every family index, shard and record shape
is defined in `world-metadata.schema.json`.

Records follow the existing `{declaration, source_bindings}` shape (see
`content/npcs/definitions/`). Positions use the same `global-target-2026-09-27` frame that
`content/services/travel/` already uses for the same CrystalServer coordinates. Mapping
those into the native `WorldTilePosition` profile
(`docs/contracts/OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1.md`) is a separate adapter step,
not part of this package.

## Source

The source is pinned in `convert_world_metadata.py` (`SOURCE`) and copied into every index
and `samples/source-capture-v1.json`:

- `zimbadev/crystalserver`, branch `summer-update`, commit `00ce02a57ca5a12e48f32a3476e37471167e4c3f`.
- `data-global/world/world.otbm`, gzip, OTBM v4, 35143×34812, floors 0–15.

This source is migration evidence (`OtsHypothesisOnly`). Only normalized facts are
committed. Map, sprite and asset bytes never are.

## Hunting places (English TibiaWiki)

`Area.HuntingPlace` has a different source from the other families: `Category:Hunting Places`
of `tibia.fandom.com` (the Portuguese wiki refuses build containers). Evidence is `Derived`
(player-observed reference, CC BY-SA), source key `oteryn:source.tibiawiki`.

- `fandom_hunting_snapshot.py fetch` (network, not run by CI) stores
  `imports/tibiawiki/hunting-places/fandom-snapshot-v1.json`: per page the page id, revision
  id, sha256 of the wikitext and the raw values of `city`, `lvlknights`/`lvlpaladins`/
  `lvlmages`, the `{{Mapper Coords}}` calls in `location` and the `CreatureList` names. No
  prose. Pages of the category without an `Infobox Hunt` (14, including the overview) are
  listed apart.
- `convert_hunting_places.py [--check]` converts the snapshot offline and writes the family,
  `samples/hunting-places-capture-v1.json` (counts and the pinned source, incl. the snapshot
  sha256).
- Key `oteryn:area.hunting_place.<slug(page title)>`, reused by page id once committed
  (namespace `tibiawiki-fandom/page-id`, the binding records the revision id). The name is
  the page title, not the infobox `name` (which carries `<br>` and typos).
- Only unambiguous facts are written; the rest is omitted and counted in the capture
  summary: `city` (exact case-insensitive match with an AREAS-1 city name, keyed `oteryn:content.area.city.<slug>`; the wiki name stays in `source_facts.city_name`; 245 resolved, 200 unmatched because only 22 cities exist), `position` (a single
  `Mapper Coords` in `sector.offset` form: x = sector * 256 + offset, same frame as the city
  temples; `recommended_levels` (plain integers per vocation) and `source_facts` (`city_name`,
  `creature_names`). Creature names are wiki text, not Creature keys.
- The validator checks the snapshot pin and canonical bytes, that every snapshot page is
  bound once with its revision, city references (checked against the AREAS-1 city records in `main`), extent and capture counts.

## Islands (map-verified)

`Area.Island` (`area_kind` `island`, `archipelago` or `continent`) is the one Area family whose
membership the **base map decides**. Owner rule: import an island only when the committed map
confirms it, meaning the wiki coordinate lies on a land component that water or lava fully
encloses. Nothing else is imported, so an island the map does not show stays out until the
map or the wiki changes.

- `fandom_island_snapshot.py fetch` (network, not run by CI) stores
  `imports/tibiawiki/islands/fandom-snapshot-v1.json`: 67 candidate pages (`Infobox Geography`
  pages naming an island or archipelago) and 7 pages that only lend a coordinate, each with
  page id, revision id, wikitext sha256, the wiki's own `Mapper Coords`, status and event
  flags and one short factual sentence. No prose. The curated decisions are constants in the
  tool and are checked against the fetched pages: coordinates borrowed from another page,
  city-temple coordinates, the Fibula correction, aliases, places inside another island.
- `island-ground-classes.json` lists the water and lava ground item ids (base map palette ids
  whose pinned `items.xml` name is one of 12 water or 4 lava names, for example `shallow
  water` 629-634,880-891,...). `convert_islands.py --crystal-root PATH` re-derives it from the
  pinned `items.xml` and fails (or rewrites) on a difference.
- `convert_islands.py [--check]` reads the snapshot, the region files, the ground classes and
  the AREAS-1 city records offline (about 20 s). A tile is land unless its first (ground) item is water
  or lava; an absent tile is void. The component of a coordinate is a 4-neighbour breadth-first
  search over land tiles of one floor, capped at 400,000 tiles. A coordinate on water starts
  from the nearest land tile within 5 tiles (squared distance, first strictly nearer wins).
  - hits the cap: `part_of_landmass`, excluded (Fibula's and Isle of the Mists' wiki
    coordinates; the mainland is 451,923 floor-7 tiles).
  - no land within 5 tiles: `not_on_map`, excluded (`event_only_not_on_map` for the
    event island Isle of Merriment); no coordinate at all: `no_coordinates`.
  - a page inside another island's component (`place_within`: Ragnir, Chyllfroest in
    Hrodmir) is excluded; a page that is the same place under another name (`alias_of`:
    Percht Island is Orcsoberfest Island) merges into `also_known_as` and one extra binding.
    Any other two pages on one component fail the conversion.
  - `archipelago` needs at least two distinct confirmed components (Ice Islands, Forbidden
    Islands, Shattered Isles), each stored with its own anchor and footprint and linked to
    the island record of the same footprint when there is one. A wiki archipelago with one
    confirmed component (Marapur, Laguna Islands) is an `island`. Darama, which the wiki calls
    a continent, is `continent`.
- Key `oteryn:area.island.<slug(page title)>`, reused by page id once committed. Each record
  holds `footprint` (floor, `tile_count` and bounding box, computed from the map), `anchor`
  (the verified start tile), `cities` (AREAS-1 city keys `oteryn:content.area.city.<slug>` whose hometown temple lies in the component on the
  same floor), `event_only: true` where the wiki says the island is event-only, and
  `source_facts` (`evidence`, `wiki_class`, `wiki_status`, `wiki_cities` as separate wiki
  claims, `anchor_origin`, `source_coordinate` when the anchor differs from it,
  `removed_from_game`).
- Fibula: the wiki coordinate is on the mainland. The anchor is the map-corrected 9,196 tile
  island west of it, named by the Meluna page (Ferryman Kamil in Fibula, 32153,32456,7), with
  `anchor_corrected_from_wiki: true`. The converter refuses a correction when the wiki
  coordinate is itself an island.
- **Evidence anchors** (owner-approved): a page without a usable wiki coordinate may be
  anchored by `island-evidence-anchors.json`, pinned in the index like the ground classes. An
  entry names the snapshot page, the anchor tile and its source: a pinned CrystalServer
  `world-npc.xml` / `world-monster.xml` spawn (`anchor_source` `crystalserver-npc:<name>` or
  `crystalserver-monster:<name>`; the spawn position is centre plus entry offset, used on the
  anchor floor) or a committed teleport destination (`teleport:<key>`). The converter requires
  the anchor to be a land tile of an enclosed component, the spawn xy to equal the anchor,
  the teleport `to` to equal it and `underground` to be set exactly below floor 7; with
  `--crystal-root` it also checks the XML sha256 values and that each spawn exists.
  `use: only` replaces the wiki coordinates (which must not be an island themselves);
  `use: primary` puts the anchor's component first and keeps the wiki coordinates as further
  components. The record carries `source_facts.anchor_origin` `evidence_anchor` and
  `anchor_source`; `underground: true` marks a component below floor 7.
  Anchors: Tutorial Island (Santiago, 32035,32272; spawn floor 6, anchor floor 7), Isle of
  Evil (Evil Mastermind spawn 32752,31458), Rascacoon (Pirat Bombardier spawn
  33839,31223), Ingol (Hawkhurst Ingol 33710,32602), Oskayaat (Tonar Oskayaat 33068,32917),
  Isle of the Mists (destination of the committed teleport at 32831,32294,
  32858,32336), Robson's Isle (Lunch 32527,32029 on floor 14, `underground`, the enclosed
  2,080 tile water-bounded floor-14 component). Not imported: Dwacatra (its floor-13/14
  pockets are void-bounded, no enclosure), Travora (no map tile),
  Redbone Castle (inside Draconia), Isle of Merriment (test server only). Temple of Light is an
  event-only island record (5,821 tiles, floor 7) because the wiki marks it an event place (`wiki_status` event); the flag comes from the wiki, not from the map, and is correct.
- **Newhaven** is one island of two components. The wiki coordinate (city temple) lies on a
  50 tile temple islet; the island itself is the 9,189 tile component west of it. Ground
  between them is water (no bridge, pier or dock), and the owner states they are joined by a
  teleport, which no committed teleport in `content/world/transitions/` represents. The
  primary anchor is the Newhaven guard Gustavo (32560,32488, `use: primary`), the islet is
  `additional_components[0]` (anchor at the temple) and `tile_count` in the capture is the
  sum (9,239). No connection is recorded because none is committed; add a
  `connected_by` link when the teleport enters the map.
- Result: 60 records (56 island, 3 archipelago, 1 continent; 3 event-only; 1 underground;
  8 evidence-anchored; 1 with an additional component) and 6 excluded pages (3 without
  coordinates, 2 part of the landmass, 1 event-only not on the map), all listed with their
  reasons in `samples/islands-capture-v1.json`. Tiny footprints such as Laguna Islands (97)
  are confirmed and kept.
- `validate_islands.py` checks the pins (snapshot, ground classes, evidence anchors, base
  map), snapshot bindings (every snapshot page bound or excluded exactly once), AREAS-1 city
  references and temple containment, footprints and anchors inside the World bounds, the
  anchor inside its footprint and within 5 tiles of the snapshot coordinate (or equal to
  its map correction or evidence anchor), additional components, `underground` (exactly the
  components below floor 7), component links and the capture summary. It does not re-run
  the search; `convert_islands.py --check` does.

## What is imported and what is not

Teleport keys are position-based.

- **Teleport:** holds a from/to position and an `object` reference that is the A12 4.6 family
  key of the item id: the WO-2 `WorldObject` key `oteryn:world-object.tibia.i<id>` or `Terrain`
  key `oteryn:terrain.tibia.i<id>` when the catalogue has the id (826 and 42 of 872), else the
  `Item` key from `imports/crystalserver/bindings/items.json` (4), else the run fails
  closed. The validator requires the family and key to exist. Only teleports whose destination is set,
  inside the map and on an existing tile are imported. The capture summary counts those
  rejected:
  - 1573 with an unset destination, which are script-driven;
  - 1 with a destination outside the map;
  - 5 with a destination on an absent tile.
- **Not here yet:**
  - floor changes that are scripted `use` actions (ladders up, rope spots, sewer grates,
    shovel and pick holes): see "Floor changes".
  - streets; islands the map does not confirm (see "Islands"); per-place skills, loot and experience ratings of hunting places.
  - the 18 editor waypoints.
  - the `data-global/world/15.30/` fragment maps.

## Coexistence with the legacy WorldProject package

`content/world/` is still the legacy WorldProject package root. The family shards in
`areas/hunting-places/`, `areas/islands/` and `transitions/` are not
WorldProject locators:

- `validate_materialized_game_tree.py` accepts a populated family index there only when its
  shards stay in that directory, the directory holds no other file, and no legacy locator
  shares it.
- `--print-world-successor-files` lists markers and shards. The package-seed workflow
  removes them before its exact `diff -r`.
- `content_world_project_repository.rs` admits them by the same rule.

**The one exception is `worlds/`**, which shares space with the legacy locator
`worlds/world.json`. Legacy lookups scan that directory, so it may hold the family index and
exactly **one** shard (`worlds-00000-00000.json`, the World record) next to the locator, and
nothing else. `validate_materialized_game_tree.py` (`SHARED_WITH_LOCATOR`,
`SHARED_MAX_SHARDS`) and the repository test (`SHARED_DIRECTORY_SUCCESSOR_SHARDS`) name that
one path. Every other directory with a legacy locator still refuses a family. The
repository test's directory-scan budget grows by exactly the one added entry
(`144 + 56 + 1`; measured 189 to 190 scanned entries in the recapture test).

## World record

`content/world/worlds/` holds one `World` record, `oteryn:world.oteryn`, generated offline by
`convert_world_record.py [--check]` from committed data only:

- `bounds` is the exact tile bounding box of the base map (`min_x` 1340, `min_y` 1643,
  `max_x_exclusive` 34264, `max_y_exclusive` 33813), computed from the region sector tables
  with only the four edge columns of sectors decoded. The maxima are exclusive, as the
  half-open envelope of `OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1` requires. `source_map`
  keeps the OTBM header extent (35143x34812, version 4) for comparison; the tiles cover
  less of it.
- `floors` is the strictly increasing list of floors that hold tiles: 0-15.
- `legacy_world_id` is the identity of `content/world/definitions/reference.json`.
- `validate_world_record.py` requires bounds and floors to equal the recomputed base map
  extent, and every teleport `from` and `to`, hunting place position and footprint corner to lie inside the bounds on a declared floor. Base map
  tiles are inside by that equality.

## Floor changes

There is no floor-change family. The WO-2 catalogues carry the `items.xml` `floorchange` value as the
`floor_change` fact of each Terrain and WorldObject record (`north`, `south`, `east`, `west`,
`southalt`, `eastalt`, `down`): 444 of the 447 item types that declare it are in the catalogues
(158 Terrain, 286 WorldObject). The other three (166 and 167 `wooden coffin`, 53431 `stone stairs`)
stay Items; `edron_rework.ITEM_FLOOR_CHANGES` lists them. `edron_rework.floor_change_kinds()` maps the
catalogue values to the kinds below and adds the two rope spots (386, 21965).

| catalogue value | kind | Engine meaning (`Tile::queryDestination`, `src/items/tile.cpp`) |
|---|---|---|
| `down` | `down` | one floor down |
| `north` / `south` / `east` / `west` | `up_north` / `up_south` / `up_east` / `up_west` | one floor up, one tile north (`y - 1`), south (`y + 1`), east (`x + 1`), west (`x - 1`) |
| `southalt` / `eastalt` | `up_south_alt` / `up_east_alt` | one floor up, two tiles south (`y + 2`) or east (`x + 2`) |

The base capture summary (`floor_changes`) counts the kinds on the map: 331 item types, 28,160
occurrences (container contents included, rope spots and fills included). Ladders that go up, rope
spots, sewer grates and shovel or pick holes are scripted `use` actions in the server, not
`floorchange` attributes, and are not invented here. Teleports stay in `Transition.Teleport`.

## Commands

```bash
pip install -r requirements.txt -r requirements-dev.txt
python validate_world_metadata.py          # committed families: schema + semantics
python test_world_authoring.py             # synthetic OTBM and wiki fixtures, converters, validator
python convert_world_record.py --check     # World record from the committed base map (offline)
python validate_world_record.py            # World record and positions inside its bounds
python test_world_record.py                # World converter, validator, worlds/ tree exception
python convert_hunting_places.py --check   # offline, from the committed TibiaWiki snapshot
python convert_islands.py --check          # offline, from the snapshot, the base map and the AREAS-1 city and region records
python validate_islands.py                 # island family, pins, footprints, capture summary
python test_islands.py                     # synthetic map: island, landmass, lava, alias, archipelago
# Needs the pinned crystalserver checkout: python convert_islands.py --crystal-root ... [--check]
# `--check` also reports (`EXTRA`) generator-owned shards no longer generated; write mode
# deletes them. Regenerate (needs the pinned crystalserver checkout, not fetched by CI):
python convert_world_metadata.py --crystal-root /path/to/crystalserver [--check]
# Refresh the wiki snapshot (network), then reconvert:
python fandom_hunting_snapshot.py fetch && python convert_hunting_places.py
python fandom_island_snapshot.py fetch && python convert_islands.py
```

## Base map (step 3)

`WorldPlacement.Base` holds every tile and item of `world.otbm` (19,325,129 tiles,
24,925,845 items, floors 0-15) plus the fills from `maps.7z` and the Edron rework and the minimap draft below
(19,351,010 tiles, 24,959,455 items in all) in `content/world/placements/`. JSON is not used: the same
data is 3.25 GB as JSON sectors and about 22 MB in the format below.

- **Layout:** `index.json` (`OTERYN_FAMILY_INDEX/v1`, family `WorldPlacement.Base`) plus one
  file per non-empty 256x256 region, `region-z{z:02}-x{rx:03}-y{ry:03}.b3`
  (`rx = x // 256`). The index lists the shards, a `regions` table with each file's
  `path`, `sha256`, `tiles` and `items`, and the item `palette`.
  `samples/world-base-capture-v1.json` records the source, extent, totals, per-floor
  tiles, bytes on disk, the zstd build, the palette counts and the count of rejected
  attributes (must be 0).
- **Format** (`OTERYN_WORLD_REGION_B3/v1`, spec in the `world_region_codec.py` docstring): a
  12-byte header (`OTRB`, version, floor, `rx`, `ry`, sector count), a sector table
  (index 0-63, offset, length) and one zstd level-3 frame per non-empty 32x32 sector. A
  sector lists its tiles sorted by (y, x) with delta-coded positions and varints. Items
  are in stacking order, container contents flattened behind their container with a depth.
- **Carried per tile:** flags, house id, tile zone ids (Canary `OTBM_TILE_ZONE`, 476 tiles
  on floor 10). **Per item:** the palette index and, when present, count,
  charges, action id, unique id, text, description, teleport destination, depot id, house
  door id. Presence is exact, so a present zero or empty text stays present.
- **Item identity (palette):** a region file never names an item. It stores, per item, an
  index into `palette` in `index.json`. The palette holds the distinct server item
  ids the map uses (and any retired ones) as `{"key", "source_item_id", "provisional"}`. A fresh build orders it by
  ascending id, and the palette is then **append-only** (see below):
  - an id bound in `imports/crystalserver/bindings/items.json` (`ots/item_server_id`) to a
    key that has an Item record in `content/items/definitions` takes that target key, either `oteryn:item.registry.iNNNNNNNN` or a named key such
    as `oteryn:item.currency.gold_coin`, with `provisional: false`;
  - else an id that a WO-2 catalogue has takes the catalogue key with `provisional: false`:
    `oteryn:terrain.tibia.i<id>` if the Terrain catalogue has it, else
    `oteryn:world-object.tibia.i<id>`;
  - any other id takes `donor:crystalserver@00ce02a5:item/<id>` (the donor-census key
    form) with `provisional: true`. This covers ids that `items.xml` declares but no Item
    binding covers yet, ids bound to an Item key without a definition (the A12 section 5 rule
    forbids such a key in `content/`) and ids no source declares; the item agent's B1b
    registry step admits the first two.

  **Resolution order (architect ruling, #162 Q1b):** the Item key of an id with an Item record in
  `content/items/definitions` (even when a catalogue also routes the id; the compiler follows
  `routed_to`), else the Terrain catalogue key, else the WorldObject catalogue key, else the
  provisional donor key. Appearance-only ids (the official client
  declares them, `items.xml` does not) have no record in any family and stay provisional; they
  are listed in `samples/appearance-only-ids-v1.json` (see "Appearance-only ids").

  An entry the map stops using is never removed: it is flagged `"retired": true`, which is
  allowed only while no region references it.

  Identity work (admitting provisional items, extending the catalogues) rewrites palette entries in
  `index.json` only. The 22 MB of region files do not change. The capture summary counts palette
  entries and occurrences per family (`palette.families`: terrain, world_object, item) and the
  provisional ones, split into ids that `items.xml` declares at the pinned revision and
  appearance-only ids. Current counts: 25,984 palette entries; 19,989 Item keys (24,168,528
  occurrences; the 7,576 Terrain and 7,699 WorldObject ids that have Item records are here);
  0 Terrain and 0 WorldObject catalogue keys; 5,995 provisional
  (814,803 occurrences): 5,950 appearance-only (760,257; 5,949 with a client appearance plus id 99,
  which neither `items.xml` nor the client declares) and 45 ids that `items.xml` declares
  (54,546, 40 of them bound to an Item key without a definition).
  `validate_world_base.py` requires every non-provisional key to follow that order: an Item
  binding target with an Item record, else the Terrain catalogue key, else the WorldObject
  catalogue key; a provisional id must have none of them. It also hashes the actual
  `imports/crystalserver/bindings/items.json` and fails when it differs from
  `item_bindings.sha256` in the index, so the palette cannot outlive a change of the bindings
  (regenerate it with `convert_world_base.py`; the region files keep their bytes).
  The converter still fails closed, and never guesses, on an item or tile attribute
  outside the carried set.
- **Fill from `maps.7z`** (owner-approved, fill-only): `data-global/world/maps.7z` at the pinned
  revision (sha256 `c770e239...`, 945,468 bytes) holds fragment maps. `blue_valley.otbm`
  (sha256 of the extracted member `9f0bd617...`, 248,546 bytes, 22,802 tiles on floors 1-7)
  uses the absolute Tibia coordinates of `world.otbm`, so no offset. After `world.otbm`,
  `convert_world_base.py` adds a fragment tile **only where the base map has no tile at that
  (x, y, z)**; a position the base (or an earlier fragment tile) already has is skipped, so
  nothing existing is overwritten or merged. A tile that would be added with a house, tile
  zone or teleport destination is refused (none is; existing tiles that have them are
  skipped like any existing tile). Items map to keys like the base import: an
  id `items.xml` declares takes its Item binding or the provisional donor key, an
  appearance-only id stays provisional; ids first used by
  the fill are appended at the end of the palette. Result (capture summary `fill`):
  2,965 tiles and 3,364 items added: floor 2 118, floor 3 219, floor 4 685, floor 5 918,
  floor 6 1,025, floor 7 0, floor 1 0; 19,837 fragment tiles skipped as already present.
  Floor 7 gains nothing from the fill because the base map already has a tile at every
  fragment position: 905 of the 10,747 floor-7 fragment tiles are land where the base has
  water ground, 529 are water on both, 10 water over land and 9,303 land on both. The 905
  are handled by the replacement below, the only place a base tile changes.
- **Replacement of Blue Valley floor-7 water** (owner decision 1a; the `replace` object of
  the `blue_valley.otbm` pin, `replacement_candidates` and `Replacer`). A base tile is
  **replaced** by the fragment tile (ground and items, mapped to keys as above) only where
  all three hold: the base tile's ground (first item) is water in
  `island-ground-classes.json`; the fragment tile's ground is neither water nor lava; and
  the official 15.30 minimap (floor 07) shows land at that position. Nothing else is ever
  replaced: base land, water on both sides and positions the minimap shows as water keep the
  base tile; a base tile to replace that carries a house, zone or teleport, or a candidate
  that does, is refused. Result (capture summary `replace`): 905 tiles on floor 7 (896 of
  them inside the Blue Valley box x33508-33654, y31378-31537), removing 1,100 base items
  and adding 962; the base ground ids were the water ids 4597-4601. The tile count is
  unchanged (the fill counts the replaced positions as skipped, the base already has a tile
  there); `world.otbm` item totals equal the index totals minus the fill
  items minus the replacement's `items_added` plus its `items_removed`. The validator
  checks the record against the pin (floors, member, at most the tiles the fill skipped per
  floor); `--check` recomputes the count. The islands family follows: the Blue Valley
  footprint grows from 9,522 to 10,427 tiles (15,612 with the minimap draft).
  The index `source.fill` pins the archive and member; the summary `fill` records the rule,
  counts per floor and skipped tiles. `totals` and `tiles_by_floor` include the fill;
  `totals` minus the fill equals the pinned `world.otbm` totals (19,325,129 tiles,
  24,925,845 items). Reading the archive needs `py7zr` (`requirements-regenerate.txt`, not
  installed by CI; the Edron rework and the draft need Pillow from the same file); `--check` reproduces everything from the pinned checkout.
- **Partial fill from `summer-update-2025.otbm`** (owner decision 2b; second `FILL` entry,
  same archive, member sha256 `d4b4baee...`, 2,964,786 bytes, 286,241 tiles on floors 0-15,
  absolute coordinates). Nothing in CrystalServer loads this file. The rule is computed by
  `select_tiles` (recorded as `select` in the pin), not a hand list. Candidates are the
  fragment tiles whose (x, y, z) the base has no tile at **after** the Blue Valley fill
  (45,047 tiles), then:
  - **Floors 8-15:** 4-connected components per floor. A component is included whole unless
    one of its tiles lies in the exclusion box `edron-underground`: x 33274-33456, y
    31786-31884, floors 8-12 (the bounds of the Edron Surroundings/Stonehome clusters). Edron
    is not part of this fill: the base already has Edron caves, so a fill-only mix would
    combine two layouts; the rework below handles it. Components below a size threshold are kept: the research
    found no noise, so there is no threshold. Result: 127 components included (16,777 tiles:
    floor 8 1,913, 9 5,678, 10 584, 11 611, 12 169, 13 3,608, 14 3,619, 15 595) and 16
    excluded (27,359 tiles: floor 8 1,816, 9 6,567, 10 7,901, 11 8,822, 12 2,253). By area
    (a description by bounding box, not part of the rule): Liberty Bay/Vandura floors 12-15
    7,519 tiles in 19 components (floor 13 3,139, 14 3,619, 15 595, 12 166), Port
    Hope/Tiquanda floors 9-13 4,808 in 73 (floor 9 4,056), Kazordoon/Femor Hills floors 8-11
    3,902 in 4, and 548 elsewhere in 31 small components.
  - **Floors 0-7:** a single tile is kept only if the official 15.30 minimap
    (`content/assets/files/minimap-32-*`, floors 0-7 only) shows land there: a pixel that is
    neither black nor water `#336699`. Result: 151 tiles (floor 2 30, floor 3 91, floor 4 4,
    floor 5 11, floor 6 15); 760 tiles without minimap land are dropped (floor 3 111, 4 241,
    5 303, 6 105).
  - Nothing existing is overwritten (a position the base has is skipped, 241,194 tiles).
    A tile with a house, zone or teleport destination is still refused; a teleport item
    whose destination is unset (0, 0, 0) leads nowhere and is carried (4 tiles, floor 14).
  Total 16,928 tiles and 19,667 items added; 6 new palette entries (append-only), all
  appearance-only and provisional. The summary `fill.sources[1]`
  holds the counts per floor, `selection` (components and tiles excluded or without land)
  and `tiles_not_selected`; `--check` reproduces it (the minimap is read from the committed
  client assets, the fragment from the pinned archive). Underground validity rests on the
  file being the official summer-2025 update placed under official surface; the official
  minimap cannot check floors 8-15.
  The gaps no source fills (Blue Valley north-east, east and south blocks, Great Expedition
  Island and Wharf, Marapur/Thalassara floors 2-6, Nargor floors 4-6, Upper Roshamuul floor 6,
  Temple of Light) are drafted from the minimap below, needing detail work.
- **Edron underground rework** (owner decision 1a; `edron_rework.py`, the `edron` object of
  the summary and `source.edron` of the index; evidence class **reference-derived**). The
  base and the summer file disagree on the Edron caves, so the player-recorded real-Tibia
  minimap decides what is real: `github.com/tibiamaps/tibia-map-data`, files
  `floor-09/10/11-path.png` and `-map.png` plus `bounds.json` (one pixel per tile, frame
  `xMin` 31744, `yMin` 30976, identity with the project frame, verified). The files are not
  committed. They are pinned like `maps.7z`: `source.edron.tibiamaps` records the fetch URL
  (`raw.githubusercontent.com/.../main/data/`, branch head, commit id not retrievable), the
  fetch date 2026-09-29 and the sha256 of every file used, and the converter reads them from
  `--tibiamaps-root` (`--check` reproduces the result; reading the PNGs needs Pillow,
  `requirements-regenerate.txt`). Everything is confined to the box x33274-33456,
  y31786-31884. Walkable means no item of the tile has the client `unpass` flag (solid rock
  is `unpass` ground); a tibiamaps pixel is walkable if its path pixel is grey.
  - **Rule 1 (floor 10):** the core is every summer tile that is walkable where tibiamaps is
    walkable; the included set is the core plus every summer tile in its 8-neighbourhood
    whose walkability agrees with tibiamaps. Where the base has no tile the summer tile is
    **filled**. A base tile is **replaced** only where its walkability disagrees with
    tibiamaps and the summer tile agrees; a base tile that agrees is kept, so a base tile
    that tibiamaps shows walkable is never removed. Result: 3,982 filled (2,785 walkable,
    1,197 wall and rock), 699 replaced (645 to walkable, 54 to blocked), 423 kept, from 5,104
    included tiles (3,534 core).
  - **Rule 2 (floors 9 and 10, after rule 1):** a position that tibiamaps shows walkable and
    that is walkable in neither the map so far nor the summer file gets a plain ground tile
    (the most common walkable ground of that floor in the box; recorded: id 22795 on floor 9,
    4394 on floor 10), and every non-walkable 8-neighbour without a tile gets the most common
    blocking ground (101 on both floors) so the cave stays closed. A blocked tile at a target
    position is replaced, a walkable tile never is. Result: floor 9 765 targets (761 added, 4
    replaced) and 379 rock; floor 10 265 targets (262 added, 3 replaced) and 135 rock.
  - **Rule 3 (entrances, report only):** a yellow pixel of the map image on floors 9-11 is a
    floor-change marker, unless the tile carries a yellow-automap item that is no floor change
    (yellow ground) or the pixel lies in a yellow area of more than 2 pixels (ground colour).
    A marker is connected if the final map has a floor-change item (catalogue `floor_change` facts,
    the three Item ids and the rope spots 386 and 21965) at that position or at the same (x, y) one floor above
    or below. Otherwise the position is listed in `unresolved_entrances`; **no item is
    invented**. Result: 15 markers (floor 9 5, floor 10 3, floor 11 7), 5 connected, 10
    unresolved. The summary also records the reachability of floor 10 (BFS over walkable
    tiles and floor-change links): the new floor-10 cave (3,534 tile main component) is **not
    reachable** from the surface or from the other floors. Its only link, the summer floor
    change `1080` at (33295, 31819, 9) to floor 10, is on a floor-9 tile the base does not
    have (floor 9 keeps the base) and is listed as unresolved.
  - Totals: 5,519 tiles added and 706 replaced (floor 9 1,140 added and 4 replaced, floor 10
    4,379 added and 702 replaced). Floors 8, 11 and 12 keep the base, as tibiamaps agrees
    with the base there and the summer file adds nothing real (floor 9 outside rule 2 as
    well). `world.otbm` totals equal the index totals minus the fills and this rework's added
    tiles and item changes (`world_otbm_totals`). The validator checks the pins, that every
    count adds up and the entrance lists.
- **Minimap draft (rough draft, owner decision 2a; `minimap_draft.py`, the `draft` object of
  the summary and `source.minimap_draft` of the index; evidence class
  **reference-derived**).** For areas that no source has, the converter drafts tiles from the
  pinned tibiamaps minimap (the files of the Edron rework plus `floor-06/07-map.png` and
  `-path.png`, sha256 in `minimap_draft.py`, read from `--tibiamaps-root`, not committed).
  **It is a rough draft: correct shape and walkability, generic ground, no borders,
  decorations, doors or furniture.** Each area is a named entry of `AREAS` with a
  committed bbox and floor set (list below). `source.minimap_draft` and `draft.areas` name every area
  and its bbox, so a draft can be removed or replaced as a whole when a real source appears.
  - **Colour mapping, learned:** over the whole base map on each floor of 2-7 (outside the
    drafted areas; every floor has its own table, roofs and upper floors differ) each base tile is paired with the map-image colour at its position. The
    path image gives the class (grey pixel walkable, other explored pixel blocked). Per
    (floor, colour, class): the most frequent ground id; for blocked also the most frequent single
    top item (none if that is most frequent). Fewer than 50 samples means unmapped, the
    pixel is skipped and counted. The table (floor, colour, class, ground, item, samples) is
    `draft.mapping.rows` in the summary. Yellow is the floor-change colour and never mapped.
  - **Rule:** a position of the bbox is drafted where the map image is coloured (not black)
    and the base has no tile or a plain water tile (water ground only, no house or zone;
    the flags are kept). Floors 0-7 also need the official 15.30 minimap of that ZZ to show land,
    unless the mapped ground is water (then the base water stays). A tile of a `maps.7z`
    fill fragment counts as a base tile. An area (or floor) where under 5% of its tibiamaps
    pixels would be drafted is skipped and reported. A base tile that is not plain water is never replaced.
    Yellow pixels are **not** turned into items; they are listed in
    `draft.unresolved_entrances`.
  - **Drafted areas (committed bbox and floors in `minimap_draft.AREAS`; tiles added, on
    plain water replaced):** `temple-of-light` x31912-32027 y31979-32099: f6 469, f7 5,832.
    `great-expedition-island` x33883-33986 y30983-31093: f3 317, f4 218, f5 356, f6 400, f7
    5,454. `great-expedition-wharf` x32190-32253 y32475-32530: f7 1,644 (107 pixels without
    official land skipped). `blue-valley` x33504-33658 y31374-31541: f4 763 (+1), f5 1,114, f6
    2,712, f7 5,575. `marapur-thalassara` x34112-34242 y32443-32626: f2 922, f3 1,736, f4
    2,817, f5 4,085, f6 4,652 (+14). `nargor` x31898-31941 y32834-32902: f4 273, f5 515, f6
    728. `upper-roshamuul` x33613-33717 y32261-32357: f6 901. Total 22,978 tiles added and
    18,520 plain-water tiles replaced (24,112 items added); no area fell under the 5% skip
    limit (lowest share of drafted tibiamaps pixels 21.4%, Blue Valley f7). 130 colour rows
    mapped, 18 unmapped (under 50 samples on their floor), 18 unresolved entrances (15 on
    floor 7, 3 on floor 6). **The drawing list is drafted, not done: it needs detail work**
    (borders, decorations, doors, furniture, floor changes). Blue Valley, Marapur/Thalassara,
    Nargor and Upper Roshamuul keep every base and fill tile; only the missing ones are drafted.
    Totals and counts per floor are validated (`validate_world_base.py`,
    `test_minimap_draft.py`).
- **Not imported / deferred:** `access.otbm`, `asura_resp.otbm`, `boss_rooms_-_part_2.otbm`
  and `final.otbm` of `data-global/world/15.30/` are unreferenced local-coordinate drafts (x
  about 945-1173, y about 999-1096, floors 5-7, 17,801 tiles, absent from the base). No
  script, XML or C++ file at the pin loads them and no boss uses their coordinates. They
  overlap the Movement Trainer area of `custom/global-custom.otbm`, which is off by default
  (`toggleMapCustom=false`). Owner decision: not imported, deferred. The other six files of
  `15.30/` (Thalassara,
  castle, asura_sanctuary, asura_sanctuary_boss, mimar_haffar, werepanther_boss_map) are
  already contained in `world.otbm` (0 missing tiles); only water ground and decoration
  variants differ, and the base wins. All 57 BossLever rooms at the pin are present in the
  base map. Known data gap: the General Murius raid spawn (32427,31131,15) has no tile in
  the base (tiles exist there only on floors 7-11). The other members of `maps.7z` are not pinned or imported.
  Measured against the base positions only (their coordinate frame is not verified):
  `newheaven` and `winter-update-2025` would add one tile each, `summer-update-2025`
  (286,241 tiles) would add 45,047 tiles on floors 2-6 and 8-15, mostly underground.
- **Measured** (prototype of this codec on the same map, one thread): full load 1.7 s into
  a naive model, one sector read about 30 us, compact in-memory base about 170 MB (a
  shared immutable base for all channels). An edit rewrites one sector but changes the
  region file as a whole, so git stores it as a binary diff (`.b3` is marked `binary`).
  Runtime loading and the per-channel overlay are out of scope here.
- **Speed:** `convert_world_base.py` takes about 4.5 minutes. `validate_world_base.py`
  decodes all 1,208 regions in about 13 s on four cores (about 50 s of CPU).

```bash
python validate_world_base.py              # committed family: sha256, decode, counts, palette
python test_world_base.py                  # codec round trips, converter, validator, negatives
# Regenerate (needs the pinned crystalserver checkout, not fetched by CI):
python convert_world_base.py --crystal-root /path/to/crystalserver \
    --tibiamaps-root /path/to/tibiamaps-data [--check]
python test_edron_rework.py                # the Edron rework rules, pins and validator negatives
python test_minimap_draft.py               # the minimap draft mapping, rules, pins and validator negatives
```

Region files are compressed by libzstd through the pinned `zstandard` package. The capture
summary records the versions, and `--check` compares bytes, so run it with the same pins.

To update from a newer CrystalServer revision, change the pin in `convert_world_metadata.py`
and the `items.xml` digest in `convert_world_base.py`, then rerun the converter. Region
files whose tiles did not change stay byte-identical, so only the changed regions differ,
plus `index.json` and the summary. The palette is append-only, so a new id never renumbers
anything: when `content/world/placements/index.json` is committed, the converter keeps every existing entry
at its index (refreshing only its key if the binding changed, for example provisional to an
Oteryn key), appends new server ids at the end in ascending id order, and keeps an id the
new map no longer uses as `"retired": true`. Untouched region files stay byte-identical.
The validator requires unique ids and keys, every non-retired entry referenced and every
retired entry unreferenced. Oteryn edits authored on top of the base map will later need a
separate patch layer that survives a regeneration. That layer is not implemented.

## Appearance-only ids

An appearance-only id is a base map palette id that the official Tibia 15.30 client declares in
`content/assets/files/appearances-<sha256>.dat` (sha256 checked against
`imports/official/client-assets/15.30/manifest.json`) and that the pinned CrystalServer `items.xml`
does not declare. It has no Item and no A12 section 4.6 catalogue record, so its palette key is the
provisional donor key. `samples/appearance-only-ids-v1.json` lists them as evidence for the WO lane
to extend the catalogue: 5,949 ids (id 99 has no appearance), 760,248 occurrences on the base map,
each row with the id, `class`, occurrences, `speed` (bank waypoints, ground only) and `name`
(7 rows).

- **Class rule** (first match on the client flags): `bank` -> `ground`; else `clip` -> `border`;
  else `unpass` -> `blocking`; else `decoration`. Counts: ground 740, border 1,534, blocking 2,614,
  decoration 1,061. There is no `liquid` class: the client declares no flag for water or lava ground.
- **Reader:** `client_appearance_reader.py` decodes the raw protobuf with the framing of
  `client_map_reader.py` (kept as the shared protobuf and frame reader; its area and mask code has no user since AREAS-1 owns the regions) (fails closed on malformed frames, repeated ids, non boolean flags and
  unsupported wire types). It reads the id, name and the flags `bank` (with waypoints), `clip`,
  `unpass`, `unmove` and `automap` (colour). `edron_rework.py` also uses it for walkability.
- `convert_appearance_only_ids.py --crystal-root PATH [--check]` writes the list from the committed
  palette, the region files and the appearances file.

```bash
python convert_appearance_only_ids.py --crystal-root /path/to/crystalserver [--check]
```
