# y3 shim cleanup report

Working tree only at starting HEAD `5e3f585d4`; no Git mutation or publication.

## Result

Removed 56 shim files and the encounter-specific `foundation` facade. Converted 247 crate-private forwarding names to ordinary private imports. Destination dependencies already existed. Removed 19 unused dependency declarations and corresponding boundary edges. No content/foundation ownership move or y1 policy section was edited.

The diff changes imports, module wiring, qualified paths and manifests only. Executable string literals, test declarations, SQL and wire constants are unchanged. No enum variant, struct field, trait method or public function was added. Whole-diff checklist review found only import/module changes and equivalent rustfmt punctuation.

## Before/after library test counts

Counts were obtained from actual `cargo +1.95.0 test -p <crate> --lib -- --list` output (the initial twelve packages were listed together; NPC was listed separately before editing). Test names and counts are unchanged.

| Package | Before | After |
|---|---:|---:|
| oteryn-game-server | 309 | 309 |
| oteryn-gameplay-combat | 36 | 36 |
| oteryn-gameplay-combat-rules | 46 | 46 |
| oteryn-gameplay-dispatch | 897 | 897 |
| oteryn-gameplay-durability | 384 | 384 |
| oteryn-gameplay-encounters | 27 | 27 |
| oteryn-gameplay-entities | 167 | 167 |
| oteryn-gameplay-foundation | 340 | 340 |
| oteryn-gameplay-monster-ai | 74 | 74 |
| oteryn-gameplay-npc | 9 | 9 |
| oteryn-gameplay-social | 103 | 103 |
| oteryn-gameplay-spell | 221 | 221 |
| oteryn-gameplay-world | 173 | 173 |

Total: 2786 -> 2786.

## Removed shim paths (old -> real owner)

