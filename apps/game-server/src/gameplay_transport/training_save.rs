//! Actual actor-end/checkpoint D151 save. Unknown outcomes retain the original
//! occurrence in the player's existing state and reconcile before new attempts.
use super::super::{ComposedFreshAdmission, SecureIdentifiers};
use crate::durability::character_build::{BuildCommitOutcome, BuildFormula, BuildOccurrence};
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ExactActorRef, GameSessionId, GameSessionState, RuntimeScopeRefV1};
use crate::spell::mana_training::RetainedTrainingCheckpoint;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::gameplay_transport) enum TrainingSave {
    NotApplicable,
    Saved,
    FencedOut,
    Unknown,
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn save_spell_training(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        force: bool,
    ) -> TrainingSave {
        let formula = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .and_then(|native| native.training_formula());
        let Some(formula) = formula else {
            return TrainingSave::NotApplicable;
        };
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let Ok((current, _)) = store.current_session_at(session).await else {
            return TrainingSave::Unknown;
        };
        if current.session_state() == GameSessionState::Terminal {
            return TrainingSave::FencedOut;
        }
        let Ok(character) =
            crate::domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes())
        else {
            return TrainingSave::Unknown;
        };
        let Ok(root) = self
            .root
            .read_current_character(self.character, character)
            .await
        else {
            return TrainingSave::Unknown;
        };
        let current_fence = CurrentCharacterGameplayFence {
            character_id: character,
            game_session_id: session,
            connection_generation: current.current_connection_generation(),
            character_lease_generation: current.current_character_lease().generation(),
            runtime_scope: current.current_runtime_scope(),
            scope_ownership_generation: current.current_scope_generation(),
            expected_character_revision: root.revision,
        };
        let runtime = self.runtime.lock().await;
        let mut states = self.spell_states.lock().await;
        let binding = runtime.binding();
        if current_fence.runtime_scope
            != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
            || current_fence.scope_ownership_generation != binding.scope_generation()
            || runtime.player_control_facts(actor, session).is_err()
            || runtime.owner_fence().is_err()
        {
            return TrainingSave::FencedOut;
        }
        let Some(state) = states.get_mut(&runtime, actor, session) else {
            return TrainingSave::NotApplicable;
        };
        let retained = if let Some(retained) = state.pending_training_checkpoint() {
            if retained.formula_digest != formula.digest() {
                return TrainingSave::Unknown;
            }
            retained.clone()
        } else {
            let Some(occurrence) = SecureIdentifiers::random::<16>().and_then(|mut bytes| {
                bytes[6] = (bytes[6] & 15) | 0x70;
                bytes[8] = (bytes[8] & 63) | 0x80;
                BuildOccurrence::from_bytes(bytes).ok()
            }) else {
                return TrainingSave::Unknown;
            };
            let prepared = match state.prepare_training_checkpoint(
                self.owner_now().get(),
                formula,
                occurrence,
                force,
            ) {
                Ok(Some(prepared)) => prepared,
                Ok(None) => return TrainingSave::NotApplicable,
                Err(_) => return TrainingSave::Unknown,
            };
            let retained = RetainedTrainingCheckpoint {
                fence: current_fence,
                prepared,
                formula_digest: formula.digest(),
            };
            if state.retain_training_checkpoint(retained.clone()).is_err() {
                return TrainingSave::Unknown;
            }
            retained
        };
        // The Channel lock preserves this exact actor/training predecessor through
        // one bounded semantic pass; all current DB fences are independently read.
        let outcome = self
            .root
            .commit_character_build(
                self.character,
                self.holder,
                retained.fence.clone(),
                retained.prepared.request.clone(),
                formula,
            )
            .await;
        let receipt = match outcome {
            Ok(
                BuildCommitOutcome::Committed(receipt)
                | BuildCommitOutcome::AlreadyCommitted(receipt),
            ) => receipt,
            Err(CharacterProgressionError::CharacterRevisionMismatch) => {
                // This writer result proves rollback. Keep the same occurrence; only
                // unchanged build data plus an independently bracketed current root
                // permits refreshing the expected revision for a later retry.
                match self
                    .root
                    .reconcile_character_build(self.character, retained.prepared.request.occurrence)
                    .await
                {
                    Ok(Some(receipt)) => receipt,
                    Ok(None) => {
                        let Ok(before_root) = self
                            .root
                            .read_current_character(self.character, character)
                            .await
                        else {
                            return TrainingSave::Unknown;
                        };
                        let Ok(build) = self
                            .root
                            .read_character_build_state(self.character, character)
                            .await
                        else {
                            return TrainingSave::Unknown;
                        };
                        let Ok(after_root) = self
                            .root
                            .read_current_character(self.character, character)
                            .await
                        else {
                            return TrainingSave::Unknown;
                        };
                        let Ok((fresh, _)) = store.current_session_at(session).await else {
                            return TrainingSave::Unknown;
                        };
                        if build != retained.prepared.request.before
                            || before_root.revision != after_root.revision
                            || fresh.session_state() == GameSessionState::Terminal
                        {
                            return TrainingSave::Unknown;
                        }
                        let refreshed = CurrentCharacterGameplayFence {
                            character_id: character,
                            game_session_id: session,
                            connection_generation: fresh.current_connection_generation(),
                            character_lease_generation: fresh
                                .current_character_lease()
                                .generation(),
                            runtime_scope: fresh.current_runtime_scope(),
                            scope_ownership_generation: fresh.current_scope_generation(),
                            expected_character_revision: after_root.revision,
                        };
                        let _ = state.refresh_training_checkpoint_revision(&retained, refreshed);
                        return TrainingSave::Unknown;
                    }
                    Err(_) => return TrainingSave::Unknown,
                }
            }
            Err(
                CharacterProgressionError::AuthorityRejected
                | CharacterProgressionError::RespawnPending,
            ) => return TrainingSave::FencedOut,
            Err(_) => match self
                .root
                .reconcile_character_build(self.character, retained.prepared.request.occurrence)
                .await
            {
                Ok(Some(receipt)) => receipt,
                _ => return TrainingSave::Unknown,
            },
        };
        // A receipt is history. Resolve the independently current session again;
        // no persisted request supplies current connection/lease/scope authority.
        let Ok((fresh, _)) = store.current_session_at(session).await else {
            return TrainingSave::Unknown;
        };
        if fresh.session_state() == GameSessionState::Terminal
            || fresh.current_connection_generation() != retained.fence.connection_generation
            || fresh.current_character_lease().generation()
                != retained.fence.character_lease_generation
            || fresh.current_runtime_scope() != retained.fence.runtime_scope
            || fresh.current_scope_generation() != retained.fence.scope_ownership_generation
        {
            return TrainingSave::FencedOut;
        }
        if state
            .commit_training_checkpoint(retained.prepared, &receipt)
            .is_err()
        {
            return TrainingSave::Unknown;
        }
        TrainingSave::Saved
    }
}
