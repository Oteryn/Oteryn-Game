//! PREY-RT-1 bounded wire projection and intent codec.
use crate::charm_wire::{
    CyclopediaWireError as Error, push_message_field, push_varint_field, read_bytes, read_varint,
    set_once,
};

pub const CAPABILITY_PREY_V1: u32 = 140;
pub const COMMAND_TYPE_PREY: u32 = 160;
/// The selected creature does not satisfy the admitted Prey loot budget. No mutation occurred.
pub const STATUS_CONTENT_UNAVAILABLE: u8 = 8;
/// A complete eligible unoffered list is unavailable today. No gold was charged.
pub const STATUS_NO_CANDIDATES: u8 = 9;
pub const STATUS_INVALID_TIER: u8 = 10;
pub const STATUS_NOT_OFFERED: u8 = 11;
pub const STATUS_OCCUPIED: u8 = 12;
/// Internal Store delivery result; no entitlement-grant action exists on the wire.
pub const STATUS_ALREADY_OWNED: u8 = 13;
// Native safety budgets: maximum key plus a confirmed i64 gold quote.
pub const MAX_REQUEST_BYTES: usize = 273;
pub const MAX_REPLY_BYTES: usize = 65_536;
// PREY-DIFF (2026-11-01): list size is server configuration (normally 9/12).
// These are transport safety ceilings, not gameplay list sizes. Creature keys
// retain the existing CREATURE_KEY_BYTES limit and migration 0152 vocabulary.
pub const MAX_SLOTS: usize = 3;
pub const MAX_CREATURES: usize = 64;
pub const MAX_OFFERED_PAGE: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Query,
    ListReroll {
        slot: u8,
    },
    Select {
        slot: u8,
        index: u8,
    },
    SelectAny {
        slot: u8,
        creature: String,
    },
    BonusReroll {
        slot: u8,
    },
    SetOption {
        slot: u8,
        option: u8,
    },
    SetTier {
        slot: u8,
        tier: u8,
    },
    OfferedToday {
        slot: u8,
        after: String,
    },
    PickAgain {
        slot: u8,
        creature: String,
        expected_price_gold: u64,
    },
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
    /// Persisted preference; retained after level loss, applied at the next draw.
    pub chosen_tier: u8,
    /// Highest tier allowed by the current character level.
    pub allowed_tier: u8,
    /// Durable selection occurrence fence; changes when a new active Prey begins.
    pub selection_epoch: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferedCreature {
    pub creature: String,
    pub price_gold: u64,
    /// Zero if available, otherwise the slot currently holding this creature.
    pub occupied_slot: u8,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub status: u8,
    pub wildcard_balance: u8,
    pub slots: Vec<Slot>,
    pub list_size: u8,
    pub expansion_owned: bool,
    /// One bounded page, ordered by creature key; prices come from the server.
    pub offered_today: Vec<OfferedCreature>,
    /// Empty on the last page, otherwise pass this key as OfferedToday.after.
    pub offered_next: String,
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
        Request::SetTier { slot, tier } => (6, *slot, u64::from(*tier), None),
        Request::OfferedToday { slot, after } => {
            (7, *slot, 0, (!after.is_empty()).then_some(after))
        }
        Request::PickAgain { slot, creature, .. } => (8, *slot, 0, Some(creature)),
    };
    if (action == 0) != (slot == 0)
        || (action != 0 && !valid_slot(slot))
        || (action == 2 && !(1..=MAX_CREATURES as u64).contains(&argument))
        || (action == 5 && argument > 2)
        || (action == 6 && !(1..=5).contains(&argument))
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
    if let Request::PickAgain {
        expected_price_gold,
        ..
    } = request
    {
        if *expected_price_gold > i64::MAX as u64 {
            return Err(Error::Malformed);
        }
        field(&mut out, 5, *expected_price_gold);
    }
    Ok(out)
}
pub fn decode_request(bytes: &[u8]) -> Result<Request, Error> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut action, mut slot, mut arg, mut creature) = (None, None, None, None);
    let mut expected_price_gold = None;
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
            40 => set_once(&mut expected_price_gold, read_varint(bytes, &mut at)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    let action = action.ok_or(Error::Malformed)?;
    if action != 8 && expected_price_gold.is_some() {
        return Err(Error::Malformed);
    }
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
        6 if creature.is_none() => Request::SetTier {
            slot,
            tier: u8::try_from(arg.ok_or(Error::Malformed)?).map_err(|_| Error::Malformed)?,
        },
        7 if arg.is_none() => Request::OfferedToday {
            slot,
            after: creature.unwrap_or_default(),
        },
        8 if arg.is_none() => Request::PickAgain {
            slot,
            creature: creature.ok_or(Error::Malformed)?,
            expected_price_gold: expected_price_gold.ok_or(Error::Malformed)?,
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
        || !(1..=5).contains(&slot.chosen_tier)
        || !(1..=5).contains(&slot.allowed_tier)
        || slot.selection_epoch > i64::MAX as u64
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
    field(&mut out, 11, u64::from(slot.chosen_tier));
    field(&mut out, 12, u64::from(slot.allowed_tier));
    field(&mut out, 13, slot.selection_epoch);
    Ok(out)
}
pub fn encode_reply(reply: &Reply) -> Result<Vec<u8>, Error> {
    // Status codes are native Oteryn wire reservations.
    if reply.status > STATUS_ALREADY_OWNED
        // PREY0 §4.1 / PREY0-RL-05 (Account + World balance).
        || reply.wildcard_balance > 52
        || reply.slots.len() > MAX_SLOTS
        || !reply.slots.windows(2).all(|p| p[0].slot < p[1].slot)
        || !(1..=MAX_CREATURES as u8).contains(&reply.list_size)
        || reply.offered_today.len() > MAX_OFFERED_PAGE
        || !reply.offered_today.windows(2).all(|p| p[0].creature < p[1].creature)
        || (!reply.offered_next.is_empty() && !valid_key(&reply.offered_next))
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    field(&mut out, 1, u64::from(reply.status));
    field(&mut out, 2, u64::from(reply.wildcard_balance));
    for s in &reply.slots {
        push_message_field(&mut out, 3, &encode_slot(s)?)
    }
    field(&mut out, 4, u64::from(reply.list_size));
    field(&mut out, 5, u64::from(reply.expansion_owned));
    for offered in &reply.offered_today {
        if !valid_key(&offered.creature)
            || offered.price_gold > i64::MAX as u64
            || offered.occupied_slot > MAX_SLOTS as u8
        {
            return Err(Error::Malformed);
        }
        let mut row = Vec::new();
        push_message_field(&mut row, 1, offered.creature.as_bytes());
        field(&mut row, 2, offered.price_gold);
        field(&mut row, 3, u64::from(offered.occupied_slot));
        push_message_field(&mut out, 6, &row);
    }
    if !reply.offered_next.is_empty() {
        push_message_field(&mut out, 7, reply.offered_next.as_bytes());
    }
    if out.len() > MAX_REPLY_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}

fn decode_slot(bytes: &[u8]) -> Result<Slot, Error> {
    let mut scalars = [None; 13];
    let mut creatures = Vec::new();
    let mut selected = None;
    let mut at = 0;
    while at < bytes.len() {
        let key = read_varint(bytes, &mut at)?;
        match key {
            8 | 16 | 40 | 48 | 56 | 64 | 72 | 80 | 88 | 96 | 104 => {
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
        chosen_tier: byte(10)?,
        allowed_tier: byte(11)?,
        selection_epoch: scalars[12].ok_or(Error::Malformed)?,
    })
}

fn read_key(bytes: &[u8], at: &mut usize) -> Result<String, Error> {
    let key = std::str::from_utf8(read_bytes(bytes, at)?).map_err(|_| Error::Malformed)?;
    if !valid_key(key) {
        return Err(Error::Malformed);
    }
    Ok(key.to_owned())
}

fn decode_offered(bytes: &[u8]) -> Result<OfferedCreature, Error> {
    let (mut creature, mut price, mut occupied) = (None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            10 => set_once(&mut creature, read_key(bytes, &mut at)?)?,
            16 => set_once(&mut price, read_varint(bytes, &mut at)?)?,
            24 => set_once(&mut occupied, read_varint(bytes, &mut at)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    Ok(OfferedCreature {
        creature: creature.ok_or(Error::Malformed)?,
        price_gold: price.ok_or(Error::Malformed)?,
        occupied_slot: u8::try_from(occupied.ok_or(Error::Malformed)?)
            .map_err(|_| Error::Malformed)?,
    })
}

/// Decodes only the bounded canonical projection emitted by `encode_reply`.
pub fn decode_reply(bytes: &[u8]) -> Result<Reply, Error> {
    if bytes.len() > MAX_REPLY_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut status, mut wildcard_balance) = (None, None);
    let (mut list_size, mut expansion_owned, mut offered_next) = (None, None, None);
    let mut offered_today = Vec::new();
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
            32 => set_once(&mut list_size, read_varint(bytes, &mut at)?)?,
            40 => set_once(&mut expansion_owned, read_varint(bytes, &mut at)?)?,
            50 => {
                if offered_today.len() == MAX_OFFERED_PAGE {
                    return Err(Error::LimitExceeded);
                }
                offered_today.push(decode_offered(read_bytes(bytes, &mut at)?)?);
            }
            58 => set_once(&mut offered_next, read_key(bytes, &mut at)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    let reply = Reply {
        status: u8::try_from(status.ok_or(Error::Malformed)?).map_err(|_| Error::Malformed)?,
        wildcard_balance: u8::try_from(wildcard_balance.ok_or(Error::Malformed)?)
            .map_err(|_| Error::Malformed)?,
        slots,
        list_size: u8::try_from(list_size.ok_or(Error::Malformed)?)
            .map_err(|_| Error::Malformed)?,
        expansion_owned: match expansion_owned {
            Some(0) => false,
            Some(1) => true,
            _ => return Err(Error::Malformed),
        },
        offered_today,
        offered_next: offered_next.unwrap_or_default(),
    };
    if encode_reply(&reply)? != bytes {
        return Err(Error::Malformed);
    }
    Ok(reply)
}

/// Upgrade reader for immutable receipts written by migration 0337's owner.
/// Network replies still require `decode_reply`; only stored receipts use this.
pub fn decode_legacy_reply(bytes: &[u8]) -> Result<Reply, Error> {
    if bytes.len() > 8192 {
        return Err(Error::LimitExceeded);
    }
    let (mut status, mut balance) = (None, None);
    let mut slots = Vec::new();
    let mut canonical_slots = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(&mut status, read_varint(bytes, &mut at)?)?,
            16 => set_once(&mut balance, read_varint(bytes, &mut at)?)?,
            26 => {
                if slots.len() == MAX_SLOTS {
                    return Err(Error::LimitExceeded);
                }
                let original = read_bytes(bytes, &mut at)?;
                let mut extended = original.to_vec();
                extended.extend([88, 1, 96, 1, 104, 0]);
                let slot = decode_slot(&extended)?;
                if slot.creatures.len() > 9 || encode_slot(&slot)? != extended {
                    return Err(Error::Malformed);
                }
                push_message_field(&mut canonical_slots, 3, original);
                slots.push(slot);
            }
            _ => return Err(Error::Malformed),
        }
    }
    let status = u8::try_from(status.ok_or(Error::Malformed)?).map_err(|_| Error::Malformed)?;
    if status > STATUS_CONTENT_UNAVAILABLE {
        return Err(Error::Malformed);
    }
    let wildcard_balance =
        u8::try_from(balance.ok_or(Error::Malformed)?).map_err(|_| Error::Malformed)?;
    let reply = Reply {
        status,
        wildcard_balance,
        slots,
        list_size: 9,
        expansion_owned: false,
        offered_today: Vec::new(),
        offered_next: String::new(),
    };
    // Reuse all common semantic bounds, then require the original old encoding.
    encode_reply(&reply)?;
    let mut canonical = Vec::new();
    field(&mut canonical, 1, u64::from(status));
    field(&mut canonical, 2, u64::from(wildcard_balance));
    canonical.extend(canonical_slots);
    if canonical != bytes {
        return Err(Error::Malformed);
    }
    Ok(reply)
}

#[cfg(test)]
#[path = "prey_tests.rs"]
mod tests;
