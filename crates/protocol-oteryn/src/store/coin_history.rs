//! Opt-in full Platform ledger view on command 184. Legacy history requests remain distinct.
use super::*;
use crate::charm_wire::{push_message_field, push_varint_field};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoinHistoryEntry {
    pub entry_id: u64,
    pub kind: String,
    pub currency: String,
    pub available_delta: i64,
    pub reserved_delta: i64,
    pub order_id: Option<[u8; 16]>,
    pub offer_key: Option<String>,
    pub quantity: Option<u8>,
    pub created_at: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoinHistoryPage {
    pub entries: Vec<CoinHistoryEntry>,
    pub next_page_cursor: String,
}
fn number<T: std::str::FromStr + ToString>(bytes: &[u8]) -> Result<T, StoreWireError> {
    let text = as_text(bytes, 21)?;
    let value = text.parse::<T>().map_err(|_| StoreWireError::Malformed)?;
    if value.to_string() != text {
        return Err(StoreWireError::Malformed);
    }
    Ok(value)
}
fn valid_cursor(cursor: &str) -> bool {
    cursor.is_empty() || number::<u64>(cursor.as_bytes()).is_ok_and(|n| n > 0)
}
pub fn encode_coin_history_request(
    request: &StoreHistoryRequest,
) -> Result<Vec<u8>, StoreWireError> {
    if !valid_cursor(&request.page_cursor) {
        return Err(StoreWireError::Malformed);
    }
    let mut out = encode_history_request(request)?;
    push_varint_field(&mut out, 2, 1);
    Ok(out)
}
pub fn decode_coin_history_request(input: &[u8]) -> Result<StoreHistoryRequest, StoreWireError> {
    if input.len() > MAX_STORE_REQUEST_BYTES {
        return Err(StoreWireError::LimitExceeded);
    }
    let (mut at, mut cursor, mut ledger) = (0, None, None);
    while at < input.len() {
        match read_varint(input, &mut at)? {
            10 => set_once(
                &mut cursor,
                as_text(read_bytes(input, &mut at)?, MAX_STORE_CURSOR_BYTES)?.to_owned(),
            )?,
            16 => set_once(&mut ledger, read_varint(input, &mut at)?)?,
            _ => return Err(StoreWireError::Malformed),
        }
    }
    let request = StoreHistoryRequest {
        page_cursor: cursor.unwrap_or_default(),
    };
    if ledger != Some(1) || !valid_cursor(&request.page_cursor) {
        return Err(StoreWireError::Malformed);
    }
    Ok(request)
}
fn encode_entry(entry: &CoinHistoryEntry) -> Result<Vec<u8>, StoreWireError> {
    let purchase = matches!(entry.kind.as_str(), "STORE_PURCHASE" | "STORE_REVERSAL");
    if entry.entry_id == 0
        || !valid_key(&entry.kind)
        || !valid_key(&entry.currency)
        || entry.available_delta == i64::MIN
        || entry.reserved_delta == i64::MIN
        || purchase != entry.order_id.is_some()
        || purchase != entry.offer_key.is_some()
        || purchase != entry.quantity.is_some()
        || entry.order_id.as_ref().is_some_and(|id| !valid_uuid_v7(id))
        || entry.offer_key.as_ref().is_some_and(|key| !valid_key(key))
        || entry.quantity.is_some_and(|n| !(1..=100).contains(&n))
    {
        return Err(StoreWireError::Malformed);
    }
    let mut out = Vec::new();
    for (field, text) in [
        (1, entry.entry_id.to_string()),
        (2, entry.kind.clone()),
        (3, entry.currency.clone()),
        (4, entry.available_delta.to_string()),
        (5, entry.reserved_delta.to_string()),
        (6, entry.created_at.to_string()),
    ] {
        push_message_field(&mut out, field, text.as_bytes());
    }
    if let Some(id) = &entry.order_id {
        push_message_field(&mut out, 7, id);
    }
    if let Some(key) = &entry.offer_key {
        push_message_field(&mut out, 8, key.as_bytes());
    }
    if let Some(quantity) = entry.quantity {
        push_message_field(&mut out, 9, quantity.to_string().as_bytes());
    }
    Ok(out)
}
fn decode_entry(bytes: &[u8]) -> Result<CoinHistoryEntry, StoreWireError> {
    let fields = decode_fields::<9>(bytes)?;
    let required = |i: usize| fields[i].ok_or(StoreWireError::Malformed);
    let entry = CoinHistoryEntry {
        entry_id: number(required(0)?)?,
        kind: as_text(required(1)?, MAX_STORE_KEY_BYTES)?.into(),
        currency: as_text(required(2)?, MAX_STORE_KEY_BYTES)?.into(),
        available_delta: number(required(3)?)?,
        reserved_delta: number(required(4)?)?,
        created_at: number(required(5)?)?,
        order_id: fields[6].map(as_uuid).transpose()?,
        offer_key: fields[7]
            .map(|v| as_text(v, MAX_STORE_KEY_BYTES).map(str::to_owned))
            .transpose()?,
        quantity: fields[8].map(number).transpose()?,
    };
    encode_entry(&entry)?;
    Ok(entry)
}
pub fn encode_coin_history_page(page: &CoinHistoryPage) -> Result<Vec<u8>, StoreWireError> {
    if page.entries.len() > MAX_STORE_PAGE_ENTRIES {
        return Err(StoreWireError::LimitExceeded);
    }
    if !valid_cursor(&page.next_page_cursor)
        || page
            .entries
            .windows(2)
            .any(|pair| pair[0].entry_id <= pair[1].entry_id)
        || (!page.next_page_cursor.is_empty()
            && page
                .entries
                .last()
                .is_none_or(|row| row.entry_id.to_string() != page.next_page_cursor))
    {
        return Err(StoreWireError::Malformed);
    }
    let mut out = Vec::new();
    for entry in &page.entries {
        push_message_field(&mut out, 1, &encode_entry(entry)?);
    }
    if !page.next_page_cursor.is_empty() {
        push_message_field(&mut out, 2, page.next_page_cursor.as_bytes());
    }
    if out.len() > MAX_STORE_RESULT_BYTES {
        return Err(StoreWireError::LimitExceeded);
    }
    Ok(out)
}
pub fn decode_coin_history_page(input: &[u8]) -> Result<CoinHistoryPage, StoreWireError> {
    if input.len() > MAX_STORE_RESULT_BYTES {
        return Err(StoreWireError::LimitExceeded);
    }
    let (mut at, mut entries, mut next) = (0, Vec::new(), None);
    while at < input.len() {
        match read_varint(input, &mut at)? {
            10 if entries.len() < MAX_STORE_PAGE_ENTRIES => {
                entries.push(decode_entry(read_bytes(input, &mut at)?)?)
            }
            18 => set_once(
                &mut next,
                as_text(read_bytes(input, &mut at)?, MAX_STORE_CURSOR_BYTES)?.to_owned(),
            )?,
            _ => return Err(StoreWireError::Malformed),
        }
    }
    let page = CoinHistoryPage {
        entries,
        next_page_cursor: next.unwrap_or_default(),
    };
    encode_coin_history_page(&page)?;
    Ok(page)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coin_history_requires_explicit_opt_in_and_canonical_cursor() -> Result<(), StoreWireError> {
        let request = StoreHistoryRequest {
            page_cursor: "8".into(),
        };
        let encoded = encode_coin_history_request(&request)?;
        assert_eq!(decode_coin_history_request(&encoded)?, request);
        assert!(decode_history_request(&encoded).is_err());
        assert!(decode_coin_history_request(&encode_history_request(&request)?).is_err());
        for cursor in ["0", "08", "+8", "-1", "18446744073709551616"] {
            assert!(
                encode_coin_history_request(&StoreHistoryRequest {
                    page_cursor: cursor.into()
                })
                .is_err()
            );
        }
        for invalid in [&[16, 1, 16, 1][..], &[16, 2][..], &[16, 1, 24, 1][..]] {
            assert!(decode_coin_history_request(invalid).is_err());
        }
        Ok(())
    }
    #[test]
    fn coin_history_rejects_oversized_misordered_or_inconsistent_entries()
    -> Result<(), StoreWireError> {
        let entry = CoinHistoryEntry {
            entry_id: 9,
            kind: "OTHER".into(),
            currency: "COINS".into(),
            available_delta: -250,
            reserved_delta: 50,
            order_id: None,
            offer_key: None,
            quantity: None,
            created_at: 100,
        };
        let mut page = CoinHistoryPage {
            entries: vec![entry.clone()],
            next_page_cursor: "9".into(),
        };
        assert_eq!(
            decode_coin_history_page(&encode_coin_history_page(&page)?)?,
            page
        );
        page.entries[0].available_delta = i64::MIN;
        assert!(encode_coin_history_page(&page).is_err());
        page.entries = vec![entry.clone(), entry.clone()];
        assert!(encode_coin_history_page(&page).is_err());
        page.entries = vec![entry.clone(); 33];
        assert!(encode_coin_history_page(&page).is_err());
        page.entries = vec![entry];
        page.next_page_cursor = "8".into();
        assert!(encode_coin_history_page(&page).is_err());
        page.next_page_cursor = "9".into();
        page.entries[0].kind = "STORE_PURCHASE".into();
        assert!(encode_coin_history_page(&page).is_err());
        Ok(())
    }
}
