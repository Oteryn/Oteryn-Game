use super::*;
use crate::durability::quest_state::putrefactory::QualifiedPutrefactoryTick;

impl RevisionSlot {
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_putrefactory_tick(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        input: QualifiedPutrefactoryTick,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| root.commit_putrefactory_tick(
                authority, node, fence, input.clone(), Arc::clone(&catalogue),
            ),
        )
        .await
    }
}
