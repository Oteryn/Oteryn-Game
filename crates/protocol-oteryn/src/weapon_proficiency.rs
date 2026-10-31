//! P08: Weapon Proficiency perk selection and perk modification (PROFICIENCY-1B §11) by the
//! active generation's item definition reference.
use std::num::NonZeroU32;

pub const CAPABILITY_WEAPON_PROFICIENCY_V1: u32 = 195;
pub const COMMAND_TYPE_PROFICIENCY_SELECT_PERK: u32 = 378;
pub const COMMAND_TYPE_PROFICIENCY_MODIFY: u32 = 379;
pub const COMMAND_TYPE_PROFICIENCY_RANK_UP: u32 = 380;
pub const COMMAND_TYPE_PROFICIENCY_ORB_RANK: u32 = 392;
pub const COMMAND_TYPE_PROFICIENCY_RESHAPE_OFFER: u32 = 393;
pub const COMMAND_TYPE_PROFICIENCY_RESHAPE_CHOOSE: u32 = 394;
pub const COMMAND_TYPE_PROFICIENCY_CLEAR: u32 = 395;

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

/// The `RESHAPE_CHOOSE` choice byte that declines the pending offer.
pub const DECLINE_OFFER: u8 = 3;
/// The reply byte of an absent offer entry.
const NO_OFFER: u8 = u8::MAX;

/// The operation of one modification command; its command type selects it on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModificationOperation {
    /// Zero-based perk level whose selected perk is modified.
    Modify {
        level: u8,
    },
    RankUp,
    OrbRank,
    ReshapeOffer,
    /// `Some(0..=2)` chooses an offered entry; `None` declines the offer.
    ReshapeChoose {
        choice: Option<u8>,
    },
    Clear,
}

impl ModificationOperation {
    pub const fn command_type(self) -> u32 {
        match self {
            Self::Modify { .. } => COMMAND_TYPE_PROFICIENCY_MODIFY,
            Self::RankUp => COMMAND_TYPE_PROFICIENCY_RANK_UP,
            Self::OrbRank => COMMAND_TYPE_PROFICIENCY_ORB_RANK,
            Self::ReshapeOffer => COMMAND_TYPE_PROFICIENCY_RESHAPE_OFFER,
            Self::ReshapeChoose { .. } => COMMAND_TYPE_PROFICIENCY_RESHAPE_CHOOSE,
            Self::Clear => COMMAND_TYPE_PROFICIENCY_CLEAR,
        }
    }
}

/// One modification command (PROFICIENCY-1B §11.1): the weapon's nonzero definition reference
/// (u32, big endian), the slot (1 or 2), the level (MODIFY) or choice (RESHAPE_CHOOSE, 3
/// declines) byte, and the track's expected committed revision (nonzero u64, big endian).
/// 13 bytes, or 14 with the extra byte; other lengths and values are invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModificationRequest {
    pub definition: NonZeroU32,
    pub slot: u8,
    pub operation: ModificationOperation,
    pub expected_revision: u64,
}

impl ModificationRequest {
    pub fn encode(self) -> Vec<u8> {
        let mut bytes = self.definition.get().to_be_bytes().to_vec();
        bytes.push(self.slot);
        match self.operation {
            ModificationOperation::Modify { level } => bytes.push(level),
            ModificationOperation::ReshapeChoose { choice } => {
                bytes.push(choice.unwrap_or(DECLINE_OFFER));
            }
            _ => {}
        }
        bytes.extend_from_slice(&self.expected_revision.to_be_bytes());
        bytes
    }

