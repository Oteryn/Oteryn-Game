//! Additive command-436 action 6. Existing Svargrond payloads are unchanged.
use super::{ArenaWireError, push_varint_field};
use crate::charm_wire::read_uint32_fields;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DromeAction {
    Status,
    Enter,
    Leave,
    LowerStart,
    ClaimPotion,
    Buy(u8),
}
impl DromeAction {
    pub(super) fn fields(self) -> (u32, u32) {
        match self {
            Self::Status => (1, 0),
            Self::Enter => (2, 0),
            Self::Leave => (3, 0),
            Self::LowerStart => (4, 0),
            Self::ClaimPotion => (5, 0),
            Self::Buy(reward) => (6, u32::from(reward)),
        }
    }
    pub(super) fn decode(value: u32, reward: u32) -> Result<Self, ArenaWireError> {
        if value == 6 && (1..=18).contains(&reward) {
            return Ok(Self::Buy(reward as u8));
        }
        if reward != 0 {
            return Err(ArenaWireError::Malformed);
        }
        match value {
            1 => Ok(Self::Status),
            2 => Ok(Self::Enter),
            3 => Ok(Self::Leave),
            4 => Ok(Self::LowerStart),
            5 => Ok(Self::ClaimPotion),
            _ => Err(ArenaWireError::Malformed),
        }
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DromeResult {
    /// 1 applied, 2 wrong state, 3 ineligible, 4 cooldown, 5 busy,
    /// 6 content unavailable, 7 no reward, 8 inventory full.
    pub disposition: u32,
    pub level: u32,
    pub start: u32,
    pub points: u32,
    pub phase: u32,
    pub wave: u32,
    pub remaining: u32,
    pub cooldown: u32,
    pub rank: u32,
    pub potions: u32,
    pub rotation_ends: u32,
}
impl DromeResult {
    pub fn encode(self) -> Vec<u8> {
        let mut out = Vec::new();
        for (index, value) in [
            2,
            self.disposition,
            self.level,
            self.start,
            self.points,
            self.phase,
            self.wave,
            self.remaining,
            self.cooldown,
            self.rank,
            self.potions,
            self.rotation_ends,
        ]
        .iter()
        .enumerate()
        {
            if *value != 0 {
                push_varint_field(&mut out, index as u64 + 1, u64::from(*value));
            }
        }
        out
    }
    pub fn decode(payload: &[u8]) -> Result<Self, ArenaWireError> {
        let scalar = super::drome_hazards::scalar_result(payload)?;
        let payload = scalar.as_slice();
        let [
            version,
            disposition,
            level,
            start,
            points,
            phase,
            wave,
            remaining,
            cooldown,
            rank,
            potions,
            rotation_ends,
        ] = read_uint32_fields::<12>(payload)?;
        if version != 2 || !(1..=8).contains(&disposition) || phase > 2 {
            return Err(ArenaWireError::Malformed);
        }
        Ok(Self {
            disposition,
            level,
            start,
            points,
            phase,
            wave,
            remaining,
            cooldown,
            rank,
            potions,
            rotation_ends,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drome_contract_is_additive_and_bounded() -> Result<(), ArenaWireError> {
        for action in [
            DromeAction::Status,
            DromeAction::Enter,
            DromeAction::Leave,
            DromeAction::LowerStart,
            DromeAction::ClaimPotion,
            DromeAction::Buy(1),
            DromeAction::Buy(16),
            DromeAction::Buy(17),
            DromeAction::Buy(18),
        ] {
            let request = super::super::ArenaAction::Drome(action);
            let bytes = super::super::encode_match_request(request)?;
            assert_eq!(super::super::decode_match_request(&bytes)?, request);
        }
        let result = DromeResult {
            disposition: 1,
            points: u32::MAX,
            rotation_ends: u32::MAX,
            ..DromeResult::default()
        };
        assert_eq!(DromeResult::decode(&result.encode())?, result);
        assert!(result.encode().len() <= super::super::MAX_MATCH_RESULT_BYTES);
        assert!(super::super::decode_match_result(&result.encode()).is_err());
        Ok(())
    }
}
