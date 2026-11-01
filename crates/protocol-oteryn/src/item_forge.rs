//! Exaltation Forge command and reply wire contract.
use std::num::NonZeroU64;

pub const CAPABILITY_ITEM_FORGE_V1: u32 = 152;
pub const COMMAND_TYPE_ITEM_FORGE: u32 = 181;
pub const MAX_HISTORY: u8 = 50;
pub const REQUEST_BYTES: usize = 51;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Operation {
    QueryHistory = 0,
    Fusion = 1,
    ConvergenceFusion = 2,
    Transfer = 3,
    ConvergenceTransfer = 4,
    DustToSlivers = 5,
    SliversToCores = 6,
    DustLimit = 7,
}

impl Operation {
    #[must_use]
    pub const fn changes_inventory(self) -> bool {
        matches!(
            self,
            Self::Fusion
                | Self::ConvergenceFusion
                | Self::Transfer
                | Self::ConvergenceTransfer
                | Self::DustToSlivers
                | Self::SliversToCores
        )
    }

    #[must_use]
    pub const fn requires_pair(self) -> bool {
        matches!(
            self,
            Self::Fusion | Self::ConvergenceFusion | Self::Transfer | Self::ConvergenceTransfer
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    pub operation: Operation,
    pub occurrence: [u8; 16],
    /// Session ItemHandle of the receiver. For transfer, `second_item` is the tier donor.
    pub first_item: Option<NonZeroU64>,
    pub second_item: Option<NonZeroU64>,
    /// Crystal protocolgame.cpp:7031-7034: optional cores apply only to normal fusion.
    pub boost: bool,
    pub protect: bool,
    pub history_limit: u8,
}

impl Request {
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != REQUEST_BYTES {
            return None;
        }
        let operation = match bytes[0] {
            0 => Operation::QueryHistory,
            1 => Operation::Fusion,
            2 => Operation::ConvergenceFusion,
            3 => Operation::Transfer,
            4 => Operation::ConvergenceTransfer,
            5 => Operation::DustToSlivers,
            6 => Operation::SliversToCores,
            7 => Operation::DustLimit,
            _ => return None,
        };
        let occurrence = bytes[1..17].try_into().ok()?;
        let flags = bytes[17];
        if flags & !15 != 0 {
            return None;
        }
        if bytes[26..34] != [0; 8] || bytes[42..50] != [0; 8] {
            return None;
        }
        let first = u64::from_le_bytes(bytes[18..26].try_into().ok()?);
        let second = u64::from_le_bytes(bytes[34..42].try_into().ok()?);
        if (flags & 1 != 0) != (first != 0) || (flags & 2 != 0) != (second != 0) {
            return None;
        }
        let request = Self {
            operation,
            occurrence,
            first_item: NonZeroU64::new(first),
            second_item: NonZeroU64::new(second),
            boost: flags & 4 != 0,
            protect: flags & 8 != 0,
            history_limit: bytes[50],
        };
        request.is_canonical().then_some(request)
    }

    /// Reject unused fields even for callers constructing a request without decoding it.
    #[must_use]
    pub fn is_canonical(self) -> bool {
        let pair = self.operation.requires_pair();
        self.first_item.is_some() == pair
            && self.second_item.is_some() == pair
            && (self.operation == Operation::Fusion || (!self.boost && !self.protect))
            && self.history_limit <= MAX_HISTORY
            && if self.operation == Operation::QueryHistory {
                self.occurrence == [0; 16]
            } else {
                self.history_limit == 0
            }
    }

    /// The payload size and legacy flag values are unchanged; bits 2/3 select normal-fusion cores.
    pub fn encode(self) -> Option<[u8; REQUEST_BYTES]> {
        if !self.is_canonical() {
            return None;
        }
        let mut bytes = [0; REQUEST_BYTES];
        bytes[0] = self.operation as u8;
        bytes[1..17].copy_from_slice(&self.occurrence);
        bytes[17] = u8::from(self.first_item.is_some())
            | (u8::from(self.second_item.is_some()) << 1)
            | (u8::from(self.boost) << 2)
            | (u8::from(self.protect) << 3);
        if let Some(item) = self.first_item {
            bytes[18..26].copy_from_slice(&item.get().to_le_bytes());
        }
        if let Some(item) = self.second_item {
            bytes[34..42].copy_from_slice(&item.get().to_le_bytes());
        }
        bytes[50] = self.history_limit;
        Some(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operations_require_canonical_items_and_history() {
        for operation in 0..=7 {
            let mut bytes = [0; REQUEST_BYTES];
            bytes[0] = operation;
            if (1..=4).contains(&operation) {
                bytes[17] = 3;
                bytes[18] = 1;
                bytes[34] = 2;
            }
            assert!(Request::decode(&bytes).is_some(), "operation {operation}");
            bytes[17] ^= 1;
            assert!(Request::decode(&bytes).is_none());
        }
        let mut bytes = [0; REQUEST_BYTES];
        bytes[50] = MAX_HISTORY;
        assert!(Request::decode(&bytes).is_some());
        bytes[50] += 1;
        assert!(Request::decode(&bytes).is_none());
        bytes[50] = 0;
        bytes[18] = 1;
        assert!(Request::decode(&bytes).is_none(), "unused item bytes");
        bytes[18] = 0;
        bytes[1] = 1;
        assert!(
            Request::decode(&bytes).is_none(),
            "history occurrence is unused"
        );
        bytes[0] = 7;
        bytes[50] = 1;
        assert!(
            Request::decode(&bytes).is_none(),
            "mutation history limit is unused"
        );
    }

    #[test]
    fn bounded_canonical_decode() {
        let mut bytes = [0; REQUEST_BYTES];
        bytes[0] = 7;
        assert!(Request::decode(&bytes).is_some());
        bytes[17] = 4;
        assert!(Request::decode(&bytes).is_none());
        assert!(Request::decode(&bytes[..50]).is_none());
        assert!(Request::decode(&[0; REQUEST_BYTES + 1]).is_none());
        bytes[0] = 8;
        bytes[17] = 0;
        assert!(Request::decode(&bytes).is_none());
    }

    #[test]
    fn fusion_core_flags_round_trip_without_changing_legacy_payload_size()
    -> Result<(), &'static str> {
        for boost in [false, true] {
            for protect in [false, true] {
                let request = Request {
                    operation: Operation::Fusion,
                    occurrence: [1; 16],
                    first_item: NonZeroU64::new(2),
                    second_item: NonZeroU64::new(3),
                    boost,
                    protect,
                    history_limit: 0,
                };
                let bytes = request.encode().ok_or("valid fusion request")?;
                assert_eq!(bytes.len(), 51);
                assert_eq!(
                    bytes[17],
                    3 | (u8::from(boost) << 2) | (u8::from(protect) << 3)
                );
                assert_eq!(Request::decode(&bytes), Some(request));
            }
        }
        Ok(())
    }

    #[test]
    fn cores_are_refused_on_convergence_transfer_conversion_and_history() {
        for operation in [0, 2, 3, 4, 5, 6, 7] {
            for option in [4, 8, 12] {
                let mut bytes = [0; REQUEST_BYTES];
                bytes[0] = operation;
                bytes[17] = option | if (1..=4).contains(&operation) { 3 } else { 0 };
                if (1..=4).contains(&operation) {
                    bytes[18] = 1;
                    bytes[34] = 2;
                }
                assert!(
                    Request::decode(&bytes).is_none(),
                    "operation={operation}, option={option}"
                );
            }
        }
        let mut bytes = [0; REQUEST_BYTES];
        bytes[0] = 1;
        bytes[17] = 19;
        assert!(Request::decode(&bytes).is_none(), "unassigned option bit");
    }

    #[test]
    fn direct_encoding_refuses_noncanonical_fields() {
        let mut request = Request {
            operation: Operation::DustLimit,
            occurrence: [1; 16],
            first_item: None,
            second_item: None,
            boost: true,
            protect: false,
            history_limit: 0,
        };
        assert!(request.encode().is_none());
        request.boost = false;
        request.first_item = NonZeroU64::new(2);
        assert!(request.encode().is_none());
        request.first_item = None;
        request.operation = Operation::QueryHistory;
        assert!(request.encode().is_none());
        request.occurrence = [0; 16];
        assert!(request.encode().is_some());
    }

    #[test]
    fn only_item_actions_request_inventory_refresh() {
        for operation in [
            Operation::Fusion,
            Operation::ConvergenceFusion,
            Operation::Transfer,
            Operation::ConvergenceTransfer,
            Operation::DustToSlivers,
            Operation::SliversToCores,
        ] {
            assert!(operation.changes_inventory());
        }
        assert!(!Operation::QueryHistory.changes_inventory());
        assert!(!Operation::DustLimit.changes_inventory());
    }

    #[test]
    fn pair_slots_are_nonzero_little_endian_handles_with_zero_padding() -> Result<(), &'static str>
    {
        let request = Request {
            operation: Operation::Transfer,
            occurrence: [1; 16],
            first_item: NonZeroU64::new(0x0102_0304_0506_0708),
            second_item: NonZeroU64::new(u64::MAX),
            boost: false,
            protect: false,
            history_limit: 0,
        };
        let bytes = request.encode().ok_or("canonical handle request")?;
        assert_eq!(&bytes[18..26], &[8, 7, 6, 5, 4, 3, 2, 1]);
        assert_eq!(&bytes[26..34], &[0; 8]);
        assert_eq!(&bytes[34..42], &[255; 8]);
        assert_eq!(&bytes[42..50], &[0; 8]);
        assert_eq!(Request::decode(&bytes), Some(request));
        for index in (26..34).chain(42..50) {
            let mut malformed = bytes;
            malformed[index] = 1;
            assert!(
                Request::decode(&malformed).is_none(),
                "nonzero padding at {index}"
            );
        }
        for range in [18..26, 34..42] {
            let mut malformed = bytes;
            malformed[range].fill(0);
            assert!(
                Request::decode(&malformed).is_none(),
                "zero handle with presence flag"
            );
        }
        let mut unused_presence = [0; REQUEST_BYTES];
        unused_presence[0] = Operation::DustLimit as u8;
        unused_presence[17] = 1;
        assert!(Request::decode(&unused_presence).is_none());
        Ok(())
    }
}
