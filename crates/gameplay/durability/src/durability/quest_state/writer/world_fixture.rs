//! Isolated world predicates exercise the production quest writer without value grants.
use super::*;

impl DurabilityRoot {
    #[cfg(feature = "test-support")]
    #[allow(clippy::too_many_arguments, reason = "fixture retains the production authority, scope and source callback inputs")]
    pub async fn commit_fixture_world_callback<G: grizzly::GrizzlyPremiumGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        permit: &super::super::super::spell_owner_commit::SpellLanePermit,
        context: super::super::super::quest_world_callback::QuestWorldContext,
        callback: oteryn_gameplay_quests::quest::world_callbacks::Callback,
        premium: &G,
    ) -> Result<QuestTransitionOutcome> {
        self.commit_quest_inner_resolved::<_, 0>(
            authority, node, fence, request, catalogue, Some((permit, context, None)),
            None, None, None, premium, None, None, None, None, None,
            Vec::new(), vec![callback],
        ).await
    }
}
