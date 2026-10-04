# OTV2-20261004-spawn-content-1

```yaml
task_id: OTV2-20261004-spawn-content-1
title: "SPAWN-CONTENT-1: the spawn family of the World Bundle (format v3)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/spawn-content-1-20261004
issue: 1622
pr: null
head_sha: "exact frozen head in the FREEZE_SHA message to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA message to the control plane"
owner: claude-code-session_01FqspUyiMBJXa3VZqUkq6ra
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - tools/world-bundle-compiler/**
  - docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - content/world/spawns/**
  - docs/agents/tasks/archive/OTV2-20261004-spawn-content-1.md
public_contracts:
  - docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
depends_on: [MAP-BUNDLE-2]
blocks: [MAP-LOAD-1, SPAWN-1b]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Bundle format v3 (CREATURE-AI-0 §6.1, packet §2.6, ruling §1.3) adds the spawn family: one
frame after the sector frames holding the creature table and the spawn sources, with the
manifest counts `spawns {sources, points}`. No dual reading: v1 and v2 are refused. The
Spawn.Source content is `content/world/spawns/` (27 files), written by
`tools/world-bundle-compiler/convert_spawns.py` from the pinned Canary `otservbr-monster.xml`
(rev `47dfd51f`, sha256 `ce70ad49...`): 51,896 sources and 83,286 points, every name bound by slug
to an admitted creature definition (897 names, none unbound). `--check` proves the committed
files are its output.

The compiler realizes a point only when its creature is admitted and its cell can admit it, and
lists every other point with a reason (`UnboundCreature`, `Boss`, `EncounterBound`,
`OutsideWorld`, `NoTile`, `UnclassifiedTerrain`, `NoGround`, `NotWalkable`, `ProtectionZone`,
`FloorChange`, `Teleport`) in the `parity` and `compile` output. Bosses go to BOSS-RAID-0 and
are not realized. A Night creature is compiled and counted as inactive. Registry rows
`CREATUREAI0-RL-01`, `-02`, `-03` and `-13` are added and tested at the maximum and one past it.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: build-time format and compiler change; no session, fence, lease or persisted recovery state.

## Implementation / findings

- Real map (`parity .`, release): 51,896 sources, 83,286 points; 48,980 sources and 79,360 points
  realized, 3,926 points left out: Boss 61, EncounterBound 1,881, NoGround 1,798, NotWalkable 164,
  ProtectionZone 15, NoTile 4, UnclassifiedTerrain 3 (cells on one of the 50 records below),
  UnboundCreature 0, FloorChange 0, Teleport 0.
- `NoGround` is a tile whose ground is not a Terrain-routed record (plain Item or WorldObject
  routes carry no walkability). It is the content lane's to classify; no point is guessed
  walkable.
- `floor_change` is UNKNOWN in 21,099 catalogue records and KNOWN non-`none` in 445; the
  admission check counts only the KNOWN ones, so the FloorChange count rises when the content lane
  classifies the rest.
- The real full compile still stops on the 50 Terrain-routed records with UNKNOWN kind (from
  MAP-BUNDLE-2); the parity path reports them (`UnclassifiedTerrain`) instead of failing.
- Format bump: MAP-LOAD-1's reader must target v3 (`crates/world-bundle` does not exist yet and
  is untouched).

## Validation

- `cargo test --locked -p oteryn-world-bundle-compiler`: pass
- `cargo check --locked --workspace --all-targets`: pass
- `cargo fmt --all -- --check`: pass
- `cargo clippy -p oteryn-world-bundle-compiler --all-targets -- -D warnings`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
- `python tools/world-bundle-compiler/convert_spawns.py --xml <pinned file> --check`: pass
