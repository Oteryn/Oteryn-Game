//! ADR-0019 gameplay crate `oteryn-gameplay-monster-ai`: modules moved verbatim from `oteryn-game-server` (re-exported there at the old path).

pub mod ai;
pub mod ai_monster_melee;
pub mod ai_think;
pub mod monster_combat_lane;
#[cfg(test)]
pub mod monster_lab;
pub mod monster_owner_cycle;
pub mod monster_summon;

#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use crate::durability::creature_source_items;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_combat::ability;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_combat::combat;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_combat::creature_appearance_content;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_combat::creature_area_heal;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_combat::creature_attack_geometry;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_combat::creature_auto_attack;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_combat::creature_chain_attack;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_combat::creature_condition_content;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_combat::creature_defense_presentation;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_content::content;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_durability::character_bootstrap_intent;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_durability::durability;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_entities as foundation;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_spell::creature_damage_spell;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_spell::creature_damage_spell_delayed;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_spell::spell;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_world::house_scope;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_world::map;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_world::movement;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_world::world_clock;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_world::world_object_revert;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_world::world_runtime;

pub(crate) use oteryn_gameplay_combat::player_lethal;
