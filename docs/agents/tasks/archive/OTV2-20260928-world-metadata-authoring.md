# OTV2-20260928-world-metadata-authoring

```yaml
task_id: OTV2-20260928-world-metadata-authoring
title: Populate world metadata families (HuntingPlace Area, teleport Transition)
mode: MIGRATE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/blissful-turing-oca168
issue: 162
pr: 1160
base_sha: 7d1134f
head_sha: null   # a commit cannot hold its own SHA; the exact frozen head is the PR head at merge
owner: owner-launched Claude Code session
created_at: 2026-09-28T00:00:00Z
updated_at: 2026-09-30T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/world-authoring/
  - tools/content-schema/validate_materialized_game_tree.py
  - content/world/areas/hunting-places/
  - imports/tibiawiki/hunting-places/
  - imports/tibiawiki/cities/
  - content/world/transitions/
  - apps/game-server/tests/content_world_project_repository.rs
  - .github/workflows/g4-canonical-worldproject-package-seed.yml
  - .github/workflows/world-metadata-authoring.yml
  - docs/agents/tasks/archive/OTV2-20260928-world-metadata-authoring.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
jira: KAN-16
```

## Outcome

Step 1 of the owner's world-map plan: metadata first, then the full map import after a
measured storage-format decision. Two families move from `READY_UNPOPULATED` to `POPULATED`:

- `Area.HuntingPlace`: 445 records from the English TibiaWiki (Fandom);
- `Transition.Teleport`: 872 records from the pinned CrystalServer map.

Scope superseded by other owners (the code, samples and snapshots here were removed):

- **City and Region** Areas: AREAS-1 (#1353) merged `content/world/areas/{cities,regions}/`
  (22 cities, regions and subregions from the client map areas; keys
  `oteryn:content.area.{city,region,subregion}.<slug>`, built by
  `tools/content-schema/area-authoring/build_areas.py`). This task no longer generates
  `convert_world_metadata.py` cities, `convert_city_facts.py`, `fandom_city_snapshot.py`,
  `imports/tibiawiki/cities/`, `convert_map_regions.py` or `client_map_reader.py`.
- **House** (995 records): HOUSES-5 (#1331) owns `content/houses/`.
- **Terrain and WorldObject**: WO-2 (#1319) owns `content/world/{terrain,objects}/`.

The source is `zimbadev/crystalserver@00ce02a5` (`summer-update`, owner-selected):
`world.otbm`, sha256-pinned, `OtsHypothesisOnly`. `Area.HuntingPlace` comes from
`Category:Hunting Places` of `tibia.fandom.com` (evidence `Derived`, source key
`oteryn:source.tibiawiki`, namespace `tibiawiki-fandom/page-id`; snapshot fetched 2026-09-29:
445 pages with the infobox, 14 without). A record has the page title as name and may have:

- `city`: a reference to an AREAS-1 city key, resolved by exact case-insensitive name against
  `content/world/areas/cities/areas-*.json` (245 records; 200 stay unresolved because only 22
  cities exist; the wiki name stays in `source_facts.city_name`);
- `position`: a single `{{Mapper Coords}}` in `sector.offset` form (321);
- `recommended_levels`: plain integer levels per vocation (386);
- `source_facts.creature_names`: plain wiki names, not Creature keys (439).

Each teleport `object` is the A12 4.6 family key of its item id: the WorldObject key
`oteryn:world-object.tibia.i<id>` (826) or Terrain key `oteryn:terrain.tibia.i<id>` (42) when
the WO-2 catalogue has the id, else the Item key (4), else the converter fails closed. The
validator requires the family and key to exist and the catalogue to win over Item.

Generator-owned shards on disk that a run no longer generates (`<stem>-NNNNN-NNNNN.json` in a
family directory whose index the run writes) are reported `EXTRA` by `--check` and deleted in
write mode; indexes and other files are never touched.

## Architecture and source of truth

- **PROVEN:** `content/world/` is still the legacy WorldProject package root. Family shards
  are admitted only in successor directories that share no directory with a legacy locator
  (`areas/hunting-places/`, `transitions/`); the seed workflow removes them through
  `--print-world-successor-files` before its exact `diff -r`. WO-2 and AREAS-1 catalogue
  shards are removed by their own `rm` steps.
- **PROVEN:** these files are not in `content/manifest.json`, which is generated only from
  the legacy package.
- **PROVEN:** the fresh-source profile pins `ff7ede59` (33 towns); this task uses the
  owner-selected `summer-update` revision and records it in every index.
- Positions use the `global-target-2026-09-27` frame; conversion to native
  `WorldTilePosition` is deferred.
- `world-metadata-authoring.yml` checks out `github.event.pull_request.head.sha` (else
  `github.sha`) and verifies `git rev-parse HEAD` against it before any check.

## Acceptance criteria

- [x] `world-metadata.schema.json` (closed shapes) covers HuntingPlace, Teleport and the index.
- [x] `otbm_reader.py` streams OTBM and fails closed on any unknown item attribute.
- [x] `convert_world_metadata.py` is sha256-pinned, deterministic, has `--check`, and reports
      and deletes extra generated shards; `convert_hunting_places.py` does the same.
- [x] `validate_world_metadata.py` checks schema, shard contiguity, canonical bytes, stray
      files, sorted unique keys, binding targets, hunting-place city references against
      AREAS-1, teleport object family and key against the WO-2 catalogue and Item bindings,
      capture counts and map extent.
- [x] `test_world_authoring.py` covers reader, converters, validator, negatives, the
      teleport family choice and the extra-shard check and delete.
- [x] The legacy package guards accept the successor shards.
- [x] Required checks pass on the PR head (CI green before archiving; merge commit: squash merge of #1160).

## Excluded scope

- Cities, Regions, Houses, Terrain and WorldObjects (other owners, above).
- Placements and floor changes (stairs, ladders, holes); the World record; islands and streets.
- Hunting-place skills, loot, experience ratings, maps and prose; the `15.30/` fragment maps.
- Runtime consumption. `runtime_source` stays `legacy_until_separately_qualified`.

## Validation

- `validate_world_metadata.py`, `convert_hunting_places.py --check` and
  `convert_world_metadata.py --crystal-root ... --check` pass.
- `test_world_authoring.py` passes; `ruff` 0.16.1 check and format pass.
- `validate_materialized_game_tree.py` passes with 13 successor files.

## Independent review

- required: pending owner/control-plane decision
- exact head: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`
