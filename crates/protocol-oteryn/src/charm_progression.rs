//! PARITY-PROGRESSION-QUERIES-1: Bestiary and charm progression as one request/reply command.
//!
//! Schema: `docs/contracts/protocol-oteryn/v1/charm_progression_v1.proto`. One command type
//! carries the four actions of the Canary Bestiary and charm windows: the query (race progress,
//! charm points, charms with unlock and assignment state), unlocking a charm's next stage for
//! charm points, assigning a charm to a race, and removing a charm from its race for a gold fee of
//! `level * 100` (Canary `IOBestiary::sendBuyCharmRune` action 2). Every reply carries the views
//! as of the reply, so the capability has no state domain and no snapshot obligation. Capability
//! 1 `BESTIARY_CHARMS_V1` stays unoffered (D170).
//!
//! Decoding is strict and encoding refuses the same values before any byte is emitted: an unknown
//! action or disposition, a charm index absent or present against its action, a race present
//! except on assign, an expected stage except on unlock, a fee except on a completed remove, and
//! any view the Bestiary and charm codecs refuse all fail closed.

use std::num::NonZeroU32;

use crate::bestiary::{
    BestiaryRaceProgress, MAX_BESTIARY_RACE, MAX_BESTIARY_VIEW_BYTES, decode_bestiary_view,
    encode_bestiary_view,
};
use crate::charm::{
    CharmAssignDisposition, CharmUnlockDisposition, CharmView, MAX_CHARM, MAX_CHARM_STAGE,
    MAX_CHARM_VIEW_BYTES, decode_charm_view, encode_charm_view,
};
pub use crate::charm_wire::CyclopediaWireError;
use crate::charm_wire::{
    WireResult, push_message_field, push_nonzero_varint_field, push_varint_field, read_bytes,
    read_uint32_fields, read_varint, set_once,
};

