//! Serialize world MINT and quest progression at the existing Character cursor.
use super::*;
impl RevisionSlot {
    #[allow(
        clippy::too_many_arguments,
        reason = "same existing world and MINT participant tuple"
    )]
    pub async fn commit_world_reward(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        context: super::super::quest_world_callback::QuestWorldContext,
        admission: super::super::reward_claim_mint::quest::QuestRewardAdmission<'_>,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        self.sequenced(
            root_revision(root, authority),
            fence,
            Expect::Cursor(OnMismatch::RetryOnce),
            |fence| {
                root.commit_character_world_reward(
                    authority,
                    node,
                    fence,
                    request.clone(),
                    Arc::clone(&catalogue),
                    permit,
                    context.clone(),
                    admission,
                )
            },
        )
        .await
    }
}
