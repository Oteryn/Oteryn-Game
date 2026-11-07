use super::*;
use crate::durability::quest_state::NativeWorldChangeEntry;
impl RevisionSlot {
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_world_change_entry(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        source: NativeWorldChangeEntry,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_world_change_entry(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    Arc::clone(&catalogue),
                    source.clone(),
                )
            },
        )
        .await
    }
}
