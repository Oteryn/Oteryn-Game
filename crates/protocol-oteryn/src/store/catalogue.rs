use super::*;
use crate::charm_wire::{push_message_field, push_varint_field, read_varint};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreCatalogueRequest {
    pub category: String,
    pub page_cursor: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreOffer {
    pub offer_key: String,
    pub title: String,
    pub price: u64,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreCataloguePage {
    pub catalog_revision: String,
    pub offers: Vec<StoreOffer>,
    pub next_page_cursor: String,
    pub total_coins: u64,
    pub transferable_coins: u64,
}

pub fn encode_catalogue_request(value: &StoreCatalogueRequest) -> Result<Vec<u8>, StoreWireError> {
    if !valid_key(&value.category)
        || (!value.page_cursor.is_empty()
            && !valid_text(&value.page_cursor, MAX_STORE_CURSOR_BYTES))
    {
        return Err(StoreWireError::Malformed);
    }
    let mut out = Vec::new();
    push_message_field(&mut out, 1, value.category.as_bytes());
    if !value.page_cursor.is_empty() {
        push_message_field(&mut out, 2, value.page_cursor.as_bytes());
    }
    Ok(out)
}

pub fn decode_catalogue_request(input: &[u8]) -> Result<StoreCatalogueRequest, StoreWireError> {
    if input.len() > MAX_STORE_REQUEST_BYTES {
        return Err(StoreWireError::LimitExceeded);
    }
    let f = decode_fields::<2>(input)?;
    let category = as_text(f[0].ok_or(StoreWireError::Malformed)?, MAX_STORE_KEY_BYTES)?;
    if !valid_key(category) {
        return Err(StoreWireError::Malformed);
    }
    let cursor = match f[1] {
        Some(v) => as_text(v, MAX_STORE_CURSOR_BYTES)?,
        None => "",
    };
    Ok(StoreCatalogueRequest {
        category: category.into(),
        page_cursor: cursor.into(),
    })
}

pub fn encode_catalogue_page(value: &StoreCataloguePage) -> Result<Vec<u8>, StoreWireError> {
    if !valid_key(&value.catalog_revision)
        || value.offers.len() > MAX_STORE_PAGE_ENTRIES
        || (!value.next_page_cursor.is_empty()
            && !valid_text(&value.next_page_cursor, MAX_STORE_CURSOR_BYTES))
        || value.transferable_coins > value.total_coins
    {
        return Err(StoreWireError::Malformed);
    }
    let mut out = Vec::new();
    push_message_field(&mut out, 1, value.catalog_revision.as_bytes());
    for offer in &value.offers {
        if !valid_key(&offer.offer_key)
            || !valid_text(&offer.title, MAX_STORE_TEXT_BYTES)
            || !valid_key(&offer.currency)
            || offer.price == 0
        {
            return Err(StoreWireError::Malformed);
        }
        let mut row = Vec::new();
        push_message_field(&mut row, 1, offer.offer_key.as_bytes());
        push_message_field(&mut row, 2, offer.title.as_bytes());
        push_varint_field(&mut row, 3, offer.price);
        push_message_field(&mut row, 4, offer.currency.as_bytes());
        push_message_field(&mut out, 2, &row);
    }
    if !value.next_page_cursor.is_empty() {
        push_message_field(&mut out, 3, value.next_page_cursor.as_bytes());
    }
    push_varint_field(&mut out, 4, value.total_coins);
    push_varint_field(&mut out, 5, value.transferable_coins);
    if out.len() > MAX_STORE_RESULT_BYTES {
        return Err(StoreWireError::LimitExceeded);
    }
    Ok(out)
}

pub fn decode_catalogue_page(input: &[u8]) -> Result<StoreCataloguePage, StoreWireError> {
    if input.len() > MAX_STORE_RESULT_BYTES {
        return Err(StoreWireError::LimitExceeded);
    }
    let mut revision = None;
    let mut cursor = None;
    let mut total = None;
    let mut transferable = None;
    let mut rows = Vec::new();
    let mut at = 0;
    while at < input.len() {
        let tag = read_varint(input, &mut at)?;
        match tag {
            10 => crate::charm_wire::set_once(
                &mut revision,
                crate::charm_wire::read_bytes(input, &mut at)?,
            )?,
            18 => {
                if rows.len() == MAX_STORE_PAGE_ENTRIES {
                    return Err(StoreWireError::LimitExceeded);
                }
                rows.push(crate::charm_wire::read_bytes(input, &mut at)?);
            }
            26 => crate::charm_wire::set_once(
                &mut cursor,
                crate::charm_wire::read_bytes(input, &mut at)?,
            )?,
            32 => crate::charm_wire::set_once(&mut total, read_varint(input, &mut at)?)?,
            40 => crate::charm_wire::set_once(&mut transferable, read_varint(input, &mut at)?)?,
            _ => return Err(StoreWireError::Malformed),
        }
    }
    let revision = as_text(
        revision.ok_or(StoreWireError::Malformed)?,
        MAX_STORE_KEY_BYTES,
    )?;
    if !valid_key(revision) {
        return Err(StoreWireError::Malformed);
    }
    let next = match cursor {
        Some(v) => as_text(v, MAX_STORE_CURSOR_BYTES)?,
        None => "",
    };
    let mut offers = Vec::with_capacity(rows.len());
    for row in rows {
        let mut key = None;
        let mut title = None;
        let mut price = None;
        let mut currency = None;
        let mut at = 0;
        while at < row.len() {
            match read_varint(row, &mut at)? {
                10 => crate::charm_wire::set_once(
                    &mut key,
                    crate::charm_wire::read_bytes(row, &mut at)?,
                )?,
                18 => crate::charm_wire::set_once(
                    &mut title,
                    crate::charm_wire::read_bytes(row, &mut at)?,
                )?,
                24 => crate::charm_wire::set_once(&mut price, read_varint(row, &mut at)?)?,
                34 => crate::charm_wire::set_once(
                    &mut currency,
                    crate::charm_wire::read_bytes(row, &mut at)?,
                )?,
                _ => return Err(StoreWireError::Malformed),
            }
        }
        let key = as_text(key.ok_or(StoreWireError::Malformed)?, MAX_STORE_KEY_BYTES)?;
        let title = as_text(
            title.ok_or(StoreWireError::Malformed)?,
            MAX_STORE_TEXT_BYTES,
        )?;
        let price = price.ok_or(StoreWireError::Malformed)?;
        let currency = as_text(
            currency.ok_or(StoreWireError::Malformed)?,
            MAX_STORE_KEY_BYTES,
        )?;
        if !valid_key(key) || !valid_key(currency) || price == 0 {
            return Err(StoreWireError::Malformed);
        }
        offers.push(StoreOffer {
            offer_key: key.into(),
            title: title.into(),
            price,
            currency: currency.into(),
        });
    }
    let total = total.ok_or(StoreWireError::Malformed)?;
    let transferable = transferable.ok_or(StoreWireError::Malformed)?;
    if transferable > total {
        return Err(StoreWireError::Malformed);
    }
    Ok(StoreCataloguePage {
        catalog_revision: revision.into(),
        offers,
        next_page_cursor: next.into(),
        total_coins: total,
        transferable_coins: transferable,
    })
}