| Old path | Real owner path | Real module/symbol |
|---|---|---|
| `crates/gameplay/combat-rules/src/formula.rs` | `crates/gameplay/content/src/content/spell_data/formula.rs` | `oteryn_gameplay_content::content::spell_data::formula` |
| `crates/gameplay/combat/src/ability/condition.rs` | `crates/gameplay/foundation/src/foundation/condition.rs` | `oteryn_gameplay_foundation::foundation::condition` |
| `crates/gameplay/combat/src/ability/field_condition.rs` | `crates/gameplay/foundation/src/foundation/condition.rs` | `oteryn_gameplay_foundation::foundation::condition::field_condition` |
| `crates/gameplay/combat/src/combat/attack/block.rs` | `crates/gameplay/combat-rules/src/attack/block.rs` | `oteryn_gameplay_combat_rules::attack::block` |
| `crates/gameplay/combat/src/combat/attack/constants.rs` | `crates/gameplay/combat-rules/src/attack/constants.rs` | `oteryn_gameplay_combat_rules::attack::constants` |
| `crates/gameplay/combat/src/combat/attack/formulas.rs` | `crates/gameplay/combat-rules/src/attack/formulas.rs` | `oteryn_gameplay_combat_rules::attack::formulas` |
| `crates/gameplay/combat/src/combat/attack/target.rs` | `crates/gameplay/combat-rules/src/attack/target.rs` | `oteryn_gameplay_combat_rules::attack::target` |
| `crates/gameplay/combat/src/combat/chain.rs` | `crates/gameplay/combat-rules/src/chain.rs` | `oteryn_gameplay_combat_rules::chain` |
| `crates/gameplay/combat/src/combat/charm_effects.rs` | `crates/gameplay/progression/src/charm_effects.rs` | `oteryn_gameplay_progression::charm_effects` |
| `crates/gameplay/combat/src/combat/formula.rs` | `crates/gameplay/content/src/content/spell_data/formula.rs` | `oteryn_gameplay_content::content::spell_data::formula` |
| `crates/gameplay/combat/src/combat/loot_plan.rs` | `crates/gameplay/items/src/loot_plan.rs` | `oteryn_gameplay_items::loot_plan` |
| `crates/gameplay/combat/src/combat/pvp/legality.rs` | `crates/gameplay/combat-rules/src/pvp/legality.rs` | `oteryn_gameplay_combat_rules::pvp::legality` |
| `crates/gameplay/combat/src/creature_attack_geometry.rs` | `crates/gameplay/content/src/content/creature_attack_geometry.rs` | `oteryn_gameplay_content::content::creature_attack_geometry` |
| `crates/gameplay/combat/src/creature_icicle.rs` | `crates/gameplay/encounters/src/bosses/icicle/creature_icicle.rs` | `oteryn_gameplay_encounters::bosses::icicle::creature_icicle` |
| `crates/gameplay/dispatch/src/gameplay_transport/look.rs` | `crates/gameplay/items/src/look.rs` | `oteryn_gameplay_items::look` |
| `crates/gameplay/dispatch/src/gameplay_transport/world_object.rs` | `crates/protocol-oteryn/src/world_object.rs` | `oteryn_protocol_oteryn::world_object` |
| `crates/gameplay/dispatch/src/welter_consume.rs` | `crates/gameplay/encounters/src/bosses/the_welter/welter_consume.rs` | `oteryn_gameplay_encounters::bosses::the_welter::welter_consume` |
| `crates/gameplay/durability/src/durability/character_authority_audit.rs` | `crates/gameplay/characters/src/character_authority_audit.rs` | `oteryn_gameplay_characters::character_authority_audit` |
| `crates/gameplay/durability/src/durability/character_vitals_soul.rs` | `crates/gameplay/characters/src/soul.rs` | `oteryn_gameplay_characters::soul` |
| `crates/gameplay/durability/src/durability/character_vitals_stamina.rs` | `crates/gameplay/characters/src/stamina.rs` | `oteryn_gameplay_characters::stamina` |
| `crates/gameplay/durability/src/durability/character_vitals_stamina/stamina_math.rs` | `crates/gameplay/characters/src/stamina.rs` | `oteryn_gameplay_characters::stamina` |
| `crates/gameplay/durability/src/durability/creature_schedule.rs` | `crates/gameplay/foundation/src/foundation/profile_schedule.rs` | `oteryn_gameplay_foundation::foundation::profile_schedule::ScheduleList` |
| `crates/gameplay/durability/src/durability/item_capacity.rs` | `crates/gameplay/items/src/item_capacity.rs` | `oteryn_gameplay_items::item_capacity` |
| `crates/gameplay/durability/src/durability/quest_state/quest/mod.rs` | `crates/gameplay/quests/src/quest/mod.rs` | `oteryn_gameplay_quests::quest` |
| `crates/gameplay/durability/src/durability/spell_wheel_abi.rs` | `crates/gameplay/foundation/src/foundation/spell_wheel_profile.rs` | `oteryn_gameplay_foundation::foundation::spell_wheel_profile` |
| `crates/gameplay/durability/src/durability/uniform_draw.rs` | `crates/gameplay/domain/src/domain/uniform_draw.rs` | `oteryn_gameplay_domain::domain::uniform_draw` |
| `crates/gameplay/encounters/src/bosses/time_guardian/mod.rs` | `crates/gameplay/encounters/src/bosses/native_combat/monster_combat_lane.rs` | `oteryn_gameplay_encounters::bosses::native_combat::monster_combat_lane` |
| `crates/gameplay/encounters/src/bosses/weak_spot/weak_spot_speech.rs` | `crates/gameplay/encounters/src/bosses/crystal_deaths/crystal_death_composition/speech.rs` | `oteryn_gameplay_encounters::bosses::crystal_deaths::crystal_death_composition::speech` |
| `crates/gameplay/encounters/src/crystal_callbacks.rs` | `crates/gameplay/encounters/src/bosses/crystal_deaths/crystal_callbacks.rs` | `oteryn_gameplay_encounters::bosses::crystal_deaths::crystal_callbacks` |
| `crates/gameplay/encounters/src/crystal_death_composition.rs` | `crates/gameplay/encounters/src/bosses/crystal_deaths/crystal_death_composition.rs` | `oteryn_gameplay_encounters::bosses::crystal_deaths::crystal_death_composition` |
| `crates/gameplay/encounters/src/crystal_herald_quest.rs` | `crates/gameplay/encounters/src/bosses/herald_of_fire/crystal_herald_quest.rs` | `oteryn_gameplay_encounters::bosses::herald_of_fire::crystal_herald_quest` |
| `crates/gameplay/encounters/src/monster_combat_lane.rs` | `crates/gameplay/encounters/src/bosses/native_combat/monster_combat_lane.rs` | `oteryn_gameplay_encounters::bosses::native_combat::monster_combat_lane` |
| `crates/gameplay/encounters/src/source_callback_cast.rs` | `crates/gameplay/encounters/src/bosses/source_callbacks/source_callback_cast.rs` | `oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_cast` |
| `crates/gameplay/encounters/src/source_callback_intent.rs` | `crates/gameplay/encounters/src/bosses/source_callbacks/source_callback_intent.rs` | `oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_intent` |
| `crates/gameplay/encounters/src/source_callback_spawn.rs` | `crates/gameplay/encounters/src/bosses/source_callbacks/source_callback_spawn.rs` | `oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_spawn` |
| `crates/gameplay/encounters/src/source_encounter_seven.rs` | `crates/gameplay/encounters/src/bosses/seven_encounters/source_encounter_seven.rs` | `oteryn_gameplay_encounters::bosses::seven_encounters::source_encounter_seven` |
| `crates/gameplay/encounters/src/source_threshold_heal.rs` | `crates/gameplay/encounters/src/bosses/threshold_heals/source_threshold_heal.rs` | `oteryn_gameplay_encounters::bosses::threshold_heals::source_threshold_heal` |
| `crates/gameplay/monster-ai/src/bone_phase_registry.rs` | `crates/gameplay/encounters/src/bosses/bone_overlord/bone_phase_registry.rs` | `oteryn_gameplay_encounters::bosses::bone_overlord::bone_phase_registry` |
| `crates/gameplay/monster-ai/src/monster_combat_lane/source_time_profile_bundle.rs` | `crates/gameplay/encounters/src/bosses/native_combat/monster_combat_lane.rs` | `oteryn_gameplay_encounters::bosses::native_combat::monster_combat_lane::TimeProfileBundle` |
| `crates/gameplay/monster-ai/src/smelly_cheese.rs` | `crates/gameplay/encounters/src/bosses/smelly_cheese/timers.rs` | `oteryn_gameplay_encounters::bosses::smelly_cheese::timers` |
| `crates/gameplay/monster-ai/src/smelly_damage.rs` | `crates/gameplay/encounters/src/bosses/smelly_cheese/smelly_damage.rs` | `oteryn_gameplay_encounters::bosses::smelly_cheese::smelly_damage` |
| `crates/gameplay/monster-ai/src/smelly_encounter_dispatch.rs` | `crates/gameplay/encounters/src/bosses/smelly_cheese/smelly_encounter_dispatch.rs` | `oteryn_gameplay_encounters::bosses::smelly_cheese::smelly_encounter_dispatch` |
| `crates/gameplay/monster-ai/src/weak_spot_speech.rs` | `crates/gameplay/encounters/src/bosses/crystal_deaths/crystal_death_composition/speech.rs` | `oteryn_gameplay_encounters::bosses::crystal_deaths::crystal_death_composition::speech` |
| `crates/gameplay/social/src/interaction/mod.rs` | `crates/gameplay/domain/src/domain/interaction/mod.rs` | `oteryn_gameplay_domain::domain::interaction` |
| `crates/gameplay/social/src/security/release_policy.rs` | `crates/gameplay/social-rules/src/release_policy.rs` | `oteryn_gameplay_social_rules::release_policy` |
| `crates/gameplay/social/src/security/trust_policy.rs` | `crates/gameplay/social-rules/src/trust_policy.rs` | `oteryn_gameplay_social_rules::trust_policy` |
| `crates/gameplay/social/src/wheel_gem_data.rs` | `crates/gameplay/progression/src/wheel_gem_data.rs` | `oteryn_gameplay_progression::wheel_gem_data` |
| `crates/gameplay/spell/src/spell/authoring.rs` | `crates/gameplay/content/src/content/spell_data/authoring.rs` | `oteryn_gameplay_content::content::spell_data::authoring` |
| `crates/gameplay/spell/src/spell/callback_speech.rs` | `crates/gameplay/encounters/src/bosses/source_callbacks/callback_speech.rs` | `oteryn_gameplay_encounters::bosses::source_callbacks::callback_speech` |
| `crates/gameplay/spell/src/spell/locate.rs` | `crates/gameplay/spell-rules/src/locate.rs` | `oteryn_gameplay_spell_rules::locate` |
| `crates/gameplay/spell/src/spell/mana_training/hit_training.rs` | `crates/gameplay/progression/src/hit_training.rs` | `oteryn_gameplay_progression::hit_training` |
| `crates/gameplay/spell/src/spell/native_actor_states.rs` | `crates/gameplay/spell-rules/src/native_actor_states.rs` | `oteryn_gameplay_spell_rules::native_actor_states` |
| `crates/gameplay/spell/src/spell/native_companions.rs` | `crates/gameplay/spell-rules/src/native_companions.rs` | `oteryn_gameplay_spell_rules::native_companions` |
| `crates/gameplay/spell/src/spell/native_items.rs` | `crates/gameplay/spell-rules/src/native_items.rs` | `oteryn_gameplay_spell_rules::native_items` |
| `crates/gameplay/spell/src/spell/world_items_execution.rs` | `crates/gameplay/durability/src/durability/native_world_items.rs` | `oteryn_gameplay_durability::durability::native_world_items` |
| `crates/gameplay/world/src/house_scope/handoff.rs` | `crates/gameplay/housing/src/house_scope/handoff.rs` | `oteryn_gameplay_housing::house_scope::handoff` |

