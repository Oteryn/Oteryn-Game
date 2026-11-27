//! Actual actor-end/checkpoint D151 save. Unknown outcomes retain the original
//! occurrence in the player's existing state and reconcile before new attempts.
use super::super::{ComposedFreshAdmission, SecureIdentifiers};
use crate::durability::DurabilityError;
use crate::durability::character_authority::CharacterAuthorityError;
use crate::durability::character_build::{BuildCommitOutcome, BuildFormula, BuildOccurrence};
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ExactActorRef, GameSessionId, GameSessionState};
use crate::gameplay_transport::AdmissionRetryBudget;
use crate::spell::mana_training::RetainedTrainingCheckpoint;
#[cfg(test)]
tokio::task_local! {
    pub(in crate::gameplay_transport) static TRAINING_READBACK_BARRIER:
        std::sync::Arc<(tokio::sync::Notify, tokio::sync::Notify)>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::gameplay_transport) enum TrainingSave {
    NotApplicable,
    Saved,
    FencedOut,
    Unknown,
}
impl ComposedFreshAdmission<'_, '_, '_> {
    /// Poll separately from combat and source timers. Runtime entries supply
    /// the bounded work set, so a refusal leaves the same actor's training
    /// bank eligible on the next cadence without allocating a retry job.
    pub(in crate::gameplay_transport) async fn run_training_checkpoints(
        &self,
        shutdown: &oteryn_foundation::CancellationToken,
    ) {
        let Some(formula) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .and_then(|native| native.training_formula())
        else {
            shutdown.cancelled().await;
            return;
        };
        let mut cadence = tokio::time::interval(std::time::Duration::from_secs(1));
        cadence.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                biased;
                () = shutdown.cancelled() => return,
                _ = cadence.tick() => {}
                () = self.states.training_checkpoint_due.notified() => {}
            }
            let now = self.owner_now().get();
            let pending: Vec<_> = self
                .states
                .spell_states
                .lock()
                .await
                .actors
                .iter()
                .filter(|(_, _, state)| state.training_checkpoint_due_at(formula, now, false))
                .map(|(actor, session, _)| (*actor, *session))
                .collect();
            for (actor, session) in pending {
                if shutdown.is_cancelled() {
                    return;
                }
                let _ = self.run_training_checkpoint_attempt(actor, session).await;
            }
        }
    }

    pub(in crate::gameplay_transport) async fn run_training_checkpoint_attempt(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> TrainingSave {
        self.root
            .with_background_operation(async {
                // A background retry uses its own fixed window with plain backoff.
                // It takes no foreground preference or FIFO waiter from a command.
                let budget = AdmissionRetryBudget {
                    deadline: tokio::time::Instant::now() + super::super::COMMAND_HOLDER_WINDOW,
                    ..AdmissionRetryBudget::new(&session)
                };
                self.save_training_checkpoint(actor, session, false, &budget)
                    .await
            })
            .await
    }

    pub(in crate::gameplay_transport) async fn save_cadence_training_support(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) {
        let budget = AdmissionRetryBudget::for_command(self.root);
        let _ = self
            .save_training_support(actor, session, false, &budget)
            .await;
    }

    pub(in crate::gameplay_transport) async fn save_spell_training(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        force: bool,
    ) -> TrainingSave {
        let Ok(_foreground) = self.root.foreground_demand() else {
            return TrainingSave::Unknown;
        };
        // Let an already running profile finish under its existing locks before
        // starting the unchanged command read deadline.
        {
            let _runtime = self.runtime.lock().await;
            let _states = self.states.spell_states.lock().await;
        }
        let budget = AdmissionRetryBudget::for_command(self.root);
        self.save_spell_training_with_budget(actor, session, force, &budget)
            .await
    }

    pub(in crate::gameplay_transport) async fn save_spell_training_with_budget(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        force: bool,
        budget: &AdmissionRetryBudget<'_>,
    ) -> TrainingSave {
        if let Err(save) = self
            .save_training_support(actor, session, force, budget)
            .await
        {
            return save;
        }
        self.save_training_checkpoint(actor, session, force, budget)
            .await
    }

    async fn save_training_support(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        force: bool,
        budget: &AdmissionRetryBudget<'_>,
    ) -> Result<(), TrainingSave> {
        match self
            .save_weapon_proficiency_with_budget(actor, session, force, budget)
            .await
        {
            TrainingSave::Unknown => return Err(TrainingSave::Unknown),
            TrainingSave::FencedOut => return Err(TrainingSave::FencedOut),
            TrainingSave::NotApplicable | TrainingSave::Saved => {}
        }
        // Resting gains use their own retained occurrence and must be acknowledged before
        // hunting debits. If activity continued after an unknown gain outcome, the debit is
        // then committed from the acknowledged gained value rather than entering the gain path.
        for _ in 0..2 {
            match self
                .save_resting_stamina_with_budget(actor, session, budget)
                .await
            {
                TrainingSave::Unknown => return Err(TrainingSave::Unknown),
                TrainingSave::FencedOut => return Err(TrainingSave::FencedOut),
                TrainingSave::NotApplicable => break,
                TrainingSave::Saved => {}
            }
        }
        // A retained receipt may leave spending accumulated after its unknown outcome.
        // Drain that successor before an actor-end save can report success.
        for _ in 0..2 {
            match self
                .save_hunting_stamina_with_budget(actor, session, budget)
                .await
            {
                TrainingSave::Unknown => return Err(TrainingSave::Unknown),
                TrainingSave::FencedOut => return Err(TrainingSave::FencedOut),
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
            return Err(TrainingSave::Unknown);
        }
        if !self.training_before_save(actor, session, force).await {
            return Err(TrainingSave::Unknown);
        }
        Ok(())
    }

    async fn save_training_checkpoint(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        force: bool,
        budget: &AdmissionRetryBudget<'_>,
    ) -> TrainingSave {
        let saved = self
            .save_spell_training_once(actor, session, force, budget)
            .await;
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
        match self
            .save_spell_training_once(actor, session, force, budget)
            .await
        {
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
        budget: &AdmissionRetryBudget<'_>,
    ) -> TrainingSave {
        let formula = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .and_then(|native| native.training_formula());
        let Some(formula) = formula else {
            return TrainingSave::NotApplicable;
        };
        {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if !states.get(&runtime, actor, session).is_some_and(|state| {
                state.training_checkpoint_due_at(formula, self.owner_now().get(), force)
            }) {
                return TrainingSave::NotApplicable;
            }
        }
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let Ok(current) = budget
            .retry_durability("training_session", async || {
                store.try_current_session(session).await
            })
            .await
        else {
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
        // Live enrichment never waits for a revision owner or holds the slot
        // while retrying a refused checkout. Forced end-path saves retain the
        // original bounded read retry and ordering.
        let mut slot = if force {
            self.revision_sequencer.acquire(character).await
        } else if let Some(slot) = self.revision_sequencer.try_acquire(character) {
            slot
        } else {
            return TrainingSave::Unknown;
        };
        let read_character = async || {
            self.root
                .read_current_character(self.character, character)
                .await
                .map_err(character_read_error)
        };
        let root = if force {
            budget
                .retry_durability("training_character", read_character)
                .await
        } else {
            read_character().await
        };
        let Ok(root) = root else {
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
        if current_fence.runtime_scope != binding.runtime_scope()
            || current_fence.scope_ownership_generation != binding.scope_generation()
            || runtime.player_control_facts(actor, session).is_err()
            || runtime.owner_fence().is_err()
        {
            return TrainingSave::FencedOut;
        }
        let Some(state) = states.get_mut(&runtime, actor, session) else {
            return TrainingSave::NotApplicable;
        };
        let mut newly_retained = false;
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
            newly_retained = true;
            retained
        };
        // The retained occurrence preserves the predecessor while SQL independently
        // checks the fences. Neither combat nor another actor's tick waits on these guards.
        drop(states);
        drop(runtime);
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
        // A refused checkout on the first attempt proves this occurrence never
        // reached the writer. Unfreeze it and keep any hits banked meanwhile.
        if newly_retained
            && outcome
                .as_ref()
                .is_err_and(super::super::progression_holder_busy)
        {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if let Some(state) = states.get_mut(&runtime, actor, session) {
                state.release_unissued_training_checkpoint(&retained);
            }
            return TrainingSave::Unknown;
        }
        // The write is never part of readback retries.
        if matches!(outcome, Err(CharacterProgressionError::RespawnPending)) {
            return TrainingSave::FencedOut;
        }
        #[cfg(test)]
        if let Ok(barrier) = TRAINING_READBACK_BARRIER.try_with(std::sync::Arc::clone) {
            barrier.0.notify_one();
            barrier.1.notified().await;
        }
        let readback = async || {
            self.finish_training_checkpoint_once(actor, session, &store, &retained, &outcome)
                .await
        };
        let result = if force {
            budget.retry_durability("training_readback", readback).await
        } else {
            readback().await
        };
        result.unwrap_or(TrainingSave::Unknown)
    }

    /// One readback attempt after a single write. Root acquisition misses leave
    /// before any live mutation, so its caller can wait without these guards.
    async fn finish_training_checkpoint_once(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        store: &FreshAdmissionStore,
        expected: &RetainedTrainingCheckpoint,
        outcome: &Result<BuildCommitOutcome, CharacterProgressionError>,
    ) -> Result<TrainingSave, DurabilityError> {
        let mut refreshed = None;
        let receipt = match outcome {
            Ok(
                BuildCommitOutcome::Committed(receipt)
                | BuildCommitOutcome::AlreadyCommitted(receipt),
            ) => Some(receipt.clone()),
            Err(CharacterProgressionError::RespawnPending) => return Ok(TrainingSave::FencedOut),
            Err(error) => {
                let receipt = self
                    .root
                    .reconcile_character_build(self.character, expected.prepared.request.occurrence)
                    .await
                    .map_err(progression_read_error)?;
                if receipt.is_none()
                    && matches!(
                        error,
                        CharacterProgressionError::CharacterRevisionMismatch
                            | CharacterProgressionError::AuthorityRejected
                    )
                {
                    refreshed = match self
                        .uncommitted_checkpoint_fence(store, session, expected)
                        .await
                    {
                        Err(error)
                            if matches!(
                                outcome,
                                Err(CharacterProgressionError::AuthorityRejected)
                            ) && !matches!(error, DurabilityError::RootUnavailable) =>
                        {
                            return Ok(TrainingSave::FencedOut);
                        }
                        result => result?,
                    };
                }
                receipt
            }
        };
        // A receipt is history. Resolve current authority before re-proving the
        // exact runtime actor and retained occurrence at live installation.
        let fresh = if receipt.is_some() {
            Some(store.try_current_session(session).await?)
        } else {
            None
        };
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if runtime.owner_fence().is_err() || runtime.player_control_facts(actor, session).is_err() {
            return Ok(TrainingSave::FencedOut);
        }
        let Some(state) = states.get_mut(&runtime, actor, session) else {
            return Ok(TrainingSave::Unknown);
        };
        let Some(retained) = state.pending_training_checkpoint() else {
            return Ok(TrainingSave::Unknown);
        };
        if retained.fence != expected.fence
            || retained.formula_digest != expected.formula_digest
            || retained.prepared.request != expected.prepared.request
        {
            return Ok(TrainingSave::Unknown);
        }
        let retained = retained.clone();
        let binding = runtime.binding();
        let (Some(receipt), Some(fresh)) = (receipt, fresh) else {
            let authority_rejected =
                matches!(outcome, Err(CharacterProgressionError::AuthorityRejected));
            let Some(refreshed) = refreshed.filter(|fence| {
                fence.runtime_scope == binding.runtime_scope()
                    && fence.scope_ownership_generation == binding.scope_generation()
            }) else {
                return Ok(if authority_rejected {
                    TrainingSave::FencedOut
                } else {
                    TrainingSave::Unknown
                });
            };
            if authority_rejected {
                if state
                    .refence_uncommitted_training_checkpoint(&retained, refreshed)
                    .is_err()
                {
                    return Ok(TrainingSave::FencedOut);
                }
            } else {
                let _ = state.refresh_training_checkpoint_revision(&retained, refreshed);
            }
            return Ok(TrainingSave::Unknown);
        };
        if fresh.session_state() == GameSessionState::Terminal
            || fresh.current_connection_generation() != retained.fence.connection_generation
            || fresh.current_character_lease().generation()
                != retained.fence.character_lease_generation
            || fresh.current_runtime_scope() != retained.fence.runtime_scope
            || fresh.current_scope_generation() != retained.fence.scope_ownership_generation
            || fresh.current_runtime_scope() != binding.runtime_scope()
            || fresh.current_scope_generation() != binding.scope_generation()
        {
            return Ok(TrainingSave::FencedOut);
        }
        if state
            .commit_training_checkpoint(retained.prepared, &receipt)
            .is_err()
        {
            return Ok(TrainingSave::Unknown);
        }
        Ok(TrainingSave::Saved)
    }
    /// The current gameplay fence for a retained checkpoint the writer proved uncommitted (a
    /// rollback, or a refusal with no receipt for its occurrence): its build is still the
    /// durable build, the Character revision is bracketed unchanged around that read, and the
    /// session is independently resolved and not terminal. A moved build, revision or
    /// session yields `Ok(None)`; acquisition failures remain typed for the readback owner.
    async fn uncommitted_checkpoint_fence(
        &self,
        store: &FreshAdmissionStore,
        session: GameSessionId,
        retained: &RetainedTrainingCheckpoint,
    ) -> Result<Option<CurrentCharacterGameplayFence>, DurabilityError> {
        let character = retained.fence.character_id;
        let before_root = self
            .root
            .read_current_character(self.character, character)
            .await
            .map_err(character_read_error)?;
        let build = self
            .root
            .read_character_build_state(self.character, character)
            .await
            .map_err(progression_read_error)?;
        let after_root = self
            .root
            .read_current_character(self.character, character)
            .await
            .map_err(character_read_error)?;
        let (fresh, _) = store.current_session_at(session).await?;
        if build != retained.prepared.request.before
            || before_root.revision != after_root.revision
            || fresh.session_state() == GameSessionState::Terminal
        {
            return Ok(None);
        }
        Ok(Some(CurrentCharacterGameplayFence {
            character_id: character,
            game_session_id: session,
            connection_generation: fresh.current_connection_generation(),
            character_lease_generation: fresh.current_character_lease().generation(),
            runtime_scope: fresh.current_runtime_scope(),
            scope_ownership_generation: fresh.current_scope_generation(),
            expected_character_revision: after_root.revision,
        }))
    }
}

fn character_read_error(error: CharacterAuthorityError) -> DurabilityError {
    match error {
        CharacterAuthorityError::Unavailable(error) => error,
        CharacterAuthorityError::Rejected
        | CharacterAuthorityError::Conflict
        | CharacterAuthorityError::NameUnavailable => DurabilityError::CommitRejected,
    }
}

fn progression_read_error(error: CharacterProgressionError) -> DurabilityError {
    match error {
        CharacterProgressionError::Unavailable(error) => error,
        _ => DurabilityError::CommitRejected,
    }
}
