//! World source predicates reuse the existing transaction-scoped Premium owner.
use super::*;

impl DurabilityRoot {
    /// A content-bound world callback uses the same fenced quest writer and receipt.
    #[allow(
        clippy::too_many_arguments,
        reason = "preserves the full fenced World occurrence and transaction-scoped Premium owner"
    )]
    pub async fn commit_character_world_callback_with_access<G: grizzly::GrizzlyPremiumGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        context: super::super::quest_world_callback::QuestWorldContext,
        native: Option<&oteryn_gameplay_content::content::native_gameplay::NativeGameplayState>,
        admission: Option<super::super::reward_claim_mint::QuestRewardAdmission<'_>>,
        premium: &G,
    ) -> Result<QuestTransitionOutcome> {
        self.commit_quest_inner::<_, 0>(
            authority,
            node,
            fence,
            request,
            catalogue,
            Some((permit, context.into(), native)),
            None,
            None,
            None,
            premium,
            None,
            None,
            admission,
            None,
            None,
        None, None, None,)
        .await
    }
}
