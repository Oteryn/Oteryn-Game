//! L3 rules: spell costs, cooldowns, target resolution, area geometry, native spell plans.

pub mod cast;
/// Pure plans used by the gameplay transport adapters.
pub mod dispatch_plans;
pub mod familiar_cast;
pub mod harmony;
#[cfg(test)]
mod harmony_tests;
pub mod locate;
pub mod native_actor_states;
pub mod native_companions;
pub mod native_items;
pub mod ordinary_combat;
pub mod parameter_cast;
pub mod plan;
pub mod target;

use oteryn_gameplay_content::content::spell_data::Vocation;
use oteryn_gameplay_content::content::spell_data::formula;
