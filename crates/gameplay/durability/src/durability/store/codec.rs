use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
pub const MAX_ANSWER_BYTES: usize = 8_192;
pub const MAX_RECEIPT_BYTES: usize = 4_096;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreValidationError {
    InvalidUuid,
    InvalidText,
    InvalidQuantity,
    InvalidBytes,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseRequest {
    pub purchase_request_id: [u8; 16],
    pub account_id: [u8; 16],
    pub character_id: [u8; 16],
    pub binding: Vec<u8>,
}
impl PurchaseRequest {
    pub fn validate(&self) -> Result<(), StoreValidationError> {
        if !uuid_v7(&self.purchase_request_id)
            || !uuid_v7(&self.account_id)
            || !uuid_v7(&self.character_id)
        {
            return Err(StoreValidationError::InvalidUuid);
        }
        if self.binding.is_empty() || self.binding.len() > 4096 {
            return Err(StoreValidationError::InvalidBytes);
        }
        Ok(())
    }
    #[must_use]
    pub fn binding_digest(&self) -> [u8; 32] {
        Sha256::digest(&self.binding).into()
    }
}
/// The immutable identity shared by item and Hireling service delivery receipts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryIdentity {
    pub account_id: [u8; 16],
    pub character_id: [u8; 16],
    pub world_id: [u8; 16],
    pub fulfilment_unit_id: String,
}
impl DeliveryIdentity {
    pub fn validate(&self) -> Result<(), StoreValidationError> {
        if !uuid_v7(&self.account_id) || !uuid_v7(&self.character_id) || !uuid_v7(&self.world_id) { return Err(StoreValidationError::InvalidUuid); }
        if self.fulfilment_unit_id.is_empty() || self.fulfilment_unit_id.len() > 512 || self.fulfilment_unit_id.chars().any(char::is_control) { return Err(StoreValidationError::InvalidText); }
        Ok(())
    }
}

/// Server-resolved output facts consumed by the existing native Inbox mint owner.
/// The command and output identity stay frozen within one native attempt. A new Hireling wire
/// command retains the Store operation/item identity and creates a separate native attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryPlan {
    pub account_id: [u8; 16],
    pub character_id: [u8; 16],
    pub world_id: [u8; 16],
    pub fulfilment_unit_id: String,
    pub item_instance_id: [u8; 16],
    pub command: crate::foundation::CommandRef,
    pub mint: crate::durability::character_inbox::RewardClaimInboxMintRequest,
}
impl DeliveryPlan {
    pub(super) fn identity(&self) -> DeliveryIdentity {
        DeliveryIdentity { account_id: self.account_id, character_id: self.character_id, world_id: self.world_id, fulfilment_unit_id: self.fulfilment_unit_id.clone() }
    }
    pub fn validate(&self) -> Result<(), StoreValidationError> {
        if !uuid_v7(&self.account_id)
            || !uuid_v7(&self.character_id)
            || !uuid_v7(&self.world_id)
            || !uuid_v7(&self.item_instance_id)
        {
            return Err(StoreValidationError::InvalidUuid);
        }
        if self.fulfilment_unit_id.is_empty()
            || self.fulfilment_unit_id.len() > 512
            || self.fulfilment_unit_id.chars().any(char::is_control)
        {
            return Err(StoreValidationError::InvalidText);
        }
        if self.mint.outputs.len() != 1 || self.mint.outputs[0].quantity == 0 {
            return Err(StoreValidationError::InvalidQuantity);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryReceipt {
    pub operation_id: String,
    pub account_id: String,
    pub character_id: String,
    pub world_id: String,
    pub fulfilment_unit_id: String,
    pub result: String,
    pub recovery_generation: String,
    pub decided_at: String,
}
pub(super) fn encode_receipt(r: &DeliveryReceipt) -> Result<Vec<u8>, StoreValidationError> {
    let b = serde_json::to_vec(r).map_err(|_| StoreValidationError::InvalidBytes)?;
    if b.is_empty() || b.len() > MAX_RECEIPT_BYTES {
        Err(StoreValidationError::InvalidBytes)
    } else {
        Ok(b)
    }
}
pub(super) fn uuid_v7(v: &[u8; 16]) -> bool {
    v[6] >> 4 == 7 && v[8] & 0xc0 == 0x80
}
pub(super) fn uuid_string(v: &[u8; 16]) -> String {
    let h: String = v.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    )
}
