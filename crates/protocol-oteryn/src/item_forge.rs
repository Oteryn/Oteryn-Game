//! Exaltation Forge command and reply wire contract.
pub const CAPABILITY_ITEM_FORGE_V1: u32 = 152;
pub const COMMAND_TYPE_ITEM_FORGE: u32 = 181;
pub const MAX_HISTORY: u8 = 50;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    pub operation: Operation,
    pub occurrence: [u8; 16],
    pub first_item: Option<[u8; 16]>,
    pub second_item: Option<[u8; 16]>,
    pub history_limit: u8,
}
impl Request {
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 51 {
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
        if flags & !3 != 0 {
            return None;
        }
        let first: [u8; 16] = bytes[18..34].try_into().ok()?;
        let second: [u8; 16] = bytes[34..50].try_into().ok()?;
        let first_item = (flags & 1 != 0).then_some(first);
        let second_item = (flags & 2 != 0).then_some(second);
        let item_operation = matches!(
            operation,
            Operation::Fusion
                | Operation::ConvergenceFusion
                | Operation::Transfer
                | Operation::ConvergenceTransfer
        );
        if flags != if item_operation { 3 } else { 0 }
            || (flags & 1 == 0 && first != [0; 16])
            || (flags & 2 == 0 && second != [0; 16])
            || (operation == Operation::QueryHistory && occurrence != [0; 16])
        {
            return None;
        }
        let history_limit = bytes[50];
        if history_limit > MAX_HISTORY
            || (!matches!(operation, Operation::QueryHistory) && history_limit != 0)
        {
            return None;
        }
        Some(Self {
            operation,
            occurrence,
            first_item,
            second_item,
            history_limit,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn operations_require_canonical_items_and_history() {
        for operation in 0..=7 {
            let mut bytes = [0; 51];
            bytes[0] = operation;
            if (1..=4).contains(&operation) {
                bytes[17] = 3;
                bytes[18..34].fill(1);
                bytes[34..50].fill(2);
            }
            assert!(Request::decode(&bytes).is_some(), "operation {operation}");
            bytes[17] ^= 1;
            assert!(Request::decode(&bytes).is_none());
        }
        let mut bytes = [0; 51];
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
        let mut b = [0u8; 51];
        b[0] = 7;
        assert!(Request::decode(&b).is_some());
        b[17] = 4;
        assert!(Request::decode(&b).is_none());
        assert!(Request::decode(&b[..50]).is_none());
    }
}
