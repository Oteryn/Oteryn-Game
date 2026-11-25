use super::*;
use crate::charm_wire::{push_message_field, push_varint_field, read_varint};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreInboxClaimRequest {
    pub fulfilment_unit_id: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StoreClaimCode {
    Committed = 1,
    Revoked = 2,
    NotClaimable = 3,
    Rejected = 4,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreClaimReceipt {
    pub operation_id: [u8; 16],
    pub fulfilment_unit_id: String,
    pub result: StoreClaimCode,
    pub recovery_generation: u64,
    pub decided_at_unix_ms: u64,
}
pub fn encode_inbox_claim_request(v: &StoreInboxClaimRequest) -> Result<Vec<u8>, StoreWireError> {
    if !valid_key(&v.fulfilment_unit_id) {
        return Err(StoreWireError::Malformed);
    }
    let mut o = Vec::new();
    push_message_field(&mut o, 1, v.fulfilment_unit_id.as_bytes());
    Ok(o)
}
pub fn decode_inbox_claim_request(i: &[u8]) -> Result<StoreInboxClaimRequest, StoreWireError> {
    if i.len() > MAX_STORE_REQUEST_BYTES {
        return Err(StoreWireError::LimitExceeded);
    }
    let f = decode_fields::<1>(i)?;
    let s = as_text(f[0].ok_or(StoreWireError::Malformed)?, MAX_STORE_KEY_BYTES)?;
    if !valid_key(s) {
        return Err(StoreWireError::Malformed);
    }
    Ok(StoreInboxClaimRequest {
        fulfilment_unit_id: s.into(),
    })
}
pub fn encode_claim_receipt(v: &StoreClaimReceipt) -> Result<Vec<u8>, StoreWireError> {
    if !valid_uuid_v7(&v.operation_id)
        || !valid_key(&v.fulfilment_unit_id)
        || v.recovery_generation == 0
        || v.decided_at_unix_ms > i64::MAX as u64
    {
        return Err(StoreWireError::Malformed);
    }
    let mut o = Vec::new();
    push_message_field(&mut o, 1, &v.operation_id);
    push_message_field(&mut o, 2, v.fulfilment_unit_id.as_bytes());
    push_varint_field(&mut o, 3, v.result as u64);
    push_varint_field(&mut o, 4, v.recovery_generation);
    push_varint_field(&mut o, 5, v.decided_at_unix_ms);
    Ok(o)
}
pub fn decode_claim_receipt(i: &[u8]) -> Result<StoreClaimReceipt, StoreWireError> {
    if i.len() > MAX_STORE_REQUEST_BYTES {
        return Err(StoreWireError::LimitExceeded);
    }
    let mut id = None;
    let mut unit = None;
    let mut result = None;
    let mut generation = None;
    let mut decided = None;
    let mut at = 0;
    while at < i.len() {
        match read_varint(i, &mut at)? {
            10 => crate::charm_wire::set_once(
                &mut id,
                as_uuid(crate::charm_wire::read_bytes(i, &mut at)?)?,
            )?,
            18 => {
                crate::charm_wire::set_once(&mut unit, crate::charm_wire::read_bytes(i, &mut at)?)?
            }
            24 => crate::charm_wire::set_once(&mut result, read_varint(i, &mut at)?)?,
            32 => crate::charm_wire::set_once(&mut generation, read_varint(i, &mut at)?)?,
            40 => crate::charm_wire::set_once(&mut decided, read_varint(i, &mut at)?)?,
            _ => return Err(StoreWireError::Malformed),
        }
    }
    let unit = as_text(unit.ok_or(StoreWireError::Malformed)?, MAX_STORE_KEY_BYTES)?;
    if !valid_key(unit) {
        return Err(StoreWireError::Malformed);
    }
    let result = match result {
        Some(1) => StoreClaimCode::Committed,
        Some(2) => StoreClaimCode::Revoked,
        Some(3) => StoreClaimCode::NotClaimable,
        Some(4) => StoreClaimCode::Rejected,
        _ => return Err(StoreWireError::Malformed),
    };
    let receipt = StoreClaimReceipt {
        operation_id: id.ok_or(StoreWireError::Malformed)?,
        fulfilment_unit_id: unit.into(),
        result,
        recovery_generation: generation.ok_or(StoreWireError::Malformed)?,
        decided_at_unix_ms: decided.ok_or(StoreWireError::Malformed)?,
    };
    encode_claim_receipt(&receipt)?;
    Ok(receipt)
}

/// A projection uses the existing Store capability and a request field distinct from a claim.
pub const STORE_INBOX_QUERY: &[u8] = &[16, 1];
const MAX_INBOX_ENTRIES: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreInboxEntry {
    pub fulfilment_unit_id: [u8; 16],
    pub item_id: u16,
}

impl StoreInboxEntry {
    pub fn claim_key(&self) -> String {
        let hex = format!("{:032x}", u128::from_be_bytes(self.fulfilment_unit_id));
        format!(
            "{}-{}-{}-{}-{}",
            &hex[..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..]
        )
    }
}

pub fn encode_inbox_entries(entries: &[StoreInboxEntry]) -> Result<Vec<u8>, StoreWireError> {
    if entries.len() > MAX_INBOX_ENTRIES {
        return Err(StoreWireError::LimitExceeded);
    }
    let mut output = Vec::new();
    for entry in entries {
        if !valid_uuid_v7(&entry.fulfilment_unit_id) || entry.item_id == 0 {
            return Err(StoreWireError::Malformed);
        }
        let mut row = Vec::new();
        push_message_field(&mut row, 1, &entry.fulfilment_unit_id);
        push_varint_field(&mut row, 2, u64::from(entry.item_id));
        push_message_field(&mut output, 1, &row);
    }
    Ok(output)
}

pub fn decode_inbox_entries(input: &[u8]) -> Result<Vec<StoreInboxEntry>, StoreWireError> {
    if input.len() > MAX_STORE_REQUEST_BYTES {
        return Err(StoreWireError::LimitExceeded);
    }
    let mut entries = Vec::new();
    let mut cursor = 0;
    while cursor < input.len() {
        if entries.len() == MAX_INBOX_ENTRIES {
            return Err(StoreWireError::LimitExceeded);
        }
        if read_varint(input, &mut cursor)? != 10 {
            return Err(StoreWireError::Malformed);
        }
        let row = read_bytes(input, &mut cursor)?;
        let mut field = 0;
        if read_varint(row, &mut field)? != 10 {
            return Err(StoreWireError::Malformed);
        }
        let fulfilment_unit_id = as_uuid(read_bytes(row, &mut field)?)?;
        if read_varint(row, &mut field)? != 16 {
            return Err(StoreWireError::Malformed);
        }
        let item_id =
            u16::try_from(read_varint(row, &mut field)?).map_err(|_| StoreWireError::Malformed)?;
        if item_id == 0 || field != row.len() {
            return Err(StoreWireError::Malformed);
        }
        entries.push(StoreInboxEntry {
            fulfilment_unit_id,
            item_id,
        });
    }
    Ok(entries)
}
