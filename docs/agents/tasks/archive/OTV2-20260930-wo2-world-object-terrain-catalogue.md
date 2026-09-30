# OTV2-20260930-wo2-world-object-terrain-catalogue

```yaml
task_id: OTV2-20260930-wo2-world-object-terrain-catalogue
title: WO-2a/2c Terrain and WorldObject catalogues in content/world/{terrain,objects}, contested routes resolved
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
  - tools/content-schema/item-authoring/{engine_items.py,test_engine_items.py,README.md}   # WO-2c routes
  - tools/content-schema/validate_materialized_game_tree.py
  - apps/game-server/tests/content_world_project_repository.rs
  - .github/workflows/item-authoring-schema.yml   # owner authorization required
  - .github/workflows/g4-canonical-worldproject-package-seed.yml   # owner authorization required
  - docs/agents/tasks/archive/OTV2-20260930-wo2-world-object-terrain-catalogue.md
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
| WO-2c | The WO-0 contested routes, decided before any key is minted (a later move between families would need a retirement) | this record, same PR |
| WO-2b | `DefinitionFamily::WorldObject`/Terrain records in the Rust model, `routed_to` on the 21,330 routed Item records, typed relation references, donor routed ids (138, B1b) | needs its own allocation |

## Outcome

- DONE: `build_catalogue.py` writes 8,548 Terrain and 12,782 WorldObject records, keyed `oteryn:terrain.tibia.i<id>`
  and `oteryn:world-object.tibia.i<id>` (A12 §4.6). Each record keeps `provenance.item_pointer` to its Item key.
- DONE: `--check` rebuilds all 46 files and the census sample from the pinned Crystal files and fails on any
  difference, missing or extra shard.
- DERIVED: the committed census sample was stale after #1305 (21,370 -> 21,330 records: D149 removed the ids
  without a CipSoft appearance, and the converter gained the `non_pickupable_blocking_prop` route). It is rebuilt
  from the same run as the catalogues, so the two always agree.
- DONE: the tree validator and the WorldProject inventory test accept `POPULATED` for exactly these two markers and
  keep the shards out of the legacy package inventory. The canonical package comparison stays exact (checked
  locally: materializer output equals the tracked package once markers and shards are removed).

## WO-2c owner decisions (2026-09-30)

The owner answered "1a, 2a" to the workflow and deferral questions (#162 5907261940), then "kontynuuj" to the
five contested-route recommendations; this record treats that as 1a-5a, stated as an assumption the owner can
override before merge. The PR's auto-merge was turned off so no key merges before the routes are settled.

| # | Decision | Effect |
|---|---|---|
| 1a | A Terrain tile named as a roof, with no ground, border or wall flag, is kind `roof` | 234 tiles |
| 2a | A `trashholder` Terrain tile keeps its kind; `behavior: trash_holder` marks it for Interaction | 103 tiles |
| 3a | An unbanked magic field (`type="magicfield"`) routes to Terrain (`magic_field`, kind `field`) | 21 ids, from WorldObject |
| 4a | A `teleport` Terrain tile keeps its kind; `behavior: teleport` marks it for Interaction | 11 tiles |
| 5a | A fixed carpet (`type="carpet"`, unmove) routes to WorldObject (`fixed_carpet`, kind `decoration`) | 18 ids, from Terrain |

- PROVEN: the converter route change moves exactly those 39 ids (per-id before/after diff over all 38,157 Crystal
  rows). A first draft that ran before family classification would have taken 66 house carpets and 3 campfires away
  from Item; the rule sits in `immovable_non_item_route`, so an id that resolves an Item family stays an Item.
- PROVEN: the donor census and the client 15.30 appearance census are unchanged by the route change.
- 65 Terrain tiles (mosaics, unbanked floors, leaves) still match no rule and stay UNKNOWN.

## Assumptions (reversible)

- Relations (`rotate_to`, bed parts, sleepers, transforms) stay `{"source_item_id": n}`. Under A12 the source id is
  the Tibia id, so each target is determined; the typed reference arrives with the Rust family in WO-2b.
- The committed item-authoring population samples are history (#1305) and are not regenerated here.

## Validation (local)

- `python build_catalogue.py --source <crystal ff7ede5> --check`: PASS (46 files).
- `python test_world_objects.py`: PASS (21,583 checks, incl. the catalogue, committed-catalogue and WO-2c tests).
- item-authoring suites (`test_engine_items.py` 598 checks incl. the WO-2c route test, donor, client census,
  lowering, membership) and `client_appearance_census.py --check` against the three pinned sources: PASS.
- `validate_materialized_game_tree.py`: PASS. `test_world_project_v2_to_tree.py`, `validate_world_project_v2_to_tree.py`: PASS.
- `cargo test -p oteryn-game-server --test content_world_project_repository`: PASS.
- Materializer package comparison, simulated as in `g4-canonical-worldproject-package-seed.yml`: exact.
- `item_key_references.py`: PASS. ruff 0.16.1 on the package: PASS.
- `validate_full_game_content_tree.py` fails with `SOURCE_ID_BOUNDARY_MISSING` identically on `main`.
