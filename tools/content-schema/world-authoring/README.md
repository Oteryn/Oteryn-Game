# World metadata authoring (HuntingPlace Area, teleport Transition)

This package is the first population of the world tree. It covers **metadata** only: teleports
and hunting places from the English TibiaWiki. Placements (19.3 M tiles, 24.9 M items) are
a later step. They need a physical storage format selected by measurement, and JSON is not
that format. Other world families have their own owners: Cities and Regions are the Area
catalogue (`content/world/areas/{cities,regions}/`, `area-authoring`, AREAS-1), Terrain and
WorldObjects are the WO-2 catalogue (`content/world/{terrain,objects}/`) and Houses are the
House catalogue in `content/houses/` (`OTERYN_HOUSE_CATALOGUE_OWNER_CONTRACT_V1`).

| Family | Path | Records | Shard schema |
|---|---|---:|---|
| `Area.HuntingPlace` | `content/world/areas/hunting-places/` | 445 | `OTERYN_AREA_AUTHORING_SHARD/v1` |
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
`areas/hunting-places/` and `transitions/` are not
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
python convert_hunting_places.py --check   # offline, from the committed TibiaWiki snapshot
# `--check` also reports (`EXTRA`) generator-owned shards no longer generated; write mode
# deletes them. Regenerate (needs the pinned crystalserver checkout, not fetched by CI):
python convert_world_metadata.py --crystal-root /path/to/crystalserver [--check]
# Refresh the wiki snapshot (network), then reconvert:
python fandom_hunting_snapshot.py fetch && python convert_hunting_places.py
```
