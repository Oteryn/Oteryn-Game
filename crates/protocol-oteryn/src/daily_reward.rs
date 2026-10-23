//! DAILY_REWARD0 §7 wire reservations, allocated by the control plane on 2026-10-22.
//!
//! Offered by the composed server only with admitted reward content. Epoch,
//! vocation and Premium authority never come from the client.
//! Payload schemas: `docs/contracts/protocol-oteryn/v1/daily_reward_v1.proto`.

pub const CAPABILITY_DAILY_REWARD_V1: u32 = 32;
pub const COMMAND_TYPE_DAILY_REWARD_CLAIM: u32 = 32;
pub const COMMAND_TYPE_DAILY_REWARD_QUERY: u32 = 33;
pub const STATE_DOMAIN_DAILY_REWARD: u32 = 24;
pub const MAX_DAILY_REWARD_CLAIM_BYTES: usize = 10_500;
pub const MAX_DAILY_REWARD_VIEW_BYTES: usize = 128;
pub const MAX_DAILY_REWARD_RESULT_BYTES: usize = 20;

use crate::charm_wire::{
    CyclopediaWireError, WireResult, push_message_field, push_varint_field, read_bytes,
    read_uint32_fields, read_varint, set_once,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyRewardPick {
    pub definition: String,
    pub count: u32,
}

/// Sorted, distinct definition keys. Order on the wire is immaterial.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyRewardClaim {
    pub picks: Vec<DailyRewardPick>,
}

impl DailyRewardClaim {
    pub fn decode(bytes: &[u8]) -> WireResult<Self> {
        if bytes.len() > MAX_DAILY_REWARD_CLAIM_BYTES {
            return Err(CyclopediaWireError::LimitExceeded);
        }
        let mut cursor = 0;
        let mut picks = Vec::new();
        while cursor < bytes.len() {
            if read_varint(bytes, &mut cursor)? != 10 || picks.len() == 20 {
                return Err(CyclopediaWireError::Malformed);
            }
            let pick = read_bytes(bytes, &mut cursor)?;
            let (mut key, mut count) = (None, None);
            let mut at = 0;
            while at < pick.len() {
                match read_varint(pick, &mut at)? {
                    10 => {
                        let value = read_bytes(pick, &mut at)?;
                        if value.is_empty()
                            || value.len() > 512
                            || !value.iter().all(|b| b.is_ascii_graphic())
                        {
                            return Err(CyclopediaWireError::Malformed);
                        }
                        set_once(
                            &mut key,
                            String::from_utf8(value.to_vec())
                                .map_err(|_| CyclopediaWireError::Malformed)?,
                        )?;
                    }
                    16 => set_once(&mut count, read_varint(pick, &mut at)?)?,
                    _ => return Err(CyclopediaWireError::Malformed),
                }
            }
            let count = count
                .filter(|n| (1..=20).contains(n))
                .ok_or(CyclopediaWireError::Malformed)?;
            picks.push(DailyRewardPick {
                definition: key.ok_or(CyclopediaWireError::Malformed)?,
                count: count as u32,
            });
        }
        picks.sort_by(|a, b| a.definition.cmp(&b.definition));
        if picks.windows(2).any(|w| w[0].definition == w[1].definition) {
            return Err(CyclopediaWireError::Malformed);
        }
        Ok(Self { picks })
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
    /// Free rewards until PREM-3; each position has one bounded part.
    pub fn encode(self) -> WireResult<Vec<u8>> {
        if !(1..=7).contains(&self.lane_day) || self.streak > 100_000 || self.jokers > 3 {
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
        for (kind, quantity) in [(1, 5), (1, 5), (2, 1), (1, 10), (3, 1), (4, 1), (5, 10)] {
            let mut part = Vec::new();
            push_varint_field(&mut part, 1, kind);
            push_varint_field(&mut part, 2, quantity);
            push_message_field(&mut bytes, 6, &part);
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
