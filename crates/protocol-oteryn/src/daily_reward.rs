//! DAILY30B one-click daily rewards; the v1 allocation remains registered history.
//!
//! Offered by the composed server only with admitted reward content. Epoch,
//! vocation and Premium authority never come from the client.
//! Payload schemas: `docs/contracts/protocol-oteryn/v1/daily-reward-v2.md`.

pub const CAPABILITY_DAILY_REWARD_V1: u32 = 32;
pub const CAPABILITY_DAILY_REWARD_V2: u32 = 182;
pub const COMMAND_TYPE_DAILY_REWARD_CLAIM: u32 = 354;
pub const COMMAND_TYPE_DAILY_REWARD_QUERY: u32 = 355;
pub const STATE_DOMAIN_DAILY_REWARD: u32 = 24;
pub const MAX_DAILY_REWARD_CLAIM_BYTES: usize = 0;
pub const MAX_DAILY_REWARD_VIEW_BYTES: usize = 128;
pub const MAX_DAILY_REWARD_RESULT_BYTES: usize = 20;

use crate::charm_wire::{
    CyclopediaWireError, WireResult, push_message_field, push_varint_field, read_uint32_fields,
};

/// One click; the server resolves day, tier and every reward amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyRewardClaim;

impl DailyRewardClaim {
    pub fn decode(bytes: &[u8]) -> WireResult<Self> {
        decode_daily_reward_query(bytes)?;
        Ok(Self)
    }
}

/// Exactly the seven registered result codes. Only CLAIMED carries a UUIDv7.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyRewardResult {
    pub disposition: u8,
    pub transaction_id: Option<[u8; 16]>,
}

impl DailyRewardResult {
    pub fn encode(self) -> WireResult<Vec<u8>> {
        if !(1..=7).contains(&self.disposition)
            || (self.disposition == 1) != self.transaction_id.is_some()
            || self
                .transaction_id
                .is_some_and(|id| id[6] >> 4 != 7 || id[8] >> 6 != 2)
        {
            return Err(CyclopediaWireError::Malformed);
        }
        let mut bytes = Vec::new();
        push_varint_field(&mut bytes, 1, u64::from(self.disposition));
        if let Some(id) = self.transaction_id {
            push_message_field(&mut bytes, 2, &id);
        }
        Ok(bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyRewardWall {
    pub lane_day: u8,
    pub streak: u32,
    pub jokers: u8,
    pub claimed: bool,
    pub claim_enabled: bool,
}

impl DailyRewardWall {
    /// V2 claim status; tier and reward amounts are resolved by the server.
    pub fn encode(self) -> WireResult<Vec<u8>> {
        if !(1..=30).contains(&self.lane_day) || self.streak > 100_000 || self.jokers > 3 {
            return Err(CyclopediaWireError::Malformed);
        }
        let mut bytes = Vec::new();
        for (field, value) in [
            u64::from(self.lane_day),
            u64::from(self.streak),
            u64::from(self.jokers),
            u64::from(self.claimed),
            u64::from(self.claim_enabled),
        ]
        .into_iter()
        .enumerate()
        {
            push_varint_field(&mut bytes, field as u64 + 1, value);
        }
        Ok(bytes)
    }
}

/// An empty message is the only query. It never carries claim authority.
pub fn decode_daily_reward_query(bytes: &[u8]) -> WireResult<()> {
    let [] = read_uint32_fields::<0>(bytes)?;
    Ok(())
}

#[cfg(test)]
#[path = "daily_reward_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "daily_reward_codec_tests.rs"]
mod codec_tests;
