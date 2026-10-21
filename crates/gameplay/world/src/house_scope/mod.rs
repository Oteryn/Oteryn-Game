//! House scope entry rules (HOUSE-RUNTIME-0 §4.1-§4.3, SCOPE-HANDOFF-1).
//!
//! The durable scope and handoff live in `crate::durability::house_scope_handoff`; this module adds the pure entry checks that run
//! before a handoff is prepared. Actor wiring is HOUSE-RUNTIME-1.

pub mod handoff;

pub use crate::durability::house_scope_handoff::{
    HOUSE_SCOPE_CHARACTERS_MAX, HouseAbortOutcome, HouseCommitOutcome, HouseEntryCommit,
    HouseEntryRefusal, HouseEntryRequest, HouseHandoffDirection, HouseHandoffError,
    HouseHandoffRecord, HouseHandoffState, HouseId, HousePrepareOutcome, HouseReconcileReport,
    HouseRecoveryReport, HouseReleaseOutcome,
};

pub use oteryn_gameplay_housing::house_scope::{HOUSE_DOOR_TRANSITION_P99_MS, HouseEntryPrecheck};

/// Leaving a house into a Channel scope, and the §4.3 fallback, await ADMIT-0.
pub const fn exit_into_channel() -> Result<(), HouseHandoffError> {
    Err(HouseHandoffError::ExitNotAdmitted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_stays_refused() {
        assert!(matches!(
            exit_into_channel(),
            Err(HouseHandoffError::ExitNotAdmitted)
        ));
    }
}
