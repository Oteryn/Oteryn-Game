# OTV2-20260928-world-base-map

```yaml
task_id: OTV2-20260928-world-base-map
title: Populate the WorldPlacement.Base full map (tiles and items) from CrystalServer as B3 region files
mode: MIGRATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/blissful-turing-oca168-world-map
issue: 162
pr: 1170
base_sha: 4e65a5e4
head_sha: null
owner: owner-launched Claude Code session
created_at: 2026-09-28T00:00:00Z
updated_at: 2026-09-28T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/world-authoring/
  - content/world/placements/
  - content/world/worlds/
  - content/world/objects/
  - content/world/areas/islands/
  - imports/tibiawiki/islands/
  - tools/content-schema/validate_materialized_game_tree.py
  - .gitattributes
  - .github/workflows/world-metadata-authoring.yml
  - apps/game-server/tests/content_world_project_repository.rs
  - docs/agents/tasks/active/OTV2-20260928-world-base-map.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
jira: KAN-16
```

## Outcome

Step 3 of the owner's world-map plan: the whole base map of `world.otbm` in
`zimbadev/crystalserver@00ce02a5` (`summer-update`) becomes `WorldPlacement.Base`:
19,325,129 tiles and 24,925,845 items on floors 0-15, in 1,208 region files
(`OTERYN_WORLD_REGION_B3/v1`, about 21.6 MB) plus a 2.5 MB `index.json`. The source is
sha256-pinned and `OtsHypothesisOnly`.

## Architecture and source of truth

- **Palette indirection (owner decision):** region files store a palette index per item.
  `index.json` holds the palette, one entry per distinct map server id in ascending id
  order: `{key, source_item_id, provisional}`.
  - A bound id (`imports/crystalserver/bindings/items.json`, `ots/item_server_id`) takes its
    binding target key, registry or named, with `provisional: false`.
  - Any other id takes `donor:crystalserver@00ce02a5:item/<id>` with `provisional: true`
    (5,943 appearance-only ids and 44 items that `items.xml` declares).
  - Item identity work later rewrites palette entries only. Region files do not change.
- **Format:** measured before selection (about 22 MB versus 3.25 GB as JSON sectors). Spec
  in the `world_region_codec.py` docstring.
- **Coexistence:** `content/world/placements/` held only the `READY_UNPOPULATED` marker. The
  legacy WorldProject seed reproduction removes the successor files before its exact diff.
- **Not authoritative for runtime:** the base is migration evidence. Runtime loading, the
  per-channel overlay and Oteryn edits are separate work.

## Acceptance criteria

- [x] `world_region_codec.py` encodes and decodes region files deterministically and rejects
      damaged input.
- [x] `convert_world_base.py` is sha256-pinned, deterministic, has a `--check` mode, and
      fails closed on unknown OTBM attributes and node types.
- [x] The palette follows the owner rule, and the capture summary records its size and the
      provisional entries and occurrences, split by presence in `items.xml`.
- [x] `validate_world_base.py` checks per-region sha256, decode, sorted tiles, bounds,
      counts, palette rules, palette use and summary totals.
- [x] `test_world_base.py` covers codec round trips, palette variants and validator
      negatives.
- [x] The world-metadata workflow validates and tests the base family.
- [x] The `World` family holds one record (`oteryn:world.oteryn`) with the exact tile bounds
      of the base map (half-open, `x` 1340-34263, `y` 1643-33812), the source header extent
      and floors 0-15, generated offline by `convert_world_record.py --check`. The one
      explicit exception to "no family beside a legacy locator" is `worlds/` (index plus
      exactly one shard); the repository test scan budget grows by exactly that entry.
      `validate_world_record.py` checks every placement extent, City, House, teleport and
      hunting place position against the bounds and floors.
- [x] `WorldObject.FloorChange` holds one record per items.xml item type with a
      `floorchange` attribute (447 types: 194 `down`, 73 `up_north`, 52 `up_south`,
      50 `up_east`, 69 `up_west`, 5 `up_south_alt`, 4 `up_east_alt`), keyed from the palette
      item key, with the engine mapping documented and `occurrences_on_base_map` counted
      from the region files (329 types, 26,919 occurrences). Ladders up, rope spots, sewer
      grates and tool holes are scripted uses and are listed as excluded, not invented.
- [x] `Area.Island` holds only islands the base map confirms (owner rule): 52 records
      (48 island, 3 archipelago, 1 continent; 2 event-only) computed by
      `convert_islands.py --check` from the committed region files, the pinned TibiaWiki
      snapshot (`imports/tibiawiki/islands/fandom-snapshot-v1.json`, 67 candidate pages) and
      `island-ground-classes.json` (water and lava ids from the pinned `items.xml` names).
      Each record has a map-computed footprint and anchor, the cities whose temple lies in
      the component, page-id bindings and the wiki evidence sentence. Percht Island merges
      into Orcsoberfest Island; Fibula uses the map-corrected 9,196 tile island
      (`anchor_corrected_from_wiki`). 14 candidates are excluded with a reason
      (9 without coordinates, 3 part of the landmass, 2 event-only not on the map) in
      `samples/islands-capture-v1.json`. `validate_islands.py` and `test_islands.py` pass.
- [ ] Required checks pass on the frozen PR head.

## Excluded scope

- Admitting provisional items (item B1b) and a Terrain identity path.
- A patch layer for authored Oteryn edits over a regenerated base.
- Runtime consumption, the per-channel overlay and the native `WorldTilePosition` mapping.
- The `15.30/` fragment maps.

## Validation

- `convert_world_base.py --check` against the pinned checkout is byte-identical.
- `convert_world_record.py --check`, `validate_world_record.py` and `test_world_record.py`
  pass.
- `convert_floor_changes.py --check` (pinned checkout), `validate_floor_changes.py` and
  `test_floor_changes.py` pass.
- `convert_islands.py --check`, `validate_islands.py` and `test_islands.py` pass.
- `validate_world_base.py` passes; `test_world_base.py` and `test_world_authoring.py` pass.
- `ruff check` and `ruff format --check` pass from the repository root.
- `validate_materialized_game_tree.py`, `validate_governance.py` and
  `validate_repository_policy.py` pass.
- `cargo test --locked -p oteryn-game-server --test content_world_project_repository` and
  `cargo fmt --all -- --check` pass; the legacy seed reproduction diff is empty.

## Independent review

- required: pending owner/control-plane decision
- exact head: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`
