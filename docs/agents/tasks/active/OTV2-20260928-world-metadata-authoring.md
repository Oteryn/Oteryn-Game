# OTV2-20260928-world-metadata-authoring

```yaml
task_id: OTV2-20260928-world-metadata-authoring
title: Populate world metadata families (City, HuntingPlace and Region Area, House, teleport Transition)
mode: MIGRATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/blissful-turing-oca168
issue: 162
pr: 1160
base_sha: 7d1134f
head_sha: null
owner: owner-launched Claude Code session
created_at: 2026-09-28T00:00:00Z
updated_at: 2026-09-28T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/world-authoring/
  - tools/content-schema/validate_materialized_game_tree.py
  - content/houses/
  - content/world/areas/cities/
  - content/world/areas/hunting-places/
  - content/world/areas/regions/
  - imports/tibiawiki/hunting-places/
  - content/world/transitions/
  - apps/game-server/tests/content_world_project_repository.rs
  - .github/workflows/g4-canonical-worldproject-package-seed.yml
  - .github/workflows/world-metadata-authoring.yml
  - docs/agents/tasks/active/OTV2-20260928-world-metadata-authoring.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
jira: KAN-16
```

## Outcome

This is step 1 of the owner's world-map plan: metadata first, then the full map import
(terrain, objects, placements) after a measured storage-format decision, then TibiaWiki
enrichment. Five families move from `READY_UNPOPULATED` to `POPULATED`:

- `Area.City`: 35 records, 29 enriched from the English TibiaWiki (below);
- `Area.HuntingPlace`: 445 records from the English TibiaWiki (Fandom), described below;
- `Area.Region`: 465 records (28 regions, 437 subregions) from the official 15.30 client
  map file, described below;
- `House`: 995 records;
- `Transition.Teleport`: 872 records.

The source is `zimbadev/crystalserver@00ce02a5` (`summer-update`, chosen by the owner):
`world.otbm` and `world-house.xml`. It is sha256-pinned and `OtsHypothesisOnly`.

`Area.HuntingPlace` comes from `Category:Hunting Places` of `tibia.fandom.com` (the
Portuguese TibiaWiki is blocked for builds). `fandom_hunting_snapshot.py fetch` stores, per
page, the page id, revision id, wikitext sha256 and only the raw values of a few factual
`Infobox Hunt` fields in `imports/tibiawiki/hunting-places/fandom-snapshot-v1.json`
(fetched 2026-09-29: 445 pages with the infobox, 14 without). `convert_hunting_places.py`
converts that snapshot offline. Evidence is `Derived`, with source key
`oteryn:source.tibiawiki` and binding namespace `tibiawiki-fandom/page-id`; the binding
records the revision id. A record always has a name (the page title) and may have:

- `city`: only when the wiki city equals one of the 35 City Area names (433 records; 12
  unmatched are omitted; the wiki city name stays in `source_facts.city_name`);
- `position`: only when the `location` field holds exactly one `{{Mapper Coords}}` in
  `sector.offset` form, converted to absolute tiles (321 records; 113 have none and 11
  hold several, which is ambiguous);
- `recommended_levels`: knight, paladin and mage levels that are plain integers (386);
- `source_facts.creature_names`: plain names from every `CreatureList` of the page (439),
  not bound to Creature keys.

`Area.Region` comes from `content/assets/files/map-<sha256>.dat` and its `subarea-*` mask
images (official client, owner-confirmed redistribution, verified against the client asset
manifest). Evidence is `OfficialClient`, source key `oteryn:source.tibia_client`, binding
namespace `tibia-client/map-area-id`. `convert_map_regions.py` converts them offline through
`client_map_reader.py`, which decodes only checked structure and fails closed. A record has an
official name and may have:

