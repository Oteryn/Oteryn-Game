use super::super::quest_state::{QuestCause, native_kills::NativeQuestDeath};
use super::*;

impl RevisionSlot {
    #[allow(
        clippy::too_many_arguments,
        reason = "same fenced writer inputs as other quest operations"
    )]
    pub async fn commit_native_quest_kill(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        cause: QuestCause,
        death: &NativeQuestDeath,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_native_quest_kill(
                    authority,
                    node,
                    fence,
                    cause,
                    death.clone(),
                    Arc::clone(&catalogue),
                )
            },
        )
        .await
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "native death evidence and actor admission share one revision cursor"
    )]
    pub async fn commit_native_quest_kill_with_admission<
        G: super::super::quest_state::grizzly::GrizzlyPremiumGate,
    >(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        cause: QuestCause,
        death: &NativeQuestDeath,
        catalogue: Arc<QuestStateCatalogue>,
        gate: &G,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_native_quest_kill_with_admission(
                    authority,
                    node,
                    fence,
                    cause,
                    death.clone(),
                    Arc::clone(&catalogue),
                    gate,
                )
            },
        )
        .await
    }
}
