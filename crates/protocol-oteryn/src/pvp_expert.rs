//! PVP-EXPERT-1: additive fight modes, without changing ATTACK_V1 payloads.
use crate::attack::{FightModes, decode_fight_modes_intent};
use crate::charm_wire::{
    CyclopediaWireError, WireResult, push_varint_field, read_varint, set_once,
};

pub const CAPABILITY_PVP_EXPERT_V1: u32 = 87;
pub const COMMAND_TYPE_EXPERT_FIGHT_MODES_INTENT: u32 = 103;
pub const STATE_DOMAIN_EXPERT_FIGHT_MODES: u32 = 52;
/// PVPX-RL-01: four one-byte keys and one-byte values.
pub const MAX_EXPERT_FIGHT_MODES_BYTES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum ExpertMode {
    #[default]
    Dove = 0,
    WhiteHand = 1,
    YellowHand = 2,
    RedFist = 3,
}

/// Command 103 and domain 52 use the same complete, authoritative mode tuple.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpertFightModes {
    pub modes: FightModes,
    pub expert: ExpertMode,
}

impl Default for ExpertFightModes {
    fn default() -> Self {
        Self {
            modes: FightModes::DEFAULT,
            expert: ExpertMode::Dove,
        }
    }
}

pub fn encode_expert_fight_modes(state: &ExpertFightModes) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(MAX_EXPERT_FIGHT_MODES_BYTES);
    push_varint_field(&mut bytes, 1, state.modes.fight_mode as u64);
    push_varint_field(&mut bytes, 2, state.modes.chase as u64);
    push_varint_field(&mut bytes, 3, u64::from(state.modes.secure));
    push_varint_field(&mut bytes, 4, state.expert as u64);
    bytes
}

/// All four fields are required, including Dove's explicit zero. Unknown/duplicate fields,
/// invalid enums/bools and oversized payloads refuse before the authority is called.
pub fn decode_expert_fight_modes(bytes: &[u8]) -> WireResult<ExpertFightModes> {
    if bytes.len() > MAX_EXPERT_FIGHT_MODES_BYTES {
        return Err(CyclopediaWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut fields = [None; 4];
    while cursor < bytes.len() {
        let index = match read_varint(bytes, &mut cursor)? {
            8 => 0,
            16 => 1,
            24 => 2,
            32 => 3,
            _ => return Err(CyclopediaWireError::Malformed),
        };
        set_once(&mut fields[index], read_varint(bytes, &mut cursor)?)?;
    }
    let mut modes = Vec::with_capacity(6);
    for (i, value) in fields[..3].iter().enumerate() {
        push_varint_field(
            &mut modes,
            i as u64 + 1,
            value.ok_or(CyclopediaWireError::Malformed)?,
        );
    }
    let expert = match fields[3] {
        Some(0) => ExpertMode::Dove,
        Some(1) => ExpertMode::WhiteHand,
        Some(2) => ExpertMode::YellowHand,
        Some(3) => ExpertMode::RedFist,
        _ => return Err(CyclopediaWireError::Malformed),
    };
    Ok(ExpertFightModes {
        modes: decode_fight_modes_intent(&modes)?,
        expert,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pvp_expert_wire_all_modes_round_trip() {
        for expert in [
            ExpertMode::Dove,
            ExpertMode::WhiteHand,
            ExpertMode::YellowHand,
            ExpertMode::RedFist,
        ] {
            for secure in [false, true] {
                let state = ExpertFightModes {
                    modes: FightModes {
                        secure,
                        ..FightModes::DEFAULT
                    },
                    expert,
                };
                let bytes = encode_expert_fight_modes(&state);
                assert_eq!(bytes.len(), MAX_EXPERT_FIGHT_MODES_BYTES);
                assert_eq!(decode_expert_fight_modes(&bytes), Ok(state));
            }
        }
    }

    #[test]
    fn pvp_expert_wire_refuses_missing_duplicate_unknown_and_invalid_fields() {
        let valid = encode_expert_fight_modes(&ExpertFightModes::default());
        for index in 0..4 {
            let mut missing = valid.clone();
            missing.drain(index * 2..index * 2 + 2);
            assert!(decode_expert_fight_modes(&missing).is_err());
            let mut duplicate = valid.clone();
            duplicate[index * 2] = valid[(index * 2 + 2) % 8];
            assert!(decode_expert_fight_modes(&duplicate).is_err());
        }
        for (index, value) in [(0, 0), (0, 4), (1, 0), (1, 3), (2, 2), (3, 4), (3, 255)] {
            let mut bad = valid.clone();
            bad[index * 2 + 1] = value;
            assert!(decode_expert_fight_modes(&bad).is_err());
        }
        let mut unknown = valid.clone();
        unknown[6] = 40;
        assert!(decode_expert_fight_modes(&unknown).is_err());
        assert!(decode_expert_fight_modes(&[0; 9]).is_err());
    }
}
