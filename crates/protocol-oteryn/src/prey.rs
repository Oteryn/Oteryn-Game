//! PREY-RT-1 bounded wire projection and intent codec.
use crate::charm_wire::{
    CyclopediaWireError as Error, push_message_field, push_varint_field, read_bytes, read_varint,
    set_once,
};

pub const CAPABILITY_PREY_V1: u32 = 140;
pub const COMMAND_TYPE_PREY: u32 = 160;
/// The selected creature does not satisfy the admitted Prey loot budget. No mutation occurred.
pub const STATUS_CONTENT_UNAVAILABLE: u8 = 8;
// Oteryn wire budgets (not Canary packet sizes); the longest SelectAny needs 263 bytes.
pub const MAX_REQUEST_BYTES: usize = 263;
pub const MAX_REPLY_BYTES: usize = 8192;
// PREY0 §§3–4: three slots and nine candidates. Creature key bytes follow
// the existing progression/prey.rs::CREATURE_KEY_BYTES and migration 0152.
pub const MAX_SLOTS: usize = 3;
pub const MAX_CREATURES: usize = 9;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Query,
    ListReroll { slot: u8 },
    Select { slot: u8, index: u8 },
    SelectAny { slot: u8, creature: String },
    BonusReroll { slot: u8 },
    SetOption { slot: u8, option: u8 },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    pub slot: u8,
    pub state: u8,
    pub creatures: Vec<String>,
    pub selected: String,
    pub bonus: u8,
    pub grade: u8,
    pub percent: u8,
    pub hunting_seconds: u32,
    pub option: u8,
    /// Unix seconds, bounded by the existing durable nonnegative BIGINT timestamp.
    pub free_reroll_at: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub status: u8,
    pub wildcard_balance: u8,
    pub slots: Vec<Slot>,
}

fn valid_slot(slot: u8) -> bool {
    (1..=3).contains(&slot)
}
fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 256
        && key.starts_with("oteryn:")
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
fn field(out: &mut Vec<u8>, number: u64, value: u64) {
    push_varint_field(out, number, value);
}