/// Registered capability `BESTIARY_CHARM_PROGRESSION_V1`: command type 84, no state domain.
pub const CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1: u32 = 67;
pub const COMMAND_TYPE_CHARM_PROGRESSION: u32 = 84;
/// Four `uint32` fields of at most two value bytes each.
pub const MAX_REQUEST_BYTES: usize = 12;
/// Action, disposition, a ten-byte fee and both views with their length prefixes.
pub const MAX_REPLY_BYTES: usize =
    2 + 2 + 11 + 4 + MAX_BESTIARY_VIEW_BYTES + 3 + MAX_CHARM_VIEW_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharmProgressionAction {
    Query,
    /// Unlock the next stage of `charm`, which the client saw at `expected_stage`.
    Unlock {
        charm: NonZeroU32,
        expected_stage: u8,
    },
    Assign {
        charm: NonZeroU32,
        race: NonZeroU32,
    },
    /// Remove `charm` from the race it holds, for the gold fee.
    Remove {
        charm: NonZeroU32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharmRemoveDisposition {
    /// Removed; the reply names the fee paid.
    Removed = 1,
    /// The charm holds no race.
    NotAssigned = 2,
    /// The carried coins and the bank balance together do not cover the fee.
    NotEnoughGold = 3,
    /// The change of the coins paid does not fit the backpack or the capacity.
    ChangeDoesNotFit = 4,
    UnknownCharm = 5,
    /// Ineligible actor, lost Character fence, or the charm system is not available.
    Rejected = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharmProgressionOutcome {
    Query,
    Unlock(CharmUnlockDisposition),
    Assign(CharmAssignDisposition),
    Remove(CharmRemoveDisposition),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharmProgressionReply {
    pub outcome: CharmProgressionOutcome,
    /// The gold paid; non-zero only on `Remove(Removed)`.
    pub fee_gold: u64,
    pub bestiary: Vec<BestiaryRaceProgress>,
    pub charms: CharmView,
}

impl CharmProgressionAction {
    fn fields(&self) -> [u32; 4] {
        match *self {
            Self::Query => [1, 0, 0, 0],
            Self::Unlock {
                charm,
                expected_stage,
            } => [2, charm.get(), 0, u32::from(expected_stage)],
            Self::Assign { charm, race } => [3, charm.get(), race.get(), 0],
            Self::Remove { charm } => [4, charm.get(), 0, 0],
        }
    }

    fn check(&self) -> WireResult<()> {
        let [_, charm, race, stage] = self.fields();
        if charm > MAX_CHARM || race > MAX_BESTIARY_RACE {
            return Err(CyclopediaWireError::LimitExceeded);
        }
        if stage >= u32::from(MAX_CHARM_STAGE) {
            return Err(CyclopediaWireError::Malformed);
        }
        Ok(())
    }
}

pub fn encode_charm_progression_request(action: &CharmProgressionAction) -> WireResult<Vec<u8>> {
    action.check()?;
    let mut output = Vec::with_capacity(MAX_REQUEST_BYTES);
    for (field, value) in (1..).zip(action.fields()) {
        push_nonzero_varint_field(&mut output, field, u64::from(value));
    }
    Ok(output)
}

pub fn decode_charm_progression_request(payload: &[u8]) -> WireResult<CharmProgressionAction> {
    if payload.len() > MAX_REQUEST_BYTES {
        return Err(CyclopediaWireError::LimitExceeded);
    }
    let [action, charm, race, stage] = read_uint32_fields::<4>(payload)?;
    let charm_index = NonZeroU32::new(charm);
    let decoded = match (action, charm_index, race, stage) {
        (1, None, 0, 0) => CharmProgressionAction::Query,
        (2, Some(charm), 0, stage) => CharmProgressionAction::Unlock {
            charm,
            expected_stage: u8::try_from(stage).map_err(|_| CyclopediaWireError::Malformed)?,
        },
        (3, Some(charm), race, 0) => CharmProgressionAction::Assign {
            charm,
            race: NonZeroU32::new(race).ok_or(CyclopediaWireError::Malformed)?,
        },
        (4, Some(charm), 0, 0) => CharmProgressionAction::Remove { charm },
        _ => return Err(CyclopediaWireError::Malformed),
    };
    decoded.check()?;
    Ok(decoded)
}

impl CharmProgressionOutcome {
    fn codes(self) -> (u32, u32) {
        match self {
            Self::Query => (1, 1),
            Self::Unlock(disposition) => (2, disposition as u32),
            Self::Assign(disposition) => (3, disposition as u32),
            Self::Remove(disposition) => (4, disposition as u32),
        }
    }

    fn from_codes(action: u32, disposition: u32) -> WireResult<Self> {
        use CharmAssignDisposition as A;
        use CharmRemoveDisposition as R;
        use CharmUnlockDisposition as U;
        Ok(match (action, disposition) {
            (1, 1) => Self::Query,
            (2, 1) => Self::Unlock(U::Unlocked),
            (2, 2) => Self::Unlock(U::NotEnoughCharmPoints),
            (2, 3) => Self::Unlock(U::NotEnoughMinorCharmEchoes),
            (2, 4) => Self::Unlock(U::StageMismatch),
            (2, 5) => Self::Unlock(U::UnknownCharm),
            (2, 6) => Self::Unlock(U::Rejected),
            (3, 1) => Self::Assign(A::Assigned),
            (3, 2) => Self::Assign(A::CharmLocked),
            (3, 3) => Self::Assign(A::AlreadyAssigned),
            (3, 4) => Self::Assign(A::RaceStageTooLow),
            (3, 5) => Self::Assign(A::RaceCharmLimit),
            (3, 6) => Self::Assign(A::UnknownCharm),
            (3, 7) => Self::Assign(A::UnknownRace),
            (3, 8) => Self::Assign(A::Rejected),
            (3, 9) => Self::Assign(A::AssignmentSlotsFull),
            (4, 1) => Self::Remove(R::Removed),
            (4, 2) => Self::Remove(R::NotAssigned),
            (4, 3) => Self::Remove(R::NotEnoughGold),
            (4, 4) => Self::Remove(R::ChangeDoesNotFit),
            (4, 5) => Self::Remove(R::UnknownCharm),
            (4, 6) => Self::Remove(R::Rejected),
            _ => return Err(CyclopediaWireError::Malformed),
        })
    }
}

pub fn encode_charm_progression_reply(reply: &CharmProgressionReply) -> WireResult<Vec<u8>> {
    let removed = reply.outcome == CharmProgressionOutcome::Remove(CharmRemoveDisposition::Removed);
    if removed != (reply.fee_gold != 0) || reply.fee_gold > i64::MAX as u64 {
        return Err(CyclopediaWireError::Malformed);
    }
    let bestiary = encode_bestiary_view(&reply.bestiary)?;
    let charms = encode_charm_view(&reply.charms)?;
    let (action, disposition) = reply.outcome.codes();
    let mut output = Vec::with_capacity(16 + bestiary.len() + charms.len());
    push_varint_field(&mut output, 1, u64::from(action));
    push_varint_field(&mut output, 2, u64::from(disposition));
    push_nonzero_varint_field(&mut output, 3, reply.fee_gold);
    push_message_field(&mut output, 4, &bestiary);
    push_message_field(&mut output, 5, &charms);
    Ok(output)
}

pub fn decode_charm_progression_reply(payload: &[u8]) -> WireResult<CharmProgressionReply> {
    if payload.len() > MAX_REPLY_BYTES {
        return Err(CyclopediaWireError::LimitExceeded);
    }
    let (mut action, mut disposition, mut fee, mut bestiary, mut charms) =
        (None, None, None, None, None);
    let mut cursor = 0;
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            0x08 => set_once(&mut action, read_varint(payload, &mut cursor)?)?,
            0x10 => set_once(&mut disposition, read_varint(payload, &mut cursor)?)?,
            0x18 => set_once(&mut fee, read_varint(payload, &mut cursor)?)?,
            0x22 => set_once(&mut bestiary, read_bytes(payload, &mut cursor)?)?,
            0x2a => set_once(&mut charms, read_bytes(payload, &mut cursor)?)?,
            _ => return Err(CyclopediaWireError::Malformed),
        }
    }
    let code = |value: Option<u64>| {
        value
            .and_then(|value| u32::try_from(value).ok())
            .ok_or(CyclopediaWireError::Malformed)
    };
    let reply = CharmProgressionReply {
        outcome: CharmProgressionOutcome::from_codes(code(action)?, code(disposition)?)?,
        fee_gold: match fee {
            Some(0) => return Err(CyclopediaWireError::Malformed),
            fee => fee.unwrap_or(0),
        },
        bestiary: decode_bestiary_view(bestiary.ok_or(CyclopediaWireError::Malformed)?)?,
        charms: decode_charm_view(charms.ok_or(CyclopediaWireError::Malformed)?)?,
    };
    encode_charm_progression_reply(&reply)?;
    Ok(reply)
}

#[cfg(test)]
#[path = "charm_progression_tests.rs"]
mod tests;
