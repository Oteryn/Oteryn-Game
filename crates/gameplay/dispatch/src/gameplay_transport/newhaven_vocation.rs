//! Newhaven portal choice composes the Character build, vitals and starter owners.
use super::actor_spell::TrainingSave;
use super::spell_character_facts::{CastFactsLoad, load_character_cast_facts};
use super::{ComposedFreshAdmission, connection::UseCommand};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildOccurrence, convert_vocation,
};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_protocol_oteryn::newhaven_vocation::NewhavenChoiceResult as Result;
use sha2::{Digest, Sha256};

macro_rules! authority_methods {
    () => {
        fn newhaven_vocation_choice(
            &self,
            _actor: $crate::foundation::ExactActorRef,
            _command: $crate::gameplay_transport::connection::UseCommand,
            _request: oteryn_protocol_oteryn::newhaven_vocation::NewhavenVocationRequest,
        ) -> impl std::future::Future<
            Output = Option<oteryn_protocol_oteryn::newhaven_vocation::NewhavenChoiceResult>,
        > {
            async {
                Some(oteryn_protocol_oteryn::newhaven_vocation::NewhavenChoiceResult::Rejected)
            }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn newhaven_vocation_choice(
            &self,
            actor: $crate::foundation::ExactActorRef,
            command: $crate::gameplay_transport::connection::UseCommand,
            request: oteryn_protocol_oteryn::newhaven_vocation::NewhavenVocationRequest,
        ) -> Option<oteryn_protocol_oteryn::newhaven_vocation::NewhavenChoiceResult> {
            self.choose_newhaven_vocation(actor, command, request.key()?)
                .await
        }
    };
}
pub(super) use composed_methods;

fn occurrence(command: UseCommand) -> Option<BuildOccurrence> {
    let digest = Sha256::new()
        .chain_update(b"oteryn:newhaven-vocation:v1")
        .chain_update(command.game_session_id.as_bytes())
        .chain_update(command.command_id.to_be_bytes())
        .finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 15) | 0x70;
    bytes[8] = (bytes[8] & 63) | 0x80;
    BuildOccurrence::from_bytes(bytes).ok()
}

