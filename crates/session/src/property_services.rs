//! Client adapters for existing property/service codecs. No registry or wire changes.
use crate::ItemHandle;
pub use oteryn_protocol_oteryn::{
    appearance, attack, bestiary, charm, charm_progression, guild_bank, house_podium,
    house_rotation, house_wrap, loop_training, mail, pvp_expert, staff, staff_tools,
};

pub use oteryn_protocol_oteryn::{house, house_move_out};
#[path = "property_services/house_handoff.rs"]
mod house_handoff;
#[path = "property_services/house_transfer.rs"]
pub mod house_transfer;
#[path = "property_services/imbuement_details.rs"]
pub mod imbuement_details;
pub mod guildhall {
    pub use oteryn_protocol_oteryn::guild::{
        GuildIntentResult, decode_guild_intent_result, encode_guild_intent_result,
    };
    pub use oteryn_protocol_oteryn::house::{
        CAPABILITY_GUILDHALL_V1, COMMAND_TYPE_GUILDHALL_LEADERSHIP, GuildhallLeadershipIntent,
        decode_guildhall_leadership, encode_guildhall_leadership,
    };
}

pub use oteryn_protocol_oteryn::{container_sort, guild_war, player_report};

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

#[path = "features/0097-character-info/protocol.rs"]
pub mod character_info;
pub use oteryn_protocol_oteryn::boss_encounter;
pub use oteryn_protocol_oteryn::raid_schedule;
pub use oteryn_protocol_oteryn::{bosstiary_slots, hazard};

pub use oteryn_protocol_oteryn::{party_skull, timed_item};
#[path = "features/0057-cyclopedia-map/protocol.rs"]
pub mod cyclopedia_map;
