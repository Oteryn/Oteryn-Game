//! Offline item promotion and protected import adapters.

pub(crate) use oteryn_gameplay_content::content::*;

mod cw2_b1_import;
mod cw2_b4_import;
pub mod item_admission;
pub mod item_bed_promotion;
pub mod item_capacity_promotion;
pub mod item_consumption_promotion;
pub mod item_description_promotion;
pub mod item_description_wiki_promotion;
pub mod item_document_promotion;
pub mod item_elemental_magic_modifier_promotion;
pub mod item_forge289_promotion;
pub mod item_forge3332_promotion;
pub mod item_hit_magic_promotion;
pub mod item_identity;
pub mod item_mantra_bond_modifier_promotion;
pub mod item_market_true_promotion;
pub mod item_movable_promotion;
pub mod item_name15_promotion;
pub mod item_name_promotion;
pub mod item_numeric_modifier_promotion;
pub mod item_physical_promotion;
pub mod item_ranged_promotion;
pub mod item_stack_default_promotion;
pub mod item_stack_default_successor8_promotion;
pub mod item_stack_false_promotion;
pub mod item_stack_historical_promotion;
pub mod item_stats_promotion;
pub mod item_timed_promotion;
pub mod item_use_observation_promotion;
pub mod item_weapon_metadata_promotion;

pub use cw2_b1_import::*;
pub use cw2_b4_import::*;