impl ComposedFreshAdmission<'_, '_, '_> {
    async fn newhaven_choice_current(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        let runtime = self.runtime.lock().await;
        runtime.owner_fence().is_ok()
            && runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
            && runtime
                .read_actor_position(actor)
                .is_ok_and(|p| super::tutorial_session::tutorial_cell(p.position()))
    }

    pub(super) async fn choose_newhaven_vocation(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        selected: &str,
    ) -> Option<Result> {
        if !["sorcerer", "druid", "paladin", "knight", "monk"].contains(&selected)
            || command.command_id == 0
            || !self.tutorial_enabled().await
            || !self
                .newhaven_choice_current(actor, command.game_session_id)
                .await
        {
            return Some(Result::Rejected);
        }
        let Some(initial) = self.npc_vocation_fence(command.game_session_id).await else {
            return Some(Result::Rejected);
        };
        // Portal progress, rather than a client-supplied coordinate, proves entry into Newhaven.
        if !self
            .tutorial_progress(initial)
            .await
            .ok()
            .flatten()
            .is_some_and(|p| {
                p.progress.step == oteryn_gameplay_quests::tutorial::TutorialStep::Island
            })
        {
            return Some(Result::Rejected);
        }
        match self
            .save_spell_training(actor, command.game_session_id, true)
            .await
        {
            TrainingSave::Saved | TrainingSave::NotApplicable => {}
            TrainingSave::Unknown => return Some(Result::Unavailable),
            TrainingSave::FencedOut => return Some(Result::Rejected),
        }
        let Some(fence) = self.npc_vocation_fence(command.game_session_id).await else {
            return Some(Result::Rejected);
        };
        if !super::vocation_choice::same_authority(&initial, &fence) {
            return Some(Result::Rejected);
        }
        let formula = self
            .active_generation?
            .native_gameplay()?
            .training_formula()?;
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let key = occurrence(command)?;
        let receipt = match self
            .root
            .reconcile_character_build(self.character, key)
            .await
        {
            Ok(Some(receipt)) => receipt,
            Ok(None) => {
                let before = self
                    .root
                    .read_character_build_state(self.character, fence.character_id)
                    .await
                    .ok()?;
                if before.vocation() != "none" {
                    return Some(if before.vocation() == selected {
                        Result::AlreadyChosen
                    } else {
                        Result::Rejected
                    });
                }
                if !self
                    .newhaven_choice_current(actor, command.game_session_id)
                    .await
                    || !self
                        .save_first_vocation_vitals(actor, command.game_session_id, fence, &before)
                        .await
                {
                    return Some(Result::Unavailable);
                }
                let after = convert_vocation(formula, &before, selected).ok()?;
                let request = BuildChangeRequest {
                    occurrence: key,
                    cause: BuildCause::NewhavenVocationChoice,
                    before,
                    after,
                    pruned_stance: None,
                };
                match slot
                    .commit_build(
                        self.root,
                        self.character,
                        self.holder,
                        fence,
                        request,
                        formula,
                        Some(fence.expected_character_revision),
                    )
                    .await
                {
                    Ok(
                        BuildCommitOutcome::Committed(receipt)
                        | BuildCommitOutcome::AlreadyCommitted(receipt),
                    ) => receipt,
                    Err(_) => return Some(Result::Unavailable),
                }
            }
            Err(_) => return Some(Result::Unavailable),
        };
        if receipt.character_id != fence.character_id
            || receipt.cause != BuildCause::NewhavenVocationChoice
            || receipt.after.vocation() != selected
        {
            return Some(Result::Rejected);
        }
        let CastFactsLoad::Ready {
            facts,
            character_revision,
        } = load_character_cast_facts(self.root, self.character, fence.character_id).await
        else {
            return Some(Result::Unavailable);
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
        let current = self.npc_vocation_fence(command.game_session_id).await?;
        if !super::vocation_choice::same_authority(&fence, &current)
            || current.expected_character_revision != character_revision
            || build != receipt.after
            || !self
                .newhaven_choice_current(actor, command.game_session_id)
                .await
        {
            return Some(Result::Unavailable);
        }
        {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if states.is_dead(actor)
                || states.has_pending_spell_commit(actor, command.game_session_id)
            {
                return Some(Result::Unavailable);
            }
            let state = states.get(&runtime, actor, command.game_session_id)?;
            let vitals = state.vitals();
            let mut next = state.after_first_vocation(
                &receipt.before,
                &build,
                facts,
                formula,
                self.owner_now(),
            )?;
            next.restore_saved_vitals(vitals.health, vitals.mana);
            *states.get_mut(&runtime, actor, command.game_session_id)? = next;
            if !states.load_owned_build(&runtime, actor, command.game_session_id, &build)
                || !states.load_owned_stance(
                    &runtime,
                    actor,
                    command.game_session_id,
                    &stance,
                    self.spells,
                )
            {
                return Some(Result::Unavailable);
            }
        }
        drop(slot);
        if !self
            .retry_newhaven_starter(actor, command.game_session_id)
            .await
        {
            return Some(Result::Unavailable);
        }
        Some(Result::Chosen)
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Eligible login/portal retry; NoRoom deliberately leaves completion absent.
    pub(super) async fn retry_newhaven_starter(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        use crate::durability::starter_grant::NewhavenStarterOutcome as O;
        if !self.tutorial_enabled().await || !self.newhaven_choice_current(actor, session).await {
            return true;
        }
        let Some(fence) = self.npc_vocation_fence(session).await else {
            return false;
        };
        if !self
            .tutorial_progress(fence)
            .await
            .ok()
            .flatten()
            .is_some_and(|p| {
                p.progress.step == oteryn_gameplay_quests::tutorial::TutorialStep::Island
            })
        {
            return true;
        }
        let Ok(build) = self
            .root
            .read_character_build_state(self.character, fence.character_id)
            .await
        else {
            return false;
        };
        if build.vocation() == "none" {
            return true;
        }
        let Some(native) = self.active_generation.and_then(|a| a.native_gameplay()) else {
            return false;
        };
        let item = crate::durability::item_transfer::CurrentCharacterItemFence {
            character_id: fence.character_id,
            game_session_id: fence.game_session_id,
            connection_generation: fence.connection_generation,
            character_lease_generation: fence.character_lease_generation,
            runtime_scope: fence.runtime_scope,
            scope_ownership_generation: fence.scope_ownership_generation,
        };
        match self
            .root
            .grant_newhaven_starter_items(
                self.character,
                self.holder,
                &item,
                &super::item_ref_admission::NativeItemWeights(native),
                build.vocation(),
            )
            .await
        {
            Ok(O::Granted) => {
                self.states.tutorial_sessions.notice(
                    session,
                    oteryn_protocol_oteryn::tutorial_notices::TutorialNotice::StatusText(
                        "You received your starter equipment.".into(),
                    ),
                );
                true
            }
            Ok(O::AlreadyGranted | O::NoRoom | O::NotAdmitted) => true,
            Err(_) => false,
        }
    }
}
