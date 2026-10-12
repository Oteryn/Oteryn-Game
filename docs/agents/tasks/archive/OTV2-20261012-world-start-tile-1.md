# OTV2-20261012-world-start-tile-1

```yaml
task_id: OTV2-20261012-world-start-tile-1
title: "Pinned World start tile: walkable ground Item profiles and an enterable entry_start"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
lane_id: content
base_branch: main
branch: claude/world-start-tile-1-20261012
pr: 2019
base_sha: 9dc3641e5
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01XYC239givrMonrYoBwwECV
created_at: 2026-10-12
updated_at: 2026-10-12
packet: "WORLD-START-TILE-1 (control plane)"
owned_paths:
  - content/items/definitions/spell-native-profiles.json
  - content/items/index.json
  - content/spells.manifest.json
  - content/manifest.json
  - content/content.lock.json
  - content/world/pins/oteryn.json
  - deploy/synology-game/preprod-topology.toml
  - tools/content-schema/native-gameplay/build_source_item_profiles.py
  - tools/content-schema/native-gameplay/coin-item-profiles.json
  - tools/content-schema/native-gameplay/door-item-profiles.json
  - tools/content-schema/native-gameplay/start-tile-item-profiles.json
  - tools/content-migration/register_spell_families.py
  - apps/game-server/src/content/native_gameplay.rs
  - apps/game-server/tests/world_bundle_artifact_boot.rs
  - docs/agents/tasks/archive/OTV2-20261012-world-start-tile-1.md
depends_on: []
public_contracts: []
external_repositories: []
```

## Outcome

The pinned World (`entry_start` 10228,10032,0) refused to boot on the staging node with E2012 `StartNotEnterable`. The start cell (region-z00-x039-y039, sector 319,313) holds the top-level Items i4394, i1104, i1105 and i1788. The server treats an Item without a native profile saying `blocks_movement` false as solid, and none of the four had a profile.

- Reference evidence (CipSoft client `appearances.dat` in `content/assets/files/`, the source the generator reads `unpass` from; Canary `items.xml` has no attributes for these ids): i4394 is a bank (ground) with no `unpass`; i1104 and i1105 are clip (border) with no `unpass`; i1788 ("stone", 1783-1793) has `unpass`, so it blocks movement in Tibia and the start cell is genuinely blocked.
- Item provider 413 -> 416 records: walkable profiles for i1104, i1105 and i4394 (`blocks_movement` and `blocks_projectile` false), produced by the generator functions from the local appearances and items.xml (the six control ids 1629, 1630, 2338, 3031, 9552, 43955 reproduce the committed records byte for byte). New overlay `start-tile-item-profiles.json`, merged by `register_spell_families.py`; the coin and door overlay digests follow the larger provider; manifest, lock, index and `content/manifest.json` are tool-generated.
- `entry_start` moves to 10228,10035,0 (same sector, three tiles south): the nearest cell whose top-level Items are all verifiably walkable and need only profiles already added (it holds ground i4394 alone). Nearer cells need profiles for i4396, i1103, i1109 or i4408.
- `content/world/pins/oteryn.json`: `entry_start` and `inputs_digest` (hashes `content/items/definitions/`); `preprod-topology.toml` `start_y`. The bundle `digest` is unchanged unless `pin-check` names another (the profiles feed only the compiler's NPC and travel cell checks).
- Regression test: `world_bundle_artifact_boot.rs` now boots the pinned bundle with Item definitions built from the committed profiles (palette id + 1, `blocks_movement`, `blocks_projectile`, pickupable, DOOR), instead of reporting the `|_| None` boot. Two profile-count constants in `native_gameplay.rs` follow the provider.

## Findings

- 1749 walkable ground (bank, no `unpass`) palette Items of the placed World still have no profile and so block as solid; the start only boots because its cell and the fix were chosen narrowly. A ground-class profile batch is the follow-up for a walkable World.
- `npc-data.pin.json` and `node.toml.template` carry no value this change affects.

## Validation

- `python3 tools/content-migration/register_spell_families.py --check`: pass.
- `test_import_spell_families.py` 9, `test_import_current_spell_sources.py` 20, `test_compose_monster_current_sources.py` 3, `test_full_spell_artifact.py` 12 (8 skipped by the suite), `test_spell_item_identities.py` 10, `test_source_world.py` 5: pass.
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass (59 of 60 locally; the one other result is a missing file outside the sparse tree).
- `git diff --check` and `rustfmt --check` on the two Rust files: pass.
- Rust build, pin-check and the new test: not run (cargo is not permitted in this session); left to the control plane and CI.

## Review

Review state: pending, requested by the control plane.
