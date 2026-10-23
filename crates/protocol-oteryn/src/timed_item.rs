//! ITEM-TIMED-1: carried timed-item inspection/use and Aldo repair.
//! Identities and elapsed time are resolved by the admitted server owner.
use crate::charm_wire::{
    CyclopediaWireError, WireResult, push_message_field, push_varint_field, read_bytes,
    read_varint, set_once,
};
use crate::item_view::ItemHandle;
use crate::world_spatial_entities::EntityRef;

pub const CAPABILITY_TIMED_ITEM_V1: u32 = 37;
pub const COMMAND_TYPE_TIMED_ITEM_INTENT: u32 = 36;
pub const MAX_TIMED_ITEM_INTENT_BYTES: usize = 64;
pub const MAX_TIMED_ITEM_RESULT_BYTES: usize = 48;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TimedAction {
    Inspect = 1,
    Use = 2,
    Repair = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedItemIntent {
    pub action: TimedAction,
    pub target: ItemHandle,
    pub expected_revision: u64,
    /// Required only for repair; the NPC must be Aldo, currently in range.
    pub npc: Option<EntityRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TimedDisposition {
    Ok = 1,
    Rejected = 2,
    Stale = 3,
    InsufficientFunds = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedItemResult {
    pub disposition: TimedDisposition,
    pub revision: u64,
    pub charges: Option<u32>,
    pub remaining_ms: Option<u64>,
    pub retired: bool,
    pub fee_gold: u32,
}

impl TimedItemResult {
    pub const fn refused(disposition: TimedDisposition) -> Self {
        Self {
            disposition,
            revision: 0,
            charges: None,
            remaining_ms: None,
            retired: false,
            fee_gold: 0,
        }
    }
}

fn malformed<T>() -> WireResult<T> {
    Err(CyclopediaWireError::Malformed)
}

pub fn encode_timed_item_intent(value: TimedItemIntent) -> WireResult<Vec<u8>> {
    if (value.action == TimedAction::Repair) != value.npc.is_some()
        || value.npc.is_some_and(|npc| npc.generation == 0)
    {
        return malformed();
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, value.action as u64);
    push_varint_field(&mut out, 2, value.target.get());
    push_varint_field(&mut out, 3, value.expected_revision);
    if let Some(npc) = value.npc {
        push_message_field(&mut out, 4, &npc.identity);
        push_varint_field(&mut out, 5, npc.generation);
    }
    Ok(out)
}

pub fn decode_timed_item_intent(input: &[u8]) -> WireResult<TimedItemIntent> {
    if input.len() > MAX_TIMED_ITEM_INTENT_BYTES {
        return malformed();
    }
    let (mut action, mut target, mut revision, mut identity, mut generation) =
        (None, None, None, None, None);
    let mut cursor = 0;
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            8 => set_once(&mut action, read_varint(input, &mut cursor)?)?,
            16 => set_once(&mut target, read_varint(input, &mut cursor)?)?,
            24 => set_once(&mut revision, read_varint(input, &mut cursor)?)?,
            34 => set_once(
                &mut identity,
                read_bytes(input, &mut cursor)?
                    .try_into()
                    .map_err(|_| CyclopediaWireError::Malformed)?,
            )?,
            40 => set_once(&mut generation, read_varint(input, &mut cursor)?)?,
            _ => return malformed(),
        }
    }
    let action = match action {
        Some(1) => TimedAction::Inspect,
        Some(2) => TimedAction::Use,
        Some(3) => TimedAction::Repair,
        _ => return malformed(),
    };
    let npc = match (identity, generation) {
        (None, None) => None,
        (Some(identity), Some(generation)) if generation > 0 => Some(EntityRef {
            identity,
            generation,
        }),
        _ => return malformed(),
    };
    let value = TimedItemIntent {
        action,
        target: target
            .and_then(ItemHandle::new)
            .ok_or(CyclopediaWireError::Malformed)?,
        expected_revision: revision.ok_or(CyclopediaWireError::Malformed)?,
        npc,
    };
    encode_timed_item_intent(value)?;
    Ok(value)
}

pub fn encode_timed_item_result(value: TimedItemResult) -> WireResult<Vec<u8>> {
    if value.charges.is_some_and(|n| n == 0 || n > 65_535)
        || value.remaining_ms.is_some_and(|n| n > 604_800_000)
        || ![0, 10_000].contains(&value.fee_gold)
        || (value.disposition != TimedDisposition::Ok
            && value != TimedItemResult::refused(value.disposition))
    {
        return malformed();
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, value.disposition as u64);
    push_varint_field(&mut out, 2, value.revision);
    if let Some(charges) = value.charges {
        push_varint_field(&mut out, 3, u64::from(charges));
    }
    if let Some(remaining) = value.remaining_ms {
        push_varint_field(&mut out, 4, remaining);
    }
    push_varint_field(&mut out, 5, u64::from(value.retired));
    push_varint_field(&mut out, 6, u64::from(value.fee_gold));
    Ok(out)
}

pub fn decode_timed_item_result(input: &[u8]) -> WireResult<TimedItemResult> {
    if input.len() > MAX_TIMED_ITEM_RESULT_BYTES {
        return malformed();
    }
    let mut fields = [None; 6];
    let mut cursor = 0;
    while cursor < input.len() {
        let tag = read_varint(input, &mut cursor)?;
        if tag & 7 != 0 || !(1..=6).contains(&(tag >> 3)) {
            return malformed();
        }
        set_once(
            &mut fields[(tag >> 3) as usize - 1],
            read_varint(input, &mut cursor)?,
        )?;
    }
    let value = TimedItemResult {
        disposition: match fields[0] {
            Some(1) => TimedDisposition::Ok,
            Some(2) => TimedDisposition::Rejected,
            Some(3) => TimedDisposition::Stale,
            Some(4) => TimedDisposition::InsufficientFunds,
            _ => return malformed(),
        },
        revision: fields[1].ok_or(CyclopediaWireError::Malformed)?,
        charges: fields[2]
            .map(u32::try_from)
            .transpose()
            .map_err(|_| CyclopediaWireError::Malformed)?,
        remaining_ms: fields[3],
        retired: match fields[4] {
            Some(0) => false,
            Some(1) => true,
            _ => return malformed(),
        },
        fee_gold: fields[5]
            .and_then(|v| u32::try_from(v).ok())
            .ok_or(CyclopediaWireError::Malformed)?,
    };
    encode_timed_item_result(value)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timed_item_codecs_are_bounded_and_closed() -> WireResult<()> {
        for action in [TimedAction::Inspect, TimedAction::Use, TimedAction::Repair] {
            let intent = TimedItemIntent {
                action,
                target: ItemHandle::new(u64::MAX).ok_or(CyclopediaWireError::Malformed)?,
                expected_revision: u64::MAX,
                npc: (action == TimedAction::Repair).then_some(EntityRef {
                    identity: [255; 16],
                    generation: u64::MAX,
                }),
            };
            let bytes = encode_timed_item_intent(intent)?;
            assert!(bytes.len() <= MAX_TIMED_ITEM_INTENT_BYTES);
            assert_eq!(decode_timed_item_intent(&bytes)?, intent);
            let mut repeated = bytes.clone();
            repeated.extend_from_slice(&[8, 1]);
            assert!(decode_timed_item_intent(&repeated).is_err());
            for cut in 0..bytes.len() {
                if cut < 6 {
                    assert!(decode_timed_item_intent(&bytes[..cut]).is_err());
                }
            }
        }
        let result = TimedItemResult {
            disposition: TimedDisposition::Ok,
            revision: u64::MAX,
            charges: Some(65_535),
            remaining_ms: Some(604_800_000),
            retired: false,
            fee_gold: 10_000,
        };
        let bytes = encode_timed_item_result(result)?;
        assert!(bytes.len() <= MAX_TIMED_ITEM_RESULT_BYTES);
        assert_eq!(decode_timed_item_result(&bytes)?, result);
        for invalid in [
            vec![],
            vec![8, 9],
            vec![0; 65],
            vec![8, 1, 16, 0, 24, 0, 40, 0, 48, 0],
        ] {
            assert!(decode_timed_item_result(&invalid).is_err());
        }
        assert!(decode_timed_item_intent(&[0; 65]).is_err());
        Ok(())
    }
}
