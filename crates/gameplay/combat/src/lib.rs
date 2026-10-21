//! ADR-0019 gameplay crate `oteryn-gameplay-combat`: modules moved verbatim from `oteryn-game-server` (re-exported there at the old path).

pub mod ability;
pub mod combat;
pub mod creature_appearance_content;
pub mod creature_area_heal;

pub mod creature_auto_attack;
pub mod creature_chain_attack;
pub mod creature_condition_content;
pub mod creature_defense_presentation;
pub mod creature_spell_facts;

#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_content::content;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_domain::domain;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_durability::character_bootstrap_intent;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_durability::durability;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_entities as foundation;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_world::house_scope;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_world::map;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_world::movement;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_world::world_clock;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_world::world_object_revert;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_world::world_runtime;

#[path = "combat/attack/mod.rs"]
pub mod combat_attack;
#[path = "combat/pickup.rs"]
pub mod combat_pickup;
#[allow(dead_code, reason = "PvP stays off until the PVP-0 go-live gate")]
#[path = "combat/pvp/mod.rs"]
pub mod combat_pvp;
#[path = "ability/player_lethal.rs"]
pub mod player_lethal;

use oteryn_gameplay_source_critical_baseline as source_critical_baseline;
