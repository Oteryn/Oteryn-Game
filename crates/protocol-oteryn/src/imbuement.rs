//! IMBUE-WIRE-1 shrine window: query an item's allowances, apply, clear (IMBUE_FORGE0 §14).
//!
//! The window is stateless: every action names the carried item by its current item-view
//! handle, so no server-side "imbuing item" survives between commands and there is no close.
use crate::charm_wire::{
    CyclopediaWireError as Error, push_message_field, push_varint_field, read_bytes, read_varint,
    set_once,
};
use crate::item_view::ItemHandle;

pub const CAPABILITY_IMBUE_V1: u32 = 68;
pub const COMMAND_TYPE_IMBUEMENT: u32 = 85;
/// IMBUEWIRE1-RL-01.
pub const MAX_REQUEST_BYTES: usize = 32;
/// IMBUEWIRE1-RL-02: 3 applied rows and 20 families x 3 tiers of offers.
pub const MAX_REPLY_BYTES: usize = 1024;
pub const MAX_SLOTS: u8 = 3;
pub const MAX_FAMILY: u8 = 20;
pub const MAX_OFFERS: usize = MAX_FAMILY as usize * 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImbuementAction {
    Query,
    Apply { slot: u8, imbuement: u8 },
    Clear { slot: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImbuementRequest {
    pub item: ItemHandle,
    pub action: ImbuementAction,
}

/// IMBUE_FORGE0 §14 results; `REVISION_CHANGED` is terminal for the occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImbuementResult {
    Ok = 1,
    RequirementNotMet = 2,
    MaterialsMissing = 3,
    InsufficientFunds = 4,
    SlotOccupied = 5,
    NotAllowed = 6,
    RevisionChanged = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedImbuement {
    pub slot: u8,
    /// Imbuement family 1-20 (stored category + 1).
    pub family: u8,
    pub tier: u8,
    pub remaining_ms: u32,
}

/// One applicable family and tier. Success is always 100% and there is no protection
/// charm (IMBUE_FORGE0 §3, ruling R2), so neither travels on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImbuementOffer {
    pub family: u8,
    pub tier: u8,
    pub gold: u32,
    pub requires_premium: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImbuementWindow {
    pub slot_count: u8,
    pub applied: Vec<AppliedImbuement>,
    /// Indexed by the apply command's `imbuement`.
    pub offers: Vec<ImbuementOffer>,
    pub clear_gold: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImbuementReply {
    pub result: ImbuementResult,
    /// Present only for an `Ok` query.
    pub window: Option<ImbuementWindow>,
}

pub fn encode_imbuement_request(r: &ImbuementRequest) -> Result<Vec<u8>, Error> {
    let (action, slot, imbuement) = match r.action {
        ImbuementAction::Query => (1, None, None),
        ImbuementAction::Apply { slot, imbuement } => (2, Some(slot), Some(imbuement)),
        ImbuementAction::Clear { slot } => (3, Some(slot), None),
    };
    if slot.is_some_and(|slot| slot >= MAX_SLOTS)
        || imbuement.is_some_and(|index| usize::from(index) >= MAX_OFFERS)
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, action);
    push_varint_field(&mut out, 2, r.item.get());
    if let Some(slot) = slot {
        push_varint_field(&mut out, 3, u64::from(slot));
    }
    if let Some(index) = imbuement {
        push_varint_field(&mut out, 4, u64::from(index));
    }
    Ok(out)
}

pub fn decode_imbuement_request(bytes: &[u8]) -> Result<ImbuementRequest, Error> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(Error::LimitExceeded);
    }
    let mut fields = [None; 4];
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            tag if tag & 7 == 0 && (1..=4).contains(&(tag >> 3)) => set_once(
                &mut fields[(tag >> 3) as usize - 1],
                read_varint(bytes, &mut at)?,
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    let small = |value: Option<u64>| {
        value
            .and_then(|value| u8::try_from(value).ok())
            .ok_or(Error::Malformed)
    };
    let item = fields[1]
        .and_then(ItemHandle::new)
        .ok_or(Error::Malformed)?;
    let action = match fields[0] {
        Some(1) if fields[2].is_none() && fields[3].is_none() => ImbuementAction::Query,
        Some(2) => ImbuementAction::Apply {
            slot: small(fields[2])?,
            imbuement: small(fields[3])?,
        },
        Some(3) if fields[3].is_none() => ImbuementAction::Clear {
            slot: small(fields[2])?,
        },
        _ => return Err(Error::Malformed),
    };
    let r = ImbuementRequest { item, action };
    // Canonical form only: the re-encoding must reproduce the exact input bytes.
    if encode_imbuement_request(&r)? != bytes {
        return Err(Error::Malformed);
    }
    Ok(r)
}

fn result_of(value: u64) -> Option<ImbuementResult> {
    use ImbuementResult::*;
    [
        Ok,
        RequirementNotMet,
        MaterialsMissing,
        InsufficientFunds,
        SlotOccupied,
        NotAllowed,
        RevisionChanged,
    ]
    .into_iter()
    .find(|result| *result as u64 == value)
}

fn valid_window(w: &ImbuementWindow) -> bool {
    let tier = |tier: u8| (1..=3).contains(&tier);
    let family = |family: u8| (1..=MAX_FAMILY).contains(&family);
    w.slot_count <= MAX_SLOTS
        && w.applied.len() <= usize::from(w.slot_count)
        && w.applied.windows(2).all(|pair| pair[0].slot < pair[1].slot)
        && w.applied.iter().all(|a| {
            a.slot < w.slot_count
                && family(a.family)
                && tier(a.tier)
                && a.remaining_ms <= 72_000_000
        })
        && w.offers.len() <= MAX_OFFERS
        && w.offers
            .windows(2)
            .all(|pair| (pair[0].family, pair[0].tier) < (pair[1].family, pair[1].tier))
        && w.offers.iter().all(|o| family(o.family) && tier(o.tier))
}

pub fn encode_imbuement_reply(r: &ImbuementReply) -> Result<Vec<u8>, Error> {
    if r.window.is_some() && r.result != ImbuementResult::Ok
        || r.window.as_ref().is_some_and(|w| !valid_window(w))
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, r.result as u64);
    if let Some(w) = &r.window {
        push_varint_field(&mut out, 2, u64::from(w.slot_count));
        for a in &w.applied {
            let mut row = Vec::new();
            for (field, value) in [
                (1, u64::from(a.slot)),
                (2, u64::from(a.family)),
                (3, u64::from(a.tier)),
                (4, u64::from(a.remaining_ms)),
            ] {
                push_varint_field(&mut row, field, value);
            }
            push_message_field(&mut out, 3, &row);
        }
        for o in &w.offers {
            let mut row = Vec::new();
            for (field, value) in [
                (1, u64::from(o.family)),
                (2, u64::from(o.tier)),
                (3, u64::from(o.gold)),
                (4, u64::from(o.requires_premium)),
            ] {
                push_varint_field(&mut row, field, value);
            }
            push_message_field(&mut out, 4, &row);
        }
        push_varint_field(&mut out, 5, u64::from(w.clear_gold));
    }
    if out.len() > MAX_REPLY_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}

