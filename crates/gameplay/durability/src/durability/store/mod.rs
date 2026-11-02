//! Store purchase retry evidence and Platform-owned delivery receipts.
//!
//! These journals are not a purchase or coin ledger. Platform owns both.
mod codec;
mod outbox;
mod prey_expansion;
mod prey_purchase;
mod purchase;
mod storage;
pub use codec::{DeliveryPlan, DeliveryReceipt, PurchaseRequest, StoreValidationError};
pub(crate) use prey_expansion::write_prey_expansion_receipt;
pub use storage::{PurchaseRequestState, StoreDurabilityError};
#[cfg(test)]
mod tests;
