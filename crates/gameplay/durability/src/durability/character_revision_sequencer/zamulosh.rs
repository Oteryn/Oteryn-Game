use super::*;

impl RevisionSlot {
    #[allow(
        clippy::too_many_arguments,
        reason = "uses the existing revision and authority fence"
    )]
    pub async fn commit_zamulosh_lovely_step(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        input: super::super::quest_state::zamulosh::QualifiedLovelyStep,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_zamulosh_lovely_step(
                    authority,
                    node,
                    fence,
                    input.clone(),
                    Arc::clone(&catalogue),
                )
            },
        )
        .await
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "uses the existing revision and authority fence"
    )]
    pub async fn commit_zamulosh_habitat_reset(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        input: super::super::quest_state::zamulosh::HabitatReset,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_zamulosh_habitat_reset(
                    authority,
                    node,
                    fence,
                    input,
                    Arc::clone(&catalogue),
                )
            },
        )
        .await
    }
}
