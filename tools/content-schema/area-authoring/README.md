# Area authoring schema candidate v1

Static Area definitions for `content/world/areas/` (owner `Area` in
[`OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md`](../../../docs/architecture/OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md)).
CANDIDATE schema: not a WorldProject/v2 contract and not runtime activation. The city
Areas are step 3 of the [House catalogue owner contract](../../../docs/architecture/OTERYN_HOUSE_CATALOGUE_OWNER_CONTRACT_V1.md)
§5: every House `town` reference resolves to one of them.

## Catalogue

`build_areas.py build` turns every 15.30 client map area
(`imports/cipsoft-staticdata/map/areas/`, 465 records) into one record:

| Kind | Client | Records | Folder | Key |
|---|---|---|---|---|
| `region` | flag 1, lists subareas | 28 | `regions/` | `oteryn:content.area.region.<slug of name>` |
| `subregion` | flag 2, listed by one region | 415 | `regions/` | `oteryn:content.area.subregion.<slug of name>` |
| `city` | the flag 2 areas in `CITY_AREAS` | 22 | `cities/` | `oteryn:content.area.city.<slug of town>` |

- A city is a House town (19; the client area of the same name or of the name plus
  " City", checked in the build) or a hometown (Rookgaard, Roshamuul, and Dawnport as
  "Dawnport Centre", whose client position is next to the Dawnport temple). Its key is
  the town slug the House catalogue already uses (owner decision 2a, 2026-09-30).
- `parent` is the region that lists the area. `position` is the client area position
  (158 areas). Area shapes (the client overlay images) are not modelled.
- Keys follow the House catalogue identity rules: allocated once, then kept by
  `provenance.source_id` on every rebuild, never reused.
- `islands/`, `hunting-places/` and `streets/` stay unpopulated: the client has no such
  classification.

## Hometowns and temples

The 19 hometowns are TibiaWiki `Template:Hometowns`. The client does not ship temples,
so a hometown temple is the CrystalServer engine town temple (`world.otbm` towns,
`samples/crystal-world-towns-00ce02a5.json`), cross-checked against the nearest TibiaWiki
Cleric or Healer NPC (`samples/tibiawiki-hometowns-2026-09-30.json`); the evidence is
kept in `provenance.hometown` and `samples/area-report.json`.

- 18 of 19 temples have the temple NPC 0-5 tiles away on the same floor.
- Ankrahmun: the engine has z 8; the owner checked in game (2026-09-30) that the temple
  is on client level 0, so the temple is z 7 (`OWNER_CHECKS`). Rahkem is 5 tiles away.
- Farmine: the engine has z 11 (client level -4). The nearest NPC, Prezil, is recorded
  on z 15 by its page but on z 11 two tiles away by the wiki `Temple` table; not yet
  checked in game.
- The 16 engine towns that are not hometowns (for example `Home`, `Targuna`, `Krailos`,
  `Moonfall`) are listed in the report and not used.

## Tools

| File | Purpose |
|---|---|
| `area.schema.json` | structural schema |
| `validate_areas.py` | semantic checks: key per kind, unique key / source id / name, parent is a region of the catalogue, hometown only on a city with evidence, a temple that differs from the engine only with an owner check |
| `build_areas.py` | `extract-crystal-towns --otbm` (local, pinned `world.otbm`), `fetch-wiki` (local), `build [--check]` (catalogue and report, byte for byte) |
| `test_validate_areas.py` | positive and negative cases on the built catalogue |

```sh
python build_areas.py build --check
python test_validate_areas.py
python build_areas.py extract-crystal-towns --otbm <crystal>/data-global/world/world.otbm --check
```
