//! NX-8 W09: current active potion durations, never inferred from historical receipts.
pub const CAPABILITY_POTION_DURATIONS_V1: u32 = 381;
pub const COMMAND_TYPE_POTION_DURATIONS_QUERY: u32 = 662;
pub const MAX_POTION_DURATIONS: usize = 32;
pub const MAX_POTION_DURATIONS_REPLY_BYTES: usize = 1 + MAX_POTION_DURATIONS * 15;

/// The source identifies the clock and effect; 1 = Drome online time, 2 = item condition,
/// 3 = store XP boost, 4 = daily reward XP boost. No Character identity is exposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PotionDuration {
    pub source: u8,
    pub item_id: u32,
    pub percent: u16,
    pub remaining_ms: u64,
}
impl PotionDuration {
    fn valid(self) -> bool {
        self.remaining_ms > 0
            && match self.source {
                1 => {
                    (36724..=36742).contains(&self.item_id)
                        && self.item_id != 36725
                        && self.percent == 0
                }
                2 => [7439, 7440, 7443, 49271, 35563].contains(&self.item_id) && self.percent == 0,
                3 => self.item_id == 0 && self.percent == 50,
                4 => self.item_id == 0 && [10, 20, 25, 50].contains(&self.percent),
                _ => false,
            }
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PotionDurations {
    pub active: Vec<PotionDuration>,
}
impl PotionDurations {
    pub fn encode(&self) -> Option<Vec<u8>> {
        if self.active.len() > MAX_POTION_DURATIONS
            || self.active.iter().any(|entry| !entry.valid())
        {
            return None;
        }
        let mut bytes = vec![u8::try_from(self.active.len()).ok()?];
        for entry in &self.active {
            bytes.push(entry.source);
            bytes.extend_from_slice(&entry.item_id.to_le_bytes());
            bytes.extend_from_slice(&entry.percent.to_le_bytes());
            bytes.extend_from_slice(&entry.remaining_ms.to_le_bytes());
        }
        Some(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let (&count, entries) = bytes.split_first()?;
        if usize::from(count) > MAX_POTION_DURATIONS || entries.len() != usize::from(count) * 15 {
            return None;
        }
        let mut active = Vec::with_capacity(usize::from(count));
        for entry in entries.as_chunks::<15>().0 {
            let value = PotionDuration {
                source: entry[0],
                item_id: u32::from_le_bytes(entry[1..5].try_into().ok()?),
                percent: u16::from_le_bytes(entry[5..7].try_into().ok()?),
                remaining_ms: u64::from_le_bytes(entry[7..15].try_into().ok()?),
            };
            if !value.valid() {
                return None;
            }
            active.push(value);
        }
        Some(Self { active })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nx8_potion_durations_codec_preserves_all_sources_and_rejects_invalid_frames()
    -> Result<(), &'static str> {
        let value = PotionDurations {
            active: vec![
                PotionDuration {
                    source: 1,
                    item_id: 36727,
                    percent: 0,
                    remaining_ms: 3_000_001,
                },
                PotionDuration {
                    source: 2,
                    item_id: 35563,
                    percent: 0,
                    remaining_ms: 60_000,
                },
                PotionDuration {
                    source: 3,
                    item_id: 0,
                    percent: 50,
                    remaining_ms: 123_456,
                },
                PotionDuration {
                    source: 4,
                    item_id: 0,
                    percent: 25,
                    remaining_ms: 66_000,
                },
            ],
        };
        // Every timed identity in the donor's Drome pool is representable; instant
        // callbacks cannot be confused with ongoing effects.
        for item_id in (36724..=36742).filter(|item| *item != 36725) {
            let all = PotionDurations {
                active: vec![PotionDuration {
                    source: 1,
                    item_id,
                    percent: 0,
                    remaining_ms: 3_600_000,
                }],
            };
            assert_eq!(
                PotionDurations::decode(&all.encode().ok_or("catalogue")?),
                Some(all)
            );
        }
        for item_id in [36723, 36725, 36743] {
            assert!(
                PotionDurations {
                    active: vec![PotionDuration {
                        source: 1,
                        item_id,
                        percent: 0,
                        remaining_ms: 3_600_000
                    }]
                }
                .encode()
                .is_none()
            );
        }
        let bytes = value.encode().ok_or("encode")?;
        assert_eq!(PotionDurations::decode(&bytes), Some(value));
        for end in 0..bytes.len() {
            assert!(PotionDurations::decode(&bytes[..end]).is_none());
        }
        let mut extra = bytes.clone();
        extra.push(0);
        assert!(PotionDurations::decode(&extra).is_none());
        let mut invalid = bytes;
        invalid[1] = 5;
        assert!(PotionDurations::decode(&invalid).is_none());
        Ok(())
    }
}
