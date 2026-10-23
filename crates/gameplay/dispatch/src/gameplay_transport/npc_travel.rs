//! Private owner bookkeeping for TRAVEL0 §7. No route admission, fee commit, actor
//! relocation or placement capability is created by this state machine. Its caller owns one
//! slot per actor, drops queued steps at `start`, refuses walking/logout/transfer while held,
//! and continues combat. Only the independently current native owner may apply a completion.

#![allow(
    dead_code,
    reason = "Private accepted travel component; production owner integration remains a CP slot"
)]

#[cfg(test)]
use std::time::{Duration, Instant};

use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::npc_travel::{ConsumeReplay, PendingArrival, TravelOccurrence};
use crate::foundation::{CommandRef, RuntimeScopeRefV1, RuntimeWorkStamp, ScopeRuntimeFence};

/// Immutable confirmed server-side facts. Construction does NOT admit a route or prove
/// eligibility; the actual talk/content/quest/Premium/fee source must supply those checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenTravel {
    pub command: CommandRef,
    pub npc: String,
    pub route: String,
    pub fare: u64,
    pub arrival: PendingArrival,
}

/// A capture to compare with independent current owner/session/content/position facts.
/// These values are not themselves current authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TravelCapture {
    pub session: CurrentCharacterItemFence,
    pub content: crate::foundation::MovementPositionContext,
    pub position_revision: u64,
}

#[allow(unused_imports, reason = "preserve the former NPC module path")]
pub use oteryn_gameplay_npc::npc_travel::{FeeOutcome, HoldError, TravelStep};
pub type TravelHoldSlot = oteryn_gameplay_npc::npc_travel::TravelHoldSlot<FrozenTravel>;

impl oteryn_gameplay_npc::npc_travel::TravelBinding for FrozenTravel {
    type Capture = TravelCapture;
    type Occurrence = TravelOccurrence;
    type Replay = ConsumeReplay;

    fn occurrence(&self) -> TravelOccurrence {
        self.arrival.occurrence
    }

    fn valid_start(
        &self,
        captured: &TravelCapture,
        work: RuntimeWorkStamp,
        owner: &ScopeRuntimeFence,
    ) -> bool {
        !self.npc.is_empty()
            && !self.route.is_empty()
            && self.command.game_session_id() == captured.session.game_session_id
            && matches!(captured.session.runtime_scope,
                RuntimeScopeRefV1::Channel { world_id, .. } if world_id == self.arrival.world_id)
            && captured.session.character_lease_generation != 0
            && owner.is_current_for_scope(
                captured.session.runtime_scope,
                captured.session.scope_ownership_generation,
            )
            && owner.accepts_stamp(work)
    }

    fn valid_placement(
        &self,
        captured: &TravelCapture,
        owner: &ScopeRuntimeFence,
        work: RuntimeWorkStamp,
    ) -> bool {
        owner.is_current_for_scope(
            captured.session.runtime_scope,
            captured.session.scope_ownership_generation,
        ) && owner.accepts_stamp(work)
    }

    fn retry_consume(replay: ConsumeReplay) -> bool {
        match replay {
            ConsumeReplay::RetryConsume => true,
            ConsumeReplay::Complete => false,
        }
    }
}

#[cfg(test)]
#[path = "npc_travel_tests.rs"]
mod tests;
