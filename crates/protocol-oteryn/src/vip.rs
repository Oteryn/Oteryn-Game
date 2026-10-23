//! VIP-1 basic request/result codec. Version 1 is strict, bounded and contains no global IDs.
//! Command 48 edits the account/world list, 49 changes consent, 50 reads the complete view.
use crate::world_interaction::WorldInteractionWireError as Error;

pub const CAPABILITY_VIP_V1: u32 = 43;
pub const COMMAND_TYPE_VIP_LIST: u32 = 48;
pub const COMMAND_TYPE_VIP_CONSENT: u32 = 49;
pub const COMMAND_TYPE_VIP_QUERY: u32 = 50;
pub const MAX_REQUEST_BYTES: usize = 640;
pub const MAX_RESULT_BYTES: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Operation {
    Add = 1,
    Remove = 2,
    Edit = 3,
    GroupAdd = 4,
    GroupRename = 5,
    GroupRemove = 6,
    Accept = 7,
    Reject = 8,
    Revoke = 9,
    Withdraw = 10,
    Status = 11,
    Query = 12,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub operation: Operation,
    pub number: u64,
    pub text: String,
    pub icon: u8,
    pub notify: bool,
    pub groups: Vec<u8>,
}

/// Strict bounded protobuf request. Fields: operation=1, number=2, text=3,
/// icon=4, notify=5, repeated (unpacked) group numbers=6.
pub fn decode_request(command_type: u32, bytes: &[u8]) -> Result<Request, Error> {
    use crate::charm_wire::{read_bytes, read_varint, set_once};
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut operation, mut number, mut text, mut icon, mut notify) =
        (None, None, None, None, None);
    let mut groups = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        match read_varint(bytes, &mut cursor)? {
            8 => set_once(&mut operation, read_varint(bytes, &mut cursor)?)?,
            16 => set_once(&mut number, read_varint(bytes, &mut cursor)?)?,
            26 => {
                let value = read_bytes(bytes, &mut cursor)?;
                if value.len() > 512 {
                    return Err(Error::LimitExceeded);
                }
                set_once(
                    &mut text,
                    std::str::from_utf8(value)
                        .map_err(|_| Error::Malformed)?
                        .to_owned(),
                )?;
            }
            32 => set_once(
                &mut icon,
                u8::try_from(read_varint(bytes, &mut cursor)?).map_err(|_| Error::Malformed)?,
            )?,
            40 => set_once(
                &mut notify,
                match read_varint(bytes, &mut cursor)? {
                    0 => false,
                    1 => true,
                    _ => return Err(Error::Malformed),
                },
            )?,
            48 => {
                if groups.len() == 8 {
                    return Err(Error::LimitExceeded);
                }
                groups.push(
                    u8::try_from(read_varint(bytes, &mut cursor)?).map_err(|_| Error::Malformed)?,
                );
            }
            _ => return Err(Error::Malformed),
        }
    }
    let operation = match (command_type, operation.ok_or(Error::Malformed)?) {
        (48, 1) => Operation::Add,
        (48, 2) => Operation::Remove,
        (48, 3) => Operation::Edit,
        (48, 4) => Operation::GroupAdd,
        (48, 5) => Operation::GroupRename,
        (48, 6) => Operation::GroupRemove,
        (49, 7) => Operation::Accept,
        (49, 8) => Operation::Reject,
        (49, 9) => Operation::Revoke,
        (49, 10) => Operation::Withdraw,
        (49, 11) => Operation::Status,
        (50, 12) => Operation::Query,
        _ => return Err(Error::Malformed),
    };
    let has_number = number.is_some();
    let has_text = text.is_some();
    let allowed = match operation {
        Operation::Add | Operation::GroupAdd => {
            !has_number && has_text && icon.is_none() && notify.is_none() && groups.is_empty()
        }
        Operation::GroupRename => {
            has_number && has_text && icon.is_none() && notify.is_none() && groups.is_empty()
        }
        Operation::Edit => has_number && has_text,
        Operation::Query => !has_text && icon.is_none() && notify.is_none() && groups.is_empty(),
        _ => has_number && !has_text && icon.is_none() && notify.is_none() && groups.is_empty(),
    };
    if !allowed {
        return Err(Error::Malformed);
    }
    Ok(Request {
        operation,
        number: number.unwrap_or(0),
        text: text.unwrap_or_default(),
        icon: icon.unwrap_or(0),
        notify: notify.unwrap_or(false),
        groups,
    })
}

/// Protobuf result: code=1, revision=2, local number=3, wait seconds=4,
/// bounded UTF-8 JSON view=5. Codes 1 OK, 2 UNKNOWN_NAME, 3 ALREADY_LISTED,
/// 4 LIST_FULL, 5 GROUP_LIMIT, 6 NAME_TAKEN, 7 NOT_FOUND, 8 EXHAUSTED,
/// 9 UNAVAILABLE, 10 REJECTED. No global identity is present in the view.
pub fn encode_result(
    code: u8,
    revision: u64,
    number: u64,
    wait: u8,
    view: &str,
) -> Result<Vec<u8>, Error> {
    use crate::charm_wire::{push_message_field, push_varint_field};
    if !(1..=10).contains(&code) || wait > 3 || (code == 8) != (wait > 0) {
        return Err(Error::Malformed);
    }
    if view.len() > MAX_RESULT_BYTES - 40 {
        return Err(Error::LimitExceeded);
    }
    let mut out = Vec::new();
    for (field, value) in [
        (1, u64::from(code)),
        (2, revision),
        (3, number),
        (4, u64::from(wait)),
    ] {
        push_varint_field(&mut out, field, value);
    }
    if !view.is_empty() {
        push_message_field(&mut out, 5, view.as_bytes());
    }
    Ok(out)
}

#[cfg(test)]
#[path = "vip_tests.rs"]
mod tests;
