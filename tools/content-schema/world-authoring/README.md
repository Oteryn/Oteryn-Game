# World metadata authoring (City and HuntingPlace Area, House, teleport Transition)

This package populates the world tree from the pinned CrystalServer map. Steps 1 and 2
cover **metadata**: towns, houses and teleports, plus hunting places from the English
TibiaWiki. Step 3 covers the **base map** (19.3 M tiles, 24.9 M items) in a binary region
format selected by measurement (see "Base map (step 3)" below).

| Family | Path | Records | Shard schema |
|---|---|---:|---|
| `Area.City` | `content/world/areas/cities/` | 35 | `OTERYN_AREA_AUTHORING_SHARD/v1` |
| `Area.HuntingPlace` | `content/world/areas/hunting-places/` | 445 | `OTERYN_AREA_AUTHORING_SHARD/v1` |
| `House` | `content/houses/` | 995 | `OTERYN_HOUSE_AUTHORING_SHARD/v1` |
| `Transition.Teleport` | `content/world/transitions/` | 872 | `OTERYN_TRANSITION_AUTHORING_SHARD/v1` |
| `World` | `content/world/worlds/` | 1 | `OTERYN_WORLD_AUTHORING_SHARD/v1` (`world-record.schema.json`) |
| `WorldObject.FloorChange` | `content/world/objects/` | 447 | `OTERYN_WORLD_OBJECT_AUTHORING_SHARD/v1` (`floor-change.schema.json`) |

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
- `data-global/world/world-house.xml`.

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
  summary: `city` (exact case-insensitive match with a City Area name), `position` (a single
  `Mapper Coords` in `sector.offset` form: x = sector * 256 + offset, same frame as the City
  temples; a 311-record check against the matched city temple gives a median distance of 177
  tiles), `recommended_levels` (plain integers per vocation) and `source_facts` (`city_name`,
  `creature_names`). Creature names are wiki text, not Creature keys.
- The validator checks the snapshot pin and canonical bytes, that every snapshot page is
  bound once with its revision, city references, extent and capture counts.

## What is imported and what is not

Keys are stable across source updates: when the family files are already committed, the
converter reuses the existing key for the same source id (`crystalserver/house-id`,
`crystalserver/town-id`) even if the source renames the house or town, and only mints a slug
key for a new id (failing closed if it collides with any committed key). Teleport keys are
position-based.

- **City:** one record per OTBM town, holding the name and temple position. All 35 towns
  are kept as the source declares them, including `Dawnport Tutorial`, `Island of
  Destiny`, `Targuna` and `Home`. TibiaWiki (Cidades) enrichment comes later.
- **House:** holds identity, name, city, entry, rent, guildhall flag and beds from
  `world-house.xml`. It also holds the footprint from OTBM house tiles (per-floor tile
  counts, bounding box) and doors (`door_id` and position).
  - `declared_size` (XML `size`) and `footprint.tile_count` (OTBM tiles) differ for 979 of
    995 houses, so both are recorded.
  - Ownership, rent payment, ACL and inventory are runtime state and never appear here.
- **Teleport:** holds a from/to position, and the teleport `Item` resolved through
  `imports/crystalserver/bindings/items.json`. Only teleports whose destination is set,
  inside the map and on an existing tile are imported. The capture summary counts those
  rejected:
  - 1573 with an unset destination, which are script-driven;
  - 1 with a destination outside the map;
  - 5 with a destination on an absent tile.
- **Not here yet:**
  - floor changes that are scripted `use` actions (ladders up, rope spots, sewer grates,
    shovel and pick holes): see "Floor-change objects".
  - islands and streets; per-place skills, loot and experience ratings of hunting places.
  - the 18 editor waypoints.
  - the `data-global/world/15.30/` fragment maps.

## Coexistence with the legacy WorldProject package

`content/world/` is still the legacy WorldProject package root. The family shards in
`areas/cities/`, `areas/hunting-places/` and `transitions/` are not WorldProject locators:

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
  extent, and every City temple, House entry, door and footprint corner, teleport `from` and
  `to` and hunting place position to lie inside the bounds on a declared floor. Base map
  tiles are inside by that equality.

## Floor-change objects

`content/world/objects/` holds `WorldObject.FloorChange`: one record per item type that the
pinned `data/items/items.xml` declares with a `floorchange` attribute (stairs, ramps, holes,
trapdoors). `convert_floor_changes.py --crystal-root ... [--check]` needs the pinned
checkout (its `items.xml` sha256 is the pin already used by `convert_world_base.py`) and the
committed base map, and writes the shard `floor-changes-00000-00446.json`, the index and
`samples/floor-changes-capture-v1.json`.

- **Key:** `oteryn:world_object.floor_change.<item key without "oteryn:item.", every
  non-alphanumeric run as "_">`, for example
  `oteryn:world_object.floor_change.registry_i00000087`. The item key is resolved as in the
  base map palette: the `ots/item_server_id` binding target, else the provisional donor
  `donor:crystalserver@00ce02a5:item/<id>` (key `..floor_change.donor_<id>`,
  `provisional_item: true`). All 447 types are bound today. Ranges (`fromid`/`toid`) expand
  to one record per id.
- **Fields:** `floor_change`, the `item` reference, the items.xml `name`,
  `source_item_id`, and `occurrences_on_base_map`. It counts top-level tile items in the
  region files (a floor-change item inside a container is not a floor change) and is `0`
  for the 118 types the map does not use.
