use super::*;
impl RevisionSlot {
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_rotten_area_think(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        input: super::super::quest_state::rotten_area::QualifiedRottenAreaThink,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_rotten_area_think(
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
}
