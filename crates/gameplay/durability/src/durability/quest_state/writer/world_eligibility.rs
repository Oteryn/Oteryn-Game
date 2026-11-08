//! World callbacks join the existing account Premium owner without inventing value grants.
use super::*;

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments, reason = "preserves the production world authority and Premium participants")]
    pub async fn commit_character_world_callback_with_eligibility<G: grizzly::GrizzlyPremiumGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        permit: &super::super::super::spell_owner_commit::SpellLanePermit,
        context: super::super::super::quest_world_callback::QuestWorldContext,
        native: Option<&oteryn_gameplay_content::content::native_gameplay::NativeGameplayState>,
        admission: Option<QuestRewardAdmission<'_>>,
        premium: &G,
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
            premium,
            None,
            None,
            admission,
            None,
            None,
        )
        .await
    }
}
