//! Operations on the current, server-issued stash row version.
use super::*;
use std::num::NonZeroU32;

pub const COMMAND_TYPE_STASH_ROW_ACTION: u32 = 6345;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StashRowAction {
    Inspect = 1,
    Market = 2,
    LootAdd = 3,
    LootRemove = 4,
}

impl StashRowAction {
    pub const fn changes_loot_list(self) -> bool {
        matches!(self, Self::LootAdd | Self::LootRemove)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StashRowIntent {
    pub row: [u8; 16],
    pub action: StashRowAction,
}

pub fn encode_stash_row_intent(value: StashRowIntent) -> Result<Vec<u8>, Error> {
    if value.row == [0; 16] {
        return Err(Error::Malformed);
    }
    let mut out = Vec::with_capacity(20);
    push_varint_field(&mut out, 1, value.action as u64);
    push_message_field(&mut out, 2, &value.row);
    Ok(out)
}

pub fn decode_stash_row_intent(input: &[u8]) -> Result<StashRowIntent, Error> {
    if input.len() > 20 {
        return Err(Error::LimitExceeded);
    }
    let (mut action, mut row) = (None, None);
    let mut at = 0;
    while at < input.len() {
        match read_varint(input, &mut at)? {
            8 => set_once(&mut action, read_varint(input, &mut at)?)?,
            18 => set_once(
                &mut row,
                read_bytes(input, &mut at)?
                    .try_into()
                    .map_err(|_| Error::Malformed)?,
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    let action = match action {
        Some(1) => StashRowAction::Inspect,
        Some(2) => StashRowAction::Market,
        Some(3) => StashRowAction::LootAdd,
        Some(4) => StashRowAction::LootRemove,
        _ => return Err(Error::Malformed),
    };
    let value = StashRowIntent {
        row: row.ok_or(Error::Malformed)?,
        action,
    };
    encode_stash_row_intent(value)?;
    Ok(value)
}

pub fn encode_stash_row_result(reference: NonZeroU32) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_varint_field(&mut bytes, 1, u64::from(reference.get()));
    bytes
}

pub fn decode_stash_row_result(bytes: &[u8]) -> Result<NonZeroU32, Error> {
    if bytes.len() > 6 {
        return Err(Error::LimitExceeded);
    }
    let mut at = 0;
    if read_varint(bytes, &mut at)? != 8 {
        return Err(Error::Malformed);
    }
    let result = u32::try_from(read_varint(bytes, &mut at)?)
        .ok()
        .and_then(NonZeroU32::new)
        .ok_or(Error::Malformed)?;
    if at != bytes.len() {
        return Err(Error::Malformed);
    }
    Ok(result)
}
