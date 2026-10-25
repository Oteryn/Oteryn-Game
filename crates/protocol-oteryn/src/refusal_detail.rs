//! `REFUSAL_DETAIL_V1` (PARITY-REFUSAL-DETAIL-1; schema in
//! `docs/contracts/protocol-oteryn/v1/refusal_detail_v1.proto`). Capability 66 owns no command
//! type or domain: it appends field 15, a [`RefusalDetail`], to a refused step, use or item move
//! result. Without capability 66 nothing is appended, so those results keep their bytes. The
//! client decodes with [`split_refusal_detail`] and hands the rest to the result's own strict
//! decoder, which still refuses field 15.

use crate::charm_wire::{CyclopediaWireError, WireResult, push_varint_field, read_varint};
use crate::item_view::ItemMoveOutcome;
use crate::world_object::UseDisposition;
use crate::world_spatial::StepDisposition;

/// Registered capability `REFUSAL_DETAIL_V1`; offered only by a node serving a bundle World.
pub const CAPABILITY_REFUSAL_DETAIL_V1: u32 = 66;
/// The result field that carries the detail.
pub const REFUSAL_DETAIL_FIELD: u64 = 15;
/// `REFUSAL-DETAIL-RL-01`: field 15 is one tag byte and one value byte.
pub const MAX_REFUSAL_DETAIL_BYTES: usize = 2;

/// `RefusalDetailV1`: the Canary `ReturnValue` subset of the common player actions, plus the
/// key-door refusal of `key_door.lua`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalDetail {
    NotPossible = 1,
    TooFarAway = 2,
    NotEnoughRoom = 3,
    NotEnoughCapacity = 4,
    CannotPickup = 5,
    FirstGoUpstairs = 6,
    FirstGoDownstairs = 7,
    DoorLocked = 8,
}

impl RefusalDetail {
    pub const ALL: [Self; 8] = [
        Self::NotPossible,
        Self::TooFarAway,
        Self::NotEnoughRoom,
        Self::NotEnoughCapacity,
        Self::CannotPickup,
        Self::FirstGoUpstairs,
        Self::FirstGoDownstairs,
        Self::DoorLocked,
    ];

    pub const fn from_wire(value: u64) -> Option<Self> {
        Some(match value {
            1 => Self::NotPossible,
            2 => Self::TooFarAway,
            3 => Self::NotEnoughRoom,
            4 => Self::NotEnoughCapacity,
            5 => Self::CannotPickup,
            6 => Self::FirstGoUpstairs,
            7 => Self::FirstGoDownstairs,
            8 => Self::DoorLocked,
            _ => return None,
        })
    }

    /// The reference text: Canary `getReturnMessage` (`src/utils/tools.cpp`) and
    /// `key_door.lua`. The client renders its own text; this pins the mapping.
    pub const fn reference_message(self) -> &'static str {
        match self {
            Self::NotPossible => "Sorry, not possible.",
            Self::TooFarAway => "Too far away.",
            Self::NotEnoughRoom => "There is not enough room.",
            Self::NotEnoughCapacity => "This object is too heavy for you to carry.",
            Self::CannotPickup => "You cannot take this object.",
            Self::FirstGoUpstairs => "First go upstairs.",
            Self::FirstGoDownstairs => "First go downstairs.",
            Self::DoorLocked => "It is locked.",
        }
    }

    /// The detail of an item move result whose outcome names its cause. `NOT_OWNER` and
    /// `REQUIREMENT_NOT_MET` carry no detail: their Canary text depends on state the result
    /// does not hold.
    pub const fn of_item_move(outcome: ItemMoveOutcome) -> Option<Self> {
        match outcome {
            ItemMoveOutcome::TooFar => Some(Self::TooFarAway),
            ItemMoveOutcome::NoRoom | ItemMoveOutcome::NoBackpack => Some(Self::NotEnoughRoom),
            ItemMoveOutcome::NotPickupable => Some(Self::CannotPickup),
            ItemMoveOutcome::Stale
            | ItemMoveOutcome::NotSupported
            | ItemMoveOutcome::Rejected
            | ItemMoveOutcome::SlotMismatch
            | ItemMoveOutcome::Blocked => Some(Self::NotPossible),
            ItemMoveOutcome::Moved
            | ItemMoveOutcome::NotOwner
            | ItemMoveOutcome::RequirementNotMet => None,
        }
    }

    /// The detail of a `USE` result whose disposition names its cause.
    pub const fn of_use(disposition: UseDisposition) -> Option<Self> {
        match disposition {
            UseDisposition::TooFar => Some(Self::TooFarAway),
            UseDisposition::Rejected | UseDisposition::NotPossible => Some(Self::NotPossible),
            UseDisposition::Locked => Some(Self::DoorLocked),
            UseDisposition::NoRoom => Some(Self::NotEnoughRoom),
            _ => None,
        }
    }

    /// The detail of a step result: Canary answers a blocked step `NOTPOSSIBLE`.
    pub const fn of_step(disposition: StepDisposition) -> Option<Self> {
        match disposition {
            StepDisposition::Blocked => Some(Self::NotPossible),
            StepDisposition::Moved | StepDisposition::Rejected | StepDisposition::TooEarly => None,
        }
    }

    /// Canary `canUse`: a target on another floor is `FIRSTGOUPSTAIRS` when it is above the
    /// actor, else `FIRSTGODOWNSTAIRS`. Floors are native (`0` is the surface, higher is up);
    /// `None` on the same floor.
    pub const fn of_floors(actor_floor: i32, target_floor: i32) -> Option<Self> {
        if actor_floor < target_floor {
            Some(Self::FirstGoUpstairs)
        } else if actor_floor > target_floor {
            Some(Self::FirstGoDownstairs)
        } else {
            None
        }
    }
}

/// Appends field 15 when the session selected capability 66 and there is a detail.
pub fn append_refusal_detail(
    output: &mut Vec<u8>,
    selected_capabilities: &[u32],
    detail: Option<RefusalDetail>,
) {
    if let Some(detail) = detail
        && selected_capabilities.contains(&CAPABILITY_REFUSAL_DETAIL_V1)
    {
        push_varint_field(output, REFUSAL_DETAIL_FIELD, detail as u64);
    }
}

/// Splits a varint-only result into the bytes before field 15 and its detail. Field 15 must be
/// the last field and appear once; a zero or unknown detail, a non-varint field and a malformed
/// varint fail closed. The caller decodes the remaining bytes with the result's own decoder.
pub fn split_refusal_detail(payload: &[u8]) -> WireResult<(&[u8], Option<RefusalDetail>)> {
    let mut cursor = 0;
    while cursor < payload.len() {
        let start = cursor;
        let key = read_varint(payload, &mut cursor)?;
        if key & 0x07 != 0 {
            return Err(CyclopediaWireError::Malformed);
        }
        let value = read_varint(payload, &mut cursor)?;
        if key >> 3 == REFUSAL_DETAIL_FIELD {
            if cursor != payload.len() {
                return Err(CyclopediaWireError::Malformed);
            }
            let detail = RefusalDetail::from_wire(value).ok_or(CyclopediaWireError::Malformed)?;
            return Ok((&payload[..start], Some(detail)));
        }
    }
    Ok((payload, None))
}

#[cfg(test)]
#[path = "refusal_detail_tests.rs"]
mod tests;
