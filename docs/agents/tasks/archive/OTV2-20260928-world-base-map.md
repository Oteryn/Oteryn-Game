# OTV2-20260928-world-base-map

```yaml
task_id: OTV2-20260928-world-base-map
title: Populate the WorldPlacement.Base full map (tiles and items) from CrystalServer as B3 region files
mode: MIGRATE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/blissful-turing-oca168-world-map
issue: 162
pr: 1170
base_sha: 4e65a5e4
head_sha: null
owner: owner-launched Claude Code session
created_at: 2026-09-28T00:00:00Z
updated_at: 2026-09-30T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/world-authoring/
  - content/world/placements/
  - content/world/worlds/
  - content/world/areas/islands/
  - imports/tibiawiki/islands/
  - tools/content-schema/validate_materialized_game_tree.py
  - .gitattributes
  - .github/workflows/world-metadata-authoring.yml
  - apps/game-server/tests/content_world_project_repository.rs
  - docs/agents/tasks/archive/OTV2-20260928-world-base-map.md
  - docs/agents/reports/OTV2-20260929-world-map-handover.md
  - docs/agents/HANDOVER_LIFECYCLE.json
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
19,325,129 tiles and 24,925,845 items on floors 0-15 (plus 19,893 `maps.7z` fill, 5,519 Edron and 22,978 draft tiles),
in 1,208 region files
(`OTERYN_WORLD_REGION_B3/v1`, about 21.6 MB) plus a 2.5 MB `index.json`. The source is
sha256-pinned and `OtsHypothesisOnly`.

## Architecture and source of truth

- **Palette indirection (owner decision):** region files store a palette index per item.
  `index.json` holds the palette, one entry per distinct map server id in ascending id
  order: `{key, source_item_id, provisional}`.
  - A bound id (`imports/crystalserver/bindings/items.json`, `ots/item_server_id`) takes its
    binding target key, registry or named, with `provisional: false`.
  - A12 section 4.6 alignment (WO-2 #1319 @907ed0f0, owner order: #1319 merges first, then this PR
    regenerates): order Terrain catalogue key `oteryn:terrain.tibia.i<id>` (7,576 palette
    entries), else WorldObject catalogue key `oteryn:world-object.tibia.i<id>` (7,699), else
    the Item binding target with an Item record (4,714), else the provisional donor key.
  - Any other id takes `donor:crystalserver@00ce02a5:item/<id>` with `provisional: true`:
    5,995 entries (5,949 appearance-only ids with a client appearance, id 99, 45 `items.xml`
    ids of which 40 are bound to an undefined Item key).
  - This PR writes neither `content/world/terrain/` nor `content/world/objects/`: they are
    the WO-2 catalogues; the earlier own `Terrain` and `WorldObject.FloorChange` families are removed.
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
      `validate_world_record.py` checks every placement extent, City, teleport and
      hunting place position against the bounds and floors.
- [x] Floor changes have no family here: the WO-2 catalogues carry the `floorchange` value as the
      `floor_change` fact (444 of 447 item types: 158 Terrain, 286 WorldObject; 166, 167 and 53431
      stay Items and are listed in `edron_rework.py`). The base capture summary keeps the counts
      (`floor_changes`: 331 types on the map, 28,160 occurrences). Ladders up, rope spots, sewer
      grates and tool holes are scripted uses and are not invented.
- [x] `Area.Island` holds only islands the base map confirms (owner rule): 60 records
      (56 island, 3 archipelago, 1 continent; 3 event-only; 1 underground) computed by
      `convert_islands.py --check` from the committed region files, the pinned TibiaWiki
      snapshot (`imports/tibiawiki/islands/fandom-snapshot-v1.json`),
      `island-ground-classes.json` and the owner-approved `island-evidence-anchors.json`
      (each record keeps `anchor_source`). Each record has a map-computed footprint and anchor,
      the cities whose temple lies in the component, page-id bindings and the wiki evidence
      sentence. Percht Island merges into Orcsoberfest Island; Fibula uses the map-corrected
      island; Newhaven has two components. 7 candidates are excluded with a reason in
      `samples/islands-capture-v1.json`. `validate_islands.py` and `test_islands.py` pass.
- [x] Blue Valley is partially filled from `maps.7z:blue_valley.otbm` (fill-only: a tile is
      added only where the base map has no tile at that position, nothing existing is
      overwritten or merged): 2,965 tiles and 3,364 items added (floor 2 118, floor 3 219,
      floor 4 685, floor 5 918, floor 6 1,025; floor 7 none from the fill), pinned by the
      sha256 of the archive and member, counts in the capture summary `fill`, reproduced by
      `convert_world_base.py --check` (needs `py7zr`, `requirements-regenerate.txt`).
- [x] `maps.7z:summer-update-2025.otbm` is partially filled (owner decision 2b, fill-only,
      pinned, `--check`): the converter takes fragment tiles the base lacks, floors 8-15 by
      4-connected component except the Edron underground (box x33274-33456, y31786-31884,
      floors 8-12; reworked by the next item), floors 0-7 only where the official 15.30
      minimap shows land. 16,928 tiles, 19,667 items (floors 2-6 151, floor 8 1,913, 9 5,678,
      10 584, 11 611, 12 169, 13 3,608, 14 3,619, 15 595), 6 provisional palette entries.
- [x] Blue Valley floor 7 (owner decision 1a): the `replace` rule of the same pin swaps a
      base tile for the fragment tile only where the base ground is water, the fragment
      ground is land and the 15.30 minimap ZZ07 shows land: 905 tiles (896 inside the Blue
      Valley box), capture summary `replace`, validated and tested; the island footprint
      grows to 10,427 tiles. Nothing else is replaced.
      The remaining gaps are drafted by the minimap draft below.
- [x] Minimap draft of every remaining gap (owner decision 2a, then approval for all gaps;
      rough draft, reference-derived): `minimap_draft.py` holds seven named `AREAS` (Temple of
      Light, Great Expedition Island and Wharf, Blue Valley, Marapur/Thalassara, Nargor, Upper
      Roshamuul) with committed bbox and floors, drafted from the pinned tibiamaps floor 2-7
      images (shape and walkability, generic ground, no borders, decorations, doors or
      furniture). Per-floor learned colour table (130 mapped, 18 unmapped rows); floors 0-7 need
      official minimap land; only missing or plain-water tiles, fill tiles count as base.
      22,978 tiles added, 18,520 replaced (counts per floor in README and summary); none under
      the 5% skip limit. 18 yellow markers are `unresolved_entrances`. Blue Valley island
      15,612 tiles. Temple of Light is event-only because the wiki marks it an event place, not
      the map: converter right. The drawing list is drafted and needs detail work.
- [x] Edron underground (owner decision 1a, reference-derived): floors 9 and 10 of the box
      are imported from the summer file and repaired with the player-recorded real-Tibia
      minimap (tibiamaps/tibia-map-data, sha256-pinned in `source.edron`, read from
      `--tibiamaps-root`; `edron_rework.py`). Floor 10: 3,982 filled, 699 replaced; floors 9
      and 10 repaired; 15 markers, 10 `unresolved_entrances`, none invented. Totals 5,519
      tiles added, 706 replaced. `test_edron_rework.py` and the validator pass.
- [x] Appearance-only ids (declared by the official 15.30 client `appearances-2dfa943b....dat`, not by
      `items.xml`) stay provisional, since no A12 family covers them. `samples/appearance-only-ids-v1.json`
      lists 5,949 of them with class and occurrences as evidence for the WO lane
      (`convert_appearance_only_ids.py --check`). The palette regenerated on the accepted
      keys with region files byte-identical; `validate_world_base.py` requires every
      non-provisional key to exist in its family.
- [x] Required checks pass on the frozen PR head; merge commit: squash merge of #1170.

## Excluded scope

- Admitting provisional items (item B1b).
- A patch layer for authored Oteryn edits over a regenerated base.
- Runtime consumption, the per-channel overlay and the native `WorldTilePosition` mapping.
- `access.otbm`, `asura_resp.otbm`, `boss_rooms_-_part_2.otbm` and `final.otbm` of
  `data-global/world/15.30/`: unreferenced local-coordinate drafts (17,801 tiles, absent from
  the base, overlapping the off-by-default Movement Trainer of `custom/global-custom.otbm`).
  Owner decision: not imported, deferred.
- The other `maps.7z` members (`newheaven`, `winter-update-2025`: one tile each) and the six
  `15.30/` fragment maps already in `world.otbm` (0 missing tiles; the base wins). Known
  gap: the General Murius raid spawn (32427,31131,15) has no tile in the base.

## Validation

- `convert_world_base.py --check` against the pinned checkout is byte-identical.
- `convert_world_record.py --check`, `validate_world_record.py` and `test_world_record.py`
  pass.
- `convert_islands.py --check`, `validate_islands.py` and `test_islands.py` pass.
- `convert_appearance_only_ids.py --check` (pinned checkout) passes.
- `tools/content-census/item_key_references.py` fails on `main` itself (991 dangling keys in
  `imports/tibiawiki/facts/items-stats.json`); this branch adds none.
- `validate_world_base.py` passes; `test_world_base.py` and `test_world_authoring.py` pass.
- `ruff check` and `ruff format --check` pass from the repository root.
- `validate_materialized_game_tree.py`, `validate_governance.py` and
  `validate_repository_policy.py` pass.
- `cargo test --locked -p oteryn-game-server --test content_world_project_repository` and
  `cargo fmt --all -- --check` pass; the legacy seed reproduction diff is empty.

## Handover

Owner decisions 1a (Blue Valley floor 7; Edron underground), 2a (Temple of Light draft) and 2b (summer-update-2025),
research results, unresolved Edron entrances and next steps: `docs/agents/reports/OTV2-20260929-world-map-handover.md`.

## Independent review

- required: pending owner/control-plane decision
- exact head: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`
