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
    MainCharacter = 13,
    FriendInvite = 14,
    FriendAccept = 15,
    FriendRevoke = 16,
    FriendsQuery = 17,
    FriendReject = 18,
}
impl Operation {
    pub fn is_query(self) -> bool {
        matches!(self, Self::Query | Self::FriendsQuery)
    }
    pub fn is_account(self) -> bool {
        matches!(
            self,
            Self::MainCharacter
                | Self::FriendInvite
                | Self::FriendAccept
                | Self::FriendRevoke
                | Self::FriendsQuery
                | Self::FriendReject
        )
    }
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

/// Encodes a request using the same strict shape accepted by [`decode_request`].
pub fn encode_request(request: &Request) -> Result<(u32, Vec<u8>), Error> {
    use crate::charm_wire::{push_message_field, push_varint_field};
    let kind = match request.operation {
        Operation::Add
        | Operation::Remove
        | Operation::Edit
        | Operation::GroupAdd
        | Operation::GroupRename
        | Operation::GroupRemove
        | Operation::MainCharacter
        | Operation::FriendInvite
        | Operation::FriendRevoke => COMMAND_TYPE_VIP_LIST,
        Operation::Accept
        | Operation::Reject
        | Operation::Revoke
        | Operation::Withdraw
        | Operation::Status
        | Operation::FriendAccept
        | Operation::FriendReject => COMMAND_TYPE_VIP_CONSENT,
        Operation::Query | Operation::FriendsQuery => COMMAND_TYPE_VIP_QUERY,
    };
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, request.operation as u64);
    match request.operation {
        Operation::Add | Operation::GroupAdd | Operation::FriendInvite => {
            push_message_field(&mut out, 3, request.text.as_bytes())
        }
        Operation::GroupRename => {
            push_varint_field(&mut out, 2, request.number);
            push_message_field(&mut out, 3, request.text.as_bytes());
        }
        Operation::Edit => {
            push_varint_field(&mut out, 2, request.number);
            push_message_field(&mut out, 3, request.text.as_bytes());
            push_varint_field(&mut out, 4, u64::from(request.icon));
            push_varint_field(&mut out, 5, u64::from(request.notify));
            for group in &request.groups {
                push_varint_field(&mut out, 6, u64::from(*group));
            }
        }
        Operation::MainCharacter => {}
        Operation::Query | Operation::FriendsQuery => {
            if request.number != 0 {
                push_varint_field(&mut out, 2, request.number);
            }
        }
        _ => push_varint_field(&mut out, 2, request.number),
    }
    decode_request(kind, &out)?;
    Ok((kind, out))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultView {
    pub code: u8,
    pub revision: u64,
    pub number: u64,
    pub wait: u8,
    pub view: String,
}

/// Decodes the server result produced by [`encode_result`].
pub fn decode_result(bytes: &[u8]) -> Result<ResultView, Error> {
    use crate::charm_wire::{read_bytes, read_varint, set_once};
    if bytes.len() > MAX_RESULT_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut code, mut revision, mut number, mut wait, mut view) = (None, None, None, None, None);
    let mut cursor = 0;
    while cursor < bytes.len() {
        match read_varint(bytes, &mut cursor)? {
            8 => set_once(
                &mut code,
                u8::try_from(read_varint(bytes, &mut cursor)?).map_err(|_| Error::Malformed)?,
            )?,
            16 => set_once(&mut revision, read_varint(bytes, &mut cursor)?)?,
            24 => set_once(&mut number, read_varint(bytes, &mut cursor)?)?,
            32 => set_once(
                &mut wait,
                u8::try_from(read_varint(bytes, &mut cursor)?).map_err(|_| Error::Malformed)?,
            )?,
            42 => set_once(
                &mut view,
                std::str::from_utf8(read_bytes(bytes, &mut cursor)?)
                    .map_err(|_| Error::Malformed)?
                    .to_owned(),
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    let value = ResultView {
        code: code.ok_or(Error::Malformed)?,
        revision: revision.ok_or(Error::Malformed)?,
        number: number.ok_or(Error::Malformed)?,
        wait: wait.ok_or(Error::Malformed)?,
        view: view.unwrap_or_default(),
    };
    encode_result(
        value.code,
        value.revision,
        value.number,
        value.wait,
        &value.view,
    )?;
    Ok(value)
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
        (48, 13) => Operation::MainCharacter,
        (48, 14) => Operation::FriendInvite,
        (49, 15) => Operation::FriendAccept,
        (48, 16) => Operation::FriendRevoke,
        (50, 17) => Operation::FriendsQuery,
        (49, 18) => Operation::FriendReject,
        _ => return Err(Error::Malformed),
    };
    let has_number = number.is_some();
    let has_text = text.is_some();
    let allowed = match operation {
        Operation::Add | Operation::GroupAdd | Operation::FriendInvite => {
            !has_number && has_text && icon.is_none() && notify.is_none() && groups.is_empty()
        }
        Operation::GroupRename => {
            has_number && has_text && icon.is_none() && notify.is_none() && groups.is_empty()
        }
        Operation::Edit => has_number && has_text,
        Operation::MainCharacter => {
            !has_number && !has_text && icon.is_none() && notify.is_none() && groups.is_empty()
        }
        Operation::Query | Operation::FriendsQuery => {
            !has_text && icon.is_none() && notify.is_none() && groups.is_empty()
        }
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

#[cfg(test)]
mod main_character_tests {
    use super::*;
    #[test]
    fn main_character_request_uses_the_acting_session_without_global_identifiers()
    -> Result<(), Error> {
        let request = Request {
            operation: Operation::MainCharacter,
            number: 0,
            text: String::new(),
            icon: 0,
            notify: false,
            groups: vec![],
        };
        let (kind, bytes) = encode_request(&request)?;
        assert_eq!((kind, bytes.as_slice()), (48, &[8, 13][..]));
        assert_eq!(decode_request(kind, &bytes)?, request);
        assert!(decode_request(48, &[8, 13, 16, 1]).is_err());
        assert!(decode_request(49, &bytes).is_err());
        Ok(())
    }
}