    pub fn decode(command_type: u32, bytes: &[u8]) -> Option<Self> {
        let (head, revision) = bytes.split_last_chunk::<8>()?;
        let [a, b, c, d, slot, extra @ ..] = head else {
            return None;
        };
        let operation = match (command_type, extra) {
            (COMMAND_TYPE_PROFICIENCY_MODIFY, [level]) if *level < MAX_LEVELS as u8 => {
                ModificationOperation::Modify { level: *level }
            }
            (COMMAND_TYPE_PROFICIENCY_RESHAPE_CHOOSE, [choice]) if *choice <= DECLINE_OFFER => {
                ModificationOperation::ReshapeChoose {
                    choice: (*choice != DECLINE_OFFER).then_some(*choice),
                }
            }
            (COMMAND_TYPE_PROFICIENCY_RANK_UP, []) => ModificationOperation::RankUp,
            (COMMAND_TYPE_PROFICIENCY_ORB_RANK, []) => ModificationOperation::OrbRank,
            (COMMAND_TYPE_PROFICIENCY_RESHAPE_OFFER, []) => ModificationOperation::ReshapeOffer,
            (COMMAND_TYPE_PROFICIENCY_CLEAR, []) => ModificationOperation::Clear,
            _ => return None,
        };
        let expected_revision = u64::from_be_bytes(*revision);
        ((1..=2).contains(slot) && expected_revision != 0).then_some(Self {
            definition: NonZeroU32::new(u32::from_be_bytes([*a, *b, *c, *d]))?,
            slot: *slot,
            operation,
            expected_revision,
        })
    }
}

/// The closed result shared by the six modification commands (PROFICIENCY-1B §11.2); command
/// results, not protocol errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ModificationResult {
    Accepted = 0,
    /// The operation, the weapon's definition or its shaping is not admitted.
    NotAdmitted = 1,
    NotUnlocked = 2,
    NotInProtectionZone = 3,
    UnknownTrack = 4,
    /// The expected revision is not the track's committed revision; nothing was written.
    StaleRevision = 5,
    /// The bound revision set moved; the command id is spent.
    RevisionChanged = 6,
    RateLimited = 7,
    /// The command id was already used for another command.
    Conflict = 8,
    NoSelection = 9,
    SlotOccupied = 10,
    LevelTaken = 11,
    NotModified = 12,
    RankMax = 13,
    OfferPending = 14,
    NoOffer = 15,
    PoolTooSmall = 16,
    ShapingOutdated = 17,
    InsufficientDust = 18,
    NoOrb = 19,
}

impl ModificationResult {
    fn from_byte(byte: u8) -> Option<Self> {
        Some(match byte {
            0 => Self::Accepted,
            1 => Self::NotAdmitted,
            2 => Self::NotUnlocked,
            3 => Self::NotInProtectionZone,
            4 => Self::UnknownTrack,
            5 => Self::StaleRevision,
            6 => Self::RevisionChanged,
            7 => Self::RateLimited,
            8 => Self::Conflict,
            9 => Self::NoSelection,
            10 => Self::SlotOccupied,
            11 => Self::LevelTaken,
            12 => Self::NotModified,
            13 => Self::RankMax,
            14 => Self::OfferPending,
            15 => Self::NoOffer,
            16 => Self::PoolTooSmall,
            17 => Self::ShapingOutdated,
            18 => Self::InsufficientDust,
            19 => Self::NoOrb,
            _ => return None,
        })
    }
}

/// A modified slot: the level, the shaping pool entry index, its rank and the pending offer's
/// three entry indexes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModifiedSlot {
    pub level: u8,
    pub entry_index: u8,
    pub rank: u8,
    pub pending_offer: Option<[u8; 3]>,
}

/// The result, then the commanded slot after an accepted command: nothing when it is cleared,
/// else six bytes (level, entry index, rank, offer or three `0xFF`). At most 7 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModificationReply {
    pub result: ModificationResult,
    pub slot: Option<ModifiedSlot>,
}

