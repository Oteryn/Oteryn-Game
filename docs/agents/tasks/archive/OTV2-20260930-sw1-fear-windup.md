# OTV2-20260930-sw1-fear-windup

```yaml
task_id: OTV2-20260930-sw1-fear-windup
title: SW-1 soulwars fear as Ability.windup, with the E4 restage
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: null
pr: null  # the PR on this branch is authoritative
base_sha: c2da49e
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: content world import"
created_at: 2026-09-30T11:30:00Z
updated_at: 2026-09-30T12:30:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/monster-authoring/build_formal_schema.py
  - tools/content-schema/monster-authoring/monster.schema.json (generated)
  - tools/content-schema/monster-authoring/monster-dependencies-template.json (generated)
  - tools/content-schema/monster-authoring/validate_monster.py
  - tools/content-schema/monster-authoring/verify_formal_schema.py
  - tools/content-schema/monster-authoring/canary_batch.py
  - tools/content-schema/monster-authoring/test_windup_template.py
  - tools/content-schema/monster-authoring/README.md
  - tools/content-schema/monster-authoring/samples/population-canary-47dfd51f.json (generated)
  - tools/content-schema/monster-authoring/samples/population-bundles-canary-47dfd51f.json (generated)
  - tools/content-migration/creature_admission_stage.py
  - apps/game-server/src/content/project/v2/creature.rs
  - apps/game-server/tests/content_world_project_v2_creature_admission.rs
  - apps/game-server/tests/content_world_project_v2_encounter_admission.rs
  - docs/agents/evidence/OTV2-20260927-creature-admission-wave-a-staged.json (generated)
  - apps/game-server/examples/materialize_content_world_project_v2.rs (pins)
  - apps/game-server/tests/content_world_project_repository.rs (pins)
  - content/world/, content/ and imports/ (generated)
  - tools/content-migration/validate_world_project_v2_to_tree.py (pinned counts)
  - tools/content-migration/test_world_project_v2_to_tree.py (pinned counts)
  - docs/agents/tasks/archive/OTV2-20260930-sw1-fear-windup.md
public_contracts: []
depends_on:
  - "OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md section 10.1 (ACCEPTED 2026-09-30; owner answers Q3 a and Q4 a of encounter format section 13.6)"
  - OTV2-20260930-sw2-destroy-magic-walls
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This is SW-1 as described in section 10.1. An Ability may carry `windup: {delay_ms, caster_asset_binding}`.
- The schema requires `delay_ms` of at least 1 and a caster binding.
- The semantic rule allows `windup` only with `needs_target: true` and without `area`, `variants`, `chain` or
  `encounter`.
- The Rust `ProjectV2AbilityDetails` mirrors it as `ProjectV2Windup`, with the same rule at admission.
- The stage tool copies `windup` into the Ability details.
- `canary_batch.py` matches `soulwars fear` as one exact template: a caster `sendMagicEffect`, then
  `addEvent(f, N, creature:getId(), var)`, where `f` runs the one Combat only if the caster exists. Any other body
  stays unresolved. The Combat converts as before, giving a `feared` condition of 3,000 ms with the blue ghost impact.
  The Ability gets `windup` 2,000 ms with `ghost_smoke`, which is the JSON of section 10.1 item 4.

The census loses the `fear` row (4 monsters) and no other row changes. Three monsters become fully resolved:
Goshnar's Spite, Hazardous Phantom and Turbulent Elemental. Blocked goes from 89 to 86 and fully resolved from 1,561
to 1,564. Bony Sea Devil stays blocked by other rows.

The E4 restage admits Goshnar's Spite and Hazardous Phantom. The stage defers Turbulent Elemental as encounter-bound
(deferred encounter 25 to 26). Totals: 1,479 creatures, 20,693 records and 19,745 profiles (was 1,477, 20,652 and
19,706). The stage tool changed, so its pin changed too. Tree family counts: Creature 1,479, Ability 5,902, Effect
4,502, Formula 4,820, Behavior 2,573, Presentation 2,573 and Loot 1,032.

## Validation

- `verify_formal_schema.py`: windup accepted; zero delay, a missing binding, no target and an area are rejected.
- `python -m unittest test_windup_template test_remove_items_probe`.
- The admission test `windup_admits_only_on_a_single_target_ability`.
- The generation chain: `population_census.py`, `creature_admission_stage.py`, `materialize_content_world_project_v2`,
  and `world_project_v2_to_tree.py` with its validator and test.
- `item_key_references.py`.
- `cargo fmt --all --check`, `cargo clippy -p oteryn-game-server --all-targets` and `cargo test -p oteryn-game-server`.
- `validate_governance.py`.