`crates/gameplay/encounters/src/lib.rs` also drops the former inline `foundation` facade; callers import kernel/carrier items or the specific boss owner. The Time Guardian type is physically included from `crates/gameplay/encounters/src/bosses/time_guardian/source_time_profile_bundle.rs` into the native combat lane.

## Retained shims and API surfaces

- `crates/gameplay/foundation/src/foundation/charm_data.rs:2`, `loot_data.rs:1`, and `crates/gameplay/content/src/content/spell_wheel_profile.rs:2`: content-facing kernel items belong to y2. Content and its foundation dependency remain untouched.
- `crates/gameplay/combat/src/ability/occurrence.rs:1`: the Ability module documents an established standalone fixture facade; the source-mounted fixture requires occurrence, revisions and `AbilityError` from the same local `ability_data` implementation. Redirecting only occurrence to the external kernel causes incompatible nominal Rust types. Retained this one coherent fixture API, with all tests unchanged.
- `crates/gameplay/entities/src/lib.rs:8`: its public mixed kernel/carrier API includes the content-facing kernel surface owned by y2. Explicit kernel callers were redirected where possible; the wildcard stays for public consumers and the concurrent y2 move.
- The server’s twelve public module aliases remain deliberate native composition/qualification APIs. Private forwarding exports were changed into ordinary implementation imports. Source-mounted qualification interfaces keep their tests and locally compiled types.
- `crates/gameplay/combat-rules/Cargo.toml:15` and `:17`: retained the otherwise-unused characters/items edges because the y1-owned `[gameplay_same_layer_edges]` requires them to exist. Removing them before y1 updates that section makes the architecture checker reject its exception table.

