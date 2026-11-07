//! A source hazard selection projects only its original immutable child receipt.
use super::*;
use crate::durability::character_progression::CharacterProgressionError;
#[derive(Clone, Debug)]
pub(in crate::gameplay_transport) struct PendingNpcHazardEffect {
    pub actor: ExactActorRef,
    pub session: GameSessionId,
    fence: CurrentCharacterGameplayFence,
    request: NpcCosmeticRequest,
    before: u16,
}
impl ComposedFreshAdmission<'_, '_, '_> {
    async fn settle_npc_hazard(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        pending: &PendingNpcHazardEffect,
    ) -> Result<(), ()> {
        if !current(runtime, pending.actor, pending.fence) {
            return Err(());
        }
        let selected = self
            .root
            .read_npc_primal_hazard_child(self.character, pending.fence, pending.request.clone())
            .await
            .map_err(|_| ())?;
        if let Some(selected) = selected {
            let frame = super::super::hazard::qualified_source_frame(runtime, self.movement_cells)
                .ok_or(())?;
            runtime
                .install_hazard_world_frame(pending.actor, pending.session, frame)
                .map_err(|_| ())?;
            runtime
                .install_hazard_level(pending.actor, pending.session, selected)
                .map_err(|_| ())?;
        } else if runtime.hazard_level(pending.actor, pending.session) != Some(pending.before) {
            return Err(());
        }
        states
            .pending_npc_hazard_effects
            .retain(|entry| entry.actor != pending.actor || entry.session != pending.session);
        Ok(())
    }
    pub(super) async fn commit_npc_hazard_owned(
        &self,
        actor: ExactActorRef,
        call: NpcSourceCall<'_>,
        effects: &crate::durability::npc_cosmetics::NpcSourceEffects,
    ) -> Result<
        Option<crate::durability::npc_cosmetics::NpcCosmeticOutcome>,
        CharacterProgressionError,
    > {
        let unknown = || {
            CharacterProgressionError::Unavailable(
                crate::durability::DurabilityError::InvalidStoredState,
            )
        };
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if !current(&runtime, actor, call.fence)
            || !runtime
                .player_control_facts(actor, call.fence.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return Err(unknown());
        }
        let pending = if let Some(pending) = states
            .pending_npc_hazard_effects
            .iter()
            .find(|p| p.actor == actor && p.session == call.fence.game_session_id)
        {
            if pending.request.command != call.request.command
                || pending.request.npc != call.request.npc
                || pending.request.message != call.request.message
                || pending.request.topic != call.request.topic
            {
                return Err(unknown());
            }
            pending.clone()
        } else {
            if !states.spell_idle(&runtime, actor, call.fence.game_session_id) {
                return Err(unknown());
            }
            let pending = PendingNpcHazardEffect {
                actor,
                session: call.fence.game_session_id,
                fence: call.fence,
                request: call.request.clone(),
                before: runtime
                    .hazard_level(actor, call.fence.game_session_id)
                    .ok_or_else(unknown)?,
            };
            states.pending_npc_hazard_effects.push(pending.clone());
            pending
        };
        let result = self
            .root
            .commit_npc_source_with_effects(
                self.character,
                self.holder,
                call.fence,
                call.request,
                call.permit,
                call.native,
                call.premium,
                effects,
            )
            .await;
        self.settle_npc_hazard(&mut runtime, &mut states, &pending)
            .await
            .map_err(|_| unknown())?;
        result
    }
    pub(in crate::gameplay_transport) async fn resolve_pending_npc_hazard_effect(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<(), ()> {
        let pending = self
            .states
            .spell_states
            .lock()
            .await
            .pending_npc_hazard_effects
            .iter()
            .find(|pending| pending.actor == actor && pending.session == session)
            .cloned();
        let Some(pending) = pending else {
            return Ok(());
        };
        let permit = self.spell_lane_permit().await.ok_or(())?;
        permit
            .check_scope(pending.fence.runtime_scope)
            .map_err(|_| ())?;
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        self.settle_npc_hazard(&mut runtime, &mut states, &pending)
            .await
    }
}
