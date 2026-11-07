//! HOUSE-ITEM-WIRE-1: explicit furniture wrapping, separate from ordinary item use.
pub const CAPABILITY_HOUSE_FURNITURE_WRAP_V1: u32 = 207;
pub const COMMAND_WRAP: u32 = 402;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Action {
    Wrap = 1,
    Unwrap = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    pub action: Action,
    pub item: [u8; 16],
    pub expected_revision: u64,
    pub acl_revision: u64,
    /// Absolute bundle cell; used only for unwrap.
    pub position: [i32; 3],
}
impl Request {
    pub fn encode(self) -> Vec<u8> {
        let mut bytes = vec![self.action as u8];
        bytes.extend(self.item);
        bytes.extend(self.expected_revision.to_be_bytes());
        bytes.extend(self.acl_revision.to_be_bytes());
        for coordinate in self.position {
            bytes.extend(coordinate.to_be_bytes());
        }
        bytes
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 45 {
            return None;
        }
        Some(Self {
            action: match bytes[0] {
                1 => Action::Wrap,
                2 => Action::Unwrap,
                _ => return None,
            },
            item: bytes[1..17].try_into().ok()?,
            expected_revision: u64::from_be_bytes(bytes[17..25].try_into().ok()?),
            acl_revision: u64::from_be_bytes(bytes[25..33].try_into().ok()?),
            position: [
                i32::from_be_bytes(bytes[33..37].try_into().ok()?),
                i32::from_be_bytes(bytes[37..41].try_into().ok()?),
                i32::from_be_bytes(bytes[41..45].try_into().ok()?),
            ],
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Refusal {
    NotInHouse = 1,
    NoAccess = 2,
    NotWrapable = 3,
    Stale = 4,
    Blocked = 5,
    InventoryFull = 6,
}

/// Successful replies contain the committed Item revision; refusals contain one byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reply {
    Committed(u64),
    Refused(Refusal),
}
impl Reply {
    pub fn encode(self) -> Vec<u8> {
        match self {
            Self::Committed(revision) => {
                let mut bytes = vec![0];
                bytes.extend(revision.to_be_bytes());
                bytes
            }
            Self::Refused(refusal) => vec![refusal as u8],
        }
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        Some(match bytes {
            [0, revision @ ..] => Self::Committed(u64::from_be_bytes(revision.try_into().ok()?)),
            [n] => Self::Refused(match n {
                1 => Refusal::NotInHouse,
                2 => Refusal::NoAccess,
                3 => Refusal::NotWrapable,
                4 => Refusal::Stale,
                5 => Refusal::Blocked,
                6 => Refusal::InventoryFull,
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
    fn house_wrap_wire_is_exact_and_bounded() {
        for action in [Action::Wrap, Action::Unwrap] {
            let request = Request {
                action,
                item: [7; 16],
                expected_revision: u64::MAX,
                acl_revision: 1,
                position: [100, 200, -7],
            };
            let bytes = request.encode();
            assert_eq!(Request::decode(&bytes), Some(request));
            assert!(Request::decode(&bytes[..44]).is_none());
            let mut long = bytes;
            long.push(0);
            assert!(Request::decode(&long).is_none());
        }
        for reply in [Reply::Committed(2), Reply::Refused(Refusal::Stale)] {
            assert_eq!(Reply::decode(&reply.encode()), Some(reply));
        }
        assert!(Reply::decode(&[7]).is_none());
    }
}
