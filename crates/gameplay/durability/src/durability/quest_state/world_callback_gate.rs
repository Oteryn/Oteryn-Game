//! Reuse the fenced world writer with the existing authoritative Premium gate.
use super::super::reward_claim_mint::quest::QuestRewardAdmission;
use super::*;

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments, reason = "existing world-writer inputs plus its transaction-bound gate")]
    pub async fn commit_character_world_callback_with_items_and_gate<G: grizzly::GrizzlyPremiumGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        context: super::super::quest_world_callback::QuestWorldContext,
        native: Option<&oteryn_gameplay_content::content::native_gameplay::NativeGameplayState>,
        admission: Option<QuestRewardAdmission<'_>>,
        gate: &G,
    ) -> Result<QuestTransitionOutcome> {
        self.commit_quest_inner::<_, 0>(
            authority,
            node,
            fence,
            request,
            catalogue,
            Some((permit, context, native)),
            None,
            None,
            None,
            gate,
            None,
            None,
            admission,
            None,
            None,
        )
        .await
    }
}
