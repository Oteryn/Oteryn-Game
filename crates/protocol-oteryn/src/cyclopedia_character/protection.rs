//! A current, read-only baseline for normal deaths; final settlement rechecks all owners.
pub const SELECTOR: u8 = 131;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Consumption {
    Keep = 0,
    Regular = 1,
    Twist = 2,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scenario {
    pub experience_lost: u64,
    pub equipment_per_mille: u16,
    pub container_per_mille: u16,
    pub consumption: Consumption,
    pub amulet_consumed: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Forecast {
    pub observed_at_unix_ms: u64,
    pub level: u32,
    pub experience: u64,
    pub regular_blessings: u8,
    pub twist: bool,
    pub promotion_benefit: bool,
    pub red_or_black: bool,
    pub adventurer: bool,
    pub pve: Scenario,
    pub pvp: Option<Scenario>,
}
impl Forecast {
    pub fn encode(self) -> Option<Vec<u8>> {
        if self.level == 0 || self.regular_blessings > 7 {
            return None;
        }
        let mut out = vec![SELECTOR];
        out.extend_from_slice(&self.observed_at_unix_ms.to_le_bytes());
        out.extend_from_slice(&self.level.to_le_bytes());
        out.extend_from_slice(&self.experience.to_le_bytes());
        out.push(self.regular_blessings);
        out.push(
            u8::from(self.twist)
                | (u8::from(self.promotion_benefit) << 1)
                | (u8::from(self.red_or_black) << 2)
                | (u8::from(self.adventurer) << 3)
                | (u8::from(self.pvp.is_some()) << 4),
        );
        for s in std::iter::once(self.pve).chain(self.pvp) {
            if s.experience_lost > self.experience
                || s.equipment_per_mille > 1000
                || s.container_per_mille > 1000
            {
                return None;
            }
            out.extend_from_slice(&s.experience_lost.to_le_bytes());
            out.extend_from_slice(&s.equipment_per_mille.to_le_bytes());
            out.extend_from_slice(&s.container_per_mille.to_le_bytes());
            out.push(s.consumption as u8);
            out.push(u8::from(s.amulet_consumed));
        }
        Some(out)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if !matches!(bytes.len(), 37 | 51) || bytes[0] != SELECTOR || bytes[22] & !31 != 0 {
            return None;
        }
        let flags = bytes[22];
        if bytes.len() != if flags & 16 != 0 { 51 } else { 37 } {
            return None;
        }
        let value = Self {
            observed_at_unix_ms: u64::from_le_bytes(bytes[1..9].try_into().ok()?),
            level: u32::from_le_bytes(bytes[9..13].try_into().ok()?),
            experience: u64::from_le_bytes(bytes[13..21].try_into().ok()?),
            regular_blessings: bytes[21],
            twist: flags & 1 != 0,
            promotion_benefit: flags & 2 != 0,
            red_or_black: flags & 4 != 0,
            adventurer: flags & 8 != 0,
            pve: scenario(&bytes[23..37])?,
            pvp: if flags & 16 != 0 {
                Some(scenario(&bytes[37..51])?)
            } else {
                None
            },
        };
        value.encode().filter(|canonical| canonical == bytes)?;
        Some(value)
    }
}
fn scenario(bytes: &[u8]) -> Option<Scenario> {
    Some(Scenario {
        experience_lost: u64::from_le_bytes(bytes[..8].try_into().ok()?),
        equipment_per_mille: u16::from_le_bytes(bytes[8..10].try_into().ok()?),
        container_per_mille: u16::from_le_bytes(bytes[10..12].try_into().ok()?),
        consumption: match bytes[12] {
            0 => Consumption::Keep,
            1 => Consumption::Regular,
            2 => Consumption::Twist,
            _ => return None,
        },
        amulet_consumed: match bytes[13] {
            0 => false,
            1 => true,
            _ => return None,
        },
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protection_forecast_codec_rejects_truncations_and_invalid_probability()
    -> Result<(), &'static str> {
        let s = Scenario {
            experience_lost: 10,
            equipment_per_mille: 100,
            container_per_mille: 1000,
            consumption: Consumption::Regular,
            amulet_consumed: false,
        };
        let value = Forecast {
            observed_at_unix_ms: 1,
            level: 20,
            experience: 20,
            regular_blessings: 0,
            twist: false,
            promotion_benefit: false,
            red_or_black: false,
            adventurer: false,
            pve: s,
            pvp: Some(s),
        };
        let bytes = value.encode().ok_or("encode")?;
        assert_eq!(Forecast::decode(&bytes), Some(value));
        for end in 0..bytes.len() {
            assert!(Forecast::decode(&bytes[..end]).is_none());
        }
        let unavailable = Forecast { pvp: None, ..value };
        let no_pvp = unavailable.encode().ok_or("no PvP")?;
        assert_eq!(no_pvp.len(), 37);
        assert_eq!(Forecast::decode(&no_pvp), Some(unavailable));
        let mut bad = bytes;
        bad[31..33].copy_from_slice(&1001_u16.to_le_bytes());
        assert!(Forecast::decode(&bad).is_none());
        Ok(())
    }
}
