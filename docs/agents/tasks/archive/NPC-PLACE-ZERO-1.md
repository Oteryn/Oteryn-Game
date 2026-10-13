# NPC-PLACE-ZERO-1

```yaml
task_id: NPC-PLACE-ZERO-1
title: "NPC-PLACE-ZERO-1: World Bundle compiler wrote 0 NPC placements (ground items counted as solid)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
implementation_branch: claude/xp-kill-1-20261013
issue: 1622
pr: 2027
governing_issue: 1622
terminal_pr_policy: archived_in_pr
owned_paths:
  - tools/world-bundle-compiler/src/compile.rs
  - content/world/pins/oteryn.json
  - deploy/synology-game/preprod-topology.toml
  - tools/world-bundle-compiler/src/resolve.rs
  - tools/world-bundle-compiler/tests/npcs.rs
  - docs/agents/tasks/archive/NPC-PLACE-ZERO-1.md
```

## Cause

Preprod logged `event=npc_actors state=placed npcs=0 placements=0 actors=0`: the bundle itself carried an empty NPC frame, not a staging or catalogue problem (the catalogue digest matched). All 25 placements were held `BlockSolid`: `cell_facts` treated a top-level Item without a native profile as solid, which includes every ground/border Item, so no cell could hold an NPC. The game server's `terrain_floor` (`map/facts.rs`) exempts grounds and borders the catalogue proves walkable.

## Fix

`cell_facts` applies the same exemption; `Registry::terrain_of` keeps a known `walkable` fact on borders, and an explicit solid profile on a ground Item still blocks. 21 of 25 placements are now written (4 remain held `BlockSolid`). The pin `digest`/`inputs_digest` and the preprod topology digest are refreshed.

## Proposal (workflow not edited)

The deploy gate greps only `state=placed`; require `npcs>0` as well (`event=npc_actors state=placed npcs=[1-9]`).

## Validation

cargo fmt --check (tools/world-bundle-compiler): pass
cargo clippy --all-targets (tools/world-bundle-compiler): pass
cargo test (tools/world-bundle-compiler): pass
pin-check (oteryn): OK
python tools/agents/validate_governance.py: pass
python -m unittest discover -s tools/agents/tests: OK

Review: Codex P2 (explicit solid profile) and P2 (walkable borders) fixed; P1 (archive record) done here.