## Complete inventory

Inventory at the original head: 80 forwarding-only files or inline facade modules, plus 247 private forwarding names and 12 stable server module aliases. The inventory includes canonical public APIs with owned implementation modules, so the 80 rows are candidates, not 80 temporary shims. The original stamina facade is included even though it forwarded through a local child shim.

| Path / original line | Kind | Disposition |
|---|---|---|
| `apps/game-server/src/lifted_tests/domain_death_tests.rs:6` `parent_imports` | inline module | kept: deliberate source-mounted qualification interface |
| `apps/game-server/src/lifted_tests/domain_timed_item_host_tests.rs:5` `parent_imports` | inline module | kept: deliberate source-mounted qualification interface |
| `apps/game-server/tests/ability_engine.rs:6` `domain` | inline module | kept: deliberate source-mounted qualification interface |
| `apps/game-server/tests/item_timed_postgres.rs:5` | file | kept: source-mounted qualification interface; mounts real modules and their tests |
| `apps/game-server/tests/support/content_shim.rs:33` | file | kept: source-mounted qualification interface; mounts real modules and their tests |
| `apps/game-server/tests/support/domain_shim.rs:2` | file | kept: source-mounted qualification interface; mounts real modules and their tests |
| `crates/gameplay/combat-rules/src/formula.rs:6` | file | removed; callers name the owning crate/module |
| `crates/gameplay/combat/src/ability/condition.rs:3` | file | removed; callers name the owning crate/module |
| `crates/gameplay/combat/src/ability/field_condition.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/combat/src/ability/occurrence.rs:1` | file | kept: documented standalone Ability API preserves local AbilityError/occurrence type identity |
| `crates/gameplay/combat/src/combat/attack/block.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/combat/src/combat/attack/constants.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/combat/src/combat/attack/formulas.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/combat/src/combat/attack/target.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/combat/src/combat/chain.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/combat/src/combat/charm_effects.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/combat/src/combat/formula.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/combat/src/combat/loot_plan.rs:1` | file | removed; callers name the owning crate/module |
| `crates/gameplay/combat/src/combat/mod.rs:20` | file | kept: canonical public module API with owned implementations |
| `crates/gameplay/combat/src/combat/pvp/legality.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/combat/src/creature_attack_geometry.rs:3` | file | removed; callers name the owning crate/module |
| `crates/gameplay/combat/src/creature_icicle.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/combat/src/lib.rs:15` | file | kept: canonical public module API with owned implementations |
| `crates/gameplay/content/src/content/spell_wheel_profile.rs:2` | file | kept: content-facing foundation surface is owned by y2 |
| `crates/gameplay/content/src/lib.rs:6` | file | kept: content-facing foundation surface is owned by y2 |
| `crates/gameplay/dispatch/src/gameplay_transport/highscores.rs:5` | file | kept: canonical public module API with owned implementations |
| `crates/gameplay/dispatch/src/gameplay_transport/look.rs:1` | file | removed; callers name the owning crate/module |
| `crates/gameplay/dispatch/src/gameplay_transport/world_object.rs:8` | file | removed; callers name the owning crate/module |
| `crates/gameplay/dispatch/src/lib.rs:6` | file | kept: canonical public module API with owned implementations |
| `crates/gameplay/dispatch/src/welter_consume.rs:6` | file | removed; callers name the owning crate/module |
| `crates/gameplay/durability/src/durability/character_authority_audit.rs:3` | file | removed; callers name the owning crate/module |
| `crates/gameplay/durability/src/durability/character_vitals_soul.rs:6` | file | removed; callers name the owning crate/module |
| `crates/gameplay/durability/src/durability/character_vitals_stamina.rs:7` | file | removed; callers name the owning crate/module |
| `crates/gameplay/durability/src/durability/character_vitals_stamina/stamina_math.rs:5` | file | removed; callers name the owning crate/module |
| `crates/gameplay/durability/src/durability/creature_schedule.rs:1` | file | removed; callers name the owning crate/module |
| `crates/gameplay/durability/src/durability/item_capacity.rs:1` | file | removed; callers name the owning crate/module |
| `crates/gameplay/durability/src/durability/quest_state/quest/mod.rs:3` | file | removed; callers name the owning crate/module |
| `crates/gameplay/durability/src/durability/spell_wheel_abi.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/durability/src/durability/uniform_draw.rs:1` | file | removed; callers name the owning crate/module |
| `crates/gameplay/encounters/src/bosses/time_guardian/mod.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/encounters/src/bosses/weak_spot/weak_spot_speech.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/encounters/src/crystal_callbacks.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/encounters/src/crystal_death_composition.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/encounters/src/crystal_herald_quest.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/encounters/src/lib.rs:43` `foundation` | inline module | removed; callers name the owning crate/module |
| `crates/gameplay/encounters/src/monster_combat_lane.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/encounters/src/source_callback_cast.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/encounters/src/source_callback_intent.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/encounters/src/source_callback_spawn.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/encounters/src/source_encounter_seven.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/encounters/src/source_threshold_heal.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/entities/src/lib.rs:6` | file | kept: canonical public module API with owned implementations |
| `crates/gameplay/foundation/src/foundation/charm_data.rs:2` | file | kept: content-facing foundation surface is owned by y2 |
| `crates/gameplay/foundation/src/foundation/loot_data.rs:1` | file | kept: content-facing foundation surface is owned by y2 |
| `crates/gameplay/foundation/src/lib.rs:5` | file | kept: canonical public module API with owned implementations |
| `crates/gameplay/monster-ai/src/bone_phase_registry.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/monster-ai/src/lib.rs:13` | file | kept: canonical public module API with owned implementations |
| `crates/gameplay/monster-ai/src/monster_combat_lane/source_time_profile_bundle.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/monster-ai/src/smelly_cheese.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/monster-ai/src/smelly_damage.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/monster-ai/src/smelly_encounter_dispatch.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/monster-ai/src/weak_spot_speech.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/npc/src/lib.rs:12` | file | kept: canonical public module API with owned implementations |
| `crates/gameplay/social/src/chat/mod.rs:19` | file | kept: canonical public module API with owned implementations |
| `crates/gameplay/social/src/interaction/mod.rs:5` | file | removed; callers name the owning crate/module |
| `crates/gameplay/social/src/lib.rs:12` | file | kept: canonical public module API with owned implementations |
| `crates/gameplay/social/src/security/release_policy.rs:3` | file | removed; callers name the owning crate/module |
| `crates/gameplay/social/src/security/trust_policy.rs:3` | file | removed; callers name the owning crate/module |
| `crates/gameplay/social/src/wheel_gem_data.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/spell/src/lib.rs:8` | file | kept: canonical public module API with owned implementations |
| `crates/gameplay/spell/src/spell/authoring.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/spell/src/spell/callback_speech.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/spell/src/spell/locate.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/spell/src/spell/mana_training/hit_training.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/spell/src/spell/native_actor_states.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/spell/src/spell/native_companions.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/spell/src/spell/native_items.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/spell/src/spell/world_items_execution.rs:2` | file | removed; callers name the owning crate/module |
| `crates/gameplay/world/src/house_scope/handoff.rs:3` | file | removed; callers name the owning crate/module |
| `crates/gameplay/world/src/lib.rs:12` | file | kept: canonical public module API with owned implementations |

