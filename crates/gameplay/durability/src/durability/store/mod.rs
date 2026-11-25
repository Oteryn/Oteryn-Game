//! Store purchase retry evidence and Platform-owned delivery receipts.
//!
//! These journals are not a purchase or coin ledger. Platform owns both.
mod blessing_delivery;
mod charm_expansion;
mod codec;
mod cosmetic_tokens;
mod hireling_capacity;
mod hireling_delivery;
mod hireling_model;
mod hireling_reservation;
mod outbox;
mod prey_expansion;
mod prey_purchase;
mod purchase;
mod purchase_history;
mod storage;
pub use blessing_delivery::StoreBlessingDelivery;
pub use codec::{
    DeliveryIdentity, DeliveryPlan, DeliveryReceipt, PurchaseRequest, StoreValidationError,
};
pub use cosmetic_tokens::{PaidStoreCosmeticIntent, PendingStorePurchase};
pub use hireling_model::{
    HirelingServiceDeliveryPlan, HirelingStoreAction, HirelingStorePurchase,
    canonical_hireling_name,
};
pub(super) use outbox::store_claim_source_in_transaction;
pub(crate) use prey_expansion::write_prey_expansion_receipt;
pub use purchase_history::{StorePurchaseHistoryEntry, StorePurchaseHistoryRow};
pub use storage::{PurchaseRequestState, StoreDurabilityError};

/// The retained receipt with the native item ID that proves the Store claim.
pub type StoreClaimSource = (DeliveryReceipt, [u8; 16]);

mod popularity;
#[cfg(test)]
mod tests;
