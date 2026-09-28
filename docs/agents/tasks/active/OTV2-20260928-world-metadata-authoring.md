# OTV2-20260928-world-metadata-authoring

```yaml
task_id: OTV2-20260928-world-metadata-authoring
title: Populate world metadata families (City Area, House, teleport Transition) from CrystalServer
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
enrichment. Three families move from `READY_UNPOPULATED` to `POPULATED`:

- `Area.City`: 35 records;
- `House`: 995 records;
- `Transition.Teleport`: 872 records.

The source is `zimbadev/crystalserver@00ce02a5` (`summer-update`, chosen by the owner):
`world.otbm` and `world-house.xml`. It is sha256-pinned and `OtsHypothesisOnly`.

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
- [x] The legacy package guards accept the successor shards: the materialized-tree
      validator, the seed workflow and the Rust inventory test.
- [ ] Required checks pass on the frozen PR head.

## Excluded scope

- Terrain, map objects, placements and floor changes (stairs, ladders, holes).
- The World record (bounds and floors).
- Hunting places, islands and streets.
- The `15.30/` fragment maps.
- TibiaWiki enrichment.
- Runtime consumption. `runtime_source` stays `legacy_until_separately_qualified`.

## Validation

- `python validate_world_metadata.py` passes.
- `python test_world_authoring.py` passes 11 tests.
- `ruff check` and `ruff format --check` pass (ruff 0.16.1).
- `convert_world_metadata.py --check` against the pinned checkout passes.
- `python3 tools/content-schema/validate_materialized_game_tree.py` passes with 13
  successor files.

## Independent review

- required: pending owner/control-plane decision
- exact head: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`
