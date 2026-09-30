# World metadata authoring (City, HuntingPlace and Region Area, House, teleport Transition)

This package is the first population of the world tree. It covers **metadata** only: towns,
houses and teleports, plus hunting places from the English TibiaWiki. Terrain, map objects and placements (19.3 M tiles, 24.9 M items) are
a later step. They need a physical storage format selected by measurement, and JSON is not
that format.

| Family | Path | Records | Shard schema |
|---|---|---:|---|
| `Area.City` | `content/world/areas/cities/` | 35 | `OTERYN_AREA_AUTHORING_SHARD/v1` |
| `Area.HuntingPlace` | `content/world/areas/hunting-places/` | 445 | `OTERYN_AREA_AUTHORING_SHARD/v1` |
| `Area.Region` | `content/world/areas/regions/` | 465 | `OTERYN_AREA_AUTHORING_SHARD/v1` |
| `House` | `content/houses/` | 995 | `OTERYN_HOUSE_AUTHORING_SHARD/v1` |
| `Transition.Teleport` | `content/world/transitions/` | 872 | `OTERYN_TRANSITION_AUTHORING_SHARD/v1` |

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

## Regions (official Tibia 15.30 client map)

`Area.Region` comes from the official client files committed under `content/assets/files/`
(owner-confirmed redistribution): `map-<sha256>.dat` and the `subarea-*` mask images. Evidence
is `OfficialClient`, source key `oteryn:source.tibia_client`, binding namespace
`tibia-client/map-area-id` (the binding records the sha256 of `map.dat`). The converter reads
the files offline and verifies each against `imports/official/client-assets/15.30/manifest.json`.

The formats are undocumented; `client_map_reader.py` decodes only what was checked against the
data and fails closed on anything else (unknown field, wire type, framing, BMP shape).

- `map.dat` is a raw protobuf. It holds 465 areas: 28 with a child list (`region`) and 437
  without (`subregion`), every subregion listed by at least one region (Tibiadrome by five).
  The rest of the file: 1270 markers, 948 satellite/minimap image entries, 209 subarea mask
  entries and two declared corner positions. It carries no third level, so there is no
  street or district tier.
- A subarea mask is a Tibia-framed LZMA stream holding a 32-bit BMP of one pixel per tile,
  anchored at the entry's position. A non-zero pixel is a tile of the area. The file name ends
  in the sha256 of the decoded BMP, which the reader checks, and the BMP size equals the map
  entry. The 209 masks hold 1,901,667 tiles, all on floor 7.
- Coordinate check: the frame is the one City temples use. The Thais temple (32369,32241,7)
  lies in the mask of `Thais City` only, the Ab'Dendriel temple (32732,31634,7) in
  `Ab'Dendriel City` only, and 21 of the 22 floor-7 City temples fall inside a mask of a
  similarly named area. 138 of 139 area anchors on masked areas lie inside their own mask
  (`Thais Trolls' Cave` does not). The region `Thais` anchors at exactly the Thais temple.
- Key `oteryn:area.region.<slug(name)>`, reused by area id once committed. 21 names occur twice
  (a region and a subregion of the same name); the region keeps the plain slug and the
  subregion gets `_subregion`. A remaining collision fails closed.
- Records: `area_kind` `region` or `subregion`, official `name`, `anchor` (map.dat position,
  158 records), `parent_regions` (subregions only, sorted), `footprint` (209 subregions: floor,
  tight bounding box, `tile_count`, and the mask image path and sha256) and `cities`.
- `cities` lists City Areas whose temple lies in a subregion's mask on the mask's floor (21 of
  35 cities). A region lists the cities of its subregions. A mask is proven for floor 7 only.
  For a temple on a floor other than 7, the candidate is the floor-7 position (x, y): the city
  is linked only when that lies inside exactly one subregion mask, and the link is recorded in
  the capture summary (`cities.projected`, method `temple_projected_to_floor_7`, 12 cities:
  Ankrahmun, Darashia, Dawnport, Edron, Farmine, Gray Beach, Issavi, Kazordoon, Krailos,
  Moonfall, Rathleton, Roshamuul). An ambiguous or outside projection stays unlinked
  (Gnomprona, `projection_outside`). The temple outside every mask (Home) stays unlinked; the
  summary records its nearest mask and Chebyshev distance (`cities.unlinked`, Greenshore, 66),
  which is information only. The validator checks the projected links against the footprint
  boxes and the unlinked list against the records (33 of 35 cities linked).
- Not imported, counted in `samples/map-regions-capture-v1.json`: the meaning of area field 6
  (28 areas) and of the secondary names of field 7 (53 areas), the 1270 markers, the satellite
  and minimap images, the declared corner positions, the 228 subregions without a mask and
  `staticmapdata-*.dat` (995 blocks of appearance-id tile grids per position, no area data).
