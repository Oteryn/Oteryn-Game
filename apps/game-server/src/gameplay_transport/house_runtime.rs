//! Closed House runtime boundary (HOUSE-RT-INBOX §1.5; CP D1062–D1065).
//!
//! This boundary precedes loading, reservation, source freeze and durable PREPARE.
//! The native handoff fixture can prepare entry, but that does not open the runtime
//! transfer gate. No actor owner, loaded interior or readiness proof is constructed
//! here. Registration and real transport/admission callers remain a CP slot.

use crate::house_scope::{
    HouseEntryRefusal, HouseEntryRequest, HouseHandoffError, HouseId, HousePrepareOutcome,
};

#[path = "house_runtime_loading.rs"]
mod loading;

/// Door entry stops before any source actor or durable handoff is changed.
///
/// Keep the native request borrowed: refusal must not consume an operation identity
/// or replace its source-session/lease facts. Even a permissive actor-local precheck
/// cannot authorize entry through the closed transfer gate.
pub(crate) fn prepare_house_entry(request: &HouseEntryRequest) -> HousePrepareOutcome {
    HousePrepareOutcome::Refused(loading::activation_refusal(&request.house))
}

/// Saved-house login is subject to the same gate as an interactive entry.
///
/// The saved HouseId is historical projection data. It cannot create a loaded
/// owner or be converted into current assignment/content/admission authority.
pub(crate) fn activate_house_for_login(house: &HouseId) -> Result<(), HouseEntryRefusal> {
    Err(loading::activation_refusal(house))
}

/// Leave, kick, access revalidation and disposition quiesce share the native exit
/// refusal. Callers must retain their source actor and pending work on refusal.
/// There is no alternate fallback or local success path.
pub(crate) fn exit_house_scope() -> Result<(), HouseHandoffError> {
    crate::house_scope::exit_into_channel()
}
