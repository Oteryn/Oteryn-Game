# World metadata authoring (City and HuntingPlace Area, House, teleport Transition)

This package is the first population of the world tree. It covers **metadata** only: towns,
houses and teleports, plus hunting places from the English TibiaWiki. Terrain, map objects and placements (19.3 M tiles, 24.9 M items) are
a later step. They need a physical storage format selected by measurement, and JSON is not
that format.

| Family | Path | Records | Shard schema |
|---|---|---:|---|
| `Area.City` | `content/world/areas/cities/` | 35 | `OTERYN_AREA_AUTHORING_SHARD/v1` |
| `Area.HuntingPlace` | `content/world/areas/hunting-places/` | 445 | `OTERYN_AREA_AUTHORING_SHARD/v1` |
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
  - the World record (bounds and floors). Its `worlds/` directory shares space with a
    legacy locator.
  - floor changes through stairs, ladders or holes. These are derived from item types
    together with terrain.
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

## Commands

```bash
pip install -r requirements.txt -r requirements-dev.txt
python validate_world_metadata.py          # committed families: schema + semantics
python test_world_authoring.py             # synthetic OTBM and wiki fixtures, converters, validator
python convert_hunting_places.py --check   # offline, from the committed TibiaWiki snapshot
# Regenerate (needs the pinned crystalserver checkout, not fetched by CI):
python convert_world_metadata.py --crystal-root /path/to/crystalserver [--check]
# Refresh the wiki snapshot (network), then reconvert:
python fandom_hunting_snapshot.py fetch && python convert_hunting_places.py
```
