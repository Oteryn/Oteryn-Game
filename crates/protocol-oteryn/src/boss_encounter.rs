//! BOSS-ENCOUNTER-1 server cooldown-list command. Identity is the admitted Character.
use crate::charm_wire::{
    CyclopediaWireError as Error, push_message_field, push_varint_field, read_bytes, read_varint,
};

pub const CAPABILITY_BOSS_ENCOUNTER_V1: u32 = 117;
pub const COMMAND_TYPE_BOSS_COOLDOWNS: u32 = 133;
pub const COMMAND_TYPE_BOSS_LEVER: u32 = 178;
pub const COMMAND_TYPE_BOSS_DEFEATED: u32 = 179;
pub const COMMAND_TYPE_BOSS_TIMEOUT: u32 = 180;
/// Opens the admitted character's personal boss reward chest.
pub const COMMAND_TYPE_BOSS_REWARD_OPEN: u32 = 340;
pub const MAX_BOSS_KEY_BYTES: usize = 512;
pub const COOLDOWN_PAGE_SIZE: usize = 64;
pub const MAX_QUERY_BYTES: usize = 515;
pub const MAX_RESULT_BYTES: usize = 34_400;
pub const REWARD_CHEST_PAGE_SIZE: usize = 64;
pub const BOSS_REWARD_EXPIRY_MS: u64 = 7 * 24 * 60 * 60 * 1_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BossCooldownEntry {
    pub boss_key: String,
    pub available_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BossRewardChestEntry {
    pub death: [u8; 16],
    pub boss_key: String,
    pub minted_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub item_count: u32,
}

/// Encodes the private, oldest-first chest view. Expired rewards are never encoded by the
/// durable owner; the explicit deadline lets a client render the remaining lifetime.
pub fn encode_reward_chest(
    now_unix_ms: u64,
    entries: &[BossRewardChestEntry],
) -> Result<Vec<u8>, Error> {
    if entries.len() > REWARD_CHEST_PAGE_SIZE || now_unix_ms > i64::MAX as u64 {
        return Err(Error::LimitExceeded);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, now_unix_ms);
    for entry in entries {
        if !valid_key(&entry.boss_key)
            || entry.minted_at_unix_ms > entry.expires_at_unix_ms
            || entry
                .expires_at_unix_ms
                .saturating_sub(entry.minted_at_unix_ms)
                != BOSS_REWARD_EXPIRY_MS
            || entry.expires_at_unix_ms <= now_unix_ms
            || entry.item_count > 1_000
        {
            return Err(Error::Malformed);
        }
        let mut value = Vec::new();
        push_message_field(&mut value, 1, &entry.death);
        push_message_field(&mut value, 2, entry.boss_key.as_bytes());
        push_varint_field(&mut value, 3, entry.minted_at_unix_ms);
        push_varint_field(&mut value, 4, entry.expires_at_unix_ms);
        push_varint_field(&mut value, 5, u64::from(entry.item_count));
        push_message_field(&mut out, 2, &value);
    }
    if out.len() > MAX_RESULT_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}

// Reserved wire proposals; no production activity owner exists yet.
#[path = "boss_encounter_staged.rs"]
mod staged;

/// An empty cursor starts a list; subsequent cursors are the last key of a full page.
/// Unknown/repeated fields, invalid keys and oversized requests fail closed.
pub fn decode_cooldown_query(payload: &[u8]) -> Result<&str, Error> {
    if payload.len() > MAX_QUERY_BYTES {
        return Err(Error::LimitExceeded);
    }
    if payload.is_empty() {
        return Ok("");
    }
    let mut at = 0;
    if read_varint(payload, &mut at)? != 10 {
        return Err(Error::Malformed);
    }
    let key = std::str::from_utf8(read_bytes(payload, &mut at)?).map_err(|_| Error::Malformed)?;
    if at != payload.len() || (!key.is_empty() && !valid_key(key)) {
        return Err(Error::Malformed);
    }
    Ok(key)
}

fn valid_key(key: &str) -> bool {
    key.len() <= MAX_BOSS_KEY_BYTES
        && key
            .split_once(':')
            .is_some_and(|(namespace, local)| !namespace.is_empty() && !local.is_empty())
        && key
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b':' | b'_' | b'.' | b'-' | b'/'))
}

/// Absolute deadlines are reported without inventing a current boosted-boss/reset producer.
/// Expired rows remain visible; a zero deadline is eligible. No progression is written.
pub fn encode_cooldown_list(
    now_unix_ms: u64,
    entries: &[BossCooldownEntry],
    more: bool,
) -> Result<Vec<u8>, Error> {
    if entries.len() > COOLDOWN_PAGE_SIZE || (more && entries.len() != COOLDOWN_PAGE_SIZE) {
        return Err(Error::LimitExceeded);
    }
    if now_unix_ms > i64::MAX as u64
        || entries
            .iter()
            .any(|e| !valid_key(&e.boss_key) || e.available_at_unix_ms > i64::MAX as u64)
        || entries.windows(2).any(|e| e[0].boss_key >= e[1].boss_key)
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, now_unix_ms);
    for entry in entries {
        let mut value = Vec::new();
        push_message_field(&mut value, 1, entry.boss_key.as_bytes());
        push_varint_field(&mut value, 2, entry.available_at_unix_ms);
        push_message_field(&mut out, 2, &value);
    }
    if more {
        let last = entries.last().ok_or(Error::Malformed)?;
        push_message_field(&mut out, 3, last.boss_key.as_bytes());
    }
    if out.len() > MAX_RESULT_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}

#[cfg(test)]
#[path = "boss_encounter_tests.rs"]
mod tests;

#[cfg(test)]
mod reward_tests {
    use super::*;

    #[test]
    fn reward_chest_encodes_only_live_seven_day_personal_groups() {
        let now = 10;
        let minted = 1;
        let result = encode_reward_chest(
            now,
            &[BossRewardChestEntry {
                death: [7; 16],
                boss_key: "oteryn:boss.fixture".into(),
                minted_at_unix_ms: minted,
                expires_at_unix_ms: minted + BOSS_REWARD_EXPIRY_MS,
                item_count: 2,
            }],
        );
        assert!(result.is_ok());
        let bytes = result.unwrap_or_default();
        assert!(!bytes.is_empty());

        let mut expired = BossRewardChestEntry {
            death: [7; 16],
            boss_key: "oteryn:boss.fixture".into(),
            minted_at_unix_ms: minted,
            expires_at_unix_ms: now,
            item_count: 2,
        };
        assert_eq!(
            encode_reward_chest(now, &[expired.clone()]),
            Err(Error::Malformed)
        );
        expired.expires_at_unix_ms = minted + BOSS_REWARD_EXPIRY_MS - 1;
        assert_eq!(encode_reward_chest(now, &[expired]), Err(Error::Malformed));
    }
}