fn read_row<const N: usize>(bytes: &[u8]) -> Result<[u64; N], Error> {
    let mut fields = [None; N];
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            tag if tag & 7 == 0 && (1..=N as u64).contains(&(tag >> 3)) => set_once(
                &mut fields[(tag >> 3) as usize - 1],
                read_varint(bytes, &mut at)?,
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    let mut out = [0; N];
    for (slot, field) in out.iter_mut().zip(fields) {
        *slot = field.ok_or(Error::Malformed)?;
    }
    Ok(out)
}

pub fn decode_imbuement_reply(bytes: &[u8]) -> Result<ImbuementReply, Error> {
    if bytes.len() > MAX_REPLY_BYTES {
        return Err(Error::LimitExceeded);
    }
    let narrow = |value: u64| u8::try_from(value).map_err(|_| Error::Malformed);
    let (mut result, mut slots, mut clear) = (None, None, None);
    let (mut applied, mut offers) = (Vec::new(), Vec::new());
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(&mut result, read_varint(bytes, &mut at)?)?,
            16 => set_once(&mut slots, read_varint(bytes, &mut at)?)?,
            26 if applied.len() < usize::from(MAX_SLOTS) => {
                let [slot, family, tier, remaining] = read_row::<4>(read_bytes(bytes, &mut at)?)?;
                applied.push(AppliedImbuement {
                    slot: narrow(slot)?,
                    family: narrow(family)?,
                    tier: narrow(tier)?,
                    remaining_ms: u32::try_from(remaining).map_err(|_| Error::Malformed)?,
                });
            }
            34 if offers.len() < MAX_OFFERS => {
                let [family, tier, gold, premium] = read_row::<4>(read_bytes(bytes, &mut at)?)?;
                offers.push(ImbuementOffer {
                    family: narrow(family)?,
                    tier: narrow(tier)?,
                    gold: u32::try_from(gold).map_err(|_| Error::Malformed)?,
                    requires_premium: match premium {
                        0 => false,
                        1 => true,
                        _ => return Err(Error::Malformed),
                    },
                });
            }
            40 => set_once(&mut clear, read_varint(bytes, &mut at)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    let window = match (slots, clear) {
        (None, None) if applied.is_empty() && offers.is_empty() => None,
        (Some(slots), Some(clear)) => Some(ImbuementWindow {
            slot_count: narrow(slots)?,
            applied,
            offers,
            clear_gold: u32::try_from(clear).map_err(|_| Error::Malformed)?,
        }),
        _ => return Err(Error::Malformed),
    };
    let r = ImbuementReply {
        result: result.and_then(result_of).ok_or(Error::Malformed)?,
        window,
    };
    if encode_imbuement_reply(&r)? != bytes {
        return Err(Error::Malformed);
    }
    Ok(r)
}

#[cfg(test)]
#[path = "imbuement_tests.rs"]
mod tests;
