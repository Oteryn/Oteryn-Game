//! Staged codecs for reserved commands 178–180. These confer no runtime authority.
#![cfg_attr(not(test), allow(dead_code))]
use super::{
    Error, MAX_QUERY_BYTES, push_message_field, push_varint_field, read_bytes, read_varint,
    valid_key,
};

/// The client identifies only admitted content and the lever use occurrence. Party membership,
/// positions, levels, cooldowns and room coordinates are deliberately server-resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BossLeverRequest {
    pub boss_key: String,
}

pub(super) fn decode_boss_key(payload: &[u8]) -> Result<BossLeverRequest, Error> {
    if payload.len() > MAX_QUERY_BYTES {
        return Err(Error::LimitExceeded);
    }
    let mut at = 0;
    if read_varint(payload, &mut at)? != 10 {
        return Err(Error::Malformed);
    }
    let key = std::str::from_utf8(read_bytes(payload, &mut at)?).map_err(|_| Error::Malformed)?;
    if at != payload.len() || !valid_key(key) {
        return Err(Error::Malformed);
    }
    Ok(BossLeverRequest {
        boss_key: key.into(),
    })
}

pub(super) fn encode_boss_key(key: &str) -> Result<Vec<u8>, Error> {
    if !valid_key(key) {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_message_field(&mut out, 1, key.as_bytes());
    Ok(out)
}

/// Opaque fresh-instance receipt. It is presentation data, never an admission credential.
pub(super) fn encode_instance_receipt(
    instance: &[u8; 16],
    deadline_unix_ms: u64,
) -> Result<Vec<u8>, Error> {
    if deadline_unix_ms > i64::MAX as u64 {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_message_field(&mut out, 1, instance);
    push_varint_field(&mut out, 2, deadline_unix_ms);
    Ok(out)
}
