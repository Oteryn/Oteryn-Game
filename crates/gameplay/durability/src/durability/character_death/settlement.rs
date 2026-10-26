//! One revision slot across training, current Premium evaluation and death.
use super::*;

impl DurabilityRoot {
    /// DEATH-2 (first player death decision §4.3-§4.5): the durable part of
    /// one runtime death, run by the Channel owner off its lane, in the
    /// Character's revision slot.  A retained receipt is replayed at its
    /// original revision; a new death commits at the slot's cursor (a missing
    /// progression row runs the D88 initializer first, as an XP award does, and
    /// an earlier death's unconsumed respawn is consumed first: the actor that
    /// died again was alive and placed, so that respawn has happened).
    /// The death's own pending respawn is left recorded: the caller places the
    /// player at its respawn position and only then consumes it under the
    /// occurrence ([`Self::consume_pending_respawn`]).  A restart before that
    /// consumption leaves the row for the next admission, which places the
    /// new actor at the recorded position (DEATH-0 §3.4).  A retry after any
    /// partial outcome converges: the death replays its first receipt.
    pub async fn settle_player_death<const N: usize>(
        &self,
        sequencer: &CharacterRevisionSequencer,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: CharacterDeathRequest<N>,
    ) -> Result<CommittedCharacterDeath> {
        let occurrence = request.occurrence;
        let mut slot = sequencer.acquire(fence.character_id).await;
        let original = self
            .reconcile_character_death(authority, occurrence)
            .await?
            .map(|death| death.original_character_revision);
        self.settle_player_death_in_slot(
            &mut slot,
            authority,
            node,
            fence,
            request,
            original,
            None,
            promotion::DeathPremiumState::default(),
        )
        .await
    }

