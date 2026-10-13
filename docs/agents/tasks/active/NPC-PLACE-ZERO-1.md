# NPC-PLACE-ZERO-1

```yaml
task_id: NPC-PLACE-ZERO-1
title: "NPC-PLACE-ZERO-1: World Bundle compiler wrote 0 NPC placements (ground items counted as solid)"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
base_branch: main
implementation_branch: claude/xp-kill-1-20261013
governing_issue: 1622
terminal_pr_policy: archive_pending
owned_paths:
  - tools/world-bundle-compiler/src/compile.rs
  - content/world/pins/oteryn.json
  - deploy/synology-game/preprod-topology.toml
  - docs/agents/tasks/active/NPC-PLACE-ZERO-1.md
```

## Cause

Preprod logged `event=npc_actors state=placed npcs=0 placements=0 actors=0`: the bundle itself carried an empty NPC frame, not a staging or catalogue problem (the catalogue digest matched). All 25 placements were held `BlockSolid`: `cell_facts` treated a top-level Item without a native profile as solid, which includes every ground/border Item, so no cell could hold an NPC. The game server's `terrain_floor` (`map/facts.rs`) exempts grounds and borders the catalogue proves walkable.

## Fix

`cell_facts` applies the same exemption. 20 of 25 placements are now written (5 remain held `BlockSolid`). The pin `digest`/`inputs_digest` and the preprod topology digest are refreshed.

## Proposal (workflow not edited)

The deploy gate greps only `state=placed`; require `npcs>0` as well (`event=npc_actors state=placed npcs=[1-9]`).
