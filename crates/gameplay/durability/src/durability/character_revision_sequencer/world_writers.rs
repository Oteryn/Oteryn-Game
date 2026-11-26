impl RevisionSlot {
    /// World callback storage and altar inputs share one revision slot and transaction.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_world_callback(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: std::sync::Arc<QuestStateCatalogue>,
        permit: &super::spell_owner_commit::SpellLanePermit,
        context: super::quest_world_callback::QuestWorldContext,
        native: Option<&oteryn_gameplay_content::content::native_gameplay::NativeGameplayState>,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.commit_world_callback_with_native(
            root, authority, node, fence, request, catalogue, permit, context, native,
        )
        .await
    }

    /// World callback storage and altar inputs share one revision slot and transaction.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_world_callback_with_native(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: std::sync::Arc<QuestStateCatalogue>,
        permit: &super::spell_owner_commit::SpellLanePermit,
        context: super::quest_world_callback::QuestWorldContext,
        native: Option<&oteryn_gameplay_content::content::native_gameplay::NativeGameplayState>,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.commit_world_callback_with_native_and_gate(
            root,
            authority,
            node,
            fence,
            request,
            catalogue,
            permit,
            context,
            native,
            &super::quest_state::grizzly::NoGrizzlyPremium,
        )
        .await
    }

    /// World callback storage and altar inputs share one revision slot and transaction.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_world_callback_with_native_and_gate<
        G: super::quest_state::grizzly::GrizzlyPremiumGate,
    >(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: std::sync::Arc<QuestStateCatalogue>,
        permit: &super::spell_owner_commit::SpellLanePermit,
        context: super::quest_world_callback::QuestWorldContext,
        native: Option<&oteryn_gameplay_content::content::native_gameplay::NativeGameplayState>,
        gate: &G,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_world_callback_with_native_and_gate(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    std::sync::Arc::clone(&catalogue),
                    permit,
                    context.clone(),
                    native,
                    gate,
                )
            },
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn commit_world_callback_with_reward_and_gate<
        G: super::quest_state::grizzly::GrizzlyPremiumGate,
    >(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: std::sync::Arc<QuestStateCatalogue>,
        permit: &super::spell_owner_commit::SpellLanePermit,
        context: super::quest_world_callback::QuestWorldContext,
        admission: super::reward_claim_mint::QuestRewardAdmission<'_>,
        gate: &G,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_world_callback_with_reward_and_gate(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    std::sync::Arc::clone(&catalogue),
                    permit,
                    context.clone(),
                    admission,
                    gate,
                )
            },
        )
        .await
    }

    /// World callback storage and altar inputs share one revision slot and transaction.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_world_callback_with_items<
        G: super::quest_state::grizzly::GrizzlyPremiumGate,
    >(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: std::sync::Arc<QuestStateCatalogue>,
        permit: &super::spell_owner_commit::SpellLanePermit,
        context: super::quest_world_callback::QuestWorldContext,
        native: Option<&oteryn_gameplay_content::content::native_gameplay::NativeGameplayState>,
        admission: Option<super::reward_claim_mint::QuestRewardAdmission<'_>>,
        gate: &G,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.commit_world_callback_with_items_selected(root,authority,node,fence,request,catalogue,permit,context.into(),native,admission,gate).await
    }

    /// Preserve an exact selected source through cursor retries.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_world_callback_with_items_selected<G: super::quest_state::grizzly::GrizzlyPremiumGate>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: std::sync::Arc<QuestStateCatalogue>,
        permit: &super::spell_owner_commit::SpellLanePermit,
        context: super::quest_world_callback::QuestWorldToolContext,
        native: Option<&oteryn_gameplay_content::content::native_gameplay::NativeGameplayState>,
        admission: Option<super::reward_claim_mint::QuestRewardAdmission<'_>>,
        gate: &G,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_world_callback_with_items_selected(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    std::sync::Arc::clone(&catalogue),
                    permit,
                    context.clone(),
                    native,
                    admission,
                    gate,
                )
            },
        )
        .await
    }

    /// World callback storage and altar inputs share one revision slot and transaction.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_world_callback_with_requirements<G: super::quest_state::grizzly::GrizzlyPremiumGate>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: std::sync::Arc<QuestStateCatalogue>,
        permit: &super::spell_owner_commit::SpellLanePermit,
        context: super::quest_world_callback::QuestWorldContext,
        native: Option<&oteryn_gameplay_content::content::native_gameplay::NativeGameplayState>,
        admission: Option<super::reward_claim_mint::QuestRewardAdmission<'_>>,
        premium: &G,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.commit_world_callback_with_items_and_premium(
            root, authority, node, fence, request, catalogue, permit, context, native,
            admission, premium,
        ).await
    }

    /// Keep the native Premium owner on both sequenced attempts, including a cursor retry.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_world_callback_with_items_and_premium<G: super::quest_state::grizzly::GrizzlyPremiumGate>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: std::sync::Arc<QuestStateCatalogue>,
        permit: &super::spell_owner_commit::SpellLanePermit,
        context: super::quest_world_callback::QuestWorldContext,
        native: Option<&oteryn_gameplay_content::content::native_gameplay::NativeGameplayState>,
        admission: Option<super::reward_claim_mint::QuestRewardAdmission<'_>>,
        premium: &G,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_world_callback_with_requirements(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    std::sync::Arc::clone(&catalogue),
                    permit,
                    context.clone(),
                    native,
                    admission,
                    premium,
                )
            },
        )
        .await
    }

    /// World rewards share the same revision slot, item owner and Premium fence.
    #[allow(
        clippy::too_many_arguments,
        reason = "forwards the complete existing world and reward participants"
    )]
    pub async fn commit_world_reward<G: super::quest_state::grizzly::GrizzlyPremiumGate>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: std::sync::Arc<QuestStateCatalogue>,
        permit: &super::spell_owner_commit::SpellLanePermit,
        context: super::quest_world_callback::QuestWorldContext,
        admission: super::reward_claim_mint::QuestRewardAdmission<'_>,
        premium: &G,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_world_reward(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    std::sync::Arc::clone(&catalogue),
                    permit,
                    context.clone(),
                    admission,
                    premium,
                )
            },
        )
        .await
    }

    /// The XP award of one pending quest XP obligation at the cursor (QUEST-GATE-0 §5.5), under
    /// the active progression `policy` (its `reward_revision`, never the quest content
    /// revision). The XP binding includes the revision, so a mismatch fails closed.
    #[allow(
        dead_code,
        reason = "standalone durability suites path-load this module without quest XP cases"
    )]
    pub async fn commit_quest_experience<const N: usize>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        obligation: &QuestXpObligation,
        policy: &FiniteProgressionPolicy<String, N>,
    ) -> Result<ExperienceCommitOutcome, CharacterProgressionError> {
        let request = obligation.award(policy);
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::FailClosed),
            |fence| root.commit_character_quest_experience(authority, node, fence, request.clone()),
        )
        .await
    }

    /// One quest transition and, when it committed a pending quest XP obligation (or replayed
    /// one still pending), that obligation's XP award, both in this slot (QUEST-GATE-0 §5.5). A
    /// refused award keeps the obligation and is reported as a defect.
    #[allow(
        clippy::too_many_arguments,
        dead_code,
        reason = "the transition's and the award's arguments; standalone durability suites \
                  path-load this module without quest XP cases"
    )]
    pub async fn commit_quest_transition_with_experience<const N: usize>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: std::sync::Arc<QuestStateCatalogue>,
        policy: &FiniteProgressionPolicy<String, N>,
    ) -> Result<QuestTransitionAward, CharacterProgressionError> {
        let transition = self
            .commit_quest_transition(root, authority, node, fence, request, catalogue)
            .await?;
        let pending = match &transition {
            QuestTransitionOutcome::Committed(receipt)
            | QuestTransitionOutcome::AlreadyCommitted(receipt) => receipt.experience,
            QuestTransitionOutcome::Refused(_) | QuestTransitionOutcome::ObligationClosed => None,
        };
        let experience = match pending {
            Some(obligation) => {
                let outcome = self
                    .commit_quest_experience(root, authority, node, fence, &obligation, policy)
                    .await;
                report_refused_experience(&outcome);
                Some(outcome)
            }
            None => None,
        };
        Ok(QuestTransitionAward {
            transition,
            experience,
        })
    }

    /// One perk modification at the cursor (PROFICIENCY-1B §6.4). Its binding excludes the
    /// CharacterRevision, so a lost race reloads the cursor and retries once under the same
    /// occurrence and draw; a later duplicate finds that receipt and never pays twice.
    #[allow(
        clippy::too_many_arguments,
        dead_code,
        reason = "the modification writer's own arguments; standalone durability suites path-load \
                  this module without modification cases"
    )]
    pub async fn commit_proficiency_modification(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        command: ProficiencyModificationCommand,
        context: ProficiencyModificationContext,
        orb: Option<super::item_use::ItemUseRequest>,
        definitions: std::sync::Arc<dyn ProficiencyDefinitions>,
        shaping: std::sync::Arc<dyn ProficiencyShapingSource>,
    ) -> Result<ProficiencyModificationOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_proficiency_modification(
                    authority,
                    node,
                    fence,
                    command.clone(),
                    context,
                    orb.clone(),
                    std::sync::Arc::clone(&definitions),
                    std::sync::Arc::clone(&shaping),
                )
            },
        )
        .await
    }

    /// One Wheel allocation change at the cursor (WHEEL-0 §4). Its binding excludes the
    /// CharacterRevision (the request carries the expected `wheel_revision`), so a mismatch
    /// reloads the cursor and retries once; a refusal writes nothing.
    #[allow(
        clippy::too_many_arguments,
        dead_code,
        reason = "the Wheel writer's own arguments; standalone durability suites path-load this \
                  module without Wheel cases"
    )]
    pub async fn commit_wheel<G: super::character_wheel::WheelPremiumGate>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: WheelChangeRequest,
        ruleset: std::sync::Arc<WheelRuleset>,
        facts: WheelChangeFacts,
        gate: &G,
    ) -> Result<WheelCommitOutcome, CharacterProgressionError> {
        self.commit_wheel_with_admission(
            root,
            authority,
            node,
            fence,
            request,
            ruleset,
            facts,
            gate,
            &|| true,
        )
        .await
    }

    /// A caller-owned decision is retained across the existing revision retry.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_wheel_with_admission<G: super::character_wheel::WheelPremiumGate>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: WheelChangeRequest,
        ruleset: std::sync::Arc<WheelRuleset>,
        facts: WheelChangeFacts,
        gate: &G,
        admit: &(impl Fn() -> bool + Sync),
    ) -> Result<WheelCommitOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_wheel_with_admission(
                    authority,
                    node,
                    fence,
                    request,
                    std::sync::Arc::clone(&ruleset),
                    facts,
                    gate,
                    admit,
                )
            },
        )
        .await
    }

    /// The admission Wheel reset at the cursor (WHEEL0-RST-1). Its occurrence and binding exclude
    /// the CharacterRevision, so a mismatch reloads the cursor and retries once.
    #[allow(
        dead_code,
        reason = "standalone durability suites path-load this module without Wheel cases"
    )]
    pub async fn reset_wheel(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        ruleset: std::sync::Arc<WheelRuleset>,
    ) -> Result<WheelResetOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.reset_character_wheel(authority, node, fence, std::sync::Arc::clone(&ruleset))
            },
        )
        .await
    }
}
