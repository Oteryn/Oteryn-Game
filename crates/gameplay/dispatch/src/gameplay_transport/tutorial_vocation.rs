//! Newhaven's first vocation choice, using the existing Character build receipt owner.
use super::vocation_choice::{
    RetainedNpcVocation, commit_retained_npc_vocation, same_authority, vocation_choice_request,
};
use super::{ComposedFreshAdmission, SecureIdentifiers, connection::AdmittedSession};
use crate::durability::character_build::{BuildFormula, BuildOccurrence, NO_VOCATION};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_gameplay_quests::tutorial::TutorialStep;
use oteryn_protocol_oteryn::tutorial_vocation as wire;

impl ComposedFreshAdmission<'_, '_, '_> {
    async fn tutorial_vocation_current(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        fence: &crate::durability::character_progression::CurrentCharacterGameplayFence,
    ) -> bool {
        if self.states.spell_states.lock().await.is_dead(actor) {
            return false;
        }
        let runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        fence.runtime_scope == binding.runtime_scope()
            && fence.scope_ownership_generation == binding.scope_generation()
            && runtime.owner_fence().is_ok()
            && runtime.player_control_facts(actor, session).is_ok()
            && runtime
                .read_actor_position(actor)
                .is_ok_and(|at| super::tutorial_session::tutorial_cell(at.position()))
    }

    pub(super) async fn tutorial_vocation_wire(
        &self,
        admitted: AdmittedSession,
        payload: &[u8],
    ) -> Option<Vec<u8>> {
        let choice = wire::decode_request(payload)?;
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_TUTORIAL_VOCATION_V1)
            || !admitted.continuity.selected_capabilities.contains(31)
            || admitted.controller.is_none()
        {
            return None;
        }
        let actor = admitted.runtime_actor?;
        let session = admitted.game_session_id;
        let initial = self.npc_vocation_fence(session).await?;
        if initial.connection_generation.get() != admitted.continuity.connection_generation
            || admitted.item_fence?.character_id != initial.character_id
        {
            return None;
        }
        if !self.tutorial_enabled().await {
            return if choice.is_none() {
                wire::Observation::default().encode()
            } else {
                None
            };
        }
        if !self
            .tutorial_vocation_current(actor, session, &initial)
            .await
        {
            return None;
        }
        let group = self
            .root
            .read_familiar_group(self.character, self.holder, initial)
            .await
            .ok()??;
        if group.access() {
            return wire::Observation::default().encode();
        }
        let stored = self.tutorial_progress(initial).await.ok()?;
        let progress = stored.is_some_and(|s| s.progress.step == TutorialStep::Island);
        let before = self
            .root
            .read_character_build_state(self.character, initial.character_id)
            .await
            .ok()?;
        let retained = self.states.tutorial_sessions.vocation(session).ok()?;
        if retained.is_none() {
            let chosen = wire::VOCATIONS
                .iter()
                .position(|key| *key == before.vocation())
                .map_or(0, |index| (index + 1) as u8);
            let offered = progress && before.vocation() == NO_VOCATION;
            if choice.is_none() {
                if chosen != 0 {
                    self.grant_newhaven_starter(initial.item_fence())
                        .await
                        .ok()?;
                }
                return wire::Observation { offered, chosen }.encode();
            }
            if !offered {
                return None;
            }
        }
        let _foreground = self.root.foreground_demand().ok()?;
        let formula = self
            .active_generation?
            .native_gameplay()?
            .training_formula()?;
        if retained.is_none()
            && !matches!(
                self.save_spell_training(actor, session, true).await,
                super::actor_spell::TrainingSave::Saved
                    | super::actor_spell::TrainingSave::NotApplicable
            )
        {
            return None;
        }
        let fence = self.npc_vocation_fence(session).await?;
        if !same_authority(&initial, &fence) {
            return None;
        }
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let retained = if let Some(retained) = retained {
            if !same_authority(&retained.fence, &fence)
                || choice.is_some_and(|id| {
                    retained.request.after.vocation() != wire::VOCATIONS[usize::from(id - 1)]
                })
            {
                return None;
            }
            retained
        } else {
            let before = self
                .root
                .read_character_build_state(self.character, fence.character_id)
                .await
                .ok()?;
            if before.vocation() != NO_VOCATION
                || !self.tutorial_vocation_current(actor, session, &fence).await
            {
                return None;
            }
            {
                let runtime = self.runtime.lock().await;
                let states = self.states.spell_states.try_lock().ok()?;
                if states.is_dead(actor)
                    || states.has_pending_spell_commit(actor, session)
                    || states
                        .get(&runtime, actor, session)
                        .is_none_or(|state| state.has_training_to_flush())
                {
                    return None;
                }
            }
            let mut bytes = SecureIdentifiers::random::<16>()?;
            bytes[6] = (bytes[6] & 15) | 0x70;
            bytes[8] = (bytes[8] & 63) | 0x80;
            let occurrence = BuildOccurrence::from_bytes(bytes).ok()?;
            let request = vocation_choice_request(
                occurrence,
                before,
                wire::VOCATIONS[usize::from(choice? - 1)],
                formula,
                None,
            )
            .ok()?;
            if !self
                .save_first_vocation_vitals(actor, session, fence, &request.before)
                .await
            {
                return None;
            }
            let retained = RetainedNpcVocation {
                fence,
                request,
                formula_digest: formula.digest(),
            };
            if !self
                .states
                .tutorial_sessions
                .retain_vocation(session, retained.clone())
            {
                return None;
            }
            retained
        };
        if let Err(error) = commit_retained_npc_vocation(
            &mut slot,
            self.root,
            self.character,
            self.holder,
            &retained,
            formula,
        )
        .await
        {
            if matches!(error, crate::durability::character_progression::CharacterProgressionError::CharacterRevisionMismatch) {
                self.states.tutorial_sessions.clear_vocation(session, retained.request.occurrence);
            }
            return None;
        }
        use super::spell_character_facts::{CastFactsLoad, load_character_cast_facts};
        let CastFactsLoad::Ready {
            facts,
            character_revision,
        } = load_character_cast_facts(self.root, self.character, fence.character_id).await
        else {
            return None;
        };
        let build = self
            .root
            .read_character_build_state(self.character, fence.character_id)
            .await
            .ok()?;
        let stance = self
            .root
            .read_character_stance(self.character, fence.character_id)
            .await
            .ok()?;
        let current = self.npc_vocation_fence(session).await?;
        if !same_authority(&retained.fence, &current)
            || current.expected_character_revision != character_revision
            || build != retained.request.after
        {
            return None;
        }
        {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.try_lock().ok()?;
            if runtime.owner_fence().is_err()
                || runtime.player_control_facts(actor, session).is_err()
                || states.has_pending_spell_commit(actor, session)
            {
                return None;
            }
            let state = states.get(&runtime, actor, session)?;
            let next = state.after_first_vocation(
                &retained.request.before,
                &build,
                facts,
                formula,
                self.owner_now(),
            )?;
            *states.get_mut(&runtime, actor, session)? = next;
            if !states.load_owned_build(&runtime, actor, session, &build)
                || !states.load_owned_stance(&runtime, actor, session, &stance, self.spells)
                || states
                    .get_mut(&runtime, actor, session)?
                    .enable_owned_training(&build, formula)
                    .is_err()
            {
                return None;
            }
        }
        drop(slot);
        self.states
            .tutorial_sessions
            .clear_vocation(session, retained.request.occurrence);
        self.grant_newhaven_starter(current.item_fence())
            .await
            .ok()?;
        let chosen = wire::VOCATIONS
            .iter()
            .position(|key| *key == build.vocation())? as u8
            + 1;
        wire::Observation {
            offered: false,
            chosen,
        }
        .encode()
    }
}
macro_rules! authority_methods {
    () => {
        fn tutorial_vocation_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _payload: &[u8],
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn tutorial_vocation_command(
            &self,
            admitted: connection::AdmittedSession,
            payload: &[u8],
        ) -> Option<Vec<u8>> {
            self.tutorial_vocation_wire(admitted, payload).await
        }
    };
}
pub(super) use composed_methods;
