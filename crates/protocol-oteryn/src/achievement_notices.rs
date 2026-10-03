//! Earned-achievement notice payloads (ACH-NOTIFY-1).
//!
//! Schema: `docs/contracts/protocol-oteryn/v1/achievement_notices_v1.proto`, from ACHIEVEMENT-0 §5
//! (`ACHIEVEMENT0-GRANT-RUNTIME-V1`, D292). Capability 8 `ACHIEVEMENT_NOTICES_V1` and state domain
//! 13 `ACCOUNT_ACHIEVEMENT_NOTICES` are registered in `PROTOCOL_OTERYN_V1_REGISTRY.json`, leased by
//! the #1622 control plane. The capability has no command type. The domain's state is the
//! account's achievement watermark (`fact_count`, `total_points`). The snapshot carries only that
//! watermark, and one delta follows each committed `Granted` outcome.
//!
//! Decoding is strict, and encoding refuses the same values as a server fault before any byte is
//! emitted. The following all fail closed: a key outside the `oteryn:achievement/<slug>` grammar,
//! an empty or oversized name, invalid UTF-8, a delta with a zero `fact_count`, a payload over its
//! bound, and unknown or repeated fields.

use crate::account_achievements::{
    MAX_ACHIEVEMENT_KEY_BYTES, MAX_ACHIEVEMENT_NAME_BYTES, valid_achievement_key,
};
pub use crate::charm_wire::CyclopediaWireError as AchievementNoticesWireError;
use crate::charm_wire::{
    WireResult, push_message_field, push_nonzero_varint_field, read_bytes, read_uint32,
    read_uint32_fields, read_varint, set_once,
};

/// Registered capability `ACHIEVEMENT_NOTICES_V1`: no command type, state domain 13.
pub const CAPABILITY_ACHIEVEMENT_NOTICES_V1: u32 = 8;
/// Registered state domain `ACCOUNT_ACHIEVEMENT_NOTICES` (capability 8).
pub const STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES: u32 = 13;
/// Delta type 1 `ACHIEVEMENT_EARNED_DELTA_V1`: one earned achievement.
pub const DELTA_TYPE_ACHIEVEMENT_EARNED_V1: u32 = 1;
/// Snapshot type 1 `ACHIEVEMENT_NOTICES_SNAPSHOT_V1`: the watermark only.
pub const SNAPSHOT_TYPE_ACHIEVEMENT_NOTICES_V1: u32 = 1;

/// §5: at most 250 bytes. Worst case 241: key 1 + 2 + 160, name 1 + 1 + 64, two uint32 1 + 5.
pub const MAX_ACHIEVEMENT_EARNED_BYTES: usize = 250;
/// §5: at most 16 bytes. Worst case 12: two uint32 1 + 5.
pub const MAX_ACHIEVEMENT_NOTICES_SNAPSHOT_BYTES: usize = 16;

const _: () = assert!(MAX_ACHIEVEMENT_EARNED_BYTES <= crate::MAX_STATE_DELTA_PAYLOAD_BYTES);

/// `AchievementNoticesSnapshotV1`, and the watermark a delta carries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AchievementWatermark {
    pub fact_count: u32,
    pub total_points: u32,
}

/// `AchievementEarnedV1`: one earned achievement and the account watermark after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AchievementEarned {
    pub key: String,
    pub name: String,
    pub watermark: AchievementWatermark,
}

fn check_earned(earned: &AchievementEarned) -> WireResult<()> {
    if earned.key.len() > MAX_ACHIEVEMENT_KEY_BYTES
        || earned.name.len() > MAX_ACHIEVEMENT_NAME_BYTES
    {
        return Err(AchievementNoticesWireError::LimitExceeded);
    }
    // A delta follows a grant, so the account holds at least that fact.
    if !valid_achievement_key(&earned.key)
        || earned.name.is_empty()
        || earned.watermark.fact_count == 0
    {
        return Err(AchievementNoticesWireError::Malformed);
    }
    Ok(())
}

/// Encodes the snapshot (type 1) payload.
pub fn encode_achievement_notices_snapshot(watermark: AchievementWatermark) -> Vec<u8> {
    let mut output = Vec::with_capacity(MAX_ACHIEVEMENT_NOTICES_SNAPSHOT_BYTES);
    push_nonzero_varint_field(&mut output, 1, u64::from(watermark.fact_count));
    push_nonzero_varint_field(&mut output, 2, u64::from(watermark.total_points));
    output
}

pub fn decode_achievement_notices_snapshot(payload: &[u8]) -> WireResult<AchievementWatermark> {
    if payload.len() > MAX_ACHIEVEMENT_NOTICES_SNAPSHOT_BYTES {
        return Err(AchievementNoticesWireError::LimitExceeded);
    }
    let [fact_count, total_points] = read_uint32_fields::<2>(payload)?;
    Ok(AchievementWatermark {
        fact_count,
        total_points,
    })
}

/// Encodes the earned delta (type 1). A notice outside the bounds is a server fault.
pub fn encode_achievement_earned(earned: &AchievementEarned) -> WireResult<Vec<u8>> {
    check_earned(earned)?;
    let mut output = Vec::with_capacity(MAX_ACHIEVEMENT_EARNED_BYTES);
    push_message_field(&mut output, 1, earned.key.as_bytes());
    push_message_field(&mut output, 2, earned.name.as_bytes());
    push_nonzero_varint_field(&mut output, 3, u64::from(earned.watermark.fact_count));
    push_nonzero_varint_field(&mut output, 4, u64::from(earned.watermark.total_points));
    Ok(output)
}

pub fn decode_achievement_earned(payload: &[u8]) -> WireResult<AchievementEarned> {
    if payload.len() > MAX_ACHIEVEMENT_EARNED_BYTES {
        return Err(AchievementNoticesWireError::LimitExceeded);
    }
    let (mut key, mut name, mut fact_count, mut total_points) = (None, None, None, None);
    let mut cursor = 0;
    while cursor < payload.len() {
        let field = read_varint(payload, &mut cursor)?;
        match (field >> 3, field & 0x07) {
            (1, 2) => set_once(&mut key, read_text(payload, &mut cursor)?)?,
            (2, 2) => set_once(&mut name, read_text(payload, &mut cursor)?)?,
            (3, 0) => set_once(&mut fact_count, read_uint32(payload, &mut cursor)?)?,
            (4, 0) => set_once(&mut total_points, read_uint32(payload, &mut cursor)?)?,
            _ => return Err(AchievementNoticesWireError::Malformed),
        }
    }
    let earned = AchievementEarned {
        key: key.unwrap_or_default(),
        name: name.unwrap_or_default(),
        watermark: AchievementWatermark {
            fact_count: fact_count.unwrap_or(0),
            total_points: total_points.unwrap_or(0),
        },
    };
    check_earned(&earned)?;
    Ok(earned)
}

fn read_text(input: &[u8], cursor: &mut usize) -> WireResult<String> {
    let bytes = read_bytes(input, cursor)?;
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| AchievementNoticesWireError::Malformed)
}

#[cfg(test)]
#[path = "achievement_notices_tests.rs"]
mod tests;
