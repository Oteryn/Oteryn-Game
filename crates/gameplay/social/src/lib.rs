//! ADR-0019 gameplay crate `oteryn-gameplay-social`: modules moved verbatim from `oteryn-game-server` (re-exported there at the old path).

pub mod chat;
pub mod party;
pub mod premium;
pub mod interaction;
pub mod wheel_gem_data;
pub mod achievement_catalogue;
pub mod security;

#[allow(unused_imports)] pub(crate) use oteryn_gameplay_foundation::foundation;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_content::content;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_durability::character_bootstrap_intent;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_durability::durability;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_world::map;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_world::movement;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_world::world_runtime;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_world::world_object_revert;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_world::world_clock;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_world::house_scope;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_combat::ability;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_combat::combat;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_combat::creature_appearance_content;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_combat::creature_area_heal;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_combat::creature_attack_geometry;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_combat::creature_auto_attack;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_combat::creature_chain_attack;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_combat::creature_condition_content;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_combat::creature_defense_presentation;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_combat::creature_icicle;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_domain::domain;
