# OTV2-20260930-wo2a-world-object-terrain-catalogue

```yaml
task_id: OTV2-20260930-wo2a-world-object-terrain-catalogue
title: WO-2a Terrain and WorldObject catalogues in content/world/{terrain,objects}
mode: MIGRATE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw
issue: 162
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: eba9a271
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-directed Claude Code session (WO-1 author), claim #162 comment 5906840143
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - content/world/terrain/**
  - content/world/objects/**
  - tools/content-schema/world-object-authoring/**
  - tools/content-schema/validate_materialized_game_tree.py
  - apps/game-server/tests/content_world_project_repository.rs
  - .github/workflows/item-authoring-schema.yml   # owner authorization required
  - .github/workflows/g4-canonical-worldproject-package-seed.yml   # owner authorization required
  - docs/agents/tasks/archive/OTV2-20260930-wo2a-world-object-terrain-catalogue.md
public_contracts: []
depends_on:
  - "docs/architecture/reviews/OTERYN_GAME_WO0_WORLD_OBJECT_AND_TERRAIN_AUTHORING_FORMAT_DECISION_2026-09-28.md (D93, D94)"
  - "A12 §4.6 family keys (#1305)"
blocks:
  - "WO-2b: Rust WorldObject/Terrain family, Item routed_to pointers, typed relation references"
  - "WO-3: LocalObject presentation reference"
external_repositories:
  - "zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a (pinned facts, read-only)"
jira: null   # sync pending (coordinator batch)
```

## Split

WO-2 in the WO-0 handoff covers the catalogue, the Rust family, the Item `routed_to` pointers and typed relation
references. The Rust part (a new definition family in the WorldProject model, fail-closed pointer validation,
materializer and tree tool) is well over the batch budget together with the catalogue, so it is split:

| Part | Scope | State |
|---|---|---|
| WO-2a | The catalogues: 21,330 records in 44 shards, directory markers `POPULATED`, generator with `--check`, CI drift check | this record |
| WO-2b | `DefinitionFamily::WorldObject`/Terrain records in the Rust model, `routed_to` on the 21,330 routed Item records, typed relation references, donor routed ids (138, B1b) | needs its own allocation |

## Outcome

- DONE: `build_catalogue.py` writes 8,545 Terrain and 12,785 WorldObject records, keyed `oteryn:terrain.tibia.i<id>`
  and `oteryn:world-object.tibia.i<id>` (A12 §4.6). Each record keeps `provenance.item_pointer` to its Item key.
- DONE: `--check` rebuilds all 46 files and the census sample from the pinned Crystal files and fails on any
  difference, missing or extra shard.
- DERIVED: the committed census sample was stale after #1305 (21,370 -> 21,330 records: D149 removed the ids
  without a CipSoft appearance, and the converter gained the `non_pickupable_blocking_prop` route). It is rebuilt
  from the same run as the catalogues, so the two always agree.
- DONE: the tree validator and the WorldProject inventory test accept `POPULATED` for exactly these two markers and
  keep the shards out of the legacy package inventory. The canonical package comparison stays exact (checked
  locally: materializer output equals the tracked package once markers and shards are removed).

## Assumptions (reversible)

- Relations (`rotate_to`, bed parts, sleepers, transforms) stay `{"source_item_id": n}`. Under A12 the source id is
  the Tibia id, so each target is determined; the typed reference arrives with the Rust family in WO-2b.
- Contested routes stay in the catalogue with `kind` UNKNOWN (never guessed), pending owner decision: 299 Terrain
  tiles without a ground, border or wall flag; 21 WorldObject magic fields; 132 Terrain ids whose `type` belongs to
  another family (103 trashholder, 18 carpet, 11 teleport).

## Validation (local)

- `python build_catalogue.py --source <crystal ff7ede5> --check`: PASS (46 files).
- `python test_world_objects.py`: PASS (21,573 checks, incl. the new catalogue and committed-catalogue tests).
- `validate_materialized_game_tree.py`: PASS. `test_world_project_v2_to_tree.py`, `validate_world_project_v2_to_tree.py`: PASS.
- `cargo test -p oteryn-game-server --test content_world_project_repository`: PASS.
- Materializer package comparison, simulated as in `g4-canonical-worldproject-package-seed.yml`: exact.
- `item_key_references.py`: PASS. ruff 0.16.1 on the package: PASS.
- `validate_full_game_content_tree.py` fails with `SOURCE_ID_BOUNDARY_MISSING` identically on `main`.
