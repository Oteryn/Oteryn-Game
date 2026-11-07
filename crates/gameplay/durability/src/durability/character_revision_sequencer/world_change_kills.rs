use super::*;
use crate::durability::{
    quest_state::world_change_kills::NativeWorldChangeKill, spell_owner_commit::SpellLanePermit,
};

impl RevisionSlot {
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_world_change_kill(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        catalogue: Arc<QuestStateCatalogue>,
        permit: &SpellLanePermit,
        context: NativeWorldChangeKill,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_world_change_kill(
                    authority,
                    node,
                    fence,
                    Arc::clone(&catalogue),
                    permit,
                    context.clone(),
                )
            },
        )
        .await
    }
}
