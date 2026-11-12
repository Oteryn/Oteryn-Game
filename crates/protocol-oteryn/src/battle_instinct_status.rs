//! C11: the own actor's current, server-owned Battle Instinct bonuses.

pub const CAPABILITY_BATTLE_INSTINCT_STATUS_V1: u32 = 278;
pub const COMMAND_TYPE_BATTLE_INSTINCT_STATUS_QUERY: u32 = 494;
pub const MAX_BATTLE_INSTINCT_STATUS_REPLY_BYTES: usize = 9;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BattleInstinctStatus {
    /// Current Wheel eligibility, independent of whether enough creatures are adjacent.
    pub active: bool,
    pub melee: u32,
    pub shielding: u32,
}

pub fn decode_query(payload: &[u8]) -> Option<()> {
    payload.is_empty().then_some(())
}

impl BattleInstinctStatus {
    pub fn encode(self) -> Option<Vec<u8>> {
        if !self.active && (self.melee != 0 || self.shielding != 0) {
            return None;
        }
        let mut payload = Vec::with_capacity(MAX_BATTLE_INSTINCT_STATUS_REPLY_BYTES);
        payload.push(u8::from(self.active));
        payload.extend_from_slice(&self.melee.to_le_bytes());
        payload.extend_from_slice(&self.shielding.to_le_bytes());
        Some(payload)
    }

    pub fn decode(payload: &[u8]) -> Option<Self> {
        if payload.len() != MAX_BATTLE_INSTINCT_STATUS_REPLY_BYTES {
            return None;
        }
        let active = match payload[0] {
            0 => false,
            1 => true,
            _ => return None,
        };
        let value = Self {
            active,
            melee: u32::from_le_bytes(payload[1..5].try_into().ok()?),
            shielding: u32::from_le_bytes(payload[5..9].try_into().ok()?),
        };
        (value.active || (value.melee == 0 && value.shielding == 0)).then_some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battle_instinct_wire_round_trips_and_rejects_noncanonical_values() -> Result<(), &'static str>
    {
        for value in [
            BattleInstinctStatus::default(),
            BattleInstinctStatus {
                active: true,
                melee: 4,
                shielding: 24,
            },
            BattleInstinctStatus {
                active: true,
                melee: 0,
                shielding: 0,
            },
        ] {
            let payload = value.encode().ok_or("encode")?;
            assert_eq!(BattleInstinctStatus::decode(&payload), Some(value));
            for end in 0..payload.len() {
                assert!(BattleInstinctStatus::decode(&payload[..end]).is_none());
            }
            let mut trailing = payload;
            trailing.push(0);
            assert!(BattleInstinctStatus::decode(&trailing).is_none());
        }
        assert!(
            BattleInstinctStatus {
                active: false,
                melee: 1,
                shielding: 0
            }
            .encode()
            .is_none()
        );
        assert!(BattleInstinctStatus::decode(&[2, 0, 0, 0, 0, 0, 0, 0, 0]).is_none());
        assert!(BattleInstinctStatus::decode(&[0, 1, 0, 0, 0, 0, 0, 0, 0]).is_none());
        assert_eq!(decode_query(&[]), Some(()));
        assert_eq!(decode_query(&[0]), None);
        Ok(())
    }
}
