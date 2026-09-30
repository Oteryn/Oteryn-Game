# OTV2-20260929-house-doors

```yaml
task_id: OTV2-20260929-house-doors
title: HOUSES-3 - House doors from the official layout
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/houses-3-doors
issue: 162
lane_id: content population (house authoring schema)
pr: null   # recorded in the FREEZE_SHA packet
base_sha: c3263e85   # main after Oteryn/Oteryn-Game#1285 (squash of 8a48763e)
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "house schema worker (claude-code-session-01AGaDHCuMKQ95cqTJBs5XRQ)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/house-authoring/**
  - docs/agents/tasks/archive/OTV2-20260929-house-doors.md
public_contracts: []
depends_on: [OTV2-20260929-house-authoring-schema-candidate]
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

The House candidate record gains `doors`: the sorted positions of the official layout cells
that hold a door item. All 995 houses convert and validate with 5,372 doors. Beds stay a
count; bed positions come with the world map placements.

## Owner decisions (2026-09-29)

- Doors in the House record; beds not (a bed is a world object on a House tile).
- A door is identified by House + position, no separate door number.
- Bed positions not taken from the engine map (OtsHypothesisOnly, 84 houses disagree).

## Source verification

- PROVEN: no layout cell holds two door items; every House has at least one door.
- PROVEN: 5,037 of 5,156 engine House doors are on client doors; 834 identical door sets
  (`samples/otbm-tile-check.json`, pinned `world.otbm`).
- DERIVED: door item ids from CrystalServer `items.xml` (`type="door"`, 745 ids, pinned
  sha256), because the client does not mark doors.
- CONFLICT (resolved): one door is in two layouts (East Lane 1a/1b); assigned to East Lane 1a
  from the engine map in `SHARED_DOOR_OWNERS`; any new shared door stops the conversion.
- DERIVED: CrystalServer door numbers are unusable as identity (677 cover several distant
  doors, 682 doors have none).
- DERIVED: the engine `entrance` is next to a door for 968 houses.

## Validation (local)

- `verify_formal_schema.py`: 28/28; `convert_houses.py convert --check`: 995 houses valid.
- `extract-door-items --check`, `extract-crystal --check`, `otbm_tile_check.py --check`
  against the pinned checkout: ok. `wiki_br_houses.py self-test`: ok.
- `ruff check` / `ruff format --check`; governance and repository policy validators: pass.
