//! Client adapters for existing property/service codecs. No registry or wire changes.
use crate::ItemHandle;
pub use oteryn_protocol_oteryn::{guild_bank, house_podium, loop_training, mail};

pub fn offline_payload(target: ItemHandle) -> Vec<u8> {
    let mut out = vec![8];
    varint(&mut out, target.get());
    out
}
pub fn exercise_payload(intent: loop_training::ExerciseIntent) -> Vec<u8> {
    use loop_training::ExerciseIntent::*;
    let mut out = vec![8];
    match intent {
        Start { weapon, dummy } => {
            out.push(1);
            out.push(16);
            varint(&mut out, weapon.get());
            out.push(24);
            varint(&mut out, dummy.get());
        }
        Status => out.push(2),
        Stop => out.push(3),
    }
    out
}
fn varint(out: &mut Vec<u8>, mut value: u64) {
    while value >= 128 {
        out.push((value as u8 & 127) | 128);
        value >>= 7;
    }
    out.push(value as u8);
}
pub fn training_result(bytes: &[u8]) -> Option<loop_training::TrainingResult> {
    if bytes.len() > loop_training::MAX_TRAINING_RESULT_BYTES {
        return None;
    }
    let mut values = [None; 4];
    let mut cursor = 0;
    while cursor < bytes.len() {
        let tag = *bytes.get(cursor)?;
        cursor += 1;
        if !matches!(tag, 8 | 16 | 24 | 32) {
            return None;
        }
        let slot = &mut values[usize::from(tag / 8 - 1)];
        if slot.is_some() {
            return None;
        }
        let mut value = 0u64;
        let mut complete = false;
        for shift in (0..70).step_by(7) {
            let byte = *bytes.get(cursor)?;
            cursor += 1;
            if shift == 63 && byte > 1 {
                return None;
            }
            value |= u64::from(byte & 127) << shift;
            if byte & 128 == 0 {
                complete = true;
                break;
            }
        }
        if !complete {
            return None;
        }
        *slot = Some(value);
    }
    let result = loop_training::TrainingResult {
        disposition: u8::try_from(values[0]?).ok()?,
        revision: values[1]?,
        pool_seconds: u32::try_from(values[2]?).ok()?,
        charges: u32::try_from(values[3]?).ok()?,
    };
    loop_training::encode_training_result(result).ok()?;
    Some(result)
}
