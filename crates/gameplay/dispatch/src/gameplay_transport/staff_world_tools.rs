//! Current-session staff world effects, with exact runtime fencing and bounded receipts.
use super::super::*;
use crate::durability::staff::{StaffLiveStatus, StaffWorldEffectState, rules};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Clone, Copy)]
struct StaffActorProof {
    actor: ExactActorRef,
    session: GameSessionId,
    control_revision: u64,
}
impl StaffActorProof {
    fn current(self, runtime: &ChannelRuntimeV1) -> bool {
        runtime
            .bound_attacker_lease(self.actor, self.session)
            .is_ok()
            && runtime.player_control_revision(self.actor, self.session)
                == Ok(self.control_revision)
            && runtime
                .player_control_facts(self.actor, self.session)
                .is_ok_and(|f| f.control_loss.is_none())
    }
}

#[derive(Default)]
pub(in crate::gameplay_transport) struct WorldTools {
    statuses: Mutex<HashMap<GameSessionId, (ExactActorRef, StaffLiveStatus)>>,
    // A live effect whose durable ACK failed must never run a second time on retry.
    receipts: Mutex<HashMap<String, bool>>,
}
impl WorldTools {
    pub(in crate::gameplay_transport) fn forget(&self, session: GameSessionId) {
        if let Ok(mut states) = self.statuses.lock() {
            states.remove(&session);
        }
    }
    fn set(&self, actor: ExactActorRef, session: GameSessionId, status: StaffLiveStatus) -> bool {
        let Ok(mut states) = self.statuses.lock() else {
            return false;
        };
        if status.group < 4 {
            // Ordinary players consume no staff cache slot; absence means no privilege.
            states.remove(&session);
            return true;
        }
        if states.len() >= 256 && !states.contains_key(&session) {
            return false;
        }
        states.insert(session, (actor, status));
        true
    }
    pub(in crate::gameplay_transport) fn invisible(&self, actor: ExactActorRef) -> bool {
        self.statuses
            .lock()
            .ok()
            .is_some_and(|s| s.values().any(|(a, f)| *a == actor && f.invisible))
    }
    pub(in crate::gameplay_transport) fn group(&self, actor: ExactActorRef) -> u8 {
        self.statuses
            .lock()
            .ok()
            .and_then(|s| {
                s.values()
                    .find_map(|(a, f)| (*a == actor).then_some(f.group))
            })
            .unwrap_or(0)
    }
    pub(in crate::gameplay_transport) fn filter(
        &self,
        observer: ExactActorRef,
        view: &mut world_spatial::ChannelEntities,
    ) {
        let Ok(states) = self.statuses.lock() else {
            view.entities
                .retain(|e| e.identity == observer.placement_identity());
            return;
        };
        let observer_group = states
            .values()
            .find_map(|(a, f)| (*a == observer).then_some(f.group))
            .unwrap_or(0);
        let ghosts: HashMap<_, _> = states
            .values()
            .filter(|(_, f)| f.ghost)
            .map(|(actor, flags)| {
                (
                    (actor.placement_identity(), actor.actor_local_generation()),
                    flags.group,
                )
            })
            .collect();
        view.entities.retain(|e| {
            e.identity == observer.placement_identity()
                || ghosts
                    .get(&(e.identity, e.generation))
                    .is_none_or(|_| observer_group >= 4)
        });
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn refresh_staff(
        &self,
        admitted: connection::AdmittedSession,
    ) -> bool {
        let Some(actor) = admitted.runtime_actor else {
            return true;
        };
        {
            let runtime = self.runtime.lock().await;
            if !runtime
                .player_control_facts(actor, admitted.game_session_id)
                .or_else(|_| runtime.house_player_control_facts(actor, admitted.game_session_id))
                .is_ok_and(|f| f.control_loss.is_none())
            {
                return false;
            }
        }
        // Enrichment gets one attempt within the existing DB budget. Missing
        // facts or a transient read failure retain visibility until the next refresh.
        let deadline = tokio::time::Instant::now() + crate::durability::DB_PASS_DEADLINE;
        // Share the gameplay lane: a background refresh must not take the
        // durability holder halfway through another session's trade command.
        // Leave a parked commit to its owner rather than resolving it here.
        let Ok(Ok(permit)) = tokio::time::timeout_at(deadline, self.spell_lane.acquire()).await
        else {
            return true;
        };
        let Ok(Some(fence)) = tokio::time::timeout_at(
            deadline,
            super::super::character_gameplay_fence::current(self, admitted.game_session_id, false),
        )
        .await
        else {
            return true;
        };
        if fence.connection_generation.get() != admitted.continuity.connection_generation {
            return false;
        }
        let Ok(Ok(status)) = tokio::time::timeout_at(
            deadline,
            self.root
                .staff_live_status(self.character, self.holder, fence.item_fence()),
        )
        .await
        else {
            return true;
        };
        {
            let mut runtime = self.runtime.lock().await;
            // Re-prove control after the asynchronous reads before changing flags.
            if !runtime
                .player_control_facts(actor, admitted.game_session_id)
                .or_else(|_| runtime.house_player_control_facts(actor, admitted.game_session_id))
                .is_ok_and(|f| f.control_loss.is_none())
            {
                return false;
            }
            let ghost = status.ghost && status.group >= 4;
            let invisible = status.invisible && status.group >= 4;
            // A House resident retains its native carrier slot but generic
            // Channel writes are closed. Unchanged flags need no writer.
            let unchanged = runtime
                .actor_staff_visibility(actor)
                .or_else(|_| runtime.house_staff_visibility(actor, admitted.game_session_id))
                .is_ok_and(|(old_ghost, old_invisible, _)| {
                    old_ghost == ghost && old_invisible == invisible
                });
            if !unchanged
                && runtime
                    .set_staff_visibility(actor, admitted.game_session_id, ghost, invisible)
                    .is_err()
            {
                return false;
            }
        }
        // Terminal release acquires its own gameplay lane.
        drop(permit);
        if status.banned || status.kicked {
            let _ = self.release_staff_session(admitted).await;
            return false;
        }
        self.states
            .staff_world
            .set(actor, admitted.game_session_id, status)
    }
}

#[path = "staff_world_tools/apply.rs"]
mod apply;
#[path = "staff_world_tools/effects.rs"]
mod effects;
#[path = "staff_world_tools/lookup.rs"]
mod lookup;
#[cfg(test)]
#[path = "staff_world_tools/owner_flag_tests.rs"]
mod owner_flag_tests;
#[cfg(test)]
#[path = "staff_world_tools/tests.rs"]
mod tests;
