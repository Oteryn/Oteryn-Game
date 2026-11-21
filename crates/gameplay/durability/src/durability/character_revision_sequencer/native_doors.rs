//! Preserve the Character revision cursor for registered native door observations.
use super::super::quest_state::{QuestCause, native_doors::NativeQuestDoorOpening};
use super::*;

impl RevisionSlot {
    #[allow(
        clippy::too_many_arguments,
        reason = "same fenced writer inputs as native kill observations"
    )]
    pub async fn commit_native_quest_door(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        cause: QuestCause,
        opening: &NativeQuestDoorOpening,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_native_quest_door(
                    authority,
                    node,
                    fence,
                    cause,
                    opening.clone(),
                    Arc::clone(&catalogue),
                )
            },
        )
        .await
    }
}
