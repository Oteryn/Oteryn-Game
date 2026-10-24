//! L3 rules: item stacking, capacity, transfer and placement, loot rolls, pickup/reach, corpses, examine text.

pub mod depot;
pub mod equipment_effects;
pub mod forge;
pub mod hotkey_search;
pub mod imbuement;
pub mod item_capacity;
pub mod item_craft;
pub mod item_timed_repair;
pub mod item_transfer;
pub mod item_weight_lookup;
pub mod look;
pub mod loot_plan;
pub mod mail;
pub mod mining;
pub mod pickup;
pub mod stacking;
pub mod starter_grant;

pub mod corpse_open;
pub mod item_ref_admission;
pub mod map_pickup;
pub mod native_map_items_abi;
pub mod native_world_items;
pub mod reach;

#[cfg(test)]
mod stacking_property_tests;

#[cfg(test)]
mod item_capacity_property_tests;

#[cfg(test)]
mod equipment_effects_property_tests;
