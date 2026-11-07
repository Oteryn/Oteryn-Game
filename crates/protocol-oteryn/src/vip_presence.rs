//! VIP presence uses list-local numbers on CHAT delta 5, gated by capability 258.
//! VIP0 supplies the 100-entry bound and ten-second visibility confirmation lifetime.
use crate::world_interaction::WorldInteractionWireError as Error;

pub const CAPABILITY_VIP_PRESENCE_V1: u32 = 258;
pub const DELTA_TYPE_VIP_PRESENCE_V1: u32 = 5;
pub const MAX_ENTRIES: usize = 100;
pub const CONFIRMED_FOR_MS: u32 = 10_000;
/// Per entry: a 120-byte name plus worst-case protobuf tags, lengths and integers.
pub const MAX_BYTES: usize = MAX_ENTRIES * (crate::chat::MAX_CHAT_NAME_BYTES + 24);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Status {
    Unconfirmed = 0,
    Offline = 1,
    Online = 2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceEntry {
    pub entry: u64,
    pub name: String,
    pub status: Status,
    pub announce: bool,
    pub confirmed_for_ms: u32,
}

/// A full replacement. No global identity or location is encoded; replacing the view also
/// removes entries deleted from the account's list on another node.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PresenceBatch {
    pub entries: Vec<PresenceEntry>,
}

fn validate_entry(entry: &PresenceEntry) -> Result<(), Error> {
    if entry.entry == 0
        || entry.name.is_empty()
        || entry.name.len() > crate::chat::MAX_CHAT_NAME_BYTES
        || entry.name.chars().any(char::is_control)
        || (entry.status == Status::Online
            && !(1..=CONFIRMED_FOR_MS).contains(&entry.confirmed_for_ms))
        || (entry.status != Status::Online && entry.confirmed_for_ms != 0)
        || (entry.status == Status::Unconfirmed && entry.announce)
    {
        return Err(Error::Malformed);
    }
    Ok(())
}

pub fn encode(batch: &PresenceBatch) -> Result<Vec<u8>, Error> {
    use crate::charm_wire::{push_message_field, push_varint_field};
    if batch.entries.len() > MAX_ENTRIES {
        return Err(Error::LimitExceeded);
    }
    let mut out = Vec::new();
    let mut previous = 0;
    for entry in &batch.entries {
        validate_entry(entry)?;
        if entry.entry <= previous {
            return Err(Error::Malformed);
        }
        previous = entry.entry;
        let mut inner = Vec::new();
        push_varint_field(&mut inner, 1, entry.entry);
        push_message_field(&mut inner, 2, entry.name.as_bytes());
        push_varint_field(&mut inner, 3, entry.status as u64);
        push_varint_field(&mut inner, 4, u64::from(entry.announce));
        push_varint_field(&mut inner, 5, u64::from(entry.confirmed_for_ms));
        push_message_field(&mut out, 1, &inner);
    }
    Ok(out)
}

pub fn decode(bytes: &[u8]) -> Result<PresenceBatch, Error> {
    use crate::charm_wire::{read_bytes, read_varint, set_once};
    if bytes.len() > MAX_BYTES {
        return Err(Error::LimitExceeded);
    }
    let mut cursor = 0;
    let mut entries = Vec::new();
    let mut previous = 0;
    while cursor < bytes.len() {
        if read_varint(bytes, &mut cursor)? != 10 {
            return Err(Error::Malformed);
        }
        if entries.len() == MAX_ENTRIES {
            return Err(Error::LimitExceeded);
        }
        let inner = read_bytes(bytes, &mut cursor)?;
        let (mut entry, mut name, mut status, mut announce, mut confirmed_for_ms) =
            (None, None, None, None, None);
        let mut at = 0;
        while at < inner.len() {
            match read_varint(inner, &mut at)? {
                8 => set_once(&mut entry, read_varint(inner, &mut at)?)?,
                18 => {
                    let raw = read_bytes(inner, &mut at)?;
                    if raw.len() > crate::chat::MAX_CHAT_NAME_BYTES {
                        return Err(Error::LimitExceeded);
                    }
                    set_once(
                        &mut name,
                        std::str::from_utf8(raw)
                            .map_err(|_| Error::Malformed)?
                            .to_owned(),
                    )?;
                }
                24 => set_once(
                    &mut status,
                    match read_varint(inner, &mut at)? {
                        0 => Status::Unconfirmed,
                        1 => Status::Offline,
                        2 => Status::Online,
                        _ => return Err(Error::Malformed),
                    },
                )?,
                32 => set_once(
                    &mut announce,
                    match read_varint(inner, &mut at)? {
                        0 => false,
                        1 => true,
                        _ => return Err(Error::Malformed),
                    },
                )?,
                40 => set_once(
                    &mut confirmed_for_ms,
                    u32::try_from(read_varint(inner, &mut at)?).map_err(|_| Error::Malformed)?,
                )?,
                _ => return Err(Error::Malformed),
            }
        }
        let value = PresenceEntry {
            entry: entry.ok_or(Error::Malformed)?,
            name: name.ok_or(Error::Malformed)?,
            status: status.ok_or(Error::Malformed)?,
            announce: announce.ok_or(Error::Malformed)?,
            confirmed_for_ms: confirmed_for_ms.ok_or(Error::Malformed)?,
        };
        validate_entry(&value)?;
        if value.entry <= previous {
            return Err(Error::Malformed);
        }
        previous = value.entry;
        entries.push(value);
    }
    Ok(PresenceBatch { entries })
}

#[cfg(test)]
#[path = "vip_presence_tests.rs"]
mod tests;
