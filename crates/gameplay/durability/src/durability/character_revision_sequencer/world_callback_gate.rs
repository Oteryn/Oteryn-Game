//! World callback gate forwarded through the existing Character revision slot.
use super::*;

impl RevisionSlot {
    #[allow(clippy::too_many_arguments, reason = "existing world-writer inputs plus its transaction-bound gate")]
    pub async fn commit_world_callback_with_items_and_gate<G: super::super::quest_state::grizzly::GrizzlyPremiumGate>(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: std::sync::Arc<QuestStateCatalogue>,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        context: super::super::quest_world_callback::QuestWorldContext,
        native: Option<&oteryn_gameplay_content::content::native_gameplay::NativeGameplayState>,
        admission: Option<super::super::reward_claim_mint::QuestRewardAdmission<'_>>,
        gate: &G,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_world_callback_with_items_and_gate(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    std::sync::Arc::clone(&catalogue),
                    permit,
                    context.clone(),
                    native,
                    admission,
                    gate,
                )
            },
        )
        .await
    }
}
