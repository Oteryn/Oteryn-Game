//! NPC-BEHAVIOUR0 §4–5: source-bound walking preparation for a native NPC actor.
//!
//! The Channel's budgeted NPC think must call this once per native think occurrence. This
//! module owns only the interval and deterministic proposal; the actor's position remains in
//! Movement. A proposal does not prove catalogue membership in the active runtime, current
//! field/trigger absence, or permission to commit a step. Those owner handoffs are not installed.

#![allow(
    dead_code,
    reason = "NPC walking preparation awaits the source pin and budgeted owner caller"
)]

use super::chat_intent::ChatRuntime;
use crate::ai_think::profile_schedule::decision_occurrence;
use crate::ai_think::targeting::sees;
use crate::ai_think::{CREATURE_THINK_INTERVAL_MILLIS, ThinkOccurrence};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, MovementLocalPosition, MovementPositionContext,
};
pub use oteryn_gameplay_npc::npc_actor::*;
use oteryn_simulation_determinism::GameplayDecisionRoot;

/// Ephemeral interval state for one exact NPC. `anchor` is the immutable, real initial
/// placement; this type never stores or replaces the actor's mutable current position.
#[derive(Debug)]
pub struct NpcWalkState {
    actor: ExactActorRef,
    key: String,
    context: MovementPositionContext,
    anchor: MovementLocalPosition,
    profile: WalkProfile,
    ticks_ms: u64,
    last_sequence: Option<u64>,
}

impl NpcWalkState {
    /// Construct at native placement, before the first move. Recreating this state from a
    /// later position would silently move the radius anchor and is therefore refused.
    pub fn new(
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        book: &NpcWalkBook,
    ) -> Result<Option<Self>, WalkError> {
        runtime
            .owner_fence()
            .map_err(|_| WalkError::NotCurrentNpc)?;
        let key = runtime
            .npc_key(actor)
            .ok_or(WalkError::NotCurrentNpc)?
            .to_owned();
        let Some(profile) = book.profile(&key) else {
            return Ok(None);
        };
        let snapshot = runtime
            .borrow_movement_position()
            .read(actor)
            .map_err(|_| WalkError::NotCurrentNpc)?;
        if snapshot.revision() != 1 {
            return Err(WalkError::InitialPlacementUnavailable);
        }
        Ok(Some(Self {
            actor,
            key,
            context: snapshot.context(),
            anchor: snapshot.position(),
            profile,
            ticks_ms: 0,
            last_sequence: None,
        }))
    }

    /// Consume one scheduled NPC think. Active thinks add exactly 1,000 ms, idle thinks do
    /// not accumulate time, and a real customer resets the interval. There is no wall-clock
    /// catch-up or alternate timer queue. The caller still owns native occurrence/timer and
    /// source admission; accepting their data here is not a proof of those authorities.
    pub fn prepare(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        chat: &ChatRuntime,
        occurrence: ThinkOccurrence,
        root: &GameplayDecisionRoot,
    ) -> Result<WalkThink, WalkError> {
        runtime
            .owner_fence()
            .map_err(|_| WalkError::NotCurrentNpc)?;
        if occurrence.actor != self.actor
            || runtime.pinned_movement_context() != self.context
            || runtime.npc_key(self.actor) != Some(self.key.as_str())
        {
            return Err(WalkError::NotCurrentNpc);
        }
        if let Some(last) = self.last_sequence {
            if occurrence.sequence < last {
                return Err(WalkError::OccurrenceSuperseded);
            }
            if occurrence.sequence == last {
                return Ok(WalkThink::Replay);
            }
        }
        let expected = runtime
            .borrow_movement_position()
            .read(self.actor)
            .map_err(|_| WalkError::NotCurrentNpc)?;
        let perceived = runtime
            .visible_entities()
            .players
            .iter()
            .any(|player| sees(player.position, expected.position()));
        // Read the supplied ChatRuntime under its actual lock, held through preparation.
        // The composing Channel owner must supply its own ChatRuntime: this type itself does
        // not certify a Channel binding. None means unavailable, never an empty queue.
        chat.with_npc_customer_state(self.actor, |talking| {
            let (next_ticks, result) = self.profile.prepare(
                occurrence,
                expected,
                self.anchor,
                self.ticks_ms,
                talking,
                perceived,
                CREATURE_THINK_INTERVAL_MILLIS,
                root,
                decision_occurrence(occurrence),
            )?;
            // A failed decision leaves both counters unchanged, allowing the same occurrence
            // to be retried deterministically. A successful preparation is never replayed.
            self.ticks_ms = next_ticks;
            self.last_sequence = Some(occurrence.sequence);
            Ok(result)
        })
        .ok_or(WalkError::ConversationUnavailable)?
    }
}

#[cfg(test)]
#[path = "npc_actor_tests.rs"]
mod tests;

#[cfg(test)]
use crate::content::{
    CollisionClass, NpcDataCatalogue, ProjectV2AuthoringProfileData, ProjectV2BehaviorAuthoring,
    ProjectV2CandidateField, ProjectV2CandidateValue, ProjectV2Declaration,
};
#[cfg(test)]
use crate::foundation::MovementFacing;

#[cfg(test)]
fn shuffled_directions(
    root: &GameplayDecisionRoot,
    occurrence: ThinkOccurrence,
) -> Result<[MovementFacing; 4], WalkError> {
    oteryn_gameplay_npc::npc_actor::shuffled_directions(root, decision_occurrence(occurrence))
}
