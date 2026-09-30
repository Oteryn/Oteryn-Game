# OTV2-20260930-area-catalogue

```yaml
task_id: OTV2-20260930-area-catalogue
title: AREAS-1 - Area authoring schema candidate v1 and the 465 client map areas with hometown temples
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/tender-mendel-06tjg2
issue: 162
lane_id: HOUSES
pr: null   # recorded in the FREEZE_SHA packet
base_sha: 09ccf36b
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "house schema worker (claude-code-session-01AGaDHCuMKQ95cqTJBs5XRQ)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/area-authoring/**
  - content/world/areas/regions/**
  - content/world/areas/cities/**
  - tools/content-schema/house-authoring/README.md
  - tools/content-schema/validate_materialized_game_tree.py
  - .github/workflows/area-authoring-schema.yml
  - .github/workflows/g4-canonical-worldproject-package-seed.yml
  - apps/game-server/tests/content_world_project_repository.rs
  - docs/agents/tasks/archive/OTV2-20260930-area-catalogue.md
public_contracts: []
depends_on: [OTV2-20260930-house-catalogue]
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

House catalogue owner contract §5 step 3, widened by the owner to all client map areas. A closed Area schema
candidate, a validator and a builder turn the 465 15.30 client map areas into 28 regions, 415 subregions and 22
cities (the 19 House towns and the hometowns Rookgaard, Roshamuul, Dawnport). The 19 hometowns carry their temple.
Every House `town` reference now resolves. No runtime, migration or protocol change.

## Owner direction (2026-09-30)

- 1 "c i b": all client areas, plus the engine towns filtered to real ones (the wiki hometown list).
- 2a: city keys are the House town slugs (`oteryn:content.area.city.ankrahmun`).
- 3: do the temples now, from the wiki if possible. Verbatim: "3 nie mozna tego jakos dobrze teraz zrobic? nie ma
  tego w wiki?", then "tak temple w ank jest na poziomie 0 w tibi bo sie zalogowalem".

## Evidence

- PROVEN: the client lists 28 flag-1 areas that list all 437 flag-2 areas; names are unique within a flag.
- PROVEN: all 19 House towns are a client area named the town or the town plus " City".
- DERIVED: temple = CrystalServer `world.otbm` town temple; 18 of 19 have the TibiaWiki temple Cleric/Healer NPC
  0-5 tiles away on the same floor; Ankrahmun is z 7 by the owner's in-game check (engine z 8).
- UNKNOWN: Farmine temple floor (engine and wiki `Temple` table z 11, Prezil's page z 15); not checked in game.
- ASSUMPTION: Dawnport's city area is "Dawnport Centre" (its client position is next to the temple).
- ASSUMPTION: Area keys follow the House catalogue identity rules (§2.1); no separate Area contract yet.

## Tree and package

- `validate_materialized_game_tree.py` and `content_world_project_repository.rs` accept `POPULATED` for the
  `areas/cities/` and `areas/regions/` markers, and the inventory test keeps their shards out of the legacy package,
  as #1319 did for Terrain and WorldObject.
- The G4 package comparison removes the area shards, as it removes the WO-2 shards (workflow edit: owner question).

## Validation (local)

- `build_areas.py build --check`: ok (465 areas); `test_validate_areas.py`: 14/14;
  `extract-crystal-towns --check` against the pinned map: ok.
- Key stability: a committed region key and revision edited by hand are kept on rebuild, and its children follow.
- ruff 0.16.1 check and format; materialized tree, governance and repository policy validators: see the FREEZE_SHA
  packet.
