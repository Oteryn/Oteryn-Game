# OTV2-20260928-wo1-world-object-terrain-tooling

```yaml
task_id: OTV2-20260928-wo1-world-object-terrain-tooling
title: WO-1 WorldObject and Terrain schema, validator and census
mode: MIGRATE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw
issue: 162
pr: 1169
base_sha: df3d21a1
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-launched Claude Code session (Item authoring lane)
created_at: 2026-09-28T20:30:00Z
updated_at: 2026-09-28T21:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/world-object-authoring/**
  - docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md
  - .github/workflows/item-authoring-schema.yml
  - docs/agents/tasks/archive/OTV2-20260928-wo1-world-object-terrain-tooling.md
public_contracts: []
depends_on:
  - "docs/architecture/reviews/OTERYN_GAME_WO0_WORLD_OBJECT_AND_TERRAIN_AUTHORING_FORMAT_DECISION_2026-09-28.md"
  - "tools/content-schema/item-authoring/engine_items.py"
blocks:
  - "WO-2 population and identity minting"
cross_repository_coordination_id: null
external_repositories:
  - "zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a (pinned facts, read-only)"
jira: KAN-16
```

## Allocation

The claim is in #162 comment 5877947780. It follows 5876526764 ("WO-1 is allocated to the Item authoring lane as soon
as WO-0 is accepted") and 5877580229 ("Now allocatable: WO-1").

The scope note in 5877980827 drops the Rust `ProjectReferenceRecord::Item` amendment and `apps/game-server/**` from
WO-1:
- `DefinitionFamily` has no `WorldObject` variant;
- nothing in `content/**` carries `routed_to` yet;
- the amendment lands in WO-2 with its first consumer.

## Outcome

New package `tools/content-schema/world-object-authoring/`:

- **`terrain.schema.json` and `world-object.schema.json`.** One static record each, per WO-0 §4.2 and §4.3, using the
  KNOWN/UNKNOWN wrapper. Kind-specific `ground_speed` and `field` (Terrain) and `bed`, `corpse` and `door`
  (WorldObject) sections appear only on their kind. Every KNOWN fact names its source field.
- **`routed-item-pointer.schema.json`.** The WO-0 §4.1 typed `routed_to {family, key, revision}`. A bare key, an
  unknown family, a family and key mismatch, a shifted sequence number and a materializable Item are all rejected.
  Formal schema §5m records it.
- **`world_objects.py`.** It provides:
  - the D93 key rule as a pure function of the frozen CW2-B1 Item key;
  - the D94 exclusions;
  - record builders over `engine_items.convert_item`'s unchanged routing;
  - `--validate`, the census, and `--check`.

  It mints no identity and writes nothing under `content/`.
- **`test_world_objects.py`.** 140 checks, no network, as a CI lane step. Ruff also covers the package.
- **`samples/census-crystal-ff7ede5.json`.** 25,846 routed ids, of which 4,476 are excluded by D94, leaving **21,370
  records** (8,581 Terrain, 12,789 WorldObject). There are no key collisions, and the records have a
  reproducible digest.

## Architecture and source of truth

- **PROVEN:**
  - The route counts equal `population-crystal-ff7ede5.json` exactly.
  - The records count equals D94's "about 21.4k".
  - Every record validates, and so does every routed Item pointer.
- **Facts:**
  - Appearance booleans are complete for ids with an appearance: an unset optional bool is false, an unset hook is
    `none` and an unset height is `0`.
  - `items.xml` attributes are sparse, so an absent attribute is UNKNOWN, never an engine default.
  - Relations stay `source_item_id` until WO-2.
  - Corpse decay and capacity stay on the Item record.
- **UNKNOWN, carried to WO-2 as WO-0 "contested routes":**
  - 330 Terrain tiles have no ground, border or wall flag (roofs, some floors and shallow water).
  - 21 WorldObject ids are `magicfield`.
  - 135 Terrain ids have a `type` outside the family: 106 trashholder, 18 carpet, 11 teleport.

## Validation

- `python test_world_objects.py`: PASS, 140 checks.
- `python world_objects.py --source <crystal ff7ede5> --check`: PASS, 21,370 records.
- `ruff check` and `ruff format --check` (0.16.1) over the package: PASS.
- `python tools/agents/validate_governance.py`, `python tools/repository/validate_repository_policy.py` and
  `git diff --check`: see the PR.

## Handoff

- **WO-2 (Content/World, with independent identity review):**
  - populate `content/world/terrain/` and `content/world/objects/` from these builders;
  - add the Item `routed_to` pointers;
  - add `DefinitionFamily::WorldObject` and `routed_to` on `ProjectReferenceRecord::Item`;
  - resolve `source_item_id` relations and the UNKNOWN kinds above;
  - the donor part follows B1b.
- **WO-3:** the LocalObject `presentation` reference.
