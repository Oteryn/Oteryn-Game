//! Compile the actual pure transport contract in the dedicated test target.
//! This avoids unrelated cfg(test) fixtures in the dispatch library.
#[path = "../../../../../crates/gameplay/dispatch/src/gameplay_transport/cosmetic_item_use/quest_targets.rs"]
mod routing_contract;

#[path = "../../../../../crates/gameplay/dispatch/src/gameplay_transport/mount_taming/quest_objects.rs"]
mod world_quest_tools;
