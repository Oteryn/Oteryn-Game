//! Native depot command. Every request resolves a reachable authored locker again.
use crate::charm_wire::{push_message_field, push_varint_field, read_bytes, read_varint, set_once};
use crate::item_view::ItemHandle;
use crate::world_interaction::WorldInteractionWireError as Error;

pub const CAPABILITY_DEPOT_V1: u32 = 37;
pub const COMMAND_TYPE_DEPOT_INTENT: u32 = 36;
pub const MAX_DEPOT_PAGE: u32 = 14_999 / 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepotAction {
    Browse,
    Deposit(ItemHandle),
    Withdraw(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DepotIntent {
    /// Absent only for an initial browse. Moves must name the town last shown.
    pub town: Option<u16>,
    pub box_number: u8,
    pub page: u32,
    pub action: DepotAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DepotOutcome {
    Opened = 1,
    Moved = 2,
    NoLocker = 3,
    Stale = 4,
    Full = 5,
    NoBackpack = 6,
    Overweight = 7,
    Unsupported = 8,
    Rejected = 9,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepotEntry {
    pub ordinal: u64,
    pub definition: u32,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepotReply {
    pub outcome: DepotOutcome,
    pub town: Option<u16>,
    pub box_number: u8,
    pub page: u32,
    /// Entries in the selected box, rather than across the town.
    pub total: u32,
    pub entries: Vec<DepotEntry>,
}

fn valid(box_number: u8, page: u32) -> bool {
    (1..=17).contains(&box_number) && page <= MAX_DEPOT_PAGE
}

pub fn encode_depot_intent(value: DepotIntent) -> Result<Vec<u8>, Error> {
    if !valid(value.box_number, value.page)
        || (value.town.is_none() && value.action != DepotAction::Browse)
        || matches!(value.action, DepotAction::Withdraw(0))
    {
        return Err(Error::Malformed);
    }
    let (action, item) = match value.action {
        DepotAction::Browse => (1, 0),
        DepotAction::Deposit(handle) => (2, handle.get()),
        DepotAction::Withdraw(ordinal) => (3, ordinal),
    };
    let mut out = Vec::new();
    for (field, number) in [
        (1, action),
        (2, u64::from(value.box_number)),
        (3, u64::from(value.page)),
        (5, item),
    ] {
        push_varint_field(&mut out, field, number);
    }
    if let Some(town) = value.town {
        push_varint_field(&mut out, 4, u64::from(town));
    }
    Ok(out)
}

pub fn decode_depot_intent(bytes: &[u8]) -> Result<DepotIntent, Error> {
    if bytes.len() > 48 {
        return Err(Error::LimitExceeded);
    }
    let mut fields = [None; 5];
    let mut at = 0;
    while at < bytes.len() {
        let tag = read_varint(bytes, &mut at)?;
        if tag & 7 != 0 || !(1..=5).contains(&(tag >> 3)) {
            return Err(Error::Malformed);
        }
        set_once(
            &mut fields[(tag >> 3) as usize - 1],
            read_varint(bytes, &mut at)?,
        )?;
    }
    let item = fields[4].unwrap_or(0);
    let action = match fields[0] {
        Some(1) if item == 0 => DepotAction::Browse,
        Some(2) => DepotAction::Deposit(ItemHandle::new(item).ok_or(Error::Malformed)?),
        Some(3) if item > 0 => DepotAction::Withdraw(item),
        _ => return Err(Error::Malformed),
    };
    let value = DepotIntent {
        town: fields[3]
            .map(u16::try_from)
            .transpose()
            .map_err(|_| Error::Malformed)?,
        box_number: u8::try_from(fields[1].unwrap_or(0)).map_err(|_| Error::Malformed)?,
        page: u32::try_from(fields[2].unwrap_or(0)).map_err(|_| Error::Malformed)?,
        action,
    };
    encode_depot_intent(value)?;
    Ok(value)
}

pub fn encode_depot_reply(value: &DepotReply) -> Result<Vec<u8>, Error> {
    if !valid(value.box_number, value.page)
        || value.total > 15_000
        || value.entries.len() > 32
        || value.entries.len() as u32 > value.total.saturating_sub(value.page * 32)
        || value
            .entries
            .windows(2)
            .any(|e| e[0].ordinal <= e[1].ordinal)
        || value
            .entries
            .iter()
            .any(|e| e.ordinal == 0 || e.definition == 0 || !(1..=100).contains(&e.count))
        || (value.town.is_none() && (!value.entries.is_empty() || value.total != 0))
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    for (field, number) in [
        (1, value.outcome as u64),
        (2, u64::from(value.box_number)),
        (3, u64::from(value.page)),
        (5, u64::from(value.total)),
    ] {
        push_varint_field(&mut out, field, number);
    }
    if let Some(town) = value.town {
        push_varint_field(&mut out, 4, u64::from(town));
    }
    for entry in &value.entries {
        let mut encoded = Vec::new();
        for (field, number) in [
            (1, entry.ordinal),
            (2, u64::from(entry.definition)),
            (3, u64::from(entry.count)),
        ] {
            push_varint_field(&mut encoded, field, number);
        }
        push_message_field(&mut out, 6, &encoded);
    }
    Ok(out)
}

pub fn decode_depot_reply(bytes: &[u8]) -> Result<DepotReply, Error> {
    if bytes.len() > 1024 {
        return Err(Error::LimitExceeded);
    }
    let mut fields = [None; 5];
    let mut entries = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let tag = read_varint(bytes, &mut at)?;
        if tag == 50 {
            if entries.len() == 32 {
                return Err(Error::LimitExceeded);
            }
            let bytes = read_bytes(bytes, &mut at)?;
            let mut entry = [None; 3];
            let mut pos = 0;
            while pos < bytes.len() {
                let tag = read_varint(bytes, &mut pos)?;
                if tag & 7 != 0 || !(1..=3).contains(&(tag >> 3)) {
                    return Err(Error::Malformed);
                }
                set_once(
                    &mut entry[(tag >> 3) as usize - 1],
                    read_varint(bytes, &mut pos)?,
                )?;
            }
            entries.push(DepotEntry {
                ordinal: entry[0].unwrap_or(0),
                definition: u32::try_from(entry[1].unwrap_or(0)).map_err(|_| Error::Malformed)?,
                count: u32::try_from(entry[2].unwrap_or(0)).map_err(|_| Error::Malformed)?,
            });
        } else {
            if tag & 7 != 0 || !(1..=5).contains(&(tag >> 3)) {
                return Err(Error::Malformed);
            }
            set_once(
                &mut fields[(tag >> 3) as usize - 1],
                read_varint(bytes, &mut at)?,
            )?;
        }
    }
    let outcome = match fields[0] {
        Some(1) => DepotOutcome::Opened,
        Some(2) => DepotOutcome::Moved,
        Some(3) => DepotOutcome::NoLocker,
        Some(4) => DepotOutcome::Stale,
        Some(5) => DepotOutcome::Full,
        Some(6) => DepotOutcome::NoBackpack,
        Some(7) => DepotOutcome::Overweight,
        Some(8) => DepotOutcome::Unsupported,
        Some(9) => DepotOutcome::Rejected,
        _ => return Err(Error::Malformed),
    };
    let value = DepotReply {
        outcome,
        entries,
        town: fields[3]
            .map(u16::try_from)
            .transpose()
            .map_err(|_| Error::Malformed)?,
        box_number: u8::try_from(fields[1].unwrap_or(0)).map_err(|_| Error::Malformed)?,
        page: u32::try_from(fields[2].unwrap_or(0)).map_err(|_| Error::Malformed)?,
        total: u32::try_from(fields[4].unwrap_or(0)).map_err(|_| Error::Malformed)?,
    };
    encode_depot_reply(&value)?;
    Ok(value)
}

#[cfg(test)]
#[path = "depot_tests.rs"]
mod tests;
