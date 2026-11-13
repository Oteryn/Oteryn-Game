use super::*;
impl RevisionSlot {
    #[allow(
        clippy::too_many_arguments,
        reason = "the existing source Quest action keeps its fenced revision slot"
    )]
    pub async fn commit_rotten_flame_use(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        input: super::super::quest_state::rotten_flame::RottenFlameUse,
        catalogue: Arc<QuestStateCatalogue>,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_rotten_flame_use(
                    authority,
                    node,
                    fence,
                    input.clone(),
                    Arc::clone(&catalogue),
                    permit,
                )
            },
        )
        .await
    }
}
