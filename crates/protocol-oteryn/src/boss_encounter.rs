//! BOSS-ENCOUNTER-1 server cooldown-list command. Identity is the admitted Character.
use crate::charm_wire::{
    CyclopediaWireError as Error, push_message_field, push_varint_field, read_bytes, read_varint,
};

pub const CAPABILITY_BOSS_ENCOUNTER_V1: u32 = 117;
pub const COMMAND_TYPE_BOSS_COOLDOWNS: u32 = 133;
pub const MAX_BOSS_KEY_BYTES: usize = 512;
pub const COOLDOWN_PAGE_SIZE: usize = 64;
pub const MAX_QUERY_BYTES: usize = 515;
pub const MAX_RESULT_BYTES: usize = 34_400;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BossCooldownEntry {
    pub boss_key: String,
    pub available_at_unix_ms: u64,
}

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
