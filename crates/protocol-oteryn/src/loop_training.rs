//! LOOP-TRAIN-1 bounded protocol-oteryn requests. All rates remain server-owned.
use crate::charm_wire::{
    CyclopediaWireError, WireResult, push_varint_field, read_varint, set_once,
};
use crate::item_view::ItemHandle;

pub const CAPABILITY_TRAINING_V1: u32 = 54;
pub const COMMAND_TYPE_OFFLINE_TRAINING: u32 = 66;
pub const COMMAND_TYPE_EXERCISE_TRAINING: u32 = 67;
pub const MAX_TRAINING_REQUEST_BYTES: usize = 32;
pub const MAX_TRAINING_RESULT_BYTES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExerciseIntent {
    Start {
        weapon: ItemHandle,
        dummy: ItemHandle,
    },
    Status,
    Stop,
}

pub fn decode_offline_training(bytes: &[u8]) -> WireResult<ItemHandle> {
    if bytes.len() > MAX_TRAINING_REQUEST_BYTES {
        return Err(CyclopediaWireError::LimitExceeded);
    }
    let mut cursor = 0;
    if read_varint(bytes, &mut cursor)? != 8 {
        return Err(CyclopediaWireError::Malformed);
    }
    let target = handle(read_varint(bytes, &mut cursor)?)?;
    if cursor != bytes.len() {
        return Err(CyclopediaWireError::Malformed);
    }
    Ok(target)
}
fn handle(value: u64) -> WireResult<ItemHandle> {
    ItemHandle::new(value).ok_or(CyclopediaWireError::Malformed)
}
pub fn decode_exercise_training(bytes: &[u8]) -> WireResult<ExerciseIntent> {
    if bytes.len() > MAX_TRAINING_REQUEST_BYTES {
        return Err(CyclopediaWireError::LimitExceeded);
    }
    let (mut action, mut weapon, mut dummy) = (None, None, None);
    let mut cursor = 0;
    while cursor < bytes.len() {
        match read_varint(bytes, &mut cursor)? {
            8 => set_once(&mut action, read_varint(bytes, &mut cursor)?)?,
            16 => set_once(&mut weapon, handle(read_varint(bytes, &mut cursor)?)?)?,
            24 => set_once(&mut dummy, handle(read_varint(bytes, &mut cursor)?)?)?,
            _ => return Err(CyclopediaWireError::Malformed),
        }
    }
    match (action, weapon, dummy) {
        (Some(1), Some(weapon), Some(dummy)) => Ok(ExerciseIntent::Start { weapon, dummy }),
        (Some(2), None, None) => Ok(ExerciseIntent::Status),
        (Some(3), None, None) => Ok(ExerciseIntent::Stop),
        _ => Err(CyclopediaWireError::Malformed),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrainingResult {
    /// 1 running, 2 stopped/settled, 3 offline logout, 4 refused.
    pub disposition: u8,
    /// Durable CharacterRevision, zero only for a refusal.
    pub revision: u64,
    /// Pool seconds in the offline logout response; zero for exercise and refusal.
    pub pool_seconds: u32,
    /// Remaining durable charges, zero after the last-charge retirement.
    pub charges: u32,
}
impl TrainingResult {
    pub const REFUSED: Self = Self {
        disposition: 4,
        revision: 0,
        pool_seconds: 0,
        charges: 0,
    };
}
pub fn encode_training_result(result: TrainingResult) -> WireResult<Vec<u8>> {
    if !(1..=4).contains(&result.disposition)
        || result.pool_seconds > 43200
        || (result.disposition == 4) != (result.revision == 0)
        || (result.disposition == 4 && (result.pool_seconds != 0 || result.charges != 0))
    {
        return Err(CyclopediaWireError::Malformed);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, u64::from(result.disposition));
    push_varint_field(&mut out, 2, result.revision);
    push_varint_field(&mut out, 3, u64::from(result.pool_seconds));
    push_varint_field(&mut out, 4, u64::from(result.charges));
    if out.len() > MAX_TRAINING_RESULT_BYTES {
        return Err(CyclopediaWireError::LimitExceeded);
    }
    Ok(out)
}

#[cfg(test)]
#[path = "loop_training_tests.rs"]
mod tests;
