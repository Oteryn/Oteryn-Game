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
mod hunting_offers;
mod outbox;
pub use outbox::StoreClaimSource;
pub(in crate::durability) use outbox::claim_source_in_transaction;
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
pub use hunting_offers::{StoreHuntingDelivery, StoreHuntingOffer, StoreHuntingPurchase};
pub(crate) use prey_expansion::write_prey_expansion_receipt;
pub use purchase_history::{StorePurchaseHistoryEntry, StorePurchaseHistoryRow};
pub use storage::{PurchaseRequestState, StoreDurabilityError};
mod popularity;
#[cfg(test)]
mod tests;