- `anchor`: the map.dat position (158 records);
- `parent_regions`: every subregion (437; Tibiadrome has five);
- `footprint`: floor 7 bounding box, tile count and mask image (209 subregions);
- `cities`: City Areas whose temple lies in the subregion mask on the same floor (21 of 35
  cities; a region lists its subregions' cities).

The Thais temple lies in the `Thais City` mask only and the Ab'Dendriel temple in
`Ab'Dendriel City` only. The hierarchy has two levels, so `areas/streets/` stays untouched.
Not imported (counted in the capture summary): the meaning of area fields 6 and 7, markers,
satellite and minimap images, and `staticmapdata-*.dat`.

`Area.City` records gain `source_facts`, optional `implemented` and `npcs`, and a second
binding (`tibiawiki-fandom/page-id`, revision id) from `imports/tibiawiki/cities/`, by the
same pattern (`fandom_city_snapshot.py fetch`, offline `convert_city_facts.py --check`).
29 city pages match by exact title with an `Infobox Geography`; 6 are listed apart with a
reason (3 without a page, 2 hunting-place pages, `Targuna` ambiguous). NPC names come from
`Category:<City> NPCs`; 969 link to NPC keys by exact slug name and 239 stay unmatched.

## Architecture and source of truth

- **PROVEN:** `content/world/**` is still the legacy WorldProject package root. The seed
  workflow and `content_world_project_repository.rs` pinned its exact file inventory.
  - Successor family shards are admitted only in successor directories that share no
    directory with a legacy locator: `areas/cities/` and `transitions/`, not `worlds/`.
  - Legacy locator lookups therefore scan no new entries, and the legacy `TREE_SHA256` and
    directory-scan budget stay unchanged.
  - `content/houses/` is outside the legacy root.
- **PROVEN:** these files are not in `content/manifest.json`, which is generated only from
  the legacy package (`world_project_v2_to_tree.py`). Adding them there would break its
  round-trip validator.
- **PROVEN:** the fresh-source profile (`OTERYN_CRYSTALSERVER_FRESH_SOURCE_GENERATION_PROFILE_V2`)
  pins `ff7ede59`, with 33 towns. This task uses the owner-selected `summer-update`
  revision, with 35 towns, and records it in every index.
- **Positions:** they use the `global-target-2026-09-27` frame, as
  `content/services/travel/` already does. Conversion to native `WorldTilePosition` is
  deferred.

## Acceptance criteria

- [x] `world-metadata.schema.json` (JSON Schema 2020-12, closed shapes) covers all three
      families and their family index.
- [x] `otbm_reader.py` streams OTBM and fails closed on any unknown item attribute.
- [x] `convert_world_metadata.py` is sha256-pinned and deterministic, and has a `--check`
      mode.
- [x] `validate_world_metadata.py` checks schema, shard contiguity, canonical bytes, stray
      files, sorted unique keys, binding targets, house→city and teleport→Item references,
      and map extent.
- [x] `test_world_authoring.py` runs synthetic OTBM fixtures through the reader,
      converter and validator.
- [x] Hunting places: schema, converter (`--check`, offline from the pinned snapshot),
      validator (city refs, snapshot pin, page/revision bindings, capture counts, extent) and
      fixture tests including negatives.
- [x] Regions: schema, reader, converter (`--check`, offline from the committed client
      files), validator (source and mask pins, hierarchy, city links, extent, counts) and
      fixture tests including negatives.
- [x] Cities: snapshot tool, converter (`--check`), schema shapes, validator (snapshot pin,
      bindings, NPC keys, counts) and fixture tests with negatives; the workflow runs them.
- [x] The legacy package guards accept the successor shards: the materialized-tree
      validator, the seed workflow and the Rust inventory test.
- [ ] Required checks pass on the frozen PR head.

## Excluded scope

- Terrain, map objects, placements and floor changes (stairs, ladders, holes).
- The World record (bounds and floors).
- Islands and streets (the client map has no street tier). Hunting-place skills, loot, experience ratings, maps and prose.
- The `15.30/` fragment maps.
- TibiaWiki enrichment.
- Runtime consumption. `runtime_source` stays `legacy_until_separately_qualified`.

## Validation

- `python validate_world_metadata.py` passes.
- `python test_world_authoring.py` passes 55 tests.
- `convert_map_regions.py --check` passes offline against the committed client files.
- `convert_hunting_places.py --check` passes offline against the committed snapshot.
- `ruff check` and `ruff format --check` pass (ruff 0.16.1).
- `convert_world_metadata.py --check` against the pinned checkout passes.
- `python3 tools/content-schema/validate_materialized_game_tree.py` passes with 15
  successor files.

## Independent review

- required: pending owner/control-plane decision
- exact head: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`
