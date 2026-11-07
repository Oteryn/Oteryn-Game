use super::*;
use crate::durability::item_use::ItemUseRequest;

impl RevisionSlot {
    /// One catalyst use at `original`, as `commit_proficiency`: a mismatch fails closed and a
    /// refusal writes nothing.
    #[allow(
        clippy::too_many_arguments,
        reason = "the catalyst writer's own arguments plus the replay revision"
    )]
    pub async fn commit_proficiency_catalyst(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        item: ItemUseRequest,
        request: ProficiencyChangeRequest,
        policy: std::sync::Arc<dyn ProficiencyDefinitions>,
        original: Option<CharacterRevision>,
    ) -> Result<ProficiencyCommitOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::from(original, OnMismatch::FailClosed),
            |fence| {
                root.commit_proficiency_catalyst(
                    authority,
                    node,
                    fence,
                    item.clone(),
                    request.clone(),
                    std::sync::Arc::clone(&policy),
                )
            },
        )
        .await
    }
}