Stable Game Server aliases:

| Original path:line | Export | Reason |
|---|---|---|
| `apps/game-server/src/lib.rs:87` | `oteryn_gameplay_character_recovery_fence as character_recovery_fence` | Native composition and qualification public API |
| `apps/game-server/src/lib.rs:88` | `oteryn_gameplay_content::content` | Native composition and qualification public API |
| `apps/game-server/src/lib.rs:90` | `oteryn_gameplay_domain::domain` | Native composition and qualification public API |
| `apps/game-server/src/lib.rs:91` | `oteryn_gameplay_durability::character_bootstrap_intent` | Native composition and qualification public API |
| `apps/game-server/src/lib.rs:92` | `oteryn_gameplay_durability::durability` | Native composition and qualification public API |
| `apps/game-server/src/lib.rs:93` | `oteryn_gameplay_entities as foundation` | Native composition and qualification public API |
| `apps/game-server/src/lib.rs:100` | `oteryn_gameplay_world::house_scope` | Native composition and qualification public API |
| `apps/game-server/src/lib.rs:101` | `oteryn_gameplay_world::map` | Native composition and qualification public API |
| `apps/game-server/src/lib.rs:109` | `oteryn_gameplay_social::premium` | Native composition and qualification public API |
| `apps/game-server/src/lib.rs:110` | `oteryn_gameplay_social::wheel_gem_data` | Native composition and qualification public API |
| `apps/game-server/src/lib.rs:731` | `oteryn_gameplay_admission_evidence as admission_evidence` | Native composition and qualification public API |
| `apps/game-server/src/lib.rs:732` | `oteryn_gameplay_native_admission_source as native_admission_source` | Native composition and qualification public API |

Private forwarding names converted to private imports:

