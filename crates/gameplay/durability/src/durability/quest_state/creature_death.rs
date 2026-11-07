//! Captured source eligibility is a predecessor, never a replacement for the live fence.
use super::*;

/// Index of the closed source binding and coherent quest revision frozen at death.
#[derive(Debug, Clone, Copy)]
pub struct CreatureDeathContext {
    pub callback: usize,
    pub observed_revision: u64,
}

impl DurabilityRoot {
    /// The ordinary native kill owner supplies a frozen damaging-player predecessor.
    /// The shared writer rechecks the explicit source track under the Character lock.
    #[allow(
        clippy::too_many_arguments,
        reason = "forwards the existing live fence and source predecessor to the shared quest writer"
    )]
    pub async fn commit_character_creature_death(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        predecessor: CreatureDeathContext,
    ) -> Result<QuestTransitionOutcome> {
        self.commit_quest_inner::<_, 0>(
            authority,
            node,
            fence,
            request,
            catalogue,
            None,
            None,
            None,
            None,
            &grizzly::NoGrizzlyPremium,
            None,
            None,
            None,
            None,
            Some(predecessor),
        )
        .await
    }
}
