//! P13 native skills read model. Loyalty never changes trained progress.
//! Crystal `201189f`, `protocolgame.cpp:9391-9402`, exposes the base and
//! loyalty-adjusted absolute levels separately.

pub const CAPABILITY_LOYALTY_SKILLS_V1: u32 = 274;
pub const COMMAND_TYPE_LOYALTY_SKILLS_QUERY: u32 = 488;
pub const MAX_LOYALTY_SKILLS_REQUEST_BYTES: usize = 0;
/// "Enlightened of Tibia", the longest title in the donor catalogue.
pub const MAX_LOYALTY_TITLE_BYTES: usize = 20;
/// Points, percentage, title length/title, then eight pairs of u16 levels.
pub const MAX_LOYALTY_SKILLS_REPLY_BYTES: usize = 6 + MAX_LOYALTY_TITLE_BYTES + 8 * 4;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkillLevel {
    pub base: u16,
    /// Absolute skill level including loyalty, not a percentage or a level delta.
    pub with_loyalty: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoyaltySkills {
    pub points: u32,
    pub bonus_percent: u8,
    pub title: Option<String>,
    /// Magic, fist, club, sword, axe, distance, shielding, fishing.
    pub skills: [SkillLevel; 8],
}

pub fn decode_query(payload: &[u8]) -> Option<()> {
    payload.is_empty().then_some(())
}

impl LoyaltySkills {
    fn valid(&self) -> bool {
        self.bonus_percent <= 50
            && self.bonus_percent.is_multiple_of(5)
            && self.title.as_ref().is_none_or(|title| {
                !title.is_empty()
                    && title.len() <= MAX_LOYALTY_TITLE_BYTES
                    && title
                        .bytes()
                        .all(|byte| byte.is_ascii_alphabetic() || byte == b' ')
            })
            && self
                .skills
                .iter()
                .all(|skill| skill.with_loyalty >= skill.base)
    }

    pub fn encode(&self) -> Option<Vec<u8>> {
        if !self.valid() {
            return None;
        }
        let title = self.title.as_deref().unwrap_or("");
        let mut bytes = Vec::with_capacity(6 + title.len() + 8 * 4);
        bytes.extend_from_slice(&self.points.to_le_bytes());
        bytes.push(self.bonus_percent);
        bytes.push(u8::try_from(title.len()).ok()?);
        bytes.extend_from_slice(title.as_bytes());
        for skill in self.skills {
            bytes.extend_from_slice(&skill.base.to_le_bytes());
            bytes.extend_from_slice(&skill.with_loyalty.to_le_bytes());
        }
        Some(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_LOYALTY_SKILLS_REPLY_BYTES {
            return None;
        }
        let points = u32::from_le_bytes(bytes.get(..4)?.try_into().ok()?);
        let bonus_percent = *bytes.get(4)?;
        let title_len = usize::from(*bytes.get(5)?);
        let title_end = 6usize.checked_add(title_len)?;
        if title_len > MAX_LOYALTY_TITLE_BYTES || bytes.len() != title_end + 8 * 4 {
            return None;
        }
        let title = std::str::from_utf8(bytes.get(6..title_end)?).ok()?;
        let mut skills = [SkillLevel::default(); 8];
        for (slot, words) in skills
            .iter_mut()
            .zip(bytes.get(title_end..)?.as_chunks::<4>().0)
        {
            *slot = SkillLevel {
                base: u16::from_le_bytes([words[0], words[1]]),
                with_loyalty: u16::from_le_bytes([words[2], words[3]]),
            };
        }
        let value = Self {
            points,
            bonus_percent,
            title: (!title.is_empty()).then(|| title.to_owned()),
            skills,
        };
        value.valid().then_some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> LoyaltySkills {
        LoyaltySkills {
            points: 7_000,
            bonus_percent: 50,
            title: Some("Enlightened of Tibia".into()),
            skills: [SkillLevel {
                base: 100,
                with_loyalty: 104,
            }; 8],
        }
    }

    #[test]
    fn loyalty_skills_query_and_result_round_trip_at_the_bound() -> Result<(), &'static str> {
        assert_eq!(decode_query(&[]), Some(()));
        assert_eq!(decode_query(&[0]), None);
        let value = snapshot();
        let encoded = value.encode().ok_or("encode")?;
        assert_eq!(encoded.len(), MAX_LOYALTY_SKILLS_REPLY_BYTES);
        assert_eq!(LoyaltySkills::decode(&encoded), Some(value));
        Ok(())
    }

    #[test]
    fn loyalty_skills_zero_projection_keeps_the_base_and_has_no_title() -> Result<(), &'static str>
    {
        let value = LoyaltySkills {
            points: 0,
            bonus_percent: 0,
            title: None,
            skills: [SkillLevel {
                base: 10,
                with_loyalty: 10,
            }; 8],
        };
        let encoded = value.encode().ok_or("encode")?;
        assert_eq!(encoded.len(), 38);
        assert_eq!(LoyaltySkills::decode(&encoded), Some(value));
        Ok(())
    }

    #[test]
    fn loyalty_skills_refuses_truncation_trailing_bytes_and_invalid_fields()
    -> Result<(), &'static str> {
        let encoded = snapshot().encode().ok_or("encode")?;
        for end in 0..encoded.len() {
            assert!(LoyaltySkills::decode(&encoded[..end]).is_none());
        }
        let mut trailing = encoded;
        trailing.push(0);
        assert!(LoyaltySkills::decode(&trailing).is_none());
        for percent in [1, 49, 51, 255] {
            let mut value = snapshot();
            value.bonus_percent = percent;
            assert!(value.encode().is_none());
        }
        for title in ["", "Longer than twenty bytes", "Scout\nof Tibia"] {
            let mut value = snapshot();
            value.title = Some(title.into());
            assert!(value.encode().is_none());
        }
        let mut value = snapshot();
        value.skills[0].with_loyalty = 99;
        assert!(value.encode().is_none());
        Ok(())
    }
}
