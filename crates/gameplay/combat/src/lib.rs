//! ADR-0019 gameplay crate `oteryn-gameplay-combat`: modules moved verbatim from `oteryn-game-server` (re-exported there at the old path).

pub mod ability;
pub mod combat;
pub mod creature_appearance_content;
pub mod creature_area_heal;
pub mod creature_attack_geometry;
pub mod creature_auto_attack;
pub mod creature_chain_attack;
pub mod creature_condition_content;
pub mod creature_defense_presentation;
pub mod creature_icicle;

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
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_domain::domain;
