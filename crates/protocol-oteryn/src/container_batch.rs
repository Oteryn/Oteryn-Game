//! Negotiated preview and confirmation of a bounded carried-container operation.
use crate::item_view::ItemHandle;
use std::num::NonZeroU64;

pub const CAPABILITY_STASH_CONTAINER_V1: u32 = 601;
pub const COMMAND_TYPE_STASH_CONTAINER: u32 = 6364;
pub const CAPABILITY_MANAGED_CONTAINER_MOVE_V1: u32 = 602;
pub const COMMAND_TYPE_MANAGED_CONTAINER_MOVE: u32 = 6365;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    Preview { source: ItemHandle, recursive: bool },
    Confirm { preview: NonZeroU64 },
}

impl Request {
    pub fn encode(self) -> Vec<u8> {
        match self {
            Self::Preview { source, recursive } => {
                let mut bytes = vec![0, u8::from(recursive)];
                bytes.extend_from_slice(&source.get().to_be_bytes());
                bytes
            }
            Self::Confirm { preview } => {
                let mut bytes = vec![1];
                bytes.extend_from_slice(&preview.get().to_be_bytes());
                bytes
            }
        }
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let id = |bytes: &[u8]| NonZeroU64::new(u64::from_be_bytes(bytes.try_into().ok()?));
        match bytes {
            [0, recursive @ 0..=1, rest @ ..] => Some(Self::Preview {
                source: id(rest)?,
                recursive: *recursive == 1,
            }),
            [1, rest @ ..] => Some(Self::Confirm { preview: id(rest)? }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reply {
    Preview(u32),
    Committed(u32),
}

impl Reply {
    pub fn encode(self) -> [u8; 5] {
        let (tag, quantity) = match self {
            Self::Preview(quantity) => (0, quantity),
            Self::Committed(quantity) => (1, quantity),
        };
        let [a, b, c, d] = quantity.to_be_bytes();
        [tag, a, b, c, d]
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let (&tag, quantity) = bytes.split_first()?;
        let quantity = u32::from_be_bytes(quantity.try_into().ok()?);
        match tag {
            0 if quantity > 0 => Some(Self::Preview(quantity)),
            1 if quantity > 0 => Some(Self::Committed(quantity)),
            _ => None,
        }
    }
}
