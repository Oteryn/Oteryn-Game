//! Native hit settlement joins the existing per-character revision queue.
use super::super::quest_state::native_hits::NativeQuestHit;
use super::*;

impl RevisionSlot {
    pub async fn commit_native_quest_hit(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        hit: &NativeQuestHit,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_native_quest_hit(
                    authority,
                    node,
                    fence,
                    hit.clone(),
                    Arc::clone(&catalogue),
                )
            },
        )
        .await
    }
}
