//! Separately negotiated live Atelier balances. Existing Atelier replies stay byte-compatible.
use crate::charm_wire::{CyclopediaWireError as Error, push_varint_field, read_varint, set_once};

pub const CAPABILITY_ATELIER_RESOURCES_V1: u32 = 272;
pub const COMMAND_TYPE_ATELIER_RESOURCES_QUERY: u32 = 484;
pub const MAX_REQUEST_BYTES: usize = 0;
/// Seven one-byte field tags and seven maximum-width u64 varints.
pub const MAX_REPLY_BYTES: usize = 7 * (1 + 10);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ResourceBalances {
    pub bank_gold: u64,
    pub carried_gold: u64,
    pub lesser_gems: u64,
    pub regular_gems: u64,
    pub greater_gems: u64,
    pub basic_fragments: u64,
    pub supreme_fragments: u64,
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

/// Every field, including zero, is present exactly once in ascending tag order.
pub fn encode_reply(value: &ResourceBalances) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(MAX_REPLY_BYTES);
    for (index, count) in [
        value.bank_gold,
        value.carried_gold,
        value.lesser_gems,
        value.regular_gems,
        value.greater_gems,
        value.basic_fragments,
        value.supreme_fragments,
    ]
    .into_iter()
    .enumerate()
    {
        push_varint_field(&mut bytes, index as u64 + 1, count);
    }
    bytes
}

pub fn decode_reply(bytes: &[u8]) -> Result<ResourceBalances, Error> {
    if bytes.len() > MAX_REPLY_BYTES {
        return Err(Error::LimitExceeded);
    }
    let mut fields = [None; 7];
    let mut at = 0;
    while at < bytes.len() {
        let index = match read_varint(bytes, &mut at)? {
            8 => 0,
            16 => 1,
            24 => 2,
            32 => 3,
            40 => 4,
            48 => 5,
            56 => 6,
            _ => return Err(Error::Malformed),
        };
        set_once(&mut fields[index], read_varint(bytes, &mut at)?)?;
    }
    let [
        Some(bank_gold),
        Some(carried_gold),
        Some(lesser_gems),
        Some(regular_gems),
        Some(greater_gems),
        Some(basic_fragments),
        Some(supreme_fragments),
    ] = fields
    else {
        return Err(Error::Malformed);
    };
    let value = ResourceBalances {
        bank_gold,
        carried_gold,
        lesser_gems,
        regular_gems,
        greater_gems,
        basic_fragments,
        supreme_fragments,
    };
    if encode_reply(&value) != bytes {
        return Err(Error::Malformed);
    }
    Ok(value)
}

#[cfg(test)]
#[path = "resources/tests.rs"]
mod tests;
