impl RevisionSlot {
    /// One proficiency change at the cursor, or the exact replay of a retained receipt at its
    /// `original` revision. A mismatch fails closed; a refusal writes nothing.
    #[allow(
        clippy::too_many_arguments,
        dead_code,
        reason = "the proficiency writer's own arguments plus the replay revision; standalone durability \
                  suites path-load this module without proficiency cases"
    )]
    pub async fn commit_proficiency(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: ProficiencyChangeRequest,
        policy: std::sync::Arc<dyn ProficiencyDefinitions>,
        original: Option<CharacterRevision>,
    ) -> Result<ProficiencyCommitOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::from(original, OnMismatch::FailClosed),
            |fence| {
                root.commit_character_proficiency(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    std::sync::Arc::clone(&policy),
                )
            },
        )
        .await
    }

    /// One quest transition at the cursor (QUEST-STATE-0 §5.2). Its binding excludes the
    /// revision, so a mismatch reloads the cursor and retries once (replay or one commit); a
    /// refusal writes nothing.
    #[allow(
        dead_code,
        reason = "standalone durability suites path-load this module without quest cases"
    )]
    pub async fn commit_quest_transition(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: std::sync::Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_quest_transition(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    std::sync::Arc::clone(&catalogue),
                )
            },
        )
        .await
    }

    /// Preserve the revision cursor while consulting the registered actor admission owner.
    #[allow(
        dead_code,
        clippy::too_many_arguments,
        reason = "complete actor admission context; standalone path-loaded suites omit quest cases"
    )]
    pub async fn commit_quest_transition_with_admission<
        G: super::quest_state::grizzly::GrizzlyPremiumGate,
    >(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: std::sync::Arc<QuestStateCatalogue>,
        gate: &G,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_quest_transition_with_admission(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    std::sync::Arc::clone(&catalogue),
                    gate,
                )
            },
        )
        .await
    }

    /// Commit a content-bound NPC exchange and all its value participants.
    #[allow(
        clippy::too_many_arguments,
        reason = "the revision slot forwards the complete existing Quest and reward admission context"
    )]
    pub async fn commit_quest_reward<G: super::quest_state::grizzly::GrizzlyPremiumGate>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: std::sync::Arc<QuestStateCatalogue>,
        admission: super::reward_claim_mint::quest::QuestRewardAdmission<'_>,
        gate: &G,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_quest_reward(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    std::sync::Arc::clone(&catalogue),
                    admission,
                    gate,
                )
            },
        )
        .await
    }

    /// Benevola uses the same Quest receipt revision cursor as ordinary NPC quests.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_benevola_handover(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        permit: &super::spell_owner_commit::SpellLanePermit,
        handover: super::world_event::changes::counters::benevola::BenevolaHandover,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_benevola_handover(
                    authority,
                    node,
                    fence,
                    permit,
                    handover.clone(),
                    Arc::clone(&catalogue),
                )
            },
        )
        .await
    }

    /// Source essence USE shares the Quest revision cursor with its Item burn.
    pub async fn commit_rotten_blood_essence_use(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        input: super::quest_state::rotten_essence::RottenEssenceUse,
        catalogue: std::sync::Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_rotten_blood_essence_use(
                    authority,
                    node,
                    fence,
                    input.clone(),
                    std::sync::Arc::clone(&catalogue),
                )
            },
        )
        .await
    }

    /// Closed native Soul War Step/USE shares the existing Quest revision cursor.
    #[allow(
        clippy::too_many_arguments,
        reason = "the existing Quest cursor retains authority, the sealed source action and physical Step context"
    )]
    pub async fn commit_soul_war_source_action(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        input: super::quest_state::soul_war_source::SoulWarSourceAction,
        catalogue: std::sync::Arc<QuestStateCatalogue>,
        context: Option<(
            &super::spell_owner_commit::SpellLanePermit,
            super::quest_world_callback::QuestWorldContext,
        )>,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_soul_war_source_action(
                    authority,
                    node,
                    fence,
                    input.clone(),
                    std::sync::Arc::clone(&catalogue),
                    context.clone(),
                )
            },
        )
        .await
    }

    /// Generated Task Board state and its quest receipts share this revision slot.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_task_board(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: super::task_board::TaskBoardCommand,
        catalogue: std::sync::Arc<super::task_board::TaskBoardCatalogue>,
        permit: Option<&super::spell_owner_commit::SpellLanePermit>,
    ) -> Result<super::task_board::TaskBoardCommit, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_task_board(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    std::sync::Arc::clone(&catalogue),
                    permit,
                )
            },
        )
        .await
    }

    /// Trusted future Store delivery calls the existing entitlement/revision owner.
    #[allow(clippy::too_many_arguments)]
    pub async fn grant_task_board_expansion(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        command: crate::foundation::CommandRef,
        delivery_reference: &str,
        catalogue: std::sync::Arc<super::task_board::TaskBoardCatalogue>,
    ) -> Result<super::task_board::TaskBoardCommit, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.grant_task_board_expansion(
                    authority,
                    node,
                    fence,
                    command,
                    delivery_reference,
                    std::sync::Arc::clone(&catalogue),
                )
            },
        )
        .await
    }
}