impl ModificationReply {
    pub fn encode(self) -> Vec<u8> {
        let mut bytes = vec![self.result as u8];
        if let Some(slot) = self.slot {
            bytes.extend([slot.level, slot.entry_index, slot.rank]);
            bytes.extend(slot.pending_offer.unwrap_or([NO_OFFER; 3]));
        }
        bytes
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let (result, slot) = bytes.split_first()?;
        let slot = match slot {
            [] => None,
            [level, entry_index, rank, offer @ ..] => {
                let offer: [u8; 3] = offer.try_into().ok()?;
                Some(ModifiedSlot {
                    level: *level,
                    entry_index: *entry_index,
                    rank: *rank,
                    pending_offer: match offer {
                        [NO_OFFER, NO_OFFER, NO_OFFER] => None,
                        offer if offer.contains(&NO_OFFER) => return None,
                        offer => Some(offer),
                    },
                })
            }
            _ => return None,
        };
        Some(Self {
            result: ModificationResult::from_byte(*result)?,
            slot,
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

    #[test]
    fn modification_requests_carry_their_operation_in_the_command_type() -> Result<(), &'static str>
    {
        let definition = NonZeroU32::new(258).ok_or("zero")?;
        let operations = [
            ModificationOperation::Modify { level: 6 },
            ModificationOperation::RankUp,
            ModificationOperation::OrbRank,
            ModificationOperation::ReshapeOffer,
            ModificationOperation::ReshapeChoose { choice: Some(2) },
            ModificationOperation::ReshapeChoose { choice: None },
            ModificationOperation::Clear,
        ];
        let mut types = Vec::new();
        for operation in operations {
            let request = ModificationRequest {
                definition,
                slot: 2,
                operation,
                expected_revision: u64::MAX,
            };
            let bytes = request.encode();
            // PROF1B-RL-01: each payload is at most 24 bytes.
            assert!(bytes.len() <= 24);
            assert_eq!(&bytes[..5], &[0, 0, 1, 2, 2]);
            assert_eq!(
                ModificationRequest::decode(operation.command_type(), &bytes),
                Some(request)
            );
            types.push(operation.command_type());
        }
        types.dedup();
        assert_eq!(types, [379, 380, 392, 393, 394, 395]);
        let decline = ModificationRequest {
            definition,
            slot: 1,
            operation: ModificationOperation::ReshapeChoose { choice: None },
            expected_revision: 7,
        };
        assert_eq!(decline.encode()[5], DECLINE_OFFER);
        Ok(())
    }

    #[test]
    fn modification_requests_reject_malformed_payloads() {
        let rank_up = [0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 7];
        assert!(ModificationRequest::decode(380, &rank_up).is_some());
        // Wrong length for the type, slot 0 or 3, revision 0, definition 0, unknown type.
        assert_eq!(ModificationRequest::decode(379, &rank_up), None);
        assert_eq!(ModificationRequest::decode(378, &rank_up), None);
        let mut bad = rank_up;
        bad[4] = 0;
        assert_eq!(ModificationRequest::decode(380, &bad), None);
        bad[4] = 3;
        assert_eq!(ModificationRequest::decode(380, &bad), None);
        let mut bad = rank_up;
        bad[12] = 0;
        assert_eq!(ModificationRequest::decode(380, &bad), None);
        let mut bad = rank_up;
        bad[3] = 0;
        assert_eq!(ModificationRequest::decode(380, &bad), None);
        let modify = |extra| [0, 0, 0, 1, 1, extra, 0, 0, 0, 0, 0, 0, 0, 7];
        assert!(ModificationRequest::decode(379, &modify(6)).is_some());
        assert_eq!(ModificationRequest::decode(379, &modify(7)), None);
        assert!(ModificationRequest::decode(394, &modify(3)).is_some());
        assert_eq!(ModificationRequest::decode(394, &modify(4)), None);
        assert_eq!(ModificationRequest::decode(380, &[]), None);
    }

    #[test]
    fn modification_replies_round_trip_within_sixteen_bytes() {
        let slot = ModifiedSlot {
            level: 2,
            entry_index: 63,
            rank: 10,
            pending_offer: Some([1, 2, 3]),
        };
        for reply in [
            ModificationReply {
                result: ModificationResult::Accepted,
                slot: Some(slot),
            },
            ModificationReply {
                result: ModificationResult::Accepted,
                slot: Some(ModifiedSlot {
                    pending_offer: None,
                    ..slot
                }),
            },
            ModificationReply {
                result: ModificationResult::NoOrb,
                slot: None,
            },
        ] {
            let bytes = reply.encode();
            assert!(bytes.len() <= 16);
            assert_eq!(ModificationReply::decode(&bytes), Some(reply));
        }
        for result in 0..=19 {
            assert!(ModificationReply::decode(&[result]).is_some());
        }
        assert_eq!(ModificationReply::decode(&[20]), None);
        assert_eq!(ModificationReply::decode(&[0, 1, 2]), None);
        assert_eq!(ModificationReply::decode(&[0, 1, 2, 3, 4, 255, 255]), None);
    }
}
