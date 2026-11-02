//! STORE_V1 bounded client codecs. Commercial authority and account identity stay on Platform.

mod catalogue;
mod hireling_purchase;
mod history;
mod inbox_claim;
mod purchase;

pub use catalogue::*;
pub use hireling_purchase::*;
pub use history::*;
pub use inbox_claim::*;
pub use purchase::*;

pub use crate::charm_wire::CyclopediaWireError as StoreWireError;
use crate::charm_wire::{read_bytes, read_varint, set_once};

pub const CAPABILITY_STORE_V1: u32 = 154;
pub const COMMAND_TYPE_STORE_CATALOGUE_PAGE: u32 = 183;
pub const COMMAND_TYPE_STORE_HISTORY_PAGE: u32 = 184;
pub const COMMAND_TYPE_STORE_PURCHASE: u32 = 185;
pub const COMMAND_TYPE_STORE_INBOX_CLAIM: u32 = 186;

pub const MAX_STORE_KEY_BYTES: usize = 128;
pub const MAX_STORE_TEXT_BYTES: usize = 256;
pub const MAX_STORE_CURSOR_BYTES: usize = 128;
pub const MAX_STORE_PAGE_ENTRIES: usize = 32;
pub const MAX_STORE_REQUEST_BYTES: usize = 512;
pub const MAX_STORE_RESULT_BYTES: usize = 24_576;

fn valid_text(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}

fn valid_key(value: &str) -> bool {
    valid_text(value, MAX_STORE_KEY_BYTES)
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b':' | b'_' | b'.' | b'-' | b'/'))
}

fn valid_uuid_v7(value: &[u8; 16]) -> bool {
    value[6] >> 4 == 7 && value[8] >> 6 == 2
}

fn decode_fields<const N: usize>(input: &[u8]) -> Result<[Option<&[u8]>; N], StoreWireError> {
    let mut fields = [None; N];
    let mut at = 0;
    while at < input.len() {
        let tag = read_varint(input, &mut at)?;
        if tag & 7 != 2 {
            return Err(StoreWireError::Malformed);
        }
        let index = usize::try_from(tag >> 3)
            .ok()
            .and_then(|field| field.checked_sub(1))
            .filter(|index| *index < N)
            .ok_or(StoreWireError::Malformed)?;
        let value = read_bytes(input, &mut at)?;
        set_once(&mut fields[index], value)?;
    }
    Ok(fields)
}

fn as_text(bytes: &[u8], maximum: usize) -> Result<&str, StoreWireError> {
    if bytes.len() > maximum {
        return Err(StoreWireError::LimitExceeded);
    }
    let value = std::str::from_utf8(bytes).map_err(|_| StoreWireError::Malformed)?;
    if !valid_text(value, maximum) {
        return Err(StoreWireError::Malformed);
    }
    Ok(value)
}

fn as_uuid(bytes: &[u8]) -> Result<[u8; 16], StoreWireError> {
    let value: [u8; 16] = bytes.try_into().map_err(|_| StoreWireError::Malformed)?;
    if !valid_uuid_v7(&value) {
        return Err(StoreWireError::Malformed);
    }
    Ok(value)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
