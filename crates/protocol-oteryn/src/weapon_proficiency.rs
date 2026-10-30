//! P08: Weapon Proficiency perk selection by the active generation's item definition reference.
use std::num::NonZeroU32;

pub const CAPABILITY_WEAPON_PROFICIENCY_V1: u32 = 195;
pub const COMMAND_TYPE_PROFICIENCY_SELECT_PERK: u32 = 378;

/// The perk byte that clears a level's selection.
pub const CLEAR_PERK: u8 = u8::MAX;
/// At most seven perk levels per definition (PROFICIENCY-0), so a reply is at most 9 bytes.
const MAX_LEVELS: usize = 7;

/// Six bytes: the weapon's nonzero definition reference (u32, big endian), the zero-based
/// level and the zero-based perk index, or [`CLEAR_PERK`]. Other lengths are invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectPerkRequest {
    pub definition: NonZeroU32,
    pub level: u8,
    pub perk: Option<u8>,
}

impl SelectPerkRequest {
    pub fn encode(self) -> [u8; 6] {
        let [a, b, c, d] = self.definition.get().to_be_bytes();
        [a, b, c, d, self.level, self.perk.unwrap_or(CLEAR_PERK)]
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let bytes: [u8; 6] = bytes.try_into().ok()?;
        Some(Self {
            definition: NonZeroU32::new(u32::from_be_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3],
            ]))?,
            level: bytes[4],
            perk: (bytes[5] != CLEAR_PERK).then_some(bytes[5]),
        })
    }
}

/// The closed result of command 378; command results, not protocol errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SelectPerkResult {
    Accepted = 0,
    /// Malformed payload, unknown definition or an item without a proficiency definition.
    Rejected = 1,
    /// The Character has no proficiency progress on this weapon.
    UnknownTrack = 2,
    NotUnlocked = 3,
    InvalidPerk = 4,
    /// Changing or clearing a selected perk needs a protection zone.
    NotInProtectionZone = 5,
    /// The level's perk is replaced by an active modification (PROFICIENCY-1B §10).
    ModifiedLevel = 6,
    /// The track moved or the writer was unavailable; nothing was written. Retry.
    Busy = 7,
    /// The command id was already used for another selection.
    Conflict = 8,
}

impl SelectPerkResult {
    fn from_byte(byte: u8) -> Option<Self> {
        Some(match byte {
            0 => Self::Accepted,
            1 => Self::Rejected,
            2 => Self::UnknownTrack,
            3 => Self::NotUnlocked,
            4 => Self::InvalidPerk,
            5 => Self::NotInProtectionZone,
            6 => Self::ModifiedLevel,
            7 => Self::Busy,
            8 => Self::Conflict,
            _ => return None,
        })
    }
}

/// The result, then the track's committed selections (one byte per level, [`CLEAR_PERK`] for
/// none). Refusals before the track is read carry no selections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectPerkReply {
    pub result: SelectPerkResult,
    pub selections: Vec<Option<u8>>,
}

impl SelectPerkReply {
    pub fn encode(&self) -> Option<Vec<u8>> {
        if self.selections.len() > MAX_LEVELS {
            return None;
        }
        let mut bytes = vec![self.result as u8, u8::try_from(self.selections.len()).ok()?];
        bytes.extend(
            self.selections
                .iter()
                .map(|selection| selection.unwrap_or(CLEAR_PERK)),
        );
        Some(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let [result, count, selections @ ..] = bytes else {
            return None;
        };
        if usize::from(*count) != selections.len() || selections.len() > MAX_LEVELS {
            return None;
        }
        Some(Self {
            result: SelectPerkResult::from_byte(*result)?,
            selections: selections
                .iter()
                .map(|perk| (*perk != CLEAR_PERK).then_some(*perk))
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_perk_request_is_six_bytes_with_a_clear_marker() -> Result<(), &'static str> {
        let request = SelectPerkRequest {
            definition: NonZeroU32::new(258).ok_or("zero")?,
            level: 2,
            perk: Some(1),
        };
        assert_eq!(request.encode(), [0, 0, 1, 2, 2, 1]);
        assert_eq!(SelectPerkRequest::decode(&request.encode()), Some(request));
        let clear = SelectPerkRequest {
            perk: None,
            ..request
        };
        assert_eq!(clear.encode()[5], CLEAR_PERK);
        assert_eq!(SelectPerkRequest::decode(&clear.encode()), Some(clear));
        for bytes in [
            &[][..],
            &[0, 0, 0, 1, 0][..],
            &[0, 0, 0, 0, 0, 0][..],
            &[0, 0, 0, 1, 0, 0, 0][..],
        ] {
            assert_eq!(SelectPerkRequest::decode(bytes), None);
        }
        Ok(())
    }

    #[test]
    fn select_perk_reply_round_trips_and_bounds_its_levels() {
        let reply = SelectPerkReply {
            result: SelectPerkResult::Accepted,
            selections: vec![Some(0), None, Some(2)],
        };
        let bytes = reply.encode();
        assert_eq!(bytes.as_deref(), Some(&[0, 3, 0, 255, 2][..]));
        assert_eq!(bytes.and_then(|b| SelectPerkReply::decode(&b)), Some(reply));
        for result in 0..=8 {
            assert!(SelectPerkReply::decode(&[result, 0]).is_some());
        }
        assert_eq!(SelectPerkReply::decode(&[9, 0]), None);
        assert_eq!(SelectPerkReply::decode(&[0, 2, 0]), None);
        assert_eq!(
            SelectPerkReply {
                result: SelectPerkResult::Accepted,
                selections: vec![None; 8],
            }
            .encode(),
            None
        );
    }
}
