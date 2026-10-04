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

/// HOUSERT0-RL-02: door transition latency target, p99, measured before activation
/// (HOUSE-RUNTIME-1); not a registered allocation bound.
pub const HOUSE_DOOR_TRANSITION_P99_MS: u64 = 500;

/// Actor-local facts read before a house entry is prepared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HouseEntryPrecheck {
    pub has_access: bool,
    pub house_open: bool,
    /// Fight, PZ or another combat lock.
    pub combat_locked: bool,
    /// Trade, an unresolved mutation or another pending operation.
    pub operation_pending: bool,
}

impl HouseEntryPrecheck {
    /// The first refusal in the §4.1 order, or `None` when the entry may be prepared.
    #[must_use]
    pub const fn evaluate(self) -> Option<HouseEntryRefusal> {
        if !self.has_access {
            Some(HouseEntryRefusal::NoAccess)
        } else if !self.house_open {
            Some(HouseEntryRefusal::HouseClosed)
        } else if self.combat_locked {
            Some(HouseEntryRefusal::InCombat)
        } else if self.operation_pending {
            Some(HouseEntryRefusal::Busy)
        } else {
            None
        }
    }
}

/// Leaving a house into a Channel scope, and the §4.3 fallback, await ADMIT-0.
pub const fn exit_into_channel() -> Result<(), HouseHandoffError> {
    Err(HouseHandoffError::ExitNotAdmitted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precheck_refuses_in_order() {
        let open = HouseEntryPrecheck {
            has_access: true,
            house_open: true,
            ..HouseEntryPrecheck::default()
        };
        assert_eq!(open.evaluate(), None);
        assert_eq!(
            HouseEntryPrecheck::default().evaluate(),
            Some(HouseEntryRefusal::NoAccess)
        );
        let closed = HouseEntryPrecheck {
            house_open: false,
            combat_locked: true,
            ..open
        };
        assert_eq!(closed.evaluate(), Some(HouseEntryRefusal::HouseClosed));
        let fighting = HouseEntryPrecheck {
            combat_locked: true,
            operation_pending: true,
            ..open
        };
        assert_eq!(fighting.evaluate(), Some(HouseEntryRefusal::InCombat));
        let busy = HouseEntryPrecheck {
            operation_pending: true,
            ..open
        };
        assert_eq!(busy.evaluate(), Some(HouseEntryRefusal::Busy));
    }

    #[test]
    fn exit_stays_refused() {
        assert!(matches!(
            exit_into_channel(),
            Err(HouseHandoffError::ExitNotAdmitted)
        ));
    }
}
