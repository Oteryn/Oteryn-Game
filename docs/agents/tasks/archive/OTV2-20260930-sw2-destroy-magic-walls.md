# OTV2-20260930-sw2-destroy-magic-walls

```yaml
task_id: OTV2-20260930-sw2-destroy-magic-walls
title: SW-2 destroy magic walls as remove_items top_item_first_tile, with the E4 restage
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: null
pr: null  # the PR on this branch is authoritative
base_sha: cbe98f9
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: content world import"
created_at: 2026-09-30T10:00:00Z
updated_at: 2026-09-30T13:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/monster-authoring/canary_batch.py
  - tools/content-schema/monster-authoring/spell_probes.py
  - tools/content-schema/monster-authoring/build_formal_schema.py
  - tools/content-schema/monster-authoring/monster.schema.json (generated)
  - tools/content-schema/monster-authoring/verify_formal_schema.py
  - tools/content-schema/monster-authoring/test_remove_items_probe.py
  - tools/content-schema/monster-authoring/README.md
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md (section 8.6 sentence only)
  - tools/content-schema/monster-authoring/samples/population-canary-47dfd51f.json (generated)
  - tools/content-schema/monster-authoring/samples/population-bundles-canary-47dfd51f.json (generated)
  - docs/agents/evidence/OTV2-20260927-creature-admission-wave-a-staged.json (generated)
  - apps/game-server/examples/materialize_content_world_project_v2.rs (pins)
  - apps/game-server/tests/content_world_project_repository.rs (pins)
  - content/world/ and content/ (generated)
  - tools/content-migration/validate_world_project_v2_to_tree.py (pinned counts)
  - tools/content-migration/test_world_project_v2_to_tree.py (pinned counts)
  - docs/agents/tasks/archive/OTV2-20260930-sw2-destroy-magic-walls.md
public_contracts: []
depends_on:
  - "OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md section 10.2 (ACCEPTED 2026-09-30, OTV2-20260930-soul-war-mechanics-design)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

SW-2 as section 10.2 describes, with no schema extension. The probe world gets the engine constants
`ITEM_MAGICWALL_SAFE` 10181, `ITEM_MAGICWALL` 2128, `ITEM_WILDGROWTH_SAFE` 10182 and `ITEM_WILDGROWTH` 2130 (Canary
`src/utils/utils_definitions.hpp`). `probe_remove_items` gains a `top_item_first_tile` branch: the cast must read
each tile of its floor once, columns west to east and each north to south. The stub's top visible item is then set
on one tile at a time, and the cast must remove exactly that item and read no further tile. Only then is the Effect
emitted. The schema description of `top_item_first_tile` gains the scan order.

`destroy magic walls` now converts to the Effect of section 10.2 item 4. The census loses the
`remove_magic_walls` row (5 monsters) and no other row changes. The Monster becomes fully resolved (blocked 90 to 89,
fully resolved 1,560 to 1,561). Bony Sea Devil, Brachiodemon, Cloak of Terror and Many Faces keep other blockers
(for example `soulwars fear`, SW-1).

Section 8.6 of the monster schema no longer lists `destroy magic walls` among the unmodelled engine item
constants.

E4 restage: 1,477 creatures, 20,652 records, 19,706 profiles (was 1,476 / 20,638 / 19,693). The stage tool is
unchanged. Tree family counts: Creature 1,477, Ability 5,890, Effect 4,492, Formula 4,809, Behavior 2,571,
Presentation 2,571, Loot 1,030.

## Validation

`python -m unittest test_remove_items_probe` (the Canary scan resolves; more than one removal, or a scan that goes
on after the removal, stays unresolved); `verify_formal_schema.py` (top_item_first_tile accepted, empty items
rejected); `build_formal_schema.py` regenerated; `population_census.py`; `creature_admission_stage.py`;
`materialize_content_world_project_v2`; `world_project_v2_to_tree.py` with its validator and test;
`item_key_references.py`; `cargo fmt --all --check`; `cargo clippy -p oteryn-game-server --all-targets`;
`cargo test -p oteryn-game-server`; `validate_governance.py`.
