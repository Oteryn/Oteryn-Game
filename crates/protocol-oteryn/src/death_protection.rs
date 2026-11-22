//! C20c: immutable protection consumed by one committed death, retained across login.
pub const CAPABILITY_DEATH_PROTECTION_V1: u32 = 382;
pub const COMMAND_TYPE_DEATH_PROTECTION_QUERY: u32 = 664;
pub const MAX_DEATH_PROTECTION_REPLY_BYTES: usize = 4146;
const MAX_BLESSINGS: usize = 32;
const MAX_KEY_BYTES: usize = 128;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeathProtection {
    pub occurrence: [u8; 16],
    /// Exact immutable receipt difference; an empty list means this death consumed none.
    pub consumed: Vec<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeathProtectionReply(pub Option<DeathProtection>);
impl DeathProtectionReply {
    pub fn encode(&self) -> Option<Vec<u8>> {
        let Some(value) = &self.0 else {
            return Some(vec![0]);
        };
        if value.occurrence == [0; 16] || value.consumed.len() > MAX_BLESSINGS {
            return None;
        }
        let mut bytes = vec![1];
        bytes.extend_from_slice(&value.occurrence);
        bytes.push(u8::try_from(value.consumed.len()).ok()?);
        let mut previous = "";
        for key in &value.consumed {
            if key.is_empty()
                || key.len() > MAX_KEY_BYTES
                || key.as_str() <= previous
                || !key.starts_with("oteryn:blessing.")
                || !key
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || matches!(byte, b':' | b'.' | b'_'))
            {
                return None;
            }
            previous = key;
            bytes.push(u8::try_from(key.len()).ok()?);
            bytes.extend_from_slice(key.as_bytes());
        }
        Some(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes == [0] {
            return Some(Self(None));
        }
        if bytes.len() < 18 || bytes.len() > MAX_DEATH_PROTECTION_REPLY_BYTES || bytes[0] != 1 {
            return None;
        }
        let occurrence = bytes[1..17].try_into().ok()?;
        let mut consumed = Vec::with_capacity(usize::from(bytes[17]));
        let mut remaining = &bytes[18..];
        for _ in 0..bytes[17] {
            let (&len, tail) = remaining.split_first()?;
            let (key, tail) = tail.split_at_checked(usize::from(len))?;
            consumed.push(std::str::from_utf8(key).ok()?.to_owned());
            remaining = tail;
        }
        if !remaining.is_empty() {
            return None;
        }
        let value = Self(Some(DeathProtection {
            occurrence,
            consumed,
        }));
        value.encode().filter(|canonical| canonical == bytes)?;
        Some(value)
    }
}
pub const CAPABILITY_DEATH_AND_BLESSINGS_V1: u32 = 584;
pub const COMMAND_TYPE_DEATH_REPORT_QUERY: u32 = 6331;
const PENALTY_BYTES: usize = 9;
pub const MAX_DEATH_REPORT_REPLY_BYTES: usize = MAX_DEATH_PROTECTION_REPLY_BYTES + PENALTY_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DeathType {
    Unknown = 0,
    Pve = 1,
    Pvp = 2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeathReport {
    pub protection: DeathProtection,
    pub death_type: DeathType,
    pub experience_lost: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeathReportReply(pub Option<DeathReport>);

impl DeathReportReply {
    pub fn encode(self) -> Option<Vec<u8>> {
        let Some(report) = self.0 else {
            return Some(vec![0]);
        };
        if report.experience_lost > i64::MAX as u64 {
            return None;
        }
        let mut bytes = DeathProtectionReply(Some(report.protection)).encode()?;
        bytes.push(report.death_type as u8);
        bytes.extend_from_slice(&report.experience_lost.to_le_bytes());
        Some(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes == [0] {
            return Some(Self(None));
        }
        let (protection, penalty) =
            bytes.split_at_checked(bytes.len().checked_sub(PENALTY_BYTES)?)?;
        let protection = DeathProtectionReply::decode(protection)?.0?;
        let death_type = match penalty[0] {
            0 => DeathType::Unknown,
            1 => DeathType::Pve,
            2 => DeathType::Pvp,
            _ => return None,
        };
        let experience_lost = u64::from_le_bytes(penalty[1..].try_into().ok()?);
        (experience_lost <= i64::MAX as u64).then_some(Self(Some(DeathReport {
            protection,
            death_type,
            experience_lost,
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn death_report_preserves_committed_penalty_and_rejects_invalid_type()
    -> Result<(), &'static str> {
        let report = DeathReportReply(Some(DeathReport {
            protection: DeathProtection {
                occurrence: [7; 16],
                consumed: vec![],
            },
            death_type: DeathType::Pvp,
            experience_lost: 1234,
        }));
        let mut bytes = report.clone().encode().ok_or("report")?;
        assert_eq!(DeathReportReply::decode(&bytes), Some(report));
        assert_eq!(DeathReportReply::decode(&[0]), Some(DeathReportReply(None)));
        for end in 0..bytes.len() {
            assert!(DeathReportReply::decode(&bytes[..end]).is_none());
        }
        bytes[18] = 3;
        assert!(DeathReportReply::decode(&bytes).is_none());
        Ok(())
    }

    #[test]
    fn death_protection_codec_retains_empty_consumption_and_rejects_malformed()
    -> Result<(), &'static str> {
        for consumed in [vec![], vec!["oteryn:blessing.twist_of_fate".into()]] {
            let value = DeathProtectionReply(Some(DeathProtection {
                occurrence: [7; 16],
                consumed,
            }));
            let bytes = value.encode().ok_or("encode")?;
            assert_eq!(DeathProtectionReply::decode(&bytes), Some(value));
            for end in 0..bytes.len() {
                assert!(DeathProtectionReply::decode(&bytes[..end]).is_none());
            }
            let mut extra = bytes;
            extra.push(0);
            assert!(DeathProtectionReply::decode(&extra).is_none());
        }
        assert_eq!(
            DeathProtectionReply::decode(&[0]),
            Some(DeathProtectionReply(None))
        );
        assert!(DeathProtectionReply::decode(&[0, 1]).is_none());
        Ok(())
    }
}

pub const CAPABILITY_COMBAT_OCCURRENCES_V1: u32 = 595;
pub const COMMAND_TYPE_DEATH_OCCURRENCE: u32 = 6352;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FinalBlowKind {
    Unattributed = 0,
    Environment = 1,
    Creature = 2,
    Player = 3,
}

impl FinalBlowKind {
    pub fn decode(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Unattributed,
            1 => Self::Environment,
            2 => Self::Creature,
            3 => Self::Player,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeathOccurrence {
    pub occurrence: [u8; 16],
    pub final_blow: FinalBlowKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeathOccurrenceReply(pub Option<DeathOccurrence>);

impl DeathOccurrenceReply {
    pub fn encode(self) -> Option<Vec<u8>> {
        let Some(death) = self.0 else {
            return Some(vec![0]);
        };
        if death.occurrence == [0; 16] {
            return None;
        }
        let mut bytes = Vec::with_capacity(18);
        bytes.push(2);
        bytes.extend_from_slice(&death.occurrence);
        bytes.push(death.final_blow as u8);
        Some(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        match bytes {
            [0] => Some(Self(None)),
            [2, occurrence @ .., kind] => {
                let occurrence = <[u8; 16]>::try_from(occurrence).ok()?;
                (occurrence != [0; 16]).then_some(Self(Some(DeathOccurrence {
                    occurrence,
                    final_blow: FinalBlowKind::decode(*kind)?,
                })))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
#[test]
fn occurrence_death_kind_is_bound_to_one_occurrence() -> Result<(), &'static str> {
    for final_blow in [
        FinalBlowKind::Unattributed,
        FinalBlowKind::Environment,
        FinalBlowKind::Creature,
        FinalBlowKind::Player,
    ] {
        let reply = DeathOccurrenceReply(Some(DeathOccurrence {
            occurrence: [7; 16],
            final_blow,
        }));
        let mut wire = reply.encode().ok_or("encode")?;
        assert_eq!(DeathOccurrenceReply::decode(&wire), Some(reply));
        assert!(DeathProtectionReply::decode(&wire).is_none());
        wire[17] = 4;
        assert!(DeathOccurrenceReply::decode(&wire).is_none());
    }
    Ok(())
}
