//! Capability 150, commands 174..=176: bounded house-podium mutation/query and
//! viewer projection. Strings are UTF-8 and length-prefixed with one byte.
pub const CAPABILITY_HOUSE_PODIUM_V1: u32 = 150;
pub const COMMAND_QUERY: u32 = 174;
pub const COMMAND_SET: u32 = 175;
pub const COMMAND_PROJECTION: u32 = 176;
pub const MAX_KEY_BYTES: usize = 160;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetRequest {
    pub item: [u8; 16],
    pub expected_revision: u64,
    pub display_kind: u8,
    pub direction: u8,
    pub key: String,
}
impl SetRequest {
    pub fn encode(&self) -> Option<Vec<u8>> {
        let key = self.key.as_bytes();
        if key.is_empty() || key.len() > MAX_KEY_BYTES {
            return None;
        }
        let mut v = Vec::with_capacity(27 + key.len());
        v.extend(self.item);
        v.extend(self.expected_revision.to_be_bytes());
        v.extend([self.display_kind, self.direction, key.len() as u8]);
        v.extend(key);
        Some(v)
    }
    pub fn decode(v: &[u8]) -> Option<Self> {
        if v.len() < 27 {
            return None;
        }
        let n = v[26] as usize;
        if n == 0 || n > MAX_KEY_BYTES || v.len() != 27 + n {
            return None;
        }
        Some(Self {
            item: v[..16].try_into().ok()?,
            expected_revision: u64::from_be_bytes(v[16..24].try_into().ok()?),
            display_kind: match v[24] {
                1..=3 => v[24],
                _ => return None,
            },
            direction: match v[25] {
                0..=3 => v[25],
                _ => return None,
            },
            key: std::str::from_utf8(&v[27..]).ok()?.to_owned(),
        })
    }
}
/// Commands 174 and 176 carry only the podium item UUID.
pub fn decode_item(v: &[u8]) -> Option<[u8; 16]> {
    v.try_into().ok()
}

/// The committed display of one podium, or `None` when it shows nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projection {
    pub podium_kind: u8,
    pub display_kind: u8,
    pub direction: u8,
    pub revision: u64,
    pub key: String,
}
impl Projection {
    /// `0` for an empty podium, else `1`, podium kind, display kind, direction, revision
    /// (`u64` big endian), key length and key: at most 174 bytes.
    pub fn encode(projection: Option<&Self>) -> Option<Vec<u8>> {
        let Some(p) = projection else {
            return Some(vec![0]);
        };
        let key = p.key.as_bytes();
        if key.is_empty() || key.len() > MAX_KEY_BYTES {
            return None;
        }
        let mut v = Vec::with_capacity(13 + key.len());
        v.extend([1, p.podium_kind, p.display_kind, p.direction]);
        v.extend(p.revision.to_be_bytes());
        v.push(key.len() as u8);
        v.extend(key);
        Some(v)
    }
    pub fn decode(v: &[u8]) -> Option<Option<Self>> {
        match v {
            [0] => Some(None),
            [1, podium_kind, display_kind, direction, rest @ ..] if rest.len() > 9 => {
                let n = rest[8] as usize;
                if n == 0 || n > MAX_KEY_BYTES || rest.len() != 9 + n {
                    return None;
                }
                Some(Some(Self {
                    podium_kind: *podium_kind,
                    display_kind: *display_kind,
                    direction: *direction,
                    revision: u64::from_be_bytes(rest[..8].try_into().ok()?),
                    key: std::str::from_utf8(&rest[9..]).ok()?.to_owned(),
                }))
            }
            _ => None,
        }
    }
}

/// Reply to 175: a status byte and, when accepted, the committed revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetResult {
    Accepted { revision: u64 },
    Refused(SetRefusal),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SetRefusal {
    StaleSession = 1,
    NoAccess = 2,
    NotHouseItem = 3,
    WrongItem = 4,
    WrongDisplay = 5,
    UnknownContent = 6,
    NotOwned = 7,
    Changed = 8,
    InvalidInput = 9,
}
impl SetResult {
    pub fn encode(self) -> Vec<u8> {
        match self {
            Self::Accepted { revision } => {
                let mut v = vec![0];
                v.extend(revision.to_be_bytes());
                v
            }
            Self::Refused(refusal) => vec![refusal as u8],
        }
    }
    pub fn decode(v: &[u8]) -> Option<Self> {
        use SetRefusal::*;
        Some(match v {
            [0, revision @ ..] => Self::Accepted {
                revision: u64::from_be_bytes(revision.try_into().ok()?),
            },
            [code] => Self::Refused(match code {
                1 => StaleSession,
                2 => NoAccess,
                3 => NotHouseItem,
                4 => WrongItem,
                5 => WrongDisplay,
                6 => UnknownContent,
                7 => NotOwned,
                8 => Changed,
                9 => InvalidInput,
                _ => return None,
            }),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn set_wire_is_canonical() {
        let r = SetRequest {
            item: [7; 16],
            expected_revision: 9,
            display_kind: 1,
            direction: 3,
            key: "oteryn:content.outfit.citizen".into(),
        };
        let encoded = r.encode().unwrap_or_default();
        assert!(!encoded.is_empty());
        assert_eq!(SetRequest::decode(&encoded), Some(r));
        assert!(SetRequest::decode(&[0; 26]).is_none());
    }
    #[test]
    fn projection_and_result_wire_are_canonical_and_bounded() {
        let p = Projection {
            podium_kind: 3,
            display_kind: 3,
            direction: 2,
            revision: 5,
            key: "k".repeat(MAX_KEY_BYTES),
        };
        let encoded = Projection::encode(Some(&p)).unwrap_or_default();
        assert_eq!(encoded.len(), 13 + MAX_KEY_BYTES);
        assert!(encoded.len() <= 4096);
        assert_eq!(Projection::decode(&encoded), Some(Some(p.clone())));
        assert_eq!(Projection::encode(None), Some(vec![0]));
        assert_eq!(Projection::decode(&[0]), Some(None));
        assert!(Projection::decode(&encoded[..encoded.len() - 1]).is_none());
        let long = Projection {
            key: "k".repeat(MAX_KEY_BYTES + 1),
            ..p
        };
        assert!(Projection::encode(Some(&long)).is_none());
        for result in [
            SetResult::Accepted { revision: u64::MAX },
            SetResult::Refused(SetRefusal::NotOwned),
            SetResult::Refused(SetRefusal::InvalidInput),
        ] {
            let encoded = result.encode();
            assert!(encoded.len() <= 9);
            assert_eq!(SetResult::decode(&encoded), Some(result));
        }
        assert!(SetResult::decode(&[10]).is_none());
        assert!(SetResult::decode(&[0, 1]).is_none());
        assert_eq!(decode_item(&[4; 16]), Some([4; 16]));
        assert!(decode_item(&[4; 17]).is_none());
    }
}
