//! Immutable test resolution of a progress-only callback, without an Item reward.
use super::*;
use crate::durability::quest_world_callback::QuestWorldContext;
use crate::durability::spell_owner_commit::SpellLanePermit;

impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "fixture retains every production authority and World context input"
    )]
    pub async fn commit_fixture_ground_callback(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        context: Option<(&SpellLanePermit, QuestWorldContext)>,
        callback: oteryn_gameplay_quests::quest::world_callbacks::Callback,
    ) -> Result<QuestTransitionOutcome> {
        self.commit_quest_inner_resolved::<_, 0>(authority,
node,
fence,
request,
catalogue,
context.map(|(permit, context)| (permit, context.into(), None)),
None,
None,
None,
&grizzly::NoGrizzlyPremium,
None,
None,
None,
None,
None,
None,
None,
None,
Vec::new(),
vec![callback],)
        .await
    }
}