| File | Names |
|---|---|
| `apps/game-server/src/lib.rs` | `ability`, `player_lethal`, `ai`, `combat`, `ai_monster_melee`, `ai_think`, `combat_pickup`, `combat_attack`, `combat_pvp`, `party`, `interaction_chest_use`, `achievement_catalogue`, `gameplay_transport`, `chat`, `movement`, `spell`, `interaction`, `world_object_revert`, `world_runtime`, `creature_appearance_content`, `creature_area_heal`, `creature_attack_geometry`, `creature_auto_attack`, `creature_chain_attack`, `creature_condition_content`, `creature_defense_presentation`, `bone_phase_registry`, `crystal_callbacks`, `crystal_death_composition`, `crystal_herald_quest`, `self_heal_qualification`, `smelly_cheese`, `smelly_encounter_dispatch`, `source_callback_intent`, `source_callback_spawn`, `creature_damage_spell`, `creature_damage_spell_delayed`, `smelly_damage`, `source_encounter_seven`, `source_threshold_heal`, `weak_spot_speech`, `world_clock`, `monster_combat_lane`, `monster_owner_cycle`, `monster_summon`, `creature_icicle`, `security`, `source_callback_cast` |
| `crates/gameplay/combat/src/lib.rs` | `content`, `domain`, `character_bootstrap_intent`, `durability`, `foundation`, `house_scope`, `map`, `movement`, `world_clock`, `world_object_revert`, `world_runtime`, `source_critical_baseline` |
| `crates/gameplay/dispatch/src/lib.rs` | `admission_evidence`, `character_recovery_fence`, `ability`, `combat`, `combat_attack`, `combat_pickup`, `combat_pvp`, `creature_appearance_content`, `creature_area_heal`, `creature_attack_geometry`, `creature_auto_attack`, `creature_chain_attack`, `creature_condition_content`, `creature_defense_presentation`, `creature_spell_facts`, `player_lethal`, `content`, `domain`, `character_bootstrap_intent`, `durability`, `bone_phase_registry`, `creature_icicle`, `crystal_callbacks`, `crystal_death_composition`, `crystal_herald_quest`, `monster_combat_lane`, `self_heal_qualification`, `smelly_cheese`, `smelly_damage`, `smelly_encounter_dispatch`, `source_callback_cast`, `source_callback_intent`, `source_callback_spawn`, `source_encounter_seven`, `source_threshold_heal`, `weak_spot_speech`, `foundation`, `ai`, `ai_monster_melee`, `ai_think`, `monster_owner_cycle`, `monster_summon`, `native_admission_source`, `account_characters`, `character_commands`, `descriptor`, `http1_mtls`, `public_projections`, `runtime_status`, `scope_assignment`, `operator_files`, `secure_file`, `achievement_catalogue`, `chat`, `interaction`, `interaction_chest_use`, `party`, `premium`, `security`, `wheel_gem_data`, `creature_damage_spell`, `creature_damage_spell_delayed`, `spell`, `house_scope`, `map`, `movement`, `world_clock`, `world_object_revert`, `world_runtime`, `world_weather` |
| `crates/gameplay/durability/src/lib.rs` | `admission_evidence`, `character_recovery_fence`, `content`, `domain`, `foundation`, `native_admission_source` |
| `crates/gameplay/encounters/src/lib.rs` | `ability`, `combat`, `creature_appearance_content`, `creature_area_heal`, `creature_attack_geometry`, `creature_auto_attack`, `creature_chain_attack`, `creature_condition_content`, `creature_defense_presentation`, `content`, `domain`, `character_bootstrap_intent`, `durability`, `creature_damage_spell`, `creature_damage_spell_delayed`, `spell`, `house_scope`, `map`, `movement`, `world_clock`, `world_object_revert`, `world_runtime`, `player_lethal`, `source_critical_baseline`, `ai_think`, `monster_owner_cycle`, `monster_summon`, `creature_source_items` |
| `crates/gameplay/entities/src/lib.rs` | `domain` |
| `crates/gameplay/foundation/src/lib.rs` | `domain` |
| `crates/gameplay/monster-ai/src/lib.rs` | `creature_source_items`, `ability`, `combat`, `creature_appearance_content`, `creature_area_heal`, `creature_attack_geometry`, `creature_auto_attack`, `creature_chain_attack`, `creature_condition_content`, `creature_defense_presentation`, `content`, `character_bootstrap_intent`, `durability`, `foundation`, `creature_damage_spell`, `creature_damage_spell_delayed`, `spell`, `house_scope`, `map`, `movement`, `world_clock`, `world_object_revert`, `world_runtime`, `player_lethal` |
| `crates/gameplay/npc/src/lib.rs` | `content`, `domain`, `foundation`, `map`, `movement` |
| `crates/gameplay/social/src/lib.rs` | `ability`, `combat`, `creature_appearance_content`, `creature_area_heal`, `creature_attack_geometry`, `creature_auto_attack`, `creature_chain_attack`, `creature_condition_content`, `creature_defense_presentation`, `content`, `domain`, `character_bootstrap_intent`, `durability`, `foundation`, `house_scope`, `map`, `movement`, `world_clock`, `world_object_revert`, `world_runtime`, `combat_pickup` |
| `crates/gameplay/spell/src/lib.rs` | `native_world_items`, `native_world_read`, `ability`, `combat`, `creature_appearance_content`, `creature_area_heal`, `creature_attack_geometry`, `creature_auto_attack`, `creature_chain_attack`, `creature_condition_content`, `creature_defense_presentation`, `content`, `domain`, `character_bootstrap_intent`, `durability`, `foundation`, `house_scope`, `map`, `movement`, `world_clock`, `world_object_revert`, `world_runtime`, `player_lethal`, `source_critical_baseline` |
| `crates/gameplay/world/src/lib.rs` | `native_world_items`, `native_world_read`, `content`, `domain`, `character_bootstrap_intent`, `durability`, `foundation` |

