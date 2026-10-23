//! PostgreSQL-backed durability and native world adapters.

include!("core_modules.rs");

pub mod bed_sleep;
pub mod daily_reward;
pub mod daily_reward_claim;

pub mod creature_source_items;
pub mod mail_text_policy;

#[allow(dead_code)]
pub mod native_world_items;
#[allow(dead_code)]
pub mod native_world_read;

pub mod hunting_xp;
pub mod party_kill_xp;

pub mod raid_schedule;
