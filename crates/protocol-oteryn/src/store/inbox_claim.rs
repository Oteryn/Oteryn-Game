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
const PAGED_INBOX_QUERY: &[u8] = &[16, 2];
pub const MAX_INBOX_ENTRIES: usize = 20;

pub fn encode_inbox_query(after: Option<[u8; 16]>) -> Vec<u8> {
    let mut output = PAGED_INBOX_QUERY.to_vec();
    if let Some(after) = after {
        push_message_field(&mut output, 3, &after);
    }
    output
}

/// The outer option distinguishes a query from a claim, the inner one is its cursor.
pub fn decode_inbox_query(input: &[u8]) -> Option<Option<[u8; 16]>> {
    let tail = input.strip_prefix(PAGED_INBOX_QUERY)?;
    match tail {
        [] => Some(None),
        [26, 16, after @ ..] => Some(Some(as_uuid(after).ok()?)),
        _ => None,
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoreInboxPage {
    pub entries: Vec<StoreInboxEntry>,
    pub next: Option<[u8; 16]>,
    pub lamp_definition: Option<std::num::NonZeroU32>,
    pub claims_require_channel: bool,
}

pub fn encode_inbox_page(page: &StoreInboxPage) -> Result<Vec<u8>, StoreWireError> {
    let mut output = encode_inbox_entries(&page.entries)?;
    if let Some(next) = page.next {
        as_uuid(&next)?;
        push_message_field(&mut output, 2, &next);
    }
    if let Some(definition) = page.lamp_definition {
        push_message_field(&mut output, 3, &definition.get().to_be_bytes());
    }
    if page.claims_require_channel {
        push_varint_field(&mut output, 4, 1);
    }
    Ok(output)
}

pub fn decode_inbox_page(input: &[u8]) -> Result<StoreInboxPage, StoreWireError> {
    let (entries, mut cursor) = decode_entry_prefix(input)?;
    let mut page = StoreInboxPage {
        entries,
        ..Default::default()
    };
    if input.get(cursor) == Some(&18) {
        cursor += 1;
        page.next = Some(as_uuid(read_bytes(input, &mut cursor)?)?);
    }
    if input.get(cursor) == Some(&26) {
        cursor += 1;
        let bytes = read_bytes(input, &mut cursor)?
            .try_into()
            .map_err(|_| StoreWireError::Malformed)?;
        page.lamp_definition = Some(
            std::num::NonZeroU32::new(u32::from_be_bytes(bytes))
                .ok_or(StoreWireError::Malformed)?,
        );
    }
    match &input[cursor..] {
        [] => {}
        [32, 1] => page.claims_require_channel = true,
        _ => return Err(StoreWireError::Malformed),
    }
    Ok(page)
}

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
    let (entries, cursor) = decode_entry_prefix(input)?;
    if cursor != input.len() {
        return Err(StoreWireError::Malformed);
    }
    Ok(entries)
}

fn decode_entry_prefix(input: &[u8]) -> Result<(Vec<StoreInboxEntry>, usize), StoreWireError> {
    if input.len() > MAX_STORE_REQUEST_BYTES {
        return Err(StoreWireError::LimitExceeded);
    }
    let mut entries = Vec::new();
    let mut cursor = 0;
    while input.get(cursor) == Some(&10) {
        if entries.len() == MAX_INBOX_ENTRIES {
            return Err(StoreWireError::LimitExceeded);
        }
        cursor += 1;
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
    Ok((entries, cursor))
}

#[cfg(test)]
mod page_tests {
    use super::*;

    #[test]
    fn full_inbox_page_with_cursor_and_house_actions_fits_store_reply() -> Result<(), StoreWireError>
    {
        let id = [0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1];
        let page = StoreInboxPage {
            entries: vec![
                StoreInboxEntry {
                    fulfilment_unit_id: id,
                    item_id: u16::MAX
                };
                MAX_INBOX_ENTRIES
            ],
            next: Some(id),
            lamp_definition: Some(std::num::NonZeroU32::MAX),
            claims_require_channel: true,
        };
        let bytes = encode_inbox_page(&page)?;
        assert!(bytes.len() <= MAX_STORE_REQUEST_BYTES);
        assert_eq!(decode_inbox_page(&bytes)?, page);
        assert_eq!(
            decode_inbox_query(&encode_inbox_query(page.next)),
            Some(page.next)
        );
        assert_eq!(decode_inbox_query(&encode_inbox_query(None)), Some(None));
        assert!(decode_inbox_query(STORE_INBOX_QUERY).is_none());
        Ok(())
    }
}