- The validator checks the pinned map file, manifest and mask files, hierarchy, sorted unique
  lists, city references and temple containment, extent, counts and stray files.

## Cities (English TibiaWiki)

The 35 City Areas keep their keys, names, temples and CrystalServer bindings, and gain facts
from `tibia.fandom.com` (evidence `Derived`, source key `oteryn:source.tibiawiki`), by the
same pattern as the hunting places.

- `fandom_city_snapshot.py fetch` (network, not run by CI) stores
  `imports/tibiawiki/cities/fandom-snapshot-v1.json`. A city page is matched by exact title
  and must hold an `Infobox Geography`; per page it records the page id, revision id,
  wikitext sha256, the raw `implemented`, `ruler` and `near` fields, and the names of the
  `Category:<City> NPCs` pages (the page's NPC list is a category query, not wikitext). No
  prose. Cities apart, each with a reason:
  - no page with the exact title: `Dawnport Tutorial`, `Home`, `Salgadora`;
  - a page without the city infobox (`Infobox Hunt`): `Bounac`, `Cobra Bastion`;
  - ambiguous: `Targuna` (a placeholder town sharing its temple with `Dawnport Tutorial`).
- `convert_city_facts.py [--check]` rewrites the City shard and index offline (29 records
  enriched) and writes `samples/cities-capture-v1.json`. It is idempotent, so run it after
  `convert_world_metadata.py` regenerates the plain records. That converter now applies this
  enrichment itself whenever the committed snapshot exists, so both `--check` modes agree. The index gains `enrichment`
  (the pinned snapshot); its `source` stays the CrystalServer pin.
- Added: a second binding `tibiawiki-fandom/page-id` (revision id); `source_facts`
  (`implemented`, cleaned `ruler` and `near` text, `npc_names_unmatched`, 29 records);
  `implemented` only for a plain version (`6.2`, `Pre-6.0`, `12.20.8834`; 25 records, 4 stay
  raw); `npcs`, sorted NPC keys (26 records, 969 names linked, 239 unmatched).
- NPC definitions carry no display name, so a wiki name links only when it equals the key
  slug with underscores read as spaces (case-insensitive). Names with an apostrophe, dot or
  hyphen stay unmatched.
- The validator checks the snapshot pin and canonical bytes, the page/revision bindings, the
  exact-name match, NPC key existence, that linked and unmatched names cover the snapshot,
  and the capture counts.

## What is imported and what is not

Keys are stable across source updates: when the family files are already committed, the
converter reuses the existing key for the same source id (`crystalserver/house-id`,
`crystalserver/town-id`) even if the source renames the house or town, and only mints a slug
key for a new id (failing closed if it collides with any committed key). Teleport keys are
position-based.

- **City:** one record per OTBM town, holding the name and temple position. All 35 towns
  are kept as the source declares them, including `Dawnport Tutorial`, `Island of
  Destiny`, `Targuna` and `Home`. The English TibiaWiki adds facts, see "Cities" below.
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
  - the World record (bounds and floors). Its `worlds/` directory shares space with a
    legacy locator.
  - floor changes through stairs, ladders or holes. These are derived from item types
    together with terrain.
  - islands and streets (the client map has no street tier); per-place skills, loot and
    experience ratings of hunting places.
  - the 18 editor waypoints.
  - the `data-global/world/15.30/` fragment maps.

## Coexistence with the legacy WorldProject package

`content/world/` is still the legacy WorldProject package root. The family shards in
`areas/cities/`, `areas/hunting-places/`, `areas/regions/` and `transitions/` are not
WorldProject locators:

- `validate_materialized_game_tree.py` accepts a populated family index there only when its
  shards stay in that directory, the directory holds no other file, and no legacy locator
  shares it.
- `--print-world-successor-files` lists markers and shards. The package-seed workflow
  removes them before its exact `diff -r`.
- `content_world_project_repository.rs` admits them by the same rule.

## Commands

```bash
pip install -r requirements.txt -r requirements-dev.txt
python validate_world_metadata.py          # committed families: schema + semantics
python test_world_authoring.py             # synthetic OTBM and wiki fixtures, converters, validator
python convert_city_facts.py --check       # offline, from the committed city snapshot
python convert_hunting_places.py --check   # offline, from the committed TibiaWiki snapshot
python convert_map_regions.py --check      # offline, from the committed official client files
# Regenerate (needs the pinned crystalserver checkout, not fetched by CI):
python convert_world_metadata.py --crystal-root /path/to/crystalserver [--check]
# Refresh the wiki snapshot (network), then reconvert:
python fandom_hunting_snapshot.py fetch && python convert_hunting_places.py
python fandom_city_snapshot.py fetch && python convert_city_facts.py
```
