use super::*;
use crate::charm_wire::{push_message_field, push_varint_field, read_varint};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorePurchaseRequest {
    pub purchase_request_id: [u8; 16],
    pub offer_key: String,
    pub catalog_revision: String,
    pub price: u64,
    pub currency: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StorePurchaseCode {
    Accepted = 1,
    StaleQuote = 2,
    OfferUnavailable = 3,
    PurchaseDisabled = 4,
    InsufficientBalance = 5,
    AccountRestricted = 6,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorePurchaseResult {
    pub code: StorePurchaseCode,
    pub order_id: Option<[u8; 16]>,
    pub operation_id: Option<[u8; 16]>,
}

pub fn encode_purchase_request(value: &StorePurchaseRequest) -> Result<Vec<u8>, StoreWireError> {
    if !valid_uuid_v7(&value.purchase_request_id)
        || !valid_key(&value.offer_key)
        || !valid_key(&value.catalog_revision)
        || value.price == 0
        || !valid_key(&value.currency)
    {
        return Err(StoreWireError::Malformed);
    }
    let mut out = Vec::new();
    push_message_field(&mut out, 1, &value.purchase_request_id);
    push_message_field(&mut out, 2, value.offer_key.as_bytes());
    push_message_field(&mut out, 3, value.catalog_revision.as_bytes());
    push_varint_field(&mut out, 4, value.price);
    push_message_field(&mut out, 5, value.currency.as_bytes());
    Ok(out)
}

pub fn decode_purchase_request(input: &[u8]) -> Result<StorePurchaseRequest, StoreWireError> {
    decode_purchase_request_with_hireling(input).map(|(request, _)| request)
}

pub fn decode_purchase_request_with_hireling(
    input: &[u8],
) -> Result<(StorePurchaseRequest, Option<StoreHirelingCustomization>), StoreWireError> {
    if input.len() > MAX_STORE_REQUEST_BYTES {
        return Err(StoreWireError::LimitExceeded);
    }
    let mut id = None;
    let mut offer = None;
    let mut revision = None;
    let mut price = None;
    let mut currency = None;
    let mut hireling = None;
    let mut at = 0;
    while at < input.len() {
        let tag = read_varint(input, &mut at)?;
        match tag {
            10 => crate::charm_wire::set_once(
                &mut id,
                crate::charm_wire::read_bytes(input, &mut at)?,
            )?,
            18 => crate::charm_wire::set_once(
                &mut offer,
                crate::charm_wire::read_bytes(input, &mut at)?,
            )?,
            26 => crate::charm_wire::set_once(
                &mut revision,
                crate::charm_wire::read_bytes(input, &mut at)?,
            )?,
            32 => crate::charm_wire::set_once(&mut price, read_varint(input, &mut at)?)?,
            42 => crate::charm_wire::set_once(
                &mut currency,
                crate::charm_wire::read_bytes(input, &mut at)?,
            )?,
            50 => crate::charm_wire::set_once(
                &mut hireling,
                StoreHirelingCustomization::decode(crate::charm_wire::read_bytes(input, &mut at)?)?,
            )?,
            _ => return Err(StoreWireError::Malformed),
        }
    }
    let offer = as_text(offer.ok_or(StoreWireError::Malformed)?, MAX_STORE_KEY_BYTES)?;
    let revision = as_text(
        revision.ok_or(StoreWireError::Malformed)?,
        MAX_STORE_KEY_BYTES,
    )?;
    let currency = as_text(
        currency.ok_or(StoreWireError::Malformed)?,
        MAX_STORE_KEY_BYTES,
    )?;
    let price = price.ok_or(StoreWireError::Malformed)?;
    if !valid_key(offer) || !valid_key(revision) || !valid_key(currency) || price == 0 {
        return Err(StoreWireError::Malformed);
    }
    Ok((
        StorePurchaseRequest {
            purchase_request_id: as_uuid(id.ok_or(StoreWireError::Malformed)?)?,
            offer_key: offer.into(),
            catalog_revision: revision.into(),
            price,
            currency: currency.into(),
        },
        hireling,
    ))
}

pub fn encode_purchase_result(value: &StorePurchaseResult) -> Result<Vec<u8>, StoreWireError> {
    let accepted = value.code == StorePurchaseCode::Accepted;
    if accepted != (value.order_id.is_some() && value.operation_id.is_some()) {
        return Err(StoreWireError::Malformed);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, value.code as u64);
    if let Some(id) = value.order_id {
        if !valid_uuid_v7(&id) {
            return Err(StoreWireError::Malformed);
        }
        push_message_field(&mut out, 2, &id)
    }
    if let Some(id) = value.operation_id {
        if !valid_uuid_v7(&id) {
            return Err(StoreWireError::Malformed);
        }
        push_message_field(&mut out, 3, &id)
    }
    Ok(out)
}

pub fn decode_purchase_result(input: &[u8]) -> Result<StorePurchaseResult, StoreWireError> {
    if input.len() > 64 {
        return Err(StoreWireError::LimitExceeded);
    }
    let mut code = None;
    let mut order = None;
    let mut operation = None;
    let mut at = 0;
    while at < input.len() {
        let tag = read_varint(input, &mut at)?;
        match tag {
            8 => crate::charm_wire::set_once(&mut code, read_varint(input, &mut at)?)?,
            18 => crate::charm_wire::set_once(
                &mut order,
                as_uuid(crate::charm_wire::read_bytes(input, &mut at)?)?,
            )?,
            26 => crate::charm_wire::set_once(
                &mut operation,
                as_uuid(crate::charm_wire::read_bytes(input, &mut at)?)?,
            )?,
            _ => return Err(StoreWireError::Malformed),
        }
    }
    let code = match code {
        Some(1) => StorePurchaseCode::Accepted,
        Some(2) => StorePurchaseCode::StaleQuote,
        Some(3) => StorePurchaseCode::OfferUnavailable,
        Some(4) => StorePurchaseCode::PurchaseDisabled,
        Some(5) => StorePurchaseCode::InsufficientBalance,
        Some(6) => StorePurchaseCode::AccountRestricted,
        _ => return Err(StoreWireError::Malformed),
    };
    let result = StorePurchaseResult {
        code,
        order_id: order,
        operation_id: operation,
    };
    encode_purchase_result(&result)?;
    Ok(result)
}