## Dependency/edge removals

No direct dependency was added. `[gameplay_same_layer_edges]`, `[gameplay_rules]`, ADR-0022 and content’s foundation dependency were not edited.

| Consumer | Removed normal dependencies / edges |
|---|---|
| `oteryn-gameplay-combat` | `oteryn-gameplay-characters`, `oteryn-gameplay-progression` |
| `oteryn-gameplay-combat-rules` | `oteryn-gameplay-domain` |
| `oteryn-gameplay-encounters` | `oteryn-gameplay-combat-rules`, `oteryn-gameplay-spell-rules` |
| `oteryn-gameplay-monster-ai` | `oteryn-gameplay-combat-rules`, `oteryn-gameplay-spell-rules`, `oteryn-gameplay-encounter-rules` |
| `oteryn-gameplay-npc` | `oteryn-gameplay-characters`, `oteryn-gameplay-foundation`, `oteryn-gameplay-items`, `oteryn-gameplay-quests`, `oteryn-gameplay-social-rules` |
| `oteryn-gameplay-social` | `oteryn-gameplay-characters`, `oteryn-gameplay-items`, `oteryn-gameplay-quests` |
| `oteryn-gameplay-spell` | `oteryn-gameplay-characters`, `oteryn-gameplay-items`, `oteryn-gameplay-housing` |

## Architecture findings left to the parallel wave

The architecture check reports the following 20 edges. A read-only comparison with `git show HEAD:workspace-boundaries.toml` confirms every reported edge existed at the starting head. No new forbidden edge was introduced. The L4 acceptance table is y1’s scope; the content-to-foundation dependency is y2’s scope.

| From | To | Wave owner |
|---|---|---|
| `oteryn-gameplay-combat` | `oteryn-gameplay-durability` | y1 |
| `oteryn-gameplay-combat` | `oteryn-gameplay-world` | y1 |
| `oteryn-gameplay-content` | `oteryn-gameplay-foundation` | y2 |
| `oteryn-gameplay-encounters` | `oteryn-gameplay-combat` | y1 |
| `oteryn-gameplay-encounters` | `oteryn-gameplay-durability` | y1 |
| `oteryn-gameplay-encounters` | `oteryn-gameplay-monster-ai` | y1 |
| `oteryn-gameplay-encounters` | `oteryn-gameplay-spell` | y1 |
| `oteryn-gameplay-encounters` | `oteryn-gameplay-world` | y1 |
| `oteryn-gameplay-monster-ai` | `oteryn-gameplay-combat` | y1 |
| `oteryn-gameplay-monster-ai` | `oteryn-gameplay-durability` | y1 |
| `oteryn-gameplay-monster-ai` | `oteryn-gameplay-spell` | y1 |
| `oteryn-gameplay-monster-ai` | `oteryn-gameplay-world` | y1 |
| `oteryn-gameplay-npc` | `oteryn-gameplay-world` | y1 |
| `oteryn-gameplay-social` | `oteryn-gameplay-combat` | y1 |
| `oteryn-gameplay-social` | `oteryn-gameplay-durability` | y1 |
| `oteryn-gameplay-social` | `oteryn-gameplay-world` | y1 |
| `oteryn-gameplay-spell` | `oteryn-gameplay-combat` | y1 |
| `oteryn-gameplay-spell` | `oteryn-gameplay-durability` | y1 |
| `oteryn-gameplay-spell` | `oteryn-gameplay-world` | y1 |
| `oteryn-gameplay-world` | `oteryn-gameplay-durability` | y1 |

## Validation

