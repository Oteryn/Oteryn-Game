use super::*;
use crate::charm_wire::{push_message_field, push_varint_field, read_varint};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreHistoryRequest {
    pub page_cursor: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreHistoryEntry {
    pub order_id: [u8; 16],
    pub offer_key: String,
    pub catalog_revision: String,
    pub price: u64,
    pub currency: String,
    pub purchased_at_unix_ms: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreHistoryPage {
    pub entries: Vec<StoreHistoryEntry>,
    pub next_page_cursor: String,
}

pub fn encode_history_request(v: &StoreHistoryRequest) -> Result<Vec<u8>, StoreWireError> {
    if !v.page_cursor.is_empty() && !valid_text(&v.page_cursor, MAX_STORE_CURSOR_BYTES) {
        return Err(StoreWireError::Malformed);
    }
    let mut o = Vec::new();
    if !v.page_cursor.is_empty() {
        push_message_field(&mut o, 1, v.page_cursor.as_bytes())
    }
    Ok(o)
}
pub fn decode_history_request(i: &[u8]) -> Result<StoreHistoryRequest, StoreWireError> {
    if i.len() > MAX_STORE_REQUEST_BYTES {
        return Err(StoreWireError::LimitExceeded);
    }
    if i.is_empty() {
        return Ok(StoreHistoryRequest {
            page_cursor: String::new(),
        });
    }
    let f = decode_fields::<1>(i)?;
    Ok(StoreHistoryRequest {
        page_cursor: as_text(
            f[0].ok_or(StoreWireError::Malformed)?,
            MAX_STORE_CURSOR_BYTES,
        )?
        .into(),
    })
}
pub fn encode_history_page(v: &StoreHistoryPage) -> Result<Vec<u8>, StoreWireError> {
    if v.entries.len() > MAX_STORE_PAGE_ENTRIES
        || (!v.next_page_cursor.is_empty()
            && !valid_text(&v.next_page_cursor, MAX_STORE_CURSOR_BYTES))
    {
        return Err(StoreWireError::LimitExceeded);
    }
    let mut o = Vec::new();
    for e in &v.entries {
        if !valid_uuid_v7(&e.order_id)
            || !valid_key(&e.offer_key)
            || !valid_key(&e.catalog_revision)
            || e.price == 0
            || !valid_key(&e.currency)
            || e.purchased_at_unix_ms > i64::MAX as u64
        {
            return Err(StoreWireError::Malformed);
        }
        let mut r = Vec::new();
        push_message_field(&mut r, 1, &e.order_id);
        push_message_field(&mut r, 2, e.offer_key.as_bytes());
        push_message_field(&mut r, 3, e.catalog_revision.as_bytes());
        push_varint_field(&mut r, 4, e.price);
        push_message_field(&mut r, 5, e.currency.as_bytes());
        push_varint_field(&mut r, 6, e.purchased_at_unix_ms);
        push_message_field(&mut o, 1, &r)
    }
    if !v.next_page_cursor.is_empty() {
        push_message_field(&mut o, 2, v.next_page_cursor.as_bytes())
    }
    if o.len() > MAX_STORE_RESULT_BYTES {
        return Err(StoreWireError::LimitExceeded);
    }
    Ok(o)
}
pub fn decode_history_page(i: &[u8]) -> Result<StoreHistoryPage, StoreWireError> {
    if i.len() > MAX_STORE_RESULT_BYTES {
        return Err(StoreWireError::LimitExceeded);
    }
    let mut rows = Vec::new();
    let mut cursor = None;
    let mut at = 0;
    while at < i.len() {
        match read_varint(i, &mut at)? {
            10 => {
                if rows.len() == MAX_STORE_PAGE_ENTRIES {
                    return Err(StoreWireError::LimitExceeded);
                }
                rows.push(crate::charm_wire::read_bytes(i, &mut at)?)
            }
            18 => crate::charm_wire::set_once(
                &mut cursor,
                crate::charm_wire::read_bytes(i, &mut at)?,
            )?,
            _ => return Err(StoreWireError::Malformed),
        }
    }
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let mut id = None;
        let mut offer = None;
        let mut rev = None;
        let mut price = None;
        let mut curr = None;
        let mut time = None;
        let mut p = 0;
        while p < row.len() {
            match read_varint(row, &mut p)? {
                10 => crate::charm_wire::set_once(
                    &mut id,
                    as_uuid(crate::charm_wire::read_bytes(row, &mut p)?)?,
                )?,
                18 => crate::charm_wire::set_once(
                    &mut offer,
                    crate::charm_wire::read_bytes(row, &mut p)?,
                )?,
                26 => crate::charm_wire::set_once(
                    &mut rev,
                    crate::charm_wire::read_bytes(row, &mut p)?,
                )?,
                32 => crate::charm_wire::set_once(&mut price, read_varint(row, &mut p)?)?,
                42 => crate::charm_wire::set_once(
                    &mut curr,
                    crate::charm_wire::read_bytes(row, &mut p)?,
                )?,
                48 => crate::charm_wire::set_once(&mut time, read_varint(row, &mut p)?)?,
                _ => return Err(StoreWireError::Malformed),
            }
        }
        let offer = as_text(offer.ok_or(StoreWireError::Malformed)?, MAX_STORE_KEY_BYTES)?;
        let revision = as_text(rev.ok_or(StoreWireError::Malformed)?, MAX_STORE_KEY_BYTES)?;
        let currency = as_text(curr.ok_or(StoreWireError::Malformed)?, MAX_STORE_KEY_BYTES)?;
        let price = price.ok_or(StoreWireError::Malformed)?;
        let purchased_at_unix_ms = time.ok_or(StoreWireError::Malformed)?;
        if !valid_key(offer)
            || !valid_key(revision)
            || !valid_key(currency)
            || price == 0
            || purchased_at_unix_ms > i64::MAX as u64
        {
            return Err(StoreWireError::Malformed);
        }
        entries.push(StoreHistoryEntry {
            order_id: id.ok_or(StoreWireError::Malformed)?,
            offer_key: offer.into(),
            catalog_revision: revision.into(),
            price,
            currency: currency.into(),
            purchased_at_unix_ms,
        })
    }
    let next = match cursor {
        Some(v) => as_text(v, MAX_STORE_CURSOR_BYTES)?.into(),
        None => String::new(),
    };
    Ok(StoreHistoryPage {
        entries,
        next_page_cursor: next,
    })
}