- **`floor_change` mapping** (engine: `TileStatesMap` in `item_parse.hpp`, destination in
  `Tile::queryDestination`, `src/items/tile.cpp`, both at the pinned revision):

  | items.xml | `floor_change` | Engine meaning |
  |---|---|---|
  | `down` | `down` | one floor down; the arrival tile is shifted by the ramp flags of the lower tile |
  | `north` | `up_north` | one floor up, one tile north (`y - 1`) |
  | `south` | `up_south` | one floor up, one tile south (`y + 1`) |
  | `east` | `up_east` | one floor up, one tile east (`x + 1`) |
  | `west` | `up_west` | one floor up, one tile west (`x - 1`) |
  | `southalt` | `up_south_alt` | one floor up, two tiles south (`y + 2`) |
  | `eastalt` | `up_east_alt` | one floor up, two tiles east (`x + 2`) |

  The converter fails closed on any other value, a second `floorchange` on one item, a
  missing name, or an id that items.xml declares in two nodes.
- **Counts** (see the capture summary): 194 `down`, 73 `up_north`, 52 `up_south`,
  50 `up_east`, 69 `up_west`, 5 `up_south_alt`, 4 `up_east_alt`; 329 of the 447 types
  occur on the map, 26,919 occurrences in all.
- **Excluded:** ladders that go up, rope spots, sewer grates and shovel or pick holes are
  scripted `use` actions (or runtime terrain changes), not static item attributes, so they
  are listed in the capture summary and not invented here. Item types named `ramp`,
  `stairs` or `ladder` that carry no `floorchange` attribute are decorative or scripted and
  are not records either. Teleports stay in `Transition.Teleport`.
- `validate_floor_changes.py` checks the schema and canonical bytes, the pinned source and
  the item bindings digest, key derivation, that each item key is a bound Item (or the
  donor key of an unbound id) and equals the base map palette key, sorted unique keys, no
  stray file, and recounts every `occurrences_on_base_map` and the summary from the region
  files (about 15 s on four cores).

## Commands

```bash
pip install -r requirements.txt -r requirements-dev.txt
python validate_world_metadata.py          # committed families: schema + semantics
python test_world_authoring.py             # synthetic OTBM and wiki fixtures, converters, validator
python convert_world_record.py --check     # World record from the committed base map (offline)
python validate_world_record.py            # World record and positions inside its bounds
python test_world_record.py                # World converter, validator, worlds/ tree exception
python validate_floor_changes.py           # WorldObject.FloorChange incl. recount on the base map
python test_floor_changes.py               # floor-change converter and validator fixtures
# Needs the pinned crystalserver checkout: python convert_floor_changes.py --crystal-root ... [--check]
python convert_hunting_places.py --check   # offline, from the committed TibiaWiki snapshot
# Regenerate (needs the pinned crystalserver checkout, not fetched by CI):
python convert_world_metadata.py --crystal-root /path/to/crystalserver [--check]
# Refresh the wiki snapshot (network), then reconvert:
python fandom_hunting_snapshot.py fetch && python convert_hunting_places.py
```

## Base map (step 3)

`WorldPlacement.Base` holds every tile and item of `world.otbm` (19,325,129 tiles,
24,925,845 items, floors 0-15) in `content/world/placements/`. JSON is not used: the same
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
  - an id bound in `imports/crystalserver/bindings/items.json` (`ots/item_server_id`) takes
    that binding's target key, either `oteryn:item.registry.iNNNNNNNN` or a named key such
    as `oteryn:item.currency.gold_coin`, with `provisional: false`;
  - any other id takes `donor:crystalserver@00ce02a5:item/<id>` (the donor-census key
    form) with `provisional: true`. This covers appearance-only terrain ids that are not in
    `items.xml` and new items that no binding covers yet.

  An entry the map stops using is never removed: it is flagged `"retired": true`, which is
  allowed only while no region references it.

  Later identity work (admitting the provisional items, a Terrain identity path) rewrites
  palette entries in `index.json` only. The 22 MB of region files do not change. The
  capture summary counts provisional entries and occurrences, split into ids that
  `items.xml` declares at the pinned revision and appearance-only ids. Current counts:
  25,963 palette entries, 5,987 provisional, 811,579 provisional item occurrences.
  The converter still fails closed, and never guesses, on an item or tile attribute
  outside the carried set.
- **Measured** (prototype of this codec on the same map, one thread): full load 1.7 s into
  a naive model, one sector read about 30 us, compact in-memory base about 170 MB (a
  shared immutable base for all channels). An edit rewrites one sector but changes the
  region file as a whole, so git stores it as a binary diff (`.b3` is marked `binary`).
  Runtime loading and the per-channel overlay are out of scope here.
- **Speed:** `convert_world_base.py` takes about 2.2 minutes. `validate_world_base.py`
  decodes all 1,208 regions in about 13 s on four cores (about 50 s of CPU).

```bash
python validate_world_base.py              # committed family: sha256, decode, counts, palette
python test_world_base.py                  # codec round trips, converter, validator, negatives
# Regenerate (needs the pinned crystalserver checkout, not fetched by CI):
python convert_world_base.py --crystal-root /path/to/crystalserver [--check]
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
