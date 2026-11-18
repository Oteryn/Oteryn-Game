use super::*;
use crate::durability::quest_state::wave_egress::WaveEgress;

impl RevisionSlot {
    pub async fn commit_quest_wave_egress(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        input: WaveEgress,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| root.commit_quest_wave_egress(authority, node, fence, input.clone(), Arc::clone(&catalogue)),
        ).await
    }
}
