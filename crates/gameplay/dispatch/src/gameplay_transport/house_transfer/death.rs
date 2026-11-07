//! A genuinely dead named-House resident exits through the existing transfer command.
//! The ordinary CharacterDeath writer owns loss, blessings and temple; this leaf
//! only moves the already settled player across the genuine House/Channel barrier.
use super::*;
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::{SessionDeath, actor_spell};
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn transfer_house_after_death(
        &self,
        admitted: AdmittedSession,
        command: u64,
        body: &[u8],
        death: actor_spell::PlayerDeath,
    ) -> TransferResult {
        let actor = admitted.runtime_actor?;
        let fence = admitted.item_fence?;
        {
            let runtime = self.runtime.lock().await;
            let house = runtime
                .house_actor_state(actor, admitted.game_session_id)
                .ok()?;
            if house.scope != fence.runtime_scope
                || house.generation != fence.scope_ownership_generation
                || house.lease.character_id().as_bytes() != fence.character_id.as_bytes()
                || house.lease.generation() != fence.character_lease_generation
            {
                return None;
            }
            let states = self.states.spell_states.lock().await;
            let held = states.player_death(&runtime, actor, admitted.game_session_id)?;
            if held.occurrence != death.occurrence || held.cell != death.cell {
                return None;
            }
        }

        let (settled, lowered) = self
            .settle_death(actor, admitted.game_session_id, death, false)
            .await;

        if settled != SessionDeath::Durable {
            return None;
        }
        let (next, payload) = self
            .transfer_house_inner(admitted, command, body, Some(death))
            .await?;
        let Some(next) = next else {
            return Some((None, payload));
        };
        if !self
            .place_and_consume_settled_respawn(actor, next.game_session_id, death.occurrence)
            .await
        {
            return None;
        }
        {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if let Some(after) = lowered.as_ref() {
                states
                    .get_mut(&runtime, actor, next.game_session_id)?
                    .rebase_after_death(after);
            }
            states.respawn(
                &runtime,
                actor,
                next.game_session_id,
                death.occurrence,
                self.owner_now(),
            )?;
        }
        Some((Some(next), payload))
    }
    pub(in crate::gameplay_transport) async fn house_death_training_clean(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        death: actor_spell::PlayerDeath,
    ) -> bool {
        if !self
            .prepare_house_transfer_checkpoints(actor, session)
            .await
        {
            return false;
        }
        let runtime = self.runtime.lock().await;
        let Ok(house) = runtime.house_actor_state(actor, session) else {
            return false;
        };
        if !runtime
            .house_player_control_facts(actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return false;
        }
        let states = self.states.spell_states.lock().await;
        let Some(held) = states.player_death(&runtime, actor, session) else {
            return false;
        };
        held.occurrence == death.occurrence
            && held.cell == death.cell
            && !states.has_pending_spell_commit(actor, session)
            && states
                .actors
                .iter()
                .find(|(a, s, _)| *a == actor && *s == session)
                .is_some_and(|(_, _, state)| state.health == 0 && !state.has_training_to_flush())
            && house.scope.world_id() == self.world_id
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// A retained genuine HouseExit can outlive its result. Resume the ordinary
    /// pending respawn before exposing its already committed Channel successor.
    pub(super) async fn finish_recovered_house_death(
        &self,
        admitted: &AdmittedSession,
    ) -> Option<()> {
        let actor = admitted.runtime_actor?;
        let fence = self
            .current_quest_fence(admitted.game_session_id)
            .await
            .ok()??;
        let Some(pending) = self
            .root
            .pending_respawn(self.character, self.holder, fence)
            .await
            .ok()?
        else {
            return Some(());
        };
        let committed = self
            .root
            .reconcile_character_death(self.character, pending.occurrence)
            .await
            .ok()??;
        if !matches!(
            committed.death_cell.runtime_scope,
            crate::foundation::RuntimeScopeRefV1::Instance { .. }
        ) || committed.respawn_position != pending.respawn_position
        {
            return None;
        }
        let temple = super::super::respawn_cell(&committed.respawn_position)?;
        if self
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .ok()?
            .position()
            != temple
        {
            return None;
        }
        if !self
            .place_and_consume_settled_respawn(actor, admitted.game_session_id, pending.occurrence)
            .await
        {
            return None;
        }
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if let Some(loss) = committed.build_loss.as_ref() {
            states
                .get_mut(&runtime, actor, admitted.game_session_id)?
                .rebase_after_death(&loss.after);
        }
        if states
            .player_death(&runtime, actor, admitted.game_session_id)
            .is_some()
        {
            states.respawn(
                &runtime,
                actor,
                admitted.game_session_id,
                pending.occurrence,
                self.owner_now(),
            )?;
        } else {
            let next = states
                .get(&runtime, actor, admitted.game_session_id)?
                .respawned(self.owner_now())?;
            if !states.commit(&runtime, actor, admitted.game_session_id, next) {
                return None;
            }
        }
        Some(())
    }
}