pub fn encode_request(request: &Request) -> Result<Vec<u8>, Error> {
    let (action, slot, argument, creature) = match request {
        Request::Query => (0, 0, 0, None),
        Request::ListReroll { slot } => (1, *slot, 0, None),
        Request::Select { slot, index } => (2, *slot, u64::from(*index), None),
        Request::SelectAny { slot, creature } => (3, *slot, 0, Some(creature)),
        Request::BonusReroll { slot } => (4, *slot, 0, None),
        Request::SetOption { slot, option } => (5, *slot, u64::from(*option), None),
    };
    if (action == 0) != (slot == 0)
        || (action != 0 && !valid_slot(slot))
        || (action == 2 && !(1..=9).contains(&argument))
        || (action == 5 && argument > 2)
        || creature.is_some_and(|v| !valid_key(v))
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    field(&mut out, 1, action);
    if slot != 0 {
        field(&mut out, 2, u64::from(slot));
    }
    if argument != 0 {
        field(&mut out, 3, argument);
    }
    if let Some(v) = creature {
        push_message_field(&mut out, 4, v.as_bytes());
    }
    Ok(out)
}
pub fn decode_request(bytes: &[u8]) -> Result<Request, Error> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut action, mut slot, mut arg, mut creature) = (None, None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(&mut action, read_varint(bytes, &mut at)?)?,
            16 => set_once(
                &mut slot,
                u8::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            24 => set_once(&mut arg, read_varint(bytes, &mut at)?)?,
            34 => set_once(
                &mut creature,
                std::str::from_utf8(read_bytes(bytes, &mut at)?)
                    .map_err(|_| Error::Malformed)?
                    .to_owned(),
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    let action = action.ok_or(Error::Malformed)?;
    let slot = slot.unwrap_or(0);
    let r = match action {
        0 if slot == 0 && arg.is_none() && creature.is_none() => Request::Query,
        1 if arg.is_none() && creature.is_none() => Request::ListReroll { slot },
        2 if creature.is_none() => Request::Select {
            slot,
            index: u8::try_from(arg.ok_or(Error::Malformed)?).map_err(|_| Error::Malformed)?,
        },
        3 if arg.is_none() => Request::SelectAny {
            slot,
            creature: creature.ok_or(Error::Malformed)?,
        },
        4 if arg.is_none() && creature.is_none() => Request::BonusReroll { slot },
        5 if creature.is_none() => Request::SetOption {
            slot,
            option: u8::try_from(arg.unwrap_or(0)).map_err(|_| Error::Malformed)?,
        },
        _ => return Err(Error::Malformed),
    };
    if encode_request(&r)? != bytes {
        return Err(Error::Malformed);
    }
    Ok(r)
}

fn encode_slot(slot: &Slot) -> Result<Vec<u8>, Error> {
    if !valid_slot(slot.slot)
        // PREY0 §4.1: native LOCKED/SELECTION/ACTIVE, not Canary UI states 0–6.
        || slot.state > 2
        || slot.creatures.len() > MAX_CREATURES
        || slot.creatures.iter().any(|v| !valid_key(v))
        || (!slot.selected.is_empty() && !valid_key(&slot.selected))
        // Canary 47dfd51f src/io/ioprey.hpp: four bonus kinds plus None, options 0–2.
        || slot.bonus > 4
        // PREY0 §3; Canary 47dfd51f src/io/ioprey.cpp::reloadBonusValue (max 40%).
        || slot.grade > 10
        || slot.percent > 40
        // PREY0 §3: two hours of hunting time.
        || slot.hunting_seconds > 7200
        || slot.option > 2
        || slot.free_reroll_at > i64::MAX as u64
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    field(&mut out, 1, u64::from(slot.slot));
    field(&mut out, 2, u64::from(slot.state));
    for v in &slot.creatures {
        push_message_field(&mut out, 3, v.as_bytes())
    }
    if !slot.selected.is_empty() {
        push_message_field(&mut out, 4, slot.selected.as_bytes())
    }
    field(&mut out, 5, u64::from(slot.bonus));
    field(&mut out, 6, u64::from(slot.grade));
    field(&mut out, 7, u64::from(slot.percent));
    field(&mut out, 8, u64::from(slot.hunting_seconds));
    field(&mut out, 9, u64::from(slot.option));
    field(&mut out, 10, slot.free_reroll_at);
    Ok(out)
}
pub fn encode_reply(reply: &Reply) -> Result<Vec<u8>, Error> {
    // Status codes are an Oteryn wire reservation; PREY0/Canary do not define 0–8.
    if reply.status > 8
        // PREY0 §4.1 / PREY0-RL-05 (Account + World balance).
        || reply.wildcard_balance > 52
        || reply.slots.len() > MAX_SLOTS
        || !reply.slots.windows(2).all(|p| p[0].slot < p[1].slot)
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    field(&mut out, 1, u64::from(reply.status));
    field(&mut out, 2, u64::from(reply.wildcard_balance));
    for s in &reply.slots {
        push_message_field(&mut out, 3, &encode_slot(s)?)
    }
    if out.len() > MAX_REPLY_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}

fn decode_slot(bytes: &[u8]) -> Result<Slot, Error> {
    let mut scalars = [None; 10];
    let mut creatures = Vec::new();
    let mut selected = None;
    let mut at = 0;
    while at < bytes.len() {
        let key = read_varint(bytes, &mut at)?;
        match key {
            8 | 16 | 40 | 48 | 56 | 64 | 72 | 80 => {
                let index = (key >> 3) as usize - 1;
                set_once(&mut scalars[index], read_varint(bytes, &mut at)?)?;
            }
            26 => {
                if creatures.len() == MAX_CREATURES {
                    return Err(Error::LimitExceeded);
                }
                creatures.push(read_key(bytes, &mut at)?);
            }
            34 => set_once(&mut selected, read_key(bytes, &mut at)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    let byte = |index: usize| {
        u8::try_from(scalars[index].ok_or(Error::Malformed)?).map_err(|_| Error::Malformed)
    };
    Ok(Slot {
        slot: byte(0)?,
        state: byte(1)?,
        creatures,
        selected: selected.unwrap_or_default(),
        bonus: byte(4)?,
        grade: byte(5)?,
        percent: byte(6)?,
        hunting_seconds: u32::try_from(scalars[7].ok_or(Error::Malformed)?)
            .map_err(|_| Error::Malformed)?,
        option: byte(8)?,
        free_reroll_at: scalars[9].ok_or(Error::Malformed)?,
    })
}

fn read_key(bytes: &[u8], at: &mut usize) -> Result<String, Error> {
    let key = std::str::from_utf8(read_bytes(bytes, at)?).map_err(|_| Error::Malformed)?;
    if !valid_key(key) {
        return Err(Error::Malformed);
    }
    Ok(key.to_owned())
}

/// Decodes only the bounded canonical projection emitted by `encode_reply`.
pub fn decode_reply(bytes: &[u8]) -> Result<Reply, Error> {
    if bytes.len() > MAX_REPLY_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut status, mut wildcard_balance) = (None, None);
    let mut slots = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(&mut status, read_varint(bytes, &mut at)?)?,
            16 => set_once(&mut wildcard_balance, read_varint(bytes, &mut at)?)?,
            26 => {
                if slots.len() == MAX_SLOTS {
                    return Err(Error::LimitExceeded);
                }
                slots.push(decode_slot(read_bytes(bytes, &mut at)?)?);
            }
            _ => return Err(Error::Malformed),
        }
    }
    let reply = Reply {
        status: u8::try_from(status.ok_or(Error::Malformed)?).map_err(|_| Error::Malformed)?,
        wildcard_balance: u8::try_from(wildcard_balance.ok_or(Error::Malformed)?)
            .map_err(|_| Error::Malformed)?,
        slots,
    };
    if encode_reply(&reply)? != bytes {
        return Err(Error::Malformed);
    }
    Ok(reply)
}

#[cfg(test)]
#[path = "prey_tests.rs"]
mod tests;
