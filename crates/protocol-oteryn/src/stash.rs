//! SUPPLY-STASH-0: bounded native stash requests and immutable conversion receipts.
use crate::charm_wire::{push_message_field, push_varint_field, read_bytes, read_varint, set_once};
use crate::item_view::ItemHandle;
use crate::world_interaction::WorldInteractionWireError as Error;

pub const CAPABILITY_STASH_V1: u32 = 62;
pub const COMMAND_TYPE_STASH_INTENT: u32 = 79;
pub const DOMAIN_STASH: u32 = 27;
pub const MAX_STASH_UNITS: u32 = 1_000_000_000;
pub const MAX_STASH_OUTPUTS: u32 = 20;
pub const MAX_STASH_REPLY_BYTES: usize = 36_000;
pub const MAX_STASH_PAGE: u32 = 14_999;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StashAction {
    Browse,
    Stow(ItemHandle),
    /// Server-issued row version. The client cannot select a definition to mint.
    Withdraw {
        row: [u8; 16],
        amount: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StashIntent {
    pub page: u32,
    pub action: StashAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StashOutcome {
    Opened = 1,
    Committed = 2,
    NoLocker = 3,
    Stale = 4,
    NoRoom = 5,
    NoBackpack = 6,
    Overweight = 7,
    Unsupported = 8,
    Rejected = 9,
    PremiumRequired = 10,
    AmountLimit = 11,
    Insufficient = 12,
}

impl StashOutcome {
    pub fn message(self) -> &'static str {
        match self {
            Self::Opened | Self::Committed => "",
            Self::NoLocker => "You can't use stash right now.",
            Self::Unsupported => "This item cannot be stowed here.",
            Self::NoRoom => "There is not enough room.",
            Self::NoBackpack => "You need a backpack to retrieve items.",
            Self::Overweight => "This object is too heavy for you to carry.",
            Self::PremiumRequired => "You need a premium account to stow items.",
            Self::AmountLimit => "You can only withdraw up to 20 stacks at a time from the stash.",
            Self::Insufficient => "You do not have enough items in your stash.",
            Self::Stale => "Your stash has changed. Please open it again.",
            Self::Rejected => "You can't use stash right now.",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashRow {
    pub handle: [u8; 16],
    pub definition_key: String,
    pub definition_revision: String,
    pub quantity: u32,
    pub migration_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashReply {
    pub outcome: StashOutcome,
    pub stowed_quantity: u32,
    /// Same transaction on replay, including a durable refusal; browse has none.
    pub transaction: Option<[u8; 16]>,
    pub page: u32,
    pub has_more: bool,
    pub rows: Vec<StashRow>,
}

impl StashReply {
    pub fn message(&self) -> String {
        if self.outcome == StashOutcome::Committed && self.stowed_quantity > 0 {
            format!(
                "Stowed {} object{}",
                self.stowed_quantity,
                if self.stowed_quantity == 1 { "." } else { "s." }
            )
        } else {
            self.outcome.message().into()
        }
    }
}

pub fn encode_stash_intent(value: StashIntent) -> Result<Vec<u8>, Error> {
    if value.page > MAX_STASH_PAGE {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, u64::from(value.page));
    match value.action {
        StashAction::Browse => push_varint_field(&mut out, 2, 1),
        StashAction::Stow(handle) => {
            push_varint_field(&mut out, 2, 2);
            push_varint_field(&mut out, 3, handle.get());
        }
        StashAction::Withdraw { row, amount } => {
            if row == [0; 16] || amount == 0 {
                return Err(Error::Malformed);
            }
            push_varint_field(&mut out, 2, 3);
            push_message_field(&mut out, 4, &row);
            push_varint_field(&mut out, 5, u64::from(amount));
        }
    }
    Ok(out)
}

pub fn decode_stash_intent(bytes: &[u8]) -> Result<StashIntent, Error> {
    if bytes.len() > 48 {
        return Err(Error::LimitExceeded);
    }
    let (mut page, mut action, mut item, mut row, mut amount) = (None, None, None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(&mut page, read_varint(bytes, &mut at)?)?,
            16 => set_once(&mut action, read_varint(bytes, &mut at)?)?,
            24 => set_once(&mut item, read_varint(bytes, &mut at)?)?,
            34 => set_once(
                &mut row,
                <[u8; 16]>::try_from(read_bytes(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            40 => set_once(&mut amount, read_varint(bytes, &mut at)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    let action = match (action, item, row, amount) {
        (Some(1), None, None, None) => StashAction::Browse,
        (Some(2), Some(item), None, None) => {
            StashAction::Stow(ItemHandle::new(item).ok_or(Error::Malformed)?)
        }
        (Some(3), None, Some(row), Some(amount)) => StashAction::Withdraw {
            row,
            amount: u32::try_from(amount).map_err(|_| Error::Malformed)?,
        },
        _ => return Err(Error::Malformed),
    };
    let value = StashIntent {
        page: u32::try_from(page.ok_or(Error::Malformed)?).map_err(|_| Error::Malformed)?,
        action,
    };
    encode_stash_intent(value)?;
    Ok(value)
}

pub fn encode_stash_reply(value: &StashReply) -> Result<Vec<u8>, Error> {
    if value.page > MAX_STASH_PAGE
        || value.stowed_quantity > 100
        || (value.stowed_quantity > 0 && value.outcome != StashOutcome::Committed)
        || value.rows.len() > 32
        || value.rows.iter().any(|r| {
            r.handle == [0; 16]
                || r.definition_key.is_empty()
                || r.definition_key.len() > 512
                || r.definition_revision.is_empty()
                || r.definition_revision.len() > 512
                || !(1..=MAX_STASH_UNITS).contains(&r.quantity)
        })
        || value.rows.windows(2).any(|r| {
            (&r[0].definition_key, &r[0].definition_revision)
                >= (&r[1].definition_key, &r[1].definition_revision)
        })
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, value.outcome as u64);
    if let Some(transaction) = value.transaction {
        push_message_field(&mut out, 2, &transaction);
    }
    push_varint_field(&mut out, 3, u64::from(value.page));
    push_varint_field(&mut out, 4, u64::from(value.has_more));
    for row in &value.rows {
        let mut encoded = Vec::new();
        push_message_field(&mut encoded, 1, &row.handle);
        push_message_field(&mut encoded, 2, row.definition_key.as_bytes());
        push_message_field(&mut encoded, 3, row.definition_revision.as_bytes());
        push_varint_field(&mut encoded, 4, u64::from(row.quantity));
        push_varint_field(&mut encoded, 5, u64::from(row.migration_required));
        push_message_field(&mut out, 5, &encoded);
    }
    push_message_field(&mut out, 6, value.message().as_bytes());
    if value.stowed_quantity > 0 {
        push_varint_field(&mut out, 7, u64::from(value.stowed_quantity));
    }
    if out.len() > MAX_STASH_REPLY_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}

pub fn decode_stash_reply(bytes: &[u8]) -> Result<StashReply, Error> {
    if bytes.len() > MAX_STASH_REPLY_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut outcome, mut transaction, mut page, mut more, mut message) =
        (None, None, None, None, None);
    let mut stowed = None;
    let mut rows = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(&mut outcome, read_varint(bytes, &mut at)?)?,
            18 => set_once(
                &mut transaction,
                <[u8; 16]>::try_from(read_bytes(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            24 => set_once(&mut page, read_varint(bytes, &mut at)?)?,
            32 => set_once(&mut more, read_varint(bytes, &mut at)?)?,
            42 => {
                if rows.len() == 32 {
                    return Err(Error::LimitExceeded);
                }
                let b = read_bytes(bytes, &mut at)?;
                let (mut handle, mut key, mut revision, mut quantity, mut migration) =
                    (None, None, None, None, None);
                let mut pos = 0;
                while pos < b.len() {
                    match read_varint(b, &mut pos)? {
                        10 => set_once(
                            &mut handle,
                            <[u8; 16]>::try_from(read_bytes(b, &mut pos)?)
                                .map_err(|_| Error::Malformed)?,
                        )?,
                        18 => set_once(
                            &mut key,
                            std::str::from_utf8(read_bytes(b, &mut pos)?)
                                .map_err(|_| Error::Malformed)?
                                .to_owned(),
                        )?,
                        26 => set_once(
                            &mut revision,
                            std::str::from_utf8(read_bytes(b, &mut pos)?)
                                .map_err(|_| Error::Malformed)?
                                .to_owned(),
                        )?,
                        32 => set_once(&mut quantity, read_varint(b, &mut pos)?)?,
                        40 => set_once(&mut migration, read_varint(b, &mut pos)?)?,
                        _ => return Err(Error::Malformed),
                    }
                }
                rows.push(StashRow {
                    handle: handle.ok_or(Error::Malformed)?,
                    definition_key: key.ok_or(Error::Malformed)?,
                    definition_revision: revision.ok_or(Error::Malformed)?,
                    quantity: u32::try_from(quantity.ok_or(Error::Malformed)?)
                        .map_err(|_| Error::Malformed)?,
                    migration_required: match migration {
                        Some(0) => false,
                        Some(1) => true,
                        _ => return Err(Error::Malformed),
                    },
                });
            }
            50 => set_once(&mut message, read_bytes(bytes, &mut at)?)?,
            56 => set_once(&mut stowed, read_varint(bytes, &mut at)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    let outcome = match outcome {
        Some(1) => StashOutcome::Opened,
        Some(2) => StashOutcome::Committed,
        Some(3) => StashOutcome::NoLocker,
        Some(4) => StashOutcome::Stale,
        Some(5) => StashOutcome::NoRoom,
        Some(6) => StashOutcome::NoBackpack,
        Some(7) => StashOutcome::Overweight,
        Some(8) => StashOutcome::Unsupported,
        Some(9) => StashOutcome::Rejected,
        Some(10) => StashOutcome::PremiumRequired,
        Some(11) => StashOutcome::AmountLimit,
        Some(12) => StashOutcome::Insufficient,
        _ => return Err(Error::Malformed),
    };
    let value = StashReply {
        outcome,
        stowed_quantity: u32::try_from(stowed.unwrap_or(0)).map_err(|_| Error::Malformed)?,
        transaction,
        page: u32::try_from(page.ok_or(Error::Malformed)?).map_err(|_| Error::Malformed)?,
        has_more: match more {
            Some(0) => false,
            Some(1) => true,
            _ => return Err(Error::Malformed),
        },
        rows,
    };
    if message != Some(value.message().as_bytes()) {
        return Err(Error::Malformed);
    }
    encode_stash_reply(&value)?;
    Ok(value)
}

#[cfg(test)]
#[path = "stash_tests.rs"]
mod tests;
