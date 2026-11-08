//! World rewards compose the existing world occurrence and complete MINT owners.
use super::*;
impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "world context and existing MINT admission share one fenced transaction"
    )]
    pub async fn commit_character_world_reward(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        context: super::super::quest_world_callback::QuestWorldContext,
        admission: super::super::reward_claim_mint::quest::QuestRewardAdmission<'_>,
    ) -> Result<QuestTransitionOutcome> {
        self.commit_quest_inner::<_, 0>(
            authority,
            node,
            fence,
            request,
            catalogue,
            Some((permit, context, Some(admission.native))),
            None,
            None,
            None,
            &super::grizzly::NoGrizzlyPremium,
            None,
            None,
            Some(admission),
            None,
            None,
        )
        .await
    }
}
