//! Store purchase retry evidence and Platform-owned delivery receipts.
//!
//! These journals are not a purchase or coin ledger. Platform owns both.
mod codec;
mod outbox;
mod purchase;
mod storage;
pub use codec::{DeliveryPlan, DeliveryReceipt, PurchaseRequest, StoreValidationError};
pub use storage::{PurchaseRequestState, StoreDurabilityError};
#[cfg(test)]
mod tests;
