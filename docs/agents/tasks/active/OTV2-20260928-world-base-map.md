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
  - content/world/terrain/
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
  - An appearance-only id (declared by the official client, not by `items.xml`) takes the
    `oteryn:terrain.a<id>` key of its `Terrain` record with `provisional: false` (5,942 ids).
  - Any other id takes `donor:crystalserver@00ce02a5:item/<id>` with `provisional: true`
    (4 items that `items.xml` declares and the appearance-less id 99).
  - **Ownership rule: ids present in `items.xml` -> Item registry (item agent, B1b);
    appearance-only ids -> Terrain (world).**
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
- [x] `Area.Island` holds only islands the base map confirms (owner rule): 59 records
      (55 island, 3 archipelago, 1 continent; 2 event-only; 1 underground) computed by
      `convert_islands.py --check` from the committed region files, the pinned TibiaWiki
      snapshot (`imports/tibiawiki/islands/fandom-snapshot-v1.json`, 67 candidate pages),
      `island-ground-classes.json` (water and lava ids from the pinned `items.xml` names) and
      `island-evidence-anchors.json` (owner-approved start tiles from pinned CrystalServer
      NPC/monster spawns or a committed teleport destination for pages without a usable wiki
      coordinate: Tutorial Island, Isle of Evil, Rascacoon, Ingol, Oskayaat, Isle of the
      Mists, Robson's Isle (floor 14, `underground`) and the Newhaven main island; each
      record keeps `anchor_source`). Each record has a map-computed footprint and anchor, the
      cities whose temple lies in the component, page-id bindings and the wiki evidence
      sentence. Percht Island merges into Orcsoberfest Island; Fibula uses the map-corrected
      9,196 tile island (`anchor_corrected_from_wiki`); Newhaven is one island of two
      components (9,189 tile main island, 50 tile temple islet in `additional_components`;
      no committed teleport links them). 7 candidates are excluded with a reason
      (3 without coordinates, 2 part of the landmass, 2 event-only not on the map) in
      `samples/islands-capture-v1.json`. `validate_islands.py` and `test_islands.py` pass.
- [x] `Terrain` holds 5,942 records (`oteryn:terrain.a<id>`), one per appearance-only palette
      id that the official 15.30 client `appearances-2dfa943b….dat` declares (owner-confirmed
      redistribution), from `convert_terrain.py --check` and `client_appearance_reader.py`.
      Ownership rule: ids present in `items.xml` -> Item registry (item agent, B1b);
      appearance-only ids -> Terrain (world). Fields come only from the appearance: `class`
      by the documented rule (ground 740, border 1,533, blocking 2,611, decoration 1,058),
      `flags`, `speed`, `name`, `automap_color` and `occurrences_on_base_map`. Client ids
      equal server ids (checked against `items.xml` names and ground evidence). The base map
      palette switched those 5,942 keys from `donor:` to Terrain keys (region files
      byte-identical; 5 provisional remain: 4 `items.xml` ids for B1b and id 99). Validators
      `validate_terrain.py` and `validate_world_base.py` (a non-provisional key is an Item
      binding target or a Terrain key of that id) and `test_terrain.py` pass.
- [ ] Required checks pass on the frozen PR head.

## Excluded scope

- Admitting provisional items (item B1b).
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
- `convert_terrain.py --check`, `validate_terrain.py` and `test_terrain.py` pass.
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
