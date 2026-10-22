//! DAILY_REWARD0 §7 wire reservations, allocated by the control plane on 2026-10-22.
//!
//! The capability remains unoffered until the native claim, authored content,
//! bound-item lifecycle and committed World-reset history are composed. Registration
//! does not make an epoch, authorize a claim or activate Premium rewards.
//! Payload schemas: `docs/contracts/protocol-oteryn/v1/daily_reward_v1.proto`.

pub const CAPABILITY_DAILY_REWARD_V1: u32 = 32;
pub const COMMAND_TYPE_DAILY_REWARD_CLAIM: u32 = 32;
pub const COMMAND_TYPE_DAILY_REWARD_QUERY: u32 = 33;
pub const STATE_DOMAIN_DAILY_REWARD: u32 = 24;
pub const MAX_DAILY_REWARD_CLAIM_BYTES: usize = 10_500;
pub const MAX_DAILY_REWARD_VIEW_BYTES: usize = 128;
pub const MAX_DAILY_REWARD_RESULT_BYTES: usize = 20;

#[cfg(test)]
#[path = "daily_reward_tests.rs"]
mod tests;