    /// A13 §4.6: settle an owner-prepared training checkpoint in its own build
    /// transaction before constructing a new death binding. One existing revision
    /// slot covers both transactions; an exact retained death replay skips training.
    ///
    /// This callable seam does not capture/drain the actor's live deltas, refresh
    /// its current session. This compatibility entry point has no current build
    /// formula; death preserves trained build state with or without one.
    /// `None` must mean the owner has no pending checkpoint, not permission to drop
    /// pending progress. A failed/stale/unknown flush prevents the death; the owner
    /// retains its deltas and retries using the existing training-save rules.
    #[allow(
        clippy::too_many_arguments,
        reason = "existing death inputs plus the owner-prepared checkpoint"
    )]
    pub async fn settle_player_death_with_training<const N: usize>(
        &self,
        sequencer: &CharacterRevisionSequencer,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: CharacterDeathRequest<N>,
        training: Option<DeathTrainingFlush<'_>>,
    ) -> Result<CommittedCharacterDeath> {
        self.settle_player_death_with_training_inner(
            sequencer, authority, node, fence, request, training, None, None,
        )
        .await
    }

    /// One sequencer slot covers the prior training receipt and the build-aware
    /// death. The runtime owner must independently capture/drain live deltas.
    #[allow(
        clippy::too_many_arguments,
        reason = "existing death/checkpoint inputs and current content formula"
    )]
    pub async fn settle_player_death_with_training_and_build<const N: usize>(
        &self,
        sequencer: &CharacterRevisionSequencer,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: CharacterDeathRequest<N>,
        training: Option<DeathTrainingFlush<'_>>,
        formula: &dyn BuildFormula,
    ) -> Result<CommittedCharacterDeath> {
        self.settle_player_death_with_training_inner(
            sequencer,
            authority,
            node,
            fence,
            request,
            training,
            Some(formula),
            None,
        )
        .await
    }

    /// The runtime owner's death settlement. A current training formula validates
    /// the build when available; all new deaths preserve every skill and magic
    /// level/progress value (D58 / NIGHT-W1 decision 19). Historical receipts
    /// retain their original outcome. The owner flushes pending training first.
    #[allow(
        clippy::too_many_arguments,
        reason = "the runtime death inputs and current Premium owner"
    )]
    pub async fn settle_runtime_player_death<const N: usize>(
        &self,
        sequencer: &CharacterRevisionSequencer,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: CharacterDeathRequest<N>,
        formula: Option<&dyn BuildFormula>,
        premium: Option<&dyn promotion::DeathPremiumOwner>,
    ) -> Result<CommittedCharacterDeath> {
        let formula =
            formula.filter(|formula| formula.content_revision() == request.context.content);
        self.settle_player_death_with_training_inner(
            sequencer, authority, node, fence, request, None, formula, premium,
        )
        .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the existing death/checkpoint inputs and optional build-aware formula"
    )]
    async fn settle_player_death_with_training_inner<const N: usize>(
        &self,
        sequencer: &CharacterRevisionSequencer,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        mut request: CharacterDeathRequest<N>,
        training: Option<DeathTrainingFlush<'_>>,
        formula: Option<&dyn BuildFormula>,
        premium: Option<&dyn promotion::DeathPremiumOwner>,
    ) -> Result<CommittedCharacterDeath> {
        validate_request(&fence, &request)?;
        let mut slot = sequencer.acquire(fence.character_id).await;
        let retained = self
            .reconcile_character_death(authority, request.occurrence)
            .await?;
        let original = retained
            .as_ref()
            .map(|death| death.original_character_revision);
        let premium = if let Some(owner) = premium {
            request.held_blessings = match retained {
                Some(death) => death.blessings_before,
                None => promotion::held_blessings(self, authority, node, fence).await?,
            };
            owner.current().await
        } else {
            promotion::DeathPremiumState::default()
        };
        if original.is_none()
            && let Some(training) = training
        {
            if formula.is_some_and(|formula| {
                training.formula_digest != formula.digest()
                    || training.formula.content_revision() != formula.content_revision()
            }) {
                return Err(CharacterProgressionError::InvalidInput);
            }
            validate_death_training_input(fence, &training)?;
            settle_death_training_flush(
                fence.character_id,
                training.fence.expected_character_revision,
                training.request,
                async |original| {
                    slot.commit_build(
                        self,
                        authority,
                        node,
                        training.fence,
                        training.request.clone(),
                        training.formula,
                        original,
                    )
                    .await
                },
                async || {
                    self.reconcile_character_build(authority, training.request.occurrence)
                        .await
                },
            )
            .await?;
        }
        // Only the revision cursor is selected here. The supplied identity/fence
        // fields remain intent; the death writer resolves current DB authority.
        let fence = if original.is_none() {
            CurrentCharacterGameplayFence {
                expected_character_revision: slot_cursor(&mut slot, self, authority).await?,
                ..fence
            }
        } else {
            fence
        };
        // A retained death supplies its historical build/formula binding. The
        // owner's newly loaded current formula is only for a new occurrence.
        let formula = if original.is_some() { None } else { formula };
        self.settle_player_death_in_slot(
            &mut slot, authority, node, fence, request, original, formula, premium,
        )
        .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the shared slot and existing death writer inputs"
    )]
    async fn settle_player_death_in_slot<const N: usize>(
        &self,
        slot: &mut RevisionSlot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: CharacterDeathRequest<N>,
        original: Option<CharacterRevision>,
        formula: Option<&dyn BuildFormula>,
        premium: promotion::DeathPremiumState,
    ) -> Result<CommittedCharacterDeath> {
        let outcome = match self
            .commit_death_in_slot(
                slot,
                authority,
                node,
                fence,
                request.clone(),
                original,
                formula,
                premium,
            )
            .await
        {
            Err(CharacterProgressionError::MissingProgressionState) if original.is_none() => {
                let expected_character_revision = slot_cursor(slot, self, authority).await?;
                self.initialize_character_progression(
                    authority,
                    node,
                    CurrentCharacterGameplayFence {
                        expected_character_revision,
                        ..fence
                    },
                    ProgressionInitializationRequest {
                        context: request.context.clone(),
                        policy_revision: request.policy_revision.clone(),
                        reward_revision: request.reward_revision.clone(),
                        policy: request.policy.clone(),
                    },
                )
                .await?;
                self.commit_death_in_slot(
                    slot, authority, node, fence, request, None, formula, premium,
                )
                .await?
            }
            // An earlier death's respawn was never consumed (its admission's consumption
            // failed), yet this actor is alive and placed: that respawn has happened.
            Err(CharacterProgressionError::RespawnPending) if original.is_none() => {
                let expected_character_revision = slot_cursor(slot, self, authority).await?;
                self.consume_pending_respawn(
                    authority,
                    node,
                    CurrentCharacterGameplayFence {
                        expected_character_revision,
                        ..fence
                    },
                    None,
                )
                .await?;
                self.commit_death_in_slot(
                    slot, authority, node, fence, request, None, formula, premium,
                )
                .await?
            }
            outcome => outcome?,
        };
        let (CharacterDeathOutcome::Committed(committed)
        | CharacterDeathOutcome::AlreadyCommitted(committed)) = outcome;
        Ok(committed)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "one common sequenced death dispatch for initial and fallback paths"
    )]
    async fn commit_death_in_slot<const N: usize>(
        &self,
        slot: &mut RevisionSlot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: CharacterDeathRequest<N>,
        original: Option<CharacterRevision>,
        formula: Option<&dyn BuildFormula>,
        premium: promotion::DeathPremiumState,
    ) -> Result<CharacterDeathOutcome> {
        slot.commit_premium_death(
            self, authority, node, fence, request, formula, original, premium,
        )
        .await
    }
}
