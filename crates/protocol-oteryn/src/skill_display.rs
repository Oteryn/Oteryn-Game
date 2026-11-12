//! NX-7 complete character skills view, negotiated independently from trained progress.
use crate::skill_progress::SkillProgressSnapshot;
pub const CAPABILITY_SKILL_DISPLAY_V1: u32 = 370;
pub const COMMAND_TYPE_SKILL_DISPLAY_QUERY: u32 = 640;
pub const MAX_SKILL_DISPLAY_REPLY_BYTES: usize = 224;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkillDisplay {
    pub trained: SkillProgressSnapshot,
    pub level: u32,
    pub experience: u64,
    pub level_percent: u16,
    pub loyalty_bonus_percent: u8,
    pub loyalty_levels: [u16; 8],
    /// Current owner values including live bonuses; absent when that owner cannot qualify them.
    pub current_levels: Option<[u32; 8]>,
}
impl SkillDisplay {
    pub fn encode(&self) -> Option<Vec<u8>> {
        if self.level == 0
            || self.level_percent > 10_000
            || self.loyalty_bonus_percent > 50
            || !self.loyalty_bonus_percent.is_multiple_of(5)
            || self
                .loyalty_levels
                .iter()
                .zip(self.trained.skills)
                .any(|(loyal, base)| *loyal < base.level)
        {
            return None;
        }
        let mut out = self.trained.encode()?;
        out.extend_from_slice(&self.level.to_le_bytes());
        out.extend_from_slice(&self.experience.to_le_bytes());
        out.extend_from_slice(&self.level_percent.to_le_bytes());
        out.push(self.loyalty_bonus_percent);
        for level in self.loyalty_levels {
            out.extend_from_slice(&level.to_le_bytes());
        }
        out.push(u8::from(self.current_levels.is_some()));
        for level in self.current_levels.unwrap_or([0; 8]) {
            out.extend_from_slice(&level.to_le_bytes());
        }
        Some(out)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != MAX_SKILL_DISPLAY_REPLY_BYTES {
            return None;
        }
        let trained = SkillProgressSnapshot::decode(&bytes[..160])?;
        let mut loyalty_levels = [0; 8];
        for (slot, word) in loyalty_levels
            .iter_mut()
            .zip(bytes[175..191].as_chunks::<2>().0)
        {
            *slot = u16::from_le_bytes(*word);
        }
        let mut current = [0; 8];
        for (slot, word) in current.iter_mut().zip(bytes[192..224].as_chunks::<4>().0) {
            *slot = u32::from_le_bytes(*word);
        }
        let current_levels = match bytes[191] {
            0 if current == [0; 8] => None,
            1 => Some(current),
            _ => return None,
        };
        let value = Self {
            trained,
            level: u32::from_le_bytes(bytes[160..164].try_into().ok()?),
            experience: u64::from_le_bytes(bytes[164..172].try_into().ok()?),
            level_percent: u16::from_le_bytes(bytes[172..174].try_into().ok()?),
            loyalty_bonus_percent: bytes[174],
            loyalty_levels,
            current_levels,
        };
        value.encode()?;
        Some(value)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nx7_skill_display_strict_roundtrip() -> Result<(), &'static str> {
        let value = SkillDisplay {
            level: 8,
            experience: 4200,
            level_percent: 5000,
            current_levels: Some([42; 8]),
            ..Default::default()
        };
        let bytes = value.encode().ok_or("encode")?;
        assert_eq!(SkillDisplay::decode(&bytes), Some(value));
        for end in 0..bytes.len() {
            assert!(SkillDisplay::decode(&bytes[..end]).is_none());
        }
        let mut bad = bytes.clone();
        bad.push(0);
        assert!(SkillDisplay::decode(&bad).is_none());
        bad = bytes;
        bad[191] = 0;
        assert!(SkillDisplay::decode(&bad).is_none());
        Ok(())
    }
}
