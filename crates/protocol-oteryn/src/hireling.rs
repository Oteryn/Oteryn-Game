//! HIRELING-1 bounded summon/query command.
#[path = "hireling/management.rs"]
pub mod management;

pub const CAPABILITY_HIRELING_V1: u32 = 165;
pub const COMMAND_TYPE_HIRELING: u32 = 220;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    Query,
    Summon { x: u16, y: u16, floor: i8 },
    Dismiss,
}

impl Request {
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        match bytes {
            [0] => Some(Self::Query),
            [1, x0, x1, y0, y1, floor] => Some(Self::Summon {
                x: u16::from_be_bytes([*x0, *x1]),
                y: u16::from_be_bytes([*y0, *y1]),
                floor: i8::from_be_bytes([*floor]),
            }),
            [2] => Some(Self::Dismiss),
            _ => None,
        }
    }
    pub fn encode(self) -> Vec<u8> {
        match self {
            Self::Query => vec![0],
            Self::Summon { x, y, floor } => {
                let [x0, x1] = x.to_be_bytes();
                let [y0, y1] = y.to_be_bytes();
                vec![1, x0, x1, y0, y1, floor.to_be_bytes()[0]]
            }
            Self::Dismiss => vec![2],
        }
    }
}

/// Decodes the server's canonical one-byte active-state reply.
pub fn decode_reply(bytes: &[u8]) -> Option<bool> {
    match bytes {
        [0] => Some(false),
        [1] => Some(true),
        _ => None,
    }
}

/// Encodes the canonical reply used by the server route.
pub fn encode_reply(active: bool) -> Vec<u8> {
    vec![u8::from(active)]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_requests_round_trip() {
        for request in [
            Request::Query,
            Request::Summon {
                x: 7,
                y: 9,
                floor: -1,
            },
            Request::Dismiss,
        ] {
            assert_eq!(Request::decode(&request.encode()), Some(request));
        }
        assert_eq!(Request::decode(&[1, 0]), None);
        assert_eq!(Request::decode(&[3]), None);
    }

    #[test]
    fn canonical_replies_round_trip() {
        for active in [false, true] {
            assert_eq!(decode_reply(&encode_reply(active)), Some(active));
        }
        assert_eq!(decode_reply(&[2]), None);
    }
}
