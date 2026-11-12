//! C14 trained progress, independent of loyalty-adjusted skill levels.
pub const CAPABILITY_SKILL_PROGRESS_V1: u32 = 279;
pub const COMMAND_TYPE_SKILL_PROGRESS_QUERY: u32 = 495;
pub const MAX_SKILL_PROGRESS_REQUEST_BYTES: usize = 0;
pub const MAX_SKILL_PROGRESS_REPLY_BYTES: usize = 160;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkillProgress {
    pub level: u16,
    pub progress: u64,
    /// Progress to the next level; zero means the next level is unreachable.
    pub required: u64,
    /// Hundredths of a percent, as Crystal AddPlayerSkills sends it.
    pub percent: u16,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkillProgressSnapshot {
    /// Magic, fist, club, sword, axe, distance, shielding, fishing.
    pub skills: [SkillProgress; 8],
}

pub fn decode_query(bytes: &[u8]) -> Option<()> {
    bytes.is_empty().then_some(())
}

impl SkillProgressSnapshot {
    pub fn encode(&self) -> Option<Vec<u8>> {
        let mut bytes = Vec::with_capacity(MAX_SKILL_PROGRESS_REPLY_BYTES);
        for skill in self.skills {
            if skill.percent > 10_000 {
                return None;
            }
            bytes.extend_from_slice(&skill.level.to_le_bytes());
            bytes.extend_from_slice(&skill.progress.to_le_bytes());
            bytes.extend_from_slice(&skill.required.to_le_bytes());
            bytes.extend_from_slice(&skill.percent.to_le_bytes());
        }
        Some(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != MAX_SKILL_PROGRESS_REPLY_BYTES {
            return None;
        }
        let mut skills = [SkillProgress::default(); 8];
        for (slot, bytes) in skills.iter_mut().zip(bytes.as_chunks::<20>().0) {
            *slot = SkillProgress {
                level: u16::from_le_bytes(bytes[..2].try_into().ok()?),
                progress: u64::from_le_bytes(bytes[2..10].try_into().ok()?),
                required: u64::from_le_bytes(bytes[10..18].try_into().ok()?),
                percent: u16::from_le_bytes(bytes[18..20].try_into().ok()?),
            };
            if slot.percent > 10_000 {
                return None;
            }
        }
        Some(Self { skills })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_progress_codec_bounds_and_every_family() -> Result<(), &'static str> {
        let snapshot = SkillProgressSnapshot {
            skills: std::array::from_fn(|family| SkillProgress {
                level: family as u16 + 10,
                progress: family as u64,
                required: 100,
                percent: family as u16 * 100,
            }),
        };
        let bytes = snapshot.encode().ok_or("encode")?;
        assert_eq!(bytes.len(), 160);
        assert_eq!(SkillProgressSnapshot::decode(&bytes), Some(snapshot));
        for length in 0..bytes.len() {
            assert!(SkillProgressSnapshot::decode(&bytes[..length]).is_none());
        }
        let mut invalid = bytes.clone();
        invalid.push(0);
        assert!(SkillProgressSnapshot::decode(&invalid).is_none());
        invalid = bytes;
        invalid[18..20].copy_from_slice(&10_001u16.to_le_bytes());
        assert!(SkillProgressSnapshot::decode(&invalid).is_none());
        let mut invalid_snapshot = snapshot;
        invalid_snapshot.skills[7].percent = 10_001;
        assert!(invalid_snapshot.encode().is_none());
        assert_eq!(decode_query(&[]), Some(()));
        assert_eq!(decode_query(&[0]), None);
        Ok(())
    }
}
