# OTV2-20261011-map-door-content-1

```yaml
task_id: OTV2-20261011-map-door-content-1
title: "Item profiles for the placed plain doors and their open items"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
lane_id: content
base_branch: main
branch: claude/map-door-content-1-20261011
pr: 1993
base_sha: 4ab138b90
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
created_at: 2026-10-11
updated_at: 2026-10-11
packet: "owner-approved content packet MAP-DOOR-CONTENT-1; companion of MAP-DOOR-1 (#1966)"
owned_paths:
  - content/items/definitions/spell-native-profiles.json
  - content/items/index.json
  - content/spells.manifest.json
  - content/manifest.json
  - content/content.lock.json
  - tools/content-schema/native-gameplay/build_source_item_profiles.py
  - tools/content-schema/native-gameplay/coin-item-profiles.json
  - tools/content-schema/native-gameplay/door-item-profiles.json
  - tools/content-migration/register_spell_families.py
  - apps/game-server/src/content/native_gameplay.rs
  - content/world/pins/oteryn.json
  - docs/agents/tasks/archive/OTV2-20261011-map-door-content-1.md
depends_on: []
public_contracts: []
external_repositories: []
```

## Outcome

#1966 decides doorhood by the Item profile's `item_type` DOOR and pairs a door with its open item through the Game-owned registry. The active generation profiled only i2338, i9552, i9561, i9573, i9574, so classic doors were not doors and a preprod node would refuse boot with `NoDoors`.

The Item provider gains 290 profiles: the 146 closed plain doors the World project places (registry pairs C / C+1, ids in `DOOR_CLOSED`) and their 146 open items, minus i9552 and i9561 which already had profiles. Closed doors: item_type DOOR, blocks movement and projectiles (143 of 146). Open items: blocks neither. Provider 121 -> 411 records. The 62 open doors the map places as plain items are now walkable too.

- `build_source_item_profiles.py` adds the door ids, refuses a profile that disagrees with the door contract and writes the overlay `door-item-profiles.json`; the coin overlay digest follows the larger provider.
- Two open doors outside the C+1 pairs get walkable DOOR profiles from the pinned Canary `transformOnUse` chain (`DOOR_OPEN_EXTRA`): i43955 (door 43953 opens into 43955; 43954 opens back into 43952) and i42629 (an open door the map places by itself, six top-level placements; it closes into 42627). Provider 411 -> 413 records, overlay 292 profiles.
- `register_spell_families.py` merges both overlays into the pinned r25 provider (generic Item 40450 stays last, as the generator emits it), checks the result equals the generator digest and repins. Manifest, lock, Item index and `content/manifest.json` are tool-generated.
- Two test constants in `native_gameplay.rs` (profile count 121 -> 411) follow the provider; no server logic changed.

## Findings

- Source-faithful, not overridden: closed fence gates 2177, 2179 and closed door 5733 do not block projectiles in the pinned Canary source, so #1966 keeps them sealed.
- Open items carry item_type DOOR from items.xml; #1966 then counts a placed open door as sealed (log counter only, it is walkable).
- Scope expansion: `content/world/pins/oteryn.json` was not in the first allocation. The World bundle compiler's `inputs_digest` hashes `content/items/definitions/`, which this task regenerates, so `pin-check` (the `world_bundle` gate job) refuses the stale pin; the pin's `inputs_digest` was refreshed to the digest `pin-check` names, nothing else in the pin changed. The path is now an owned path.
- Open question for MAP-DOOR-1 (#1966, not owned here): its registry `plain_doors_v1.json` pairs 43953 with 43954 and 42627 with 42628, while Canary transforms 43953 to 43955 and 42627 to 42629. Both Canary open items now have profiles; which open item the registry names is that PR's decision.
- Door set = registry pairs whose closed id the committed World project places (146 of 173); the other 27 pairs are not placed.

## Validation

- `python3 tools/content-migration/register_spell_families.py --check`: pass.
- `python3 tools/content-migration/test_import_spell_families.py`: OK.
- `python3 tools/content-migration/test_import_current_spell_sources.py`: OK.
- `python3 tools/content-migration/test_compose_monster_current_sources.py`: OK.
- `python3 tools/content-schema/native-gameplay/test_full_spell_artifact.py`: OK.
- `python3 tools/content-schema/native-gameplay/test_spell_item_identities.py`: OK.
- `python3 tools/content-schema/native-gameplay/test_source_world.py`: OK.
- Generator rerun at the pinned Canary and Crystal revisions reproduces the baseline 121-record provider byte for byte; the new provider equals the generator output.
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass.
- Sparse-checkout note: that run had 59 of 60 passing locally; the one error is a missing `experiments/` file outside the sparse tree.
- Rust build and tests: not run locally (disk); left to CI.

## Review

Review state: pending, requested by the control plane.
