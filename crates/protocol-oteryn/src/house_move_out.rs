//! HOUSE-1b owner notice/cancel and single-house disposition status.
use crate::charm_wire::{
    CyclopediaWireError as Error, push_message_field, push_varint_field, read_bytes, read_varint,
    set_once,
};

pub const CAPABILITY_HOUSE_MOVE_OUT_V1: u32 = 59;
pub const COMMAND_TYPE_HOUSE_MOVE_OUT: u32 = 76;
pub const COMMAND_TYPE_HOUSE_MOVE_OUT_QUERY: u32 = 77;
pub const MAX_REQUEST_BYTES: usize = 544;
pub const MAX_REPLY_BYTES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseMoveOutRequest {
    pub house: String,
    /// 0 query, 1 absolute notice, 2 cancel the actual pending notice.
    pub action: u8,
    pub due_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseMoveOutReply {
    pub operation: Option<[u8; 16]>,
    /// 0 no notice, 1 notice, 2 frozen, 3 drained (owner retained), 4 cancelled.
    pub state: u8,
    pub due_at_unix_ms: i64,
    pub census_count: u32,
    pub delivered: u32,
}

fn valid_house(house: &str) -> bool {
    house.len() <= 512
        && house
            .strip_prefix("oteryn:content.house.")
            .is_some_and(|tail| {
                !tail.is_empty()
                    && tail
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            })
}

pub fn encode_house_move_out_request(r: &HouseMoveOutRequest) -> Result<Vec<u8>, Error> {
    if !valid_house(&r.house)
        || r.action > 2
        || (r.action == 1 && r.due_at_unix_ms <= 0)
        || (r.action != 1 && r.due_at_unix_ms != 0)
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_message_field(&mut out, 1, r.house.as_bytes());
    push_varint_field(&mut out, 2, u64::from(r.action));
    push_varint_field(&mut out, 3, r.due_at_unix_ms as u64);
    Ok(out)
}

pub fn decode_house_move_out_request(bytes: &[u8]) -> Result<HouseMoveOutRequest, Error> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut house, mut action, mut due) = (None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            10 => set_once(
                &mut house,
                std::str::from_utf8(read_bytes(bytes, &mut at)?)
                    .map_err(|_| Error::Malformed)?
                    .to_owned(),
            )?,
            16 => set_once(
                &mut action,
                u8::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            24 => set_once(
                &mut due,
                i64::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    let r = HouseMoveOutRequest {
        house: house.ok_or(Error::Malformed)?,
        action: action.ok_or(Error::Malformed)?,
        due_at_unix_ms: due.ok_or(Error::Malformed)?,
    };
    encode_house_move_out_request(&r)?;
    Ok(r)
}

pub fn encode_house_move_out_reply(r: &HouseMoveOutReply) -> Result<Vec<u8>, Error> {
    if r.state > 4
        || r.census_count > 2000
        || r.delivered > r.census_count
        || (r.state == 0) != r.operation.is_none()
        || r.due_at_unix_ms < 0
        || (r.state == 0 && (r.due_at_unix_ms != 0 || r.census_count != 0 || r.delivered != 0))
        || (r.state != 0 && r.due_at_unix_ms == 0)
        || (matches!(r.state, 1 | 4) && (r.census_count != 0 || r.delivered != 0))
        || (r.state == 3 && r.delivered != r.census_count)
        || r.operation
            .is_some_and(|id| id[6] >> 4 != 7 || id[8] & 0xc0 != 0x80)
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    if let Some(id) = r.operation {
        push_message_field(&mut out, 1, &id);
    }
    for (field, value) in [
        (2, u64::from(r.state)),
        (3, r.due_at_unix_ms as u64),
        (4, u64::from(r.census_count)),
        (5, u64::from(r.delivered)),
    ] {
        push_varint_field(&mut out, field, value);
    }
    Ok(out)
}

pub fn decode_house_move_out_reply(bytes: &[u8]) -> Result<HouseMoveOutReply, Error> {
    if bytes.len() > MAX_REPLY_BYTES {
        return Err(Error::LimitExceeded);
    }
    let mut id = None;
    let mut fields = [None; 4];
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            10 => set_once(
                &mut id,
                <[u8; 16]>::try_from(read_bytes(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            tag if tag & 7 == 0 && (2..=5).contains(&(tag >> 3)) => set_once(
                &mut fields[(tag >> 3) as usize - 2],
                read_varint(bytes, &mut at)?,
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    let r = HouseMoveOutReply {
        operation: id,
        state: u8::try_from(fields[0].ok_or(Error::Malformed)?).map_err(|_| Error::Malformed)?,
        due_at_unix_ms: i64::try_from(fields[1].ok_or(Error::Malformed)?)
            .map_err(|_| Error::Malformed)?,
        census_count: u32::try_from(fields[2].ok_or(Error::Malformed)?)
            .map_err(|_| Error::Malformed)?,
        delivered: u32::try_from(fields[3].ok_or(Error::Malformed)?)
            .map_err(|_| Error::Malformed)?,
    };
    encode_house_move_out_reply(&r)?;
    Ok(r)
}

#[cfg(test)]
#[path = "house_move_out_tests.rs"]
mod tests;
