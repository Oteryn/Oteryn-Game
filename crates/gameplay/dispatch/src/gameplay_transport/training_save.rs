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
        // A retained receipt may leave spending accumulated after its unknown outcome.
        // Drain that successor before an actor-end save can report success.
        for _ in 0..2 {
            match self.save_hunting_stamina(actor, session).await {
                TrainingSave::Unknown => return TrainingSave::Unknown,
                TrainingSave::FencedOut => return TrainingSave::FencedOut,
                TrainingSave::NotApplicable => break,
                TrainingSave::Saved => {}
            }
        }
        if force
            && self
                .states
                .spell_states
                .lock()
                .await
                .stamina_to_flush(actor, session)
        {
            return TrainingSave::Unknown;
        }
        if !self.training_before_save(actor, session, force).await {
            return TrainingSave::Unknown;
        }
        let saved = self.save_spell_training_once(actor, session, force).await;
        if saved != TrainingSave::Saved
            || !self
                .spell_training_successor_due(actor, session, force)
                .await
        {
            return saved;
        }
        // A replayed receipt merged the hits resolved while it was retained. A level they
        // reach is written now, never left to the cadence (SKILLS-0 §3.4); a forced save
        // writes all of them.
        match self.save_spell_training_once(actor, session, force).await {
            TrainingSave::NotApplicable => TrainingSave::Saved,
            other => other,
        }
    }
    async fn spell_training_successor_due(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        force: bool,
    ) -> bool {
        let Some(formula) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .and_then(|native| native.training_formula())
        else {
            return false;
        };
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        states
            .get_mut(&runtime, actor, session)
            .is_some_and(|state| state.training_successor_due(formula, force))
    }
    async fn save_spell_training_once(
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
        // CHAR-REV-SEQ-1: this Character's revision writer, held across the save and its
        // reconciliation and taken before the runtime lock.
        let mut slot = self.revision_sequencer.acquire(character).await;
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
        let mut states = self.states.spell_states.lock().await;
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
        // A retained checkpoint replays at the exact revision it was prepared for.
        let outcome = slot
            .commit_build(
                self.root,
                self.character,
                self.holder,
                retained.fence,
                retained.prepared.request.clone(),
                formula,
                Some(retained.fence.expected_character_revision),
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
                        let Some(refreshed) = self
                            .uncommitted_checkpoint_fence(&store, session, character, &retained)
                            .await
                        else {
                            return TrainingSave::Unknown;
                        };
                        let _ = state.refresh_training_checkpoint_revision(&retained, refreshed);
                        return TrainingSave::Unknown;
                    }
                    Err(_) => return TrainingSave::Unknown,
                }
            }
            Err(CharacterProgressionError::RespawnPending) => return TrainingSave::FencedOut,
            Err(CharacterProgressionError::AuthorityRejected) => {
                // #2193 Codex P1 4307264455: the store refused the retained fence. Without a
                // receipt for its occurrence nothing committed, and replaying the same immutable
                // fence would be refused for ever, so a proven-uncommitted checkpoint is
                // re-fenced to the session's independently resolved current authority (same
                // occurrence, request and formula) and the next save retries under it. A
                // receipt is history and takes the post-write authority check below.
                match self
                    .root
                    .reconcile_character_build(self.character, retained.prepared.request.occurrence)
                    .await
                {
                    Ok(Some(receipt)) => receipt,
                    Ok(None) => {
                        let Some(current) = self
                            .uncommitted_checkpoint_fence(&store, session, character, &retained)
                            .await
                        else {
                            return TrainingSave::FencedOut;
                        };
                        if current.runtime_scope
                            != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
                            || current.scope_ownership_generation != binding.scope_generation()
                            || state
                                .refence_uncommitted_training_checkpoint(&retained, current)
                                .is_err()
                        {
                            return TrainingSave::FencedOut;
                        }
                        return TrainingSave::Unknown;
                    }
                    Err(_) => return TrainingSave::Unknown,
                }
            }
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
    /// The current gameplay fence for a retained checkpoint the writer proved uncommitted (a
    /// rollback, or a refusal with no receipt for its occurrence): its build is still the
    /// durable build, the Character revision is bracketed unchanged around that read, and the
    /// session is independently resolved and not terminal. `None` on any failed read or a
    /// moved build, revision or session.
    async fn uncommitted_checkpoint_fence(
        &self,
        store: &FreshAdmissionStore,
        session: GameSessionId,
        character: crate::domain::CharacterId,
        retained: &RetainedTrainingCheckpoint,
    ) -> Option<CurrentCharacterGameplayFence> {
        let before_root = self
            .root
            .read_current_character(self.character, character)
            .await
            .ok()?;
        let build = self
            .root
            .read_character_build_state(self.character, character)
            .await
            .ok()?;
        let after_root = self
            .root
            .read_current_character(self.character, character)
            .await
            .ok()?;
        let (fresh, _) = store.current_session_at(session).await.ok()?;
        if build != retained.prepared.request.before
            || before_root.revision != after_root.revision
            || fresh.session_state() == GameSessionState::Terminal
        {
            return None;
        }
        Some(CurrentCharacterGameplayFence {
            character_id: character,
            game_session_id: session,
            connection_generation: fresh.current_connection_generation(),
            character_lease_generation: fresh.current_character_lease().generation(),
            runtime_scope: fresh.current_runtime_scope(),
            scope_ownership_generation: fresh.current_scope_generation(),
            expected_character_revision: after_root.revision,
        })
    }
}
