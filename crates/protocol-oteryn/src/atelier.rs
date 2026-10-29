//! Gem Atelier query and lock wire contract.
use crate::charm_wire::{CyclopediaWireError as Error, push_varint_field, read_varint, set_once};

pub const CAPABILITY_WHEEL_ALLOCATION_V1: u32 = 148;
pub const COMMAND_TYPE_ATELIER_QUERY: u32 = 308;
pub const COMMAND_TYPE_ATELIER_SET_LOCK: u32 = 309;
pub const MAX_REQUEST_BYTES: usize = 64;
pub const MAX_REPLY_BYTES: usize = 16_384;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetLockRequest {
    pub occurrence: [u8; 16],
    pub expected_revision: u64,
    pub gem: [u8; 16],
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GemView {
    pub id: [u8; 16],
    pub quality: u8,
    pub domain: u8,
    pub locked: bool,
    pub vessel: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtelierReply {
    pub refused: bool,
    pub revision: u64,
    pub gems: Vec<GemView>,
}

pub fn encode_query() -> Vec<u8> {
    Vec::new()
}

pub fn decode_query(bytes: &[u8]) -> Result<(), Error> {
    if bytes.is_empty() {
        Ok(())
    } else {
        Err(Error::Malformed)
    }
}

pub fn encode_set_lock(value: &SetLockRequest) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    crate::charm_wire::push_message_field(&mut out, 1, &value.occurrence);
    push_varint_field(&mut out, 2, value.expected_revision);
    crate::charm_wire::push_message_field(&mut out, 3, &value.gem);
    push_varint_field(&mut out, 4, u64::from(value.locked));
    if out.len() > MAX_REQUEST_BYTES {
        Err(Error::LimitExceeded)
    } else {
        Ok(out)
    }
}

pub fn decode_set_lock(bytes: &[u8]) -> Result<SetLockRequest, Error> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut occurrence, mut revision, mut gem, mut locked) = (None, None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            10 => read_id(bytes, &mut at, &mut occurrence)?,
            16 => set_once(&mut revision, read_varint(bytes, &mut at)?)?,
            26 => read_id(bytes, &mut at, &mut gem)?,
            32 => set_once(
                &mut locked,
                match read_varint(bytes, &mut at)? {
                    0 => false,
                    1 => true,
                    _ => return Err(Error::Malformed),
                },
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    let value = SetLockRequest {
        occurrence: occurrence.ok_or(Error::Malformed)?,
        expected_revision: revision.ok_or(Error::Malformed)?,
        gem: gem.ok_or(Error::Malformed)?,
        locked: locked.ok_or(Error::Malformed)?,
    };
    encode_set_lock(&value)?;
    Ok(value)
}

fn read_id(bytes: &[u8], at: &mut usize, slot: &mut Option<[u8; 16]>) -> Result<(), Error> {
    let value = crate::charm_wire::read_bytes(bytes, at)?;
    if value.len() != 16 || slot.is_some() {
        return Err(Error::Malformed);
    }
    let mut id = [0; 16];
    id.copy_from_slice(value);
    *slot = Some(id);
    Ok(())
}

pub fn encode_reply(value: &AtelierReply) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, u64::from(value.refused));
    push_varint_field(&mut out, 2, value.revision);
    for gem in &value.gems {
        let mut row = Vec::new();
        crate::charm_wire::push_message_field(&mut row, 1, &gem.id);
        push_varint_field(&mut row, 2, u64::from(gem.quality));
        push_varint_field(&mut row, 3, u64::from(gem.domain));
        push_varint_field(&mut row, 4, u64::from(gem.locked));
        push_varint_field(&mut row, 5, u64::from(gem.vessel));
        crate::charm_wire::push_message_field(&mut out, 3, &row);
    }
    if out.len() > MAX_REPLY_BYTES {
        Err(Error::LimitExceeded)
    } else {
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lock_round_trip_and_query_shape() -> Result<(), Error> {
        let value = SetLockRequest {
            occurrence: [7; 16],
            expected_revision: 9,
            gem: [3; 16],
            locked: true,
        };
        assert_eq!(decode_set_lock(&encode_set_lock(&value)?)?, value);
        assert!(decode_query(&[]).is_ok());
        assert!(decode_query(&[0]).is_err());
        assert!(decode_set_lock(&[32, 2]).is_err());
        Ok(())
    }
}