- Workspace check: PASS.
- Workspace clippy, all targets, `-D warnings`: PASS.
- `cargo +1.95.0 fmt --all` and final `--check`: PASS.
- Library listing: PASS, 2,786 -> 2,786 across thirteen crates.
- Architecture: FAIL only for the 20 unchanged y1/y2 edges listed above.
- Initial PG execution found the wrapper’s `postgres://` port 55432 URL rejected by the unchanged tests’ exact port 5432 guard. A temporary loopback forwarding runner is used with an explicit native target so Cargo invokes it. It normalizes the equivalent scheme to `postgresql://`, changes no repository test or guard, and prints no URL or credential.
- Library execution: PASS across all thirteen crates: 2,759 passed, 27 pre-existing ignored, zero failures. Twelve packages passed the batched run; the complete server library passed serially after one transient parallel fixture rejection.
- Focused integration execution: 128 passed, with one bank retention failure reproduced unchanged at the starting HEAD. Pure targets: 112 passed; chest PG: 8; bank PG: 5; pickup PG: 3.
- Sandbox-only failures: none. PostgreSQL tests connected to the configured test PostgreSQL 17.6 through the forwarding runner; none was excused as missing environment.

## Pre-existing bank retention failure

`apps/game-server/tests/support/bank_postgres_cases.rs:47` fixes `OCCURRED_AT` at `1_790_000_000_000` (2026-09-21 14:13:20 UTC). Its P30D expiry is 2026-10-21 14:13:20 UTC, before this run. At `:1137`, the “before expiry” assertion consequently deletes two events instead of zero.

The identical test fails with `left: 2`, `right: 0` in the changed tree and in an immutable `git archive HEAD` snapshot of `5e3f585d4`, built with a separate target directory. The test, bank implementation and bank audit implementation are byte-identical to HEAD; hashes are recorded in `test-results.json`. No `git stash`, checkout, reset, commit or push was used. The temporary snapshot and its build directory were removed after the proof. Bank behavior, SQL and the test were left unchanged because this is outside y3’s shim move.

Exact isolated baseline invocation (after recreating the HEAD snapshot inside `tmp/y3/baseline`):

```bash
/home/mole/oteryn-cache/codex-bin/cargo +1.95.0 \
  --config 'target.x86_64-unknown-linux-gnu.runner=["python3","/home/mole/oteryn-cache/rs-wt/rs-y3/tmp/y3/pg-runner.py"]' \
  test --manifest-path tmp/y3/baseline/Cargo.toml --target-dir tmp/y3/baseline-target \
  --target x86_64-unknown-linux-gnu -p oteryn-game-server --test bank_postgres \
  bank_postgres_cases::bank_audit_expires_after_p30d_unless_held_and_the_ledger_never_does \
  -- --exact --test-threads=1
```

## Exact test commands / filters

All commands use `/home/mole/oteryn-cache/codex-bin/cargo +1.95.0`, offline, without `--locked`. The forwarding runner is `tmp/y3/pg-runner.py`. Runtime execution uses:

```bash
cargo=/home/mole/oteryn-cache/codex-bin/cargo
runner='target.x86_64-unknown-linux-gnu.runner=["python3","/home/mole/oteryn-cache/rs-wt/rs-y3/tmp/y3/pg-runner.py"]'
"$cargo" +1.95.0 --config "$runner" test --target x86_64-unknown-linux-gnu \
  -p oteryn-game-server \
  -p oteryn-gameplay-combat \
  -p oteryn-gameplay-combat-rules \
  -p oteryn-gameplay-dispatch \
  -p oteryn-gameplay-durability \
  -p oteryn-gameplay-encounters \
  -p oteryn-gameplay-entities \
  -p oteryn-gameplay-foundation \
  -p oteryn-gameplay-monster-ai \
  -p oteryn-gameplay-npc \
  -p oteryn-gameplay-social \
  -p oteryn-gameplay-spell \
  -p oteryn-gameplay-world \
  --lib --no-fail-fast

# Complete server package rerun, with the same tests scheduled serially:
"$cargo" +1.95.0 --config "$runner" test --target x86_64-unknown-linux-gnu -p oteryn-game-server --lib -- --test-threads=1

"$cargo" +1.95.0 --config "$runner" test --target x86_64-unknown-linux-gnu -p oteryn-game-server \
  --test ability_engine --test interaction_workflow --test loop_vitals_stamina
"$cargo" +1.95.0 --config "$runner" test --target x86_64-unknown-linux-gnu -p oteryn-game-server --test chest_use_postgres chest_use_postgres_cases
"$cargo" +1.95.0 --config "$runner" test --target x86_64-unknown-linux-gnu -p oteryn-game-server --test bank_postgres bank_postgres_cases -- --test-threads=1
"$cargo" +1.95.0 --config "$runner" test --target x86_64-unknown-linux-gnu -p oteryn-game-server --test combat_pickup_postgres combat_pickup_postgres_cases
```

## Changed files

The complete list of 265 changed repository files and eight saved report artifacts is in [changed-files.txt](changed-files.txt). Inventory, counts, dependency removals, architecture findings and the final test results are saved alongside this report.

## Unfinished

Parallel y1/y2 boundary changes, followed by z1’s merged CI gates. The retained source-mounted Ability shim and y2-facing kernel shims are explicitly listed above. The pre-existing bank retention fixture needs its separate owner’s repair; its unchanged-HEAD reproduction is documented above. All y3 crate library tests pass.
