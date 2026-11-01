//! Store purchase retry evidence and Platform-owned delivery receipts.
//!
//! These journals are not a purchase or coin ledger. Platform owns both.
mod codec;
mod hireling_capacity;
mod hireling_delivery;
mod hireling_model;
mod hireling_reservation;
mod outbox;
mod purchase;
mod storage;
pub use codec::{DeliveryIdentity, DeliveryPlan, DeliveryReceipt, PurchaseRequest, StoreValidationError};
pub use hireling_model::{HirelingServiceDeliveryPlan, HirelingStoreAction, HirelingStorePurchase, canonical_hireling_name};
pub use storage::{PurchaseRequestState, StoreDurabilityError};
#[cfg(test)]
mod tests;
